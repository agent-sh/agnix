//! Kiro workflow recipes (KR-WF-001..009).
//!
//! A recipe (`.kiro/workflows/*.workflow.{json,yaml,yml}`, `~/.kiro/workflows/`)
//! declares `inputs` and a tree of nodes that Kiro V3 runs as a workflow. The
//! checks mirror the V3 engine's load-time validation (`validate_workflow` and
//! the recipe loader): the zod recipe schema, the static limits (50 `step`
//! nodes, nesting depth 8, `maxIterations` at most 1000, unique node ids),
//! stop conditions, template happens-before, interactive steps inside
//! `parallel`, and the run-time watch handler rules documented at
//! https://kiro.dev/docs/workflows/authoring/. An `include` node is resolved
//! by Kiro before validation; its target cannot be read here, so references
//! that may come from an included recipe are not reported.

use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
};
use regex::Regex;
use rust_i18n::t;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;

const RULE_IDS: &[&str] = &[
    "KR-WF-001",
    "KR-WF-002",
    "KR-WF-003",
    "KR-WF-004",
    "KR-WF-005",
    "KR-WF-006",
    "KR-WF-007",
    "KR-WF-008",
    "KR-WF-009",
];

/// Static limits of the V3 engine (`maxStepNodes`, `maxNestingDepth`,
/// `maxRepeatIterations`) and the include-node cap.
const MAX_STEP_NODES: usize = 50;
const MAX_NESTING_DEPTH: usize = 8;
const MAX_REPEAT_ITERATIONS: u64 = 1000;
const MAX_INCLUDE_NODES: usize = 50;

const NODE_TYPES: &[&str] = &["step", "sequence", "repeat", "parallel", "watch", "include"];
const JOIN_POLICIES: &[&str] = &["all", "allSettled", "any"];
const ON_MAX_ITERATIONS: &[&str] = &["abort", "continue", "pause"];
const COMPLETION_SIGNALS: &[&str] = &["success", "need_input", "error"];
const INCLUDE_KEYS: &[&str] = &["type", "id", "path", "inputs", "includedFrom"];

static TEMPLATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{\s*([^{}]+?)\s*\}\}").expect("template regex"));
static INPUT_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").expect("input regex"));
static INCLUDE_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9_-]+$").expect("include id regex"));
static GENERATED_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9_-]+$").expect("generated id regex"));

pub struct KiroWorkflowValidator;

impl Validator for KiroWorkflowValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        let mut checker = Checker {
            path,
            content,
            config,
            diagnostics: Vec::new(),
        };
        checker.check_suffix_conflict();
        let Some(recipe) = checker.parse() else {
            return checker.diagnostics;
        };
        checker.check_recipe(&recipe);
        checker.diagnostics
    }
}

/// How a node sits in its container: an ordered list (root `steps`,
/// `sequence`, `repeat`) or the concurrent branches of a `parallel`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Ordered,
    Concurrent,
}

type Lineage = Vec<(Kind, usize)>;

/// Mirrors the engine's happens-before test: `producer` runs before
/// `consumer` when, at the first position their lineages differ, both are in
/// the same ordered container and the producer comes first.
fn runs_before(producer: &Lineage, consumer: &Lineage) -> bool {
    let common = producer.len().min(consumer.len());
    let mut i = 0;
    while i < common && producer[i] == consumer[i] {
        i += 1;
    }
    if i == producer.len() || i == consumer.len() || producer[i].0 == Kind::Concurrent {
        return false;
    }
    producer[i].1 < consumer[i].1
}

/// Returns true when `inner` is a descendant of the node at `outer`.
fn is_descendant(outer: &Lineage, inner: &Lineage) -> bool {
    inner.len() > outer.len() && inner[..outer.len()] == outer[..]
}

struct NodeInfo {
    id: String,
    lineage: Lineage,
}

struct StepTemplates {
    id: String,
    location: (usize, usize),
    lineage: Lineage,
    parent_kind: Kind,
    /// Ids of the earlier siblings in the same container.
    earlier_siblings: Vec<String>,
    /// The prompt, then the artifact paths.
    templates: Vec<String>,
    prompt: Option<String>,
}

struct StopContext {
    owner: String,
    location: (usize, usize),
    lineage: Lineage,
    include_descendants: bool,
    templates: Vec<String>,
}

/// A repeat's `fileCheck` stop path and the prompts and artifact paths of
/// the steps inside it.
struct RepeatFileCheck {
    repeat: String,
    location: (usize, usize),
    path: String,
    texts: Vec<String>,
}

#[derive(Default)]
struct Walk {
    ids: HashSet<String>,
    nodes: HashMap<String, NodeInfo>,
    producers: HashSet<String>,
    artifact_producers: HashMap<String, Vec<String>>,
    watch_ids: HashSet<String>,
    step_count: usize,
    include_count: usize,
    depth_reported: bool,
    steps_reported: bool,
    step_templates: Vec<StepTemplates>,
    stop_contexts: Vec<StopContext>,
    /// (repeat id, location, stopWhen watch id)
    stop_when_watches: Vec<(String, (usize, usize), String)>,
    repeat_file_checks: Vec<RepeatFileCheck>,
    /// How often each id has been seen so far, to locate duplicates.
    id_occurrences: HashMap<String, usize>,
}

struct Checker<'a, 'c> {
    path: &'a Path,
    content: &'a str,
    config: &'a PerFileLintConfig<'c>,
    diagnostics: Vec<Diagnostic>,
}

fn is_yaml_path(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.ends_with(".yaml") || name.ends_with(".yml")
}

/// A non-negative whole number (JSON `3` or `3.0`, as JavaScript reads it).
fn whole_number(value: &Value) -> Option<u64> {
    if let Some(n) = value.as_u64() {
        return Some(n);
    }
    value
        .as_f64()
        .filter(|f| *f >= 0.0 && f.fract() == 0.0 && *f <= u64::MAX as f64)
        .map(|f| f as u64)
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

impl Checker<'_, '_> {
    fn enabled(&self, rule: &str) -> bool {
        self.config.is_rule_enabled(rule)
    }

    fn push(&mut self, rule: &'static str, error: bool, location: (usize, usize), message: String) {
        if !self.enabled(rule) {
            return;
        }
        let suggestion = match rule {
            "KR-WF-001" => t!("rules.kr_wf_001.suggestion"),
            "KR-WF-002" => t!("rules.kr_wf_002.suggestion"),
            "KR-WF-003" => t!("rules.kr_wf_003.suggestion"),
            "KR-WF-004" => t!("rules.kr_wf_004.suggestion"),
            "KR-WF-005" => t!("rules.kr_wf_005.suggestion"),
            "KR-WF-006" => t!("rules.kr_wf_006.suggestion"),
            "KR-WF-007" => t!("rules.kr_wf_007.suggestion"),
            "KR-WF-008" => t!("rules.kr_wf_008.suggestion"),
            _ => t!("rules.kr_wf_009.suggestion"),
        };
        let (line, column) = location;
        let diagnostic = if error {
            Diagnostic::error(self.path.to_path_buf(), line, column, rule, message)
        } else {
            Diagnostic::warning(self.path.to_path_buf(), line, column, rule, message)
        };
        self.diagnostics
            .push(diagnostic.with_suggestion(suggestion));
    }

    /// Location of the `n`-th `id: <id>` (JSON or YAML) in the file, or 1:0.
    fn id_location(&self, id: &str, occurrence: usize) -> (usize, usize) {
        let escaped = regex::escape(id);
        let pattern = format!(
            r#"(?m)"id"\s*:\s*"{escaped}"|^[ \t-]*id\s*:\s*["']?{escaped}["']?[ \t]*(?:#.*)?$|\bid\s*:\s*["']?{escaped}["']?\s*[,}}]"#
        );
        Regex::new(&pattern)
            .ok()
            .and_then(|re| re.find_iter(self.content).nth(occurrence))
            .map(|m| {
                let offset = self.content[m.start()..]
                    .find("id")
                    .map_or(m.start(), |i| m.start() + i);
                crate::rules::line_col_at_offset(self.content, offset)
            })
            .unwrap_or((1, 0))
    }

    fn check_suffix_conflict(&mut self) {
        let Some(name) = self.path.file_name().and_then(|n| n.to_str()) else {
            return;
        };
        let stem = [".workflow.yaml", ".workflow.yml"]
            .iter()
            .find_map(|suffix| name.strip_suffix(suffix));
        let Some(stem) = stem else {
            return;
        };
        let sibling = self.path.with_file_name(format!("{stem}.workflow.json"));
        if self.config.fs().exists(&sibling) {
            self.push(
                "KR-WF-003",
                false,
                (1, 0),
                t!(
                    "rules.kr_wf_003.message",
                    name = name,
                    json = format!("{stem}.workflow.json")
                )
                .to_string(),
            );
        }
    }

    fn parse(&mut self) -> Option<Value> {
        let parsed: Result<Value, (String, usize, usize)> = if is_yaml_path(self.path) {
            serde_yaml::from_str::<Value>(self.content).map_err(|e| {
                let (line, column) = e.location().map_or((1, 0), |l| (l.line(), l.column()));
                (e.to_string(), line, column)
            })
        } else {
            serde_json::from_str::<Value>(self.content)
                .map_err(|e| (e.to_string(), e.line().max(1), e.column()))
        };
        match parsed {
            Ok(value) => Some(value),
            Err((error, line, column)) => {
                let format = if is_yaml_path(self.path) {
                    "YAML"
                } else {
                    "JSON"
                };
                self.push(
                    "KR-WF-004",
                    true,
                    (line, column),
                    t!("rules.kr_wf_004.parse", format = format, error = error).to_string(),
                );
                None
            }
        }
    }

    fn invalid(&mut self, location: (usize, usize), detail: String) {
        self.push(
            "KR-WF-004",
            true,
            location,
            t!("rules.kr_wf_004.message", detail = detail).to_string(),
        );
    }

    fn check_recipe(&mut self, recipe: &Value) {
        let Some(root) = recipe.as_object() else {
            self.invalid(
                (1, 0),
                format!("the recipe must be an object, not {}", type_name(recipe)),
            );
            return;
        };
        match root.get("name") {
            Some(Value::String(_)) => {}
            Some(other) => self.invalid(
                (1, 0),
                format!("`name` must be a string, not {}", type_name(other)),
            ),
            None => self.invalid((1, 0), "the required `name` field is missing".to_string()),
        }
        for key in ["description", "modelId", "effortLevel"] {
            if let Some(value) = root.get(key).filter(|v| !v.is_string()) {
                self.invalid(
                    (1, 0),
                    format!("`{key}` must be a string, not {}", type_name(value)),
                );
            }
        }
        if let Some(value) = root
            .get("injectOriginalUserRequest")
            .filter(|v| !v.is_boolean())
        {
            self.invalid(
                (1, 0),
                format!(
                    "`injectOriginalUserRequest` must be a boolean, not {}",
                    type_name(value)
                ),
            );
        }
        if let Some(value) = root
            .get("planRevision")
            .filter(|v| whole_number(v).is_none())
        {
            self.invalid(
                (1, 0),
                format!("`planRevision` must be a non-negative integer, not {value}"),
            );
        }
        let inputs: HashSet<String> = match root.get("inputs") {
            None => HashSet::new(),
            Some(Value::Object(map)) => {
                for (name, hint) in map {
                    if !hint.is_string() {
                        self.invalid(
                            (1, 0),
                            format!(
                                "input `{name}` must have a string type hint, not {}",
                                type_name(hint)
                            ),
                        );
                    }
                }
                map.keys().cloned().collect()
            }
            Some(other) => {
                self.invalid(
                    (1, 0),
                    format!("`inputs` must be an object, not {}", type_name(other)),
                );
                HashSet::new()
            }
        };
        let steps = match root.get("steps") {
            Some(Value::Array(steps)) => steps,
            Some(other) => {
                self.invalid(
                    (1, 0),
                    format!("`steps` must be an array, not {}", type_name(other)),
                );
                return;
            }
            None => {
                self.invalid((1, 0), "the required `steps` array is missing".to_string());
                return;
            }
        };

        let mut walk = Walk::default();
        self.walk(steps, Kind::Ordered, &Vec::new(), &mut walk);
        let has_include = walk.include_count > 0;

        if walk.include_count > MAX_INCLUDE_NODES {
            self.push(
                "KR-WF-001",
                true,
                (1, 0),
                t!(
                    "rules.kr_wf_001.includes",
                    count = walk.include_count,
                    limit = MAX_INCLUDE_NODES
                )
                .to_string(),
            );
        }

        // stopWhen must name a watch of this recipe (an included recipe may
        // declare it, so skip when there are includes).
        if !has_include {
            for (repeat, location, watch) in std::mem::take(&mut walk.stop_when_watches) {
                if !walk.watch_ids.contains(&watch) {
                    self.push(
                        "KR-WF-006",
                        true,
                        location,
                        t!("rules.kr_wf_006.unknown_watch", id = repeat, watch = watch).to_string(),
                    );
                }
            }
        }

        self.check_references(&walk, &inputs, has_include);

        for check in std::mem::take(&mut walk.repeat_file_checks) {
            let RepeatFileCheck {
                repeat,
                location,
                path: file,
                texts,
            } = check;
            if !texts.iter().any(|text| text.contains(&file)) {
                self.push(
                    "KR-WF-009",
                    false,
                    location,
                    t!("rules.kr_wf_009.message", id = repeat, path = file).to_string(),
                );
            }
        }
    }

    fn walk(&mut self, nodes: &[Value], kind: Kind, parent: &Lineage, walk: &mut Walk) {
        let mut sibling_ids: Vec<String> = Vec::new();
        for (index, node) in nodes.iter().enumerate() {
            let mut lineage = parent.clone();
            lineage.push((kind, index));
            let id = node.get("id").and_then(Value::as_str).map(str::to_string);
            self.check_node(node, kind, &lineage, &sibling_ids, walk);
            if let Some(id) = id {
                sibling_ids.push(id);
            }
        }
    }

    fn check_node(
        &mut self,
        node: &Value,
        kind: Kind,
        lineage: &Lineage,
        earlier_siblings: &[String],
        walk: &mut Walk,
    ) {
        if lineage.len() > MAX_NESTING_DEPTH {
            if !walk.depth_reported {
                walk.depth_reported = true;
                let location = node.get("id").and_then(Value::as_str).map_or((1, 0), |id| {
                    let n = walk.id_occurrences.get(id).copied().unwrap_or(0);
                    self.id_location(id, n)
                });
                self.push(
                    "KR-WF-001",
                    true,
                    location,
                    t!("rules.kr_wf_001.depth", limit = MAX_NESTING_DEPTH).to_string(),
                );
            }
            return;
        }
        let Some(map) = node.as_object() else {
            self.invalid(
                (1, 0),
                format!("a workflow node must be an object, not {}", type_name(node)),
            );
            return;
        };
        let id = match map.get("id") {
            Some(Value::String(id)) => Some(id.clone()),
            _ => None,
        };
        // The n-th node with this id is located at the n-th `id: <id>`.
        let occurrence = id.as_ref().map_or(0, |id| {
            let seen = walk.id_occurrences.entry(id.clone()).or_insert(0);
            *seen += 1;
            *seen - 1
        });
        let location = id
            .as_deref()
            .map_or((1, 0), |id| self.id_location(id, occurrence));
        let label = id.clone().unwrap_or_else(|| "<no id>".to_string());

        let node_type = match map.get("type") {
            Some(Value::String(t)) if NODE_TYPES.contains(&t.as_str()) => t.as_str(),
            Some(Value::String(t)) => {
                self.invalid(
                    location,
                    format!(
                        "node '{label}' has unknown type '{t}' (expected one of {})",
                        NODE_TYPES.join(", ")
                    ),
                );
                return;
            }
            Some(other) => {
                self.invalid(
                    location,
                    format!(
                        "node '{label}' `type` must be a string, not {}",
                        type_name(other)
                    ),
                );
                return;
            }
            None => {
                self.invalid(location, format!("node '{label}' is missing `type`"));
                return;
            }
        };

        match map.get("id") {
            Some(Value::String(id)) => {
                if node_type == "include" && !INCLUDE_ID.is_match(id) {
                    self.invalid(
                        location,
                        format!("include id '{id}' may contain only letters, digits, '_' and '-'"),
                    );
                }
                if !walk.ids.insert(id.clone()) {
                    self.push(
                        "KR-WF-001",
                        true,
                        location,
                        t!("rules.kr_wf_001.duplicate", id = id).to_string(),
                    );
                } else {
                    walk.nodes.insert(
                        id.clone(),
                        NodeInfo {
                            id: id.clone(),
                            lineage: lineage.clone(),
                        },
                    );
                }
            }
            Some(other) => self.invalid(
                location,
                format!(
                    "a {node_type} node `id` must be a string, not {}",
                    type_name(other)
                ),
            ),
            None => self.invalid(
                location,
                format!("a {node_type} node is missing the required `id`"),
            ),
        }

        match node_type {
            "step" => self.check_step(map, &label, location, kind, lineage, earlier_siblings, walk),
            "sequence" => {
                if let Some(children) = self.children(map, "steps", &label, location) {
                    self.walk(children, Kind::Ordered, lineage, walk);
                }
            }
            "repeat" => self.check_repeat(map, &label, location, lineage, walk),
            "parallel" => {
                match map.get("joinPolicy") {
                    Some(Value::String(p)) if JOIN_POLICIES.contains(&p.as_str()) => {}
                    Some(other) => self.invalid(
                        location,
                        format!(
                            "parallel '{label}' has joinPolicy {other} (expected one of {})",
                            JOIN_POLICIES.join(", ")
                        ),
                    ),
                    None => self.invalid(
                        location,
                        format!("parallel '{label}' is missing `joinPolicy`"),
                    ),
                }
                if let Some(children) = self.children(map, "branches", &label, location) {
                    self.walk(children, Kind::Concurrent, lineage, walk);
                }
            }
            "watch" => self.check_watch(map, &label, location, walk),
            "include" => self.check_include(map, &label, location, walk),
            _ => {}
        }
    }

    fn children<'v>(
        &mut self,
        map: &'v Map<String, Value>,
        key: &str,
        label: &str,
        location: (usize, usize),
    ) -> Option<&'v [Value]> {
        match map.get(key) {
            Some(Value::Array(children)) => Some(children),
            Some(other) => {
                self.invalid(
                    location,
                    format!(
                        "node '{label}' `{key}` must be an array, not {}",
                        type_name(other)
                    ),
                );
                None
            }
            None => {
                self.invalid(
                    location,
                    format!("node '{label}' is missing the required `{key}` array"),
                );
                None
            }
        }
    }

    fn require_string(
        &mut self,
        map: &Map<String, Value>,
        key: &str,
        label: &str,
        location: (usize, usize),
    ) -> Option<String> {
        match map.get(key) {
            Some(Value::String(value)) => Some(value.clone()),
            Some(other) => {
                self.invalid(
                    location,
                    format!(
                        "node '{label}' `{key}` must be a string, not {}",
                        type_name(other)
                    ),
                );
                None
            }
            None => {
                self.invalid(
                    location,
                    format!("node '{label}' is missing the required `{key}`"),
                );
                None
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check_step(
        &mut self,
        map: &Map<String, Value>,
        label: &str,
        location: (usize, usize),
        kind: Kind,
        lineage: &Lineage,
        earlier_siblings: &[String],
        walk: &mut Walk,
    ) {
        walk.step_count += 1;
        if walk.step_count > MAX_STEP_NODES && !walk.steps_reported {
            walk.steps_reported = true;
            self.push(
                "KR-WF-001",
                true,
                location,
                t!("rules.kr_wf_001.steps", limit = MAX_STEP_NODES).to_string(),
            );
        }
        self.require_string(map, "agent", label, location);
        let prompt = self.require_string(map, "prompt", label, location);
        if map.contains_key("input") {
            self.invalid(
                location,
                format!("step '{label}' uses the removed `input` field; embed the data in `prompt` with {{{{...}}}} templates"),
            );
        }
        for key in ["modelId", "effortLevel"] {
            if let Some(value) = map.get(key).filter(|v| !v.is_string()) {
                self.invalid(
                    location,
                    format!(
                        "step '{label}' `{key}` must be a string, not {}",
                        type_name(value)
                    ),
                );
            }
        }
        let capture = match map.get("captureOutput") {
            None => true,
            Some(Value::Bool(b)) => *b,
            Some(other) => {
                self.invalid(
                    location,
                    format!(
                        "step '{label}' `captureOutput` must be a boolean, not {}",
                        type_name(other)
                    ),
                );
                true
            }
        };
        let mut artifact_paths = Vec::new();
        match map.get("artifacts") {
            None => {}
            Some(Value::Object(artifacts)) => {
                for (name, value) in artifacts {
                    match value {
                        Value::String(path) => {
                            artifact_paths.push(path.clone());
                            walk.artifact_producers
                                .entry(name.clone())
                                .or_default()
                                .push(label.to_string());
                        }
                        other => self.invalid(
                            location,
                            format!(
                                "step '{label}' artifact `{name}` must be a path string, not {}",
                                type_name(other)
                            ),
                        ),
                    }
                }
            }
            Some(other) => self.invalid(
                location,
                format!(
                    "step '{label}' `artifacts` must be an object, not {}",
                    type_name(other)
                ),
            ),
        }
        if capture && map.get("id").is_some_and(Value::is_string) {
            walk.producers.insert(label.to_string());
        }
        if let Some(completion) = map.get("completion") {
            if lineage.iter().any(|(k, _)| *k == Kind::Concurrent) {
                self.push(
                    "KR-WF-005",
                    true,
                    location,
                    t!("rules.kr_wf_005.message", id = label).to_string(),
                );
            }
            if let Some(file) = self.check_stop_condition(completion, label, location) {
                walk.stop_contexts.push(StopContext {
                    owner: label.to_string(),
                    location,
                    lineage: lineage.clone(),
                    include_descendants: false,
                    templates: vec![file],
                });
            }
        }
        let mut templates: Vec<String> = prompt.iter().cloned().collect();
        templates.extend(artifact_paths);
        walk.step_templates.push(StepTemplates {
            id: label.to_string(),
            location,
            lineage: lineage.clone(),
            parent_kind: kind,
            earlier_siblings: earlier_siblings.to_vec(),
            templates,
            prompt,
        });
    }

    fn check_repeat(
        &mut self,
        map: &Map<String, Value>,
        label: &str,
        location: (usize, usize),
        lineage: &Lineage,
        walk: &mut Walk,
    ) {
        match map.get("maxIterations") {
            None => self.push(
                "KR-WF-002",
                true,
                location,
                t!("rules.kr_wf_002.missing_max", id = label).to_string(),
            ),
            Some(value) => match whole_number(value) {
                Some(0) => self.push(
                    "KR-WF-002",
                    true,
                    location,
                    t!(
                        "rules.kr_wf_002.invalid_max",
                        id = label,
                        value = value.to_string()
                    )
                    .to_string(),
                ),
                Some(n) if n > MAX_REPEAT_ITERATIONS => self.push(
                    "KR-WF-002",
                    true,
                    location,
                    t!(
                        "rules.kr_wf_002.too_many",
                        id = label,
                        value = n,
                        limit = MAX_REPEAT_ITERATIONS
                    )
                    .to_string(),
                ),
                Some(_) => {}
                None => self.push(
                    "KR-WF-002",
                    true,
                    location,
                    t!(
                        "rules.kr_wf_002.invalid_max",
                        id = label,
                        value = value.to_string()
                    )
                    .to_string(),
                ),
            },
        }
        match map.get("onMaxIterations") {
            Some(Value::String(v)) if ON_MAX_ITERATIONS.contains(&v.as_str()) => {}
            Some(other) => self.push(
                "KR-WF-002",
                true,
                location,
                t!(
                    "rules.kr_wf_002.invalid_cap",
                    id = label,
                    value = other.to_string()
                )
                .to_string(),
            ),
            None => self.push(
                "KR-WF-002",
                true,
                location,
                t!("rules.kr_wf_002.missing_cap", id = label).to_string(),
            ),
        }
        if map.contains_key("stopCondition") && map.contains_key("stopWhen") {
            self.push(
                "KR-WF-002",
                true,
                location,
                t!("rules.kr_wf_002.both_stops", id = label).to_string(),
            );
        }

        let mut context_templates = Vec::new();
        if let Some(condition) = map.get("stopCondition")
            && let Some(file) = self.check_stop_condition(condition, label, location)
        {
            context_templates.push(file);
        }
        match map.get("stopWhen") {
            None => {}
            Some(Value::String(expr)) => match parse_stop_when(expr) {
                Ok(StopWhen::WatchTerminal(watch)) => {
                    walk.stop_when_watches
                        .push((label.to_string(), location, watch));
                }
                Ok(StopWhen::Contains(template)) => context_templates.push(template),
                Err(detail) => self.push(
                    "KR-WF-006",
                    true,
                    location,
                    t!(
                        "rules.kr_wf_006.stop_when",
                        id = label,
                        value = expr,
                        detail = detail
                    )
                    .to_string(),
                ),
            },
            Some(other) => self.invalid(
                location,
                format!(
                    "repeat '{label}' `stopWhen` must be a string, not {}",
                    type_name(other)
                ),
            ),
        }
        if !context_templates.is_empty() {
            walk.stop_contexts.push(StopContext {
                owner: label.to_string(),
                location,
                lineage: lineage.clone(),
                include_descendants: true,
                templates: context_templates,
            });
        }

        if let Some(children) = self.children(map, "steps", label, location) {
            let first_step = walk.step_templates.len();
            self.walk(children, Kind::Ordered, lineage, walk);
            if let Some(file) = map
                .get("stopCondition")
                .and_then(|c| c.get("fileCheck"))
                .and_then(|f| f.get("path"))
                .and_then(Value::as_str)
            {
                let texts = walk.step_templates[first_step..]
                    .iter()
                    .flat_map(|step| step.templates.iter().cloned())
                    .collect();
                walk.repeat_file_checks.push(RepeatFileCheck {
                    repeat: label.to_string(),
                    location,
                    path: file.to_string(),
                    texts,
                });
            }
        }
    }

    /// Validates a `stopCondition` / `completion` object; returns its
    /// `fileCheck.path` for template checks.
    fn check_stop_condition(
        &mut self,
        condition: &Value,
        label: &str,
        location: (usize, usize),
    ) -> Option<String> {
        let Some(map) = condition.as_object() else {
            self.push(
                "KR-WF-006",
                true,
                location,
                t!("rules.kr_wf_006.not_object", id = label).to_string(),
            );
            return None;
        };
        let has_any = ["containsText", "fileCheck", "completionSignal"]
            .iter()
            .any(|key| map.get(*key).is_some_and(|v| !v.is_null()));
        if !has_any {
            self.push(
                "KR-WF-006",
                true,
                location,
                t!("rules.kr_wf_006.empty", id = label).to_string(),
            );
        }
        if let Some(value) = map.get("containsText").filter(|v| !v.is_string()) {
            self.invalid(
                location,
                format!(
                    "node '{label}' `containsText` must be a string, not {}",
                    type_name(value)
                ),
            );
        }
        if let Some(signal) = map.get("completionSignal") {
            let valid = signal
                .as_str()
                .is_some_and(|s| COMPLETION_SIGNALS.contains(&s));
            if !valid {
                self.push(
                    "KR-WF-006",
                    true,
                    location,
                    t!(
                        "rules.kr_wf_006.signal",
                        id = label,
                        value = signal.to_string()
                    )
                    .to_string(),
                );
            }
        }
        let file_check = map.get("fileCheck")?;
        let Some(check) = file_check.as_object() else {
            self.push(
                "KR-WF-006",
                true,
                location,
                t!(
                    "rules.kr_wf_006.file_check",
                    id = label,
                    detail = "fileCheck must be an object"
                )
                .to_string(),
            );
            return None;
        };
        let mut problems: Vec<String> = Vec::new();
        let unknown: Vec<&str> = check
            .keys()
            .map(String::as_str)
            .filter(|k| !["path", "jsonPath", "value"].contains(k))
            .collect();
        if !unknown.is_empty() {
            problems.push(format!(
                "unrecognized key(s) {}; allowed keys are path, jsonPath and value",
                unknown.join(", ")
            ));
        }
        let path = check
            .get("path")
            .and_then(Value::as_str)
            .map(str::to_string);
        if path.is_none() {
            problems.push("`path` must be a string".to_string());
        }
        match check.get("jsonPath") {
            Some(Value::String(json_path)) => {
                if let Some(token) = unsupported_json_path(json_path) {
                    problems.push(format!(
                        "jsonPath '{json_path}' uses unsupported '{token}'; use a dotted key path such as 'verdict' or '$.verdict'"
                    ));
                }
            }
            _ => problems.push("`jsonPath` must be a string".to_string()),
        }
        match check.get("value") {
            None => {
                problems.push("`value` is required (the comparison field is named value)".to_string());
            }
            Some(Value::Array(items)) if items.is_empty() => problems.push(
                "`value` must not be an empty array: an array is the any-of form; wrap a literal array as [[]]".to_string(),
            ),
            _ => {}
        }
        for detail in problems {
            self.push(
                "KR-WF-006",
                true,
                location,
                t!("rules.kr_wf_006.file_check", id = label, detail = detail).to_string(),
            );
        }
        path
    }

    fn check_watch(
        &mut self,
        map: &Map<String, Value>,
        label: &str,
        location: (usize, usize),
        walk: &mut Walk,
    ) {
        let handler = self.require_string(map, "handler", label, location);
        if map.get("id").is_some_and(Value::is_string) {
            walk.watch_ids.insert(label.to_string());
            walk.producers.insert(label.to_string());
        }
        if let Some(value) = map.get("idleTimeoutSec")
            && !value.as_f64().is_some_and(|n| n > 0.0)
        {
            self.invalid(
                location,
                format!("watch '{label}' `idleTimeoutSec` must be a positive number, not {value}"),
            );
        }
        let config = match map.get("config") {
            None => None,
            Some(Value::Object(config)) => Some(config),
            Some(other) => {
                self.invalid(
                    location,
                    format!(
                        "watch '{label}' `config` must be an object, not {}",
                        type_name(other)
                    ),
                );
                return;
            }
        };
        let mut problems: Vec<String> = Vec::new();
        let poll_minimum = match handler.as_deref() {
            Some("command") => {
                match config.and_then(|c| c.get("command")) {
                    Some(Value::String(command)) => {
                        if command.contains("{{") {
                            problems.push("`command` must be one static command line; a {{...}} template in it is rejected".to_string());
                        }
                    }
                    Some(other) => problems.push(format!(
                        "`command` must be a string, not {}",
                        type_name(other)
                    )),
                    None => {
                        problems.push("the command handler requires `config.command`".to_string())
                    }
                }
                if config.is_some_and(|c| c.contains_key("args")) {
                    problems.push(
                        "there is no `args` key; put fixed arguments inside `command`".to_string(),
                    );
                }
                if let Some(timeout) = config.and_then(|c| c.get("commandTimeoutSec"))
                    && !timeout.as_f64().is_some_and(|n| n > 0.0)
                {
                    problems.push(format!(
                        "`commandTimeoutSec` must be a positive number, not {timeout}"
                    ));
                }
                Some(10.0)
            }
            Some("github-pr") => {
                let has_ref = config.is_some_and(|c| {
                    c.get("prRef").is_some_and(Value::is_string)
                        || c.get("url").is_some_and(Value::is_string)
                });
                if !has_ref {
                    problems.push(
                        "the github-pr handler requires `config.prRef` or `config.url`".to_string(),
                    );
                }
                Some(30.0)
            }
            // Other handlers are checked by Kiro when the run is created.
            _ => None,
        };
        if let (Some(minimum), Some(poll)) =
            (poll_minimum, config.and_then(|c| c.get("pollIntervalSec")))
            && !poll.as_f64().is_some_and(|n| n >= minimum)
        {
            problems.push(format!(
                "`pollIntervalSec` is {poll}; the {} handler polls at most every {minimum} seconds",
                handler.as_deref().unwrap_or("")
            ));
        }
        for detail in problems {
            self.push(
                "KR-WF-008",
                true,
                location,
                t!("rules.kr_wf_008.message", id = label, detail = detail).to_string(),
            );
        }
    }

    fn check_include(
        &mut self,
        map: &Map<String, Value>,
        label: &str,
        location: (usize, usize),
        walk: &mut Walk,
    ) {
        walk.include_count += 1;
        let unknown: Vec<&str> = map
            .keys()
            .map(String::as_str)
            .filter(|k| !INCLUDE_KEYS.contains(k))
            .collect();
        if !unknown.is_empty() {
            self.invalid(
                location,
                format!(
                    "include '{label}' has unrecognized key(s) {}",
                    unknown.join(", ")
                ),
            );
        }
        if let Some(path) = self.require_string(map, "path", label, location)
            && let Some(problem) = include_path_problem(&path)
        {
            self.invalid(location, format!("include '{label}': {problem}"));
        }
        match map.get("inputs") {
            None => {}
            Some(Value::Object(inputs)) => {
                if inputs.values().any(|v| !v.is_string()) {
                    self.invalid(
                        location,
                        format!("include '{label}' input values must be strings"),
                    );
                }
            }
            Some(other) => self.invalid(
                location,
                format!(
                    "include '{label}' `inputs` must be an object, not {}",
                    type_name(other)
                ),
            ),
        }
    }

    fn check_references(&mut self, walk: &Walk, inputs: &HashSet<String>, has_include: bool) {
        for step in &walk.step_templates {
            let location = step.location;
            let mut seen = HashSet::new();
            for template in &step.templates {
                for expr in template_expressions(template) {
                    if !seen.insert(expr.clone()) {
                        continue;
                    }
                    let reference = parse_reference(&expr);
                    let problem = match reference {
                        Reference::Bare(_) => None,
                        Reference::Previous => {
                            if step.parent_kind == Kind::Concurrent {
                                Some(
                                    t!(
                                        "rules.kr_wf_007.previous_parallel",
                                        id = step.id,
                                        template = format!("{{{{{expr}}}}}")
                                    )
                                    .to_string(),
                                )
                            } else if !step
                                .earlier_siblings
                                .iter()
                                .any(|s| walk.producers.contains(s))
                            {
                                Some(
                                    t!(
                                        "rules.kr_wf_007.previous_first",
                                        id = step.id,
                                        template = format!("{{{{{expr}}}}}")
                                    )
                                    .to_string(),
                                )
                            } else {
                                None
                            }
                        }
                        Reference::Output(ref id) => self.output_problem(
                            walk,
                            &step.id,
                            &step.lineage,
                            id,
                            &expr,
                            has_include,
                            None,
                        ),
                        Reference::Artifact(ref name) => self.artifact_problem(
                            walk,
                            &step.id,
                            &step.lineage,
                            name,
                            &expr,
                            has_include,
                            None,
                        ),
                    };
                    if let Some(message) = problem {
                        self.push("KR-WF-007", true, location, message);
                    }
                }
            }
            // Undeclared bare inputs stay literal; Kiro warns (prompts only).
            if let Some(prompt) = &step.prompt {
                let mut warned = HashSet::new();
                for expr in template_expressions(prompt) {
                    if let Reference::Bare(name) = parse_reference(&expr)
                        && name != "previous"
                        && INPUT_NAME.is_match(&name)
                        && !inputs.contains(&name)
                        && !has_include
                        && warned.insert(name.clone())
                    {
                        self.push(
                            "KR-WF-007",
                            false,
                            location,
                            t!(
                                "rules.kr_wf_007.undeclared_input",
                                id = step.id,
                                name = name
                            )
                            .to_string(),
                        );
                    }
                }
            }
        }
        for context in &walk.stop_contexts {
            let location = context.location;
            let mut seen = HashSet::new();
            for template in &context.templates {
                for expr in template_expressions(template) {
                    if !seen.insert(expr.clone()) {
                        continue;
                    }
                    let problem = match parse_reference(&expr) {
                        Reference::Bare(_) => None,
                        Reference::Previous => Some(
                            t!(
                                "rules.kr_wf_007.previous_stop",
                                id = context.owner,
                                template = format!("{{{{{expr}}}}}")
                            )
                            .to_string(),
                        ),
                        Reference::Output(ref id) => self.output_problem(
                            walk,
                            &context.owner,
                            &context.lineage,
                            id,
                            &expr,
                            has_include,
                            Some(context.include_descendants),
                        ),
                        Reference::Artifact(ref name) => self.artifact_problem(
                            walk,
                            &context.owner,
                            &context.lineage,
                            name,
                            &expr,
                            has_include,
                            Some(context.include_descendants),
                        ),
                    };
                    if let Some(message) = problem {
                        self.push("KR-WF-007", true, location, message);
                    }
                }
            }
        }
    }

    /// `stop` is `Some(include_descendants)` for stop/completion templates.
    #[allow(clippy::too_many_arguments)]
    fn output_problem(
        &self,
        walk: &Walk,
        owner: &str,
        lineage: &Lineage,
        id: &str,
        expr: &str,
        has_include: bool,
        stop: Option<bool>,
    ) -> Option<String> {
        let template = format!("{{{{{expr}}}}}");
        let Some(node) = walk.nodes.get(id) else {
            return (!has_include).then(|| {
                t!(
                    "rules.kr_wf_007.unknown_node",
                    id = owner,
                    template = template,
                    node = id
                )
                .to_string()
            });
        };
        if !walk.producers.contains(id) {
            return Some(
                t!(
                    "rules.kr_wf_007.no_output",
                    id = owner,
                    template = template,
                    node = id
                )
                .to_string(),
            );
        }
        let before = match stop {
            None => runs_before(&node.lineage, lineage),
            Some(include_descendants) => {
                node.id == owner
                    || runs_before(&node.lineage, lineage)
                    || (include_descendants && is_descendant(lineage, &node.lineage))
            }
        };
        (!before).then(|| {
            t!(
                "rules.kr_wf_007.not_before",
                id = owner,
                template = template,
                node = id
            )
            .to_string()
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn artifact_problem(
        &self,
        walk: &Walk,
        owner: &str,
        lineage: &Lineage,
        name: &str,
        expr: &str,
        has_include: bool,
        stop: Option<bool>,
    ) -> Option<String> {
        let template = format!("{{{{{expr}}}}}");
        let Some(producers) = walk.artifact_producers.get(name) else {
            return (!has_include).then(|| {
                t!(
                    "rules.kr_wf_007.unknown_artifact",
                    id = owner,
                    template = template,
                    name = name
                )
                .to_string()
            });
        };
        let before = producers.iter().any(|producer| {
            if stop.is_some() && producer == owner {
                return true;
            }
            walk.nodes.get(producer).is_some_and(|node| match stop {
                None => runs_before(&node.lineage, lineage),
                Some(include_descendants) => {
                    runs_before(&node.lineage, lineage)
                        || (include_descendants && is_descendant(lineage, &node.lineage))
                }
            })
        });
        (!before).then(|| {
            t!(
                "rules.kr_wf_007.artifact_not_before",
                id = owner,
                template = template,
                name = name,
                producers = producers.join(", ")
            )
            .to_string()
        })
    }
}

enum Reference {
    Previous,
    Output(String),
    Artifact(String),
    Bare(String),
}

/// The engine's template forms: `previous.output`, `steps.<id>.output`,
/// `artifacts.<name>`, `<id>.output`, and a bare input name.
fn parse_reference(expr: &str) -> Reference {
    if expr == "previous.output" {
        Reference::Previous
    } else if let Some(id) = expr
        .strip_prefix("steps.")
        .and_then(|rest| rest.strip_suffix(".output"))
    {
        Reference::Output(id.to_string())
    } else if let Some(name) = expr.strip_prefix("artifacts.") {
        Reference::Artifact(name.to_string())
    } else if let Some(id) = expr.strip_suffix(".output") {
        Reference::Output(id.to_string())
    } else {
        Reference::Bare(expr.to_string())
    }
}

fn template_expressions(text: &str) -> Vec<String> {
    TEMPLATE
        .captures_iter(text)
        .filter_map(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
        .collect()
}

enum StopWhen {
    WatchTerminal(String),
    Contains(String),
}

/// The engine's `stopWhen` grammar: `<watchId>.terminal` or
/// `{{expr}} contains <text>`.
fn parse_stop_when(expr: &str) -> Result<StopWhen, String> {
    const CONTAINS: &str = " contains ";
    const TERMINAL: &str = ".terminal";
    if expr.starts_with("{{") {
        let Some(end) = expr.find("}}") else {
            return Err("the '{{' template is never closed with '}}'".to_string());
        };
        let template = &expr[..end + 2];
        let rest = &expr[end + 2..];
        let Some(literal) = rest.strip_prefix(CONTAINS) else {
            return Err(format!(
                "expected ' contains ' (single space on each side) immediately after '{template}'"
            ));
        };
        if literal.is_empty() {
            return Err("the text after ' contains ' is empty".to_string());
        }
        return Ok(StopWhen::Contains(template.to_string()));
    }
    if expr.contains(CONTAINS) {
        return Err("the left side of ' contains ' must be a '{{...}}' template".to_string());
    }
    if expr.contains("{{") || expr.contains("}}") {
        return Err(
            "a '{{...}}' template is only allowed at the start, as '{{expr}} contains <text>'"
                .to_string(),
        );
    }
    let Some(watch) = expr.strip_suffix(TERMINAL) else {
        return Err("expected '<watchId>.terminal' or '{{expr}} contains <text>'".to_string());
    };
    if watch.is_empty() || watch.contains('.') || watch.chars().any(char::is_whitespace) {
        return Err(format!(
            "the watch id before '.terminal' must be a single identifier without dots or whitespace (got '{watch}')"
        ));
    }
    Ok(StopWhen::WatchTerminal(watch.to_string()))
}

/// The engine's `fileCheck.jsonPath` support: dotted keys, an optional
/// leading `$`, no `..`, `[`, `]` or `*`.
fn unsupported_json_path(json_path: &str) -> Option<&'static str> {
    let segments: Vec<&str> = json_path.split('.').collect();
    let last = segments.len().saturating_sub(1);
    for (index, segment) in segments.iter().enumerate() {
        if segment.is_empty() {
            if index > 0 && index < last {
                return Some("..");
            }
            continue;
        }
        if index == 0 && *segment == "$" {
            continue;
        }
        for (ch, token) in [('[', "["), (']', "]"), ('*', "*")] {
            if segment.contains(ch) {
                return Some(token);
            }
        }
        if *segment == "$" {
            return Some("$");
        }
    }
    None
}

/// Include paths must be absolute, `bundled://<name>` or
/// `generated://<id>`; relative paths and bare names are rejected.
fn include_path_problem(path: &str) -> Option<String> {
    if path.is_empty() {
        return Some("`path` must not be empty".to_string());
    }
    if let Some(name) = path.strip_prefix("bundled://") {
        return name
            .is_empty()
            .then(|| "the bundled recipe URI is missing a name".to_string());
    }
    if let Some(id) = path.strip_prefix("generated://") {
        return (!GENERATED_ID.is_match(id))
            .then(|| format!("invalid generated workflow id '{id}'"));
    }
    let has_scheme = path.split_once("://").is_some_and(|(scheme, _)| {
        !scheme.is_empty()
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
    });
    if has_scheme {
        return Some(format!(
            "unsupported include path '{path}': use an absolute recipe file path, 'bundled://<name>' or 'generated://<id>'"
        ));
    }
    let absolute = path.starts_with('/')
        || path.starts_with('\\')
        || (path.len() > 2
            && path.as_bytes()[1] == b':'
            && path.as_bytes()[0].is_ascii_alphabetic());
    (!absolute).then(|| {
        format!(
            "include path must be absolute, 'bundled://<name>' or 'generated://<id>'; relative paths and bare recipe names are not supported (got '{path}')"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate(name: &str, content: &str) -> Vec<Diagnostic> {
        let config = LintConfig::default();
        let path = Path::new(name);
        KiroWorkflowValidator.validate_per_file(path, content, &config.for_path(path))
    }

    fn rules(diagnostics: &[Diagnostic]) -> Vec<&str> {
        diagnostics.iter().map(|d| d.rule.as_str()).collect()
    }

    fn json(name: &str, content: serde_json::Value) -> Vec<Diagnostic> {
        validate(
            &format!(".kiro/workflows/{name}.workflow.json"),
            &content.to_string(),
        )
    }

    fn step(id: &str, prompt: &str) -> serde_json::Value {
        serde_json::json!({"type": "step", "id": id, "agent": "wf-coder", "prompt": prompt})
    }

    const DOC_EXAMPLES: &[(&str, &str)] = &[
        // kiro.dev/docs/workflows/authoring: two-step recipe.
        (
            "plan-and-implement.workflow.json",
            r#"{
  "name": "plan-and-implement",
  "description": "Plan a change, then implement it",
  "inputs": { "task": "prompt" },
  "steps": [
    { "type": "step", "id": "plan", "agent": "wf-planner",
      "prompt": "Create an implementation plan for: {{task}}. Do not edit files." },
    { "type": "step", "id": "implement", "agent": "wf-coder",
      "prompt": "Implement this plan, then run relevant tests:\n\n{{previous.output}}" }
  ]
}"#,
        ),
        // kiro.dev/docs/workflows/authoring: command watch inside a repeat.
        (
            "mock-review-watch.workflow.json",
            r#"{
  "name": "mock-review-watch",
  "steps": [
    { "type": "repeat", "id": "review-loop", "maxIterations": 5, "onMaxIterations": "pause",
      "stopWhen": "review.terminal",
      "steps": [
        { "type": "watch", "id": "review", "handler": "command",
          "config": { "command": "node watch-review.mjs", "reviewFile": "mock-review.json",
                      "pollIntervalSec": 10, "commandTimeoutSec": 5 } },
        { "type": "step", "id": "respond-to-review", "agent": "wf-coder",
          "prompt": "Summarize this mock review without changing files.\n\n{{review.output}}" }
      ] }
  ]
}"#,
        ),
        // kiro.dev/docs/workflows/patterns: troubleshoot until verified.
        (
            "troubleshoot-until-verified.workflow.yaml",
            "name: troubleshoot-until-verified\ninputs:\n  issue: prompt\n  result_path: file\nsteps:\n  - type: repeat\n    id: fix-loop\n    maxIterations: 10\n    onMaxIterations: pause\n    stopCondition:\n      fileCheck:\n        path: \"{{result_path}}\"\n        jsonPath: verified\n        value: true\n    steps:\n      - type: step\n        id: diagnose-and-fix\n        agent: wf-coder\n        prompt: >\n          Reproduce {{issue}}. Inspect the latest result at {{result_path}} if it exists.\n          Test one hypothesis, apply the smallest justified change, rerun the reproduction,\n          and write JSON with verified, evidence, and remaining_risk fields.\n",
        ),
        // kiro.dev/docs/workflows/patterns: publish and respond.
        (
            "publish-and-respond.workflow.yml",
            "name: publish-and-respond\ninputs:\n  branch: string\n  run_dir: string\nsteps:\n  - type: step\n    id: submit\n    agent: wf-pr-submitter\n    prompt: >\n      Publish {{branch}} as a pull request, or reuse its open PR.\n      Write its URL and metadata to {{run_dir}}/pr.json.\n  - type: repeat\n    id: pr-loop\n    maxIterations: 200\n    onMaxIterations: pause\n    stopWhen: wait.terminal\n    steps:\n      - type: watch\n        id: wait\n        handler: github-pr\n        config:\n          prRef: \"{{run_dir}}/pr.json\"\n      - type: step\n        id: respond\n        agent: wf-pr-responder\n        prompt: >\n          Respond to the new PR activity in {{wait.output}}.\n          Address feedback that is safe and in scope.\n",
        ),
        // kiro.dev/docs/workflows/patterns: investigate a question.
        (
            "investigate-question.workflow.yaml",
            "name: investigate-question\ninputs:\n  brief: prompt\n  report_path: file\nsteps:\n  - type: step\n    id: investigate\n    agent: wf-planner\n    artifacts:\n      report: \"{{report_path}}\"\n    prompt: >\n      READ-ONLY investigation; do not edit code or commit.\n      Investigate {{brief}} and write findings to {{report_path}}.\n",
        ),
    ];

    #[test]
    fn documented_recipes_are_valid() {
        for (name, content) in DOC_EXAMPLES {
            let diagnostics = validate(&format!(".kiro/workflows/{name}"), content);
            assert!(diagnostics.is_empty(), "{name}: {diagnostics:#?}");
        }
    }

    #[test]
    fn deliver_feature_pattern_is_valid() {
        // kiro.dev/docs/workflows/patterns "Deliver a feature" (abridged
        // prompts, same structure: repeats with fileCheck, a parallel review,
        // artifacts referenced across loops).
        let content = r#"
name: deliver-feature
inputs: {task: prompt, worktree_path: string, run_id: string}
steps:
  - {type: step, id: setup, agent: wf-coder, prompt: "Prepare {{worktree_path}} for {{run_id}}"}
  - type: step
    id: requirements
    agent: wf-design
    artifacts: {requirements: "{{worktree_path}}/r.md"}
    prompt: "Write requirements for {{task}} to {{worktree_path}}/r.md"
  - type: repeat
    id: code-loop
    maxIterations: 3
    onMaxIterations: abort
    stopCondition: {fileCheck: {path: "{{worktree_path}}/code-review.json", jsonPath: verdict, value: APPROVED}}
    steps:
      - type: step
        id: implement
        agent: wf-coder
        artifacts: {code_summary: "{{worktree_path}}/summary.md"}
        prompt: "Implement {{artifacts.requirements}} in {{worktree_path}}"
      - type: parallel
        id: code-reviews
        joinPolicy: all
        branches:
          - {type: step, id: review-a, agent: semantic_reviewer, artifacts: {review_a: "{{worktree_path}}/a.md"}, prompt: "Review {{artifacts.code_summary}}"}
          - {type: step, id: review-b, agent: semantic_reviewer, artifacts: {review_b: "{{worktree_path}}/b.md"}, prompt: "Review {{artifacts.code_summary}}"}
      - type: step
        id: code-aggregate
        agent: wf-review-aggregator
        artifacts: {code_review: "{{worktree_path}}/code-review.json"}
        prompt: "Merge {{artifacts.review_a}} and {{artifacts.review_b}} into {{worktree_path}}/code-review.json"
"#;
        let diagnostics = validate(".kiro/workflows/deliver-feature.workflow.yaml", content);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }

    #[test]
    fn kr_wf_001_limits_and_duplicate_ids() {
        let steps: Vec<_> = (0..51).map(|i| step(&format!("s{i}"), "x")).collect();
        let diagnostics = json("many", serde_json::json!({"name": "many", "steps": steps}));
        assert_eq!(rules(&diagnostics), vec!["KR-WF-001"]);
        assert!(diagnostics[0].message.contains("50"));

        let mut node = step("leaf", "x");
        for depth in 0..8 {
            node = serde_json::json!({"type": "sequence", "id": format!("seq{depth}"), "steps": [node]});
        }
        let diagnostics = json("deep", serde_json::json!({"name": "deep", "steps": [node]}));
        assert_eq!(rules(&diagnostics), vec!["KR-WF-001"]);
        assert!(diagnostics[0].message.contains('8'));

        // Exactly 50 steps and depth 8 are fine.
        let steps: Vec<_> = (0..50).map(|i| step(&format!("s{i}"), "x")).collect();
        assert!(json("ok", serde_json::json!({"name": "ok", "steps": steps})).is_empty());
        let mut node = step("leaf", "x");
        for depth in 0..7 {
            node = serde_json::json!({"type": "sequence", "id": format!("seq{depth}"), "steps": [node]});
        }
        assert!(
            json(
                "ok-depth",
                serde_json::json!({"name": "ok", "steps": [node]})
            )
            .is_empty()
        );

        let content = "{\n  \"name\": \"dup\",\n  \"steps\": [\n    {\"type\": \"step\", \"id\": \"a\", \"agent\": \"x\", \"prompt\": \"p\"},\n    {\"type\": \"step\", \"id\": \"a\", \"agent\": \"x\", \"prompt\": \"p\"}\n  ]\n}";
        let diagnostics = validate(".kiro/workflows/dup.workflow.json", content);
        assert_eq!(rules(&diagnostics), vec!["KR-WF-001"]);
        // Points at the second `id`.
        assert_eq!(diagnostics[0].line, 5);
    }

    #[test]
    fn kr_wf_002_repeat_caps() {
        let body = [step("a", "x")];
        for (repeat, expected) in [
            (
                serde_json::json!({"type": "repeat", "id": "r", "onMaxIterations": "pause", "steps": body}),
                "maxIterations",
            ),
            (
                serde_json::json!({"type": "repeat", "id": "r", "maxIterations": 0, "onMaxIterations": "pause", "steps": body}),
                "0",
            ),
            (
                serde_json::json!({"type": "repeat", "id": "r", "maxIterations": 2.5, "onMaxIterations": "pause", "steps": body}),
                "2.5",
            ),
            (
                serde_json::json!({"type": "repeat", "id": "r", "maxIterations": 1001, "onMaxIterations": "pause", "steps": body}),
                "1000",
            ),
            (
                serde_json::json!({"type": "repeat", "id": "r", "maxIterations": 3, "steps": body}),
                "onMaxIterations",
            ),
            (
                serde_json::json!({"type": "repeat", "id": "r", "maxIterations": 3, "onMaxIterations": "stop", "steps": body}),
                "stop",
            ),
            (
                serde_json::json!({"type": "repeat", "id": "r", "maxIterations": 3, "onMaxIterations": "pause",
                "stopCondition": {"containsText": "DONE"}, "stopWhen": "w.terminal", "steps": body}),
                "stopWhen",
            ),
        ] {
            let diagnostics = json("r", serde_json::json!({"name": "r", "steps": [repeat]}));
            let wf002: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "KR-WF-002")
                .collect();
            assert_eq!(wf002.len(), 1, "{diagnostics:#?}");
            assert!(wf002[0].message.contains(expected), "{}", wf002[0].message);
            assert_eq!(wf002[0].level, DiagnosticLevel::Error);
        }
        // No stop condition at all is allowed: it runs to maxIterations.
        let repeat = serde_json::json!({"type": "repeat", "id": "r", "maxIterations": 3, "onMaxIterations": "continue", "steps": body});
        assert!(json("r", serde_json::json!({"name": "r", "steps": [repeat]})).is_empty());
    }

    #[test]
    fn kr_wf_003_yaml_shadowed_by_json() {
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path().join(".kiro/workflows");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plan.workflow.json"), "{}").unwrap();
        let yaml = dir.join("plan.workflow.yaml");
        let content = "name: plan\nsteps: []\n";
        std::fs::write(&yaml, content).unwrap();
        let config = LintConfig::default();
        let diagnostics =
            KiroWorkflowValidator.validate_per_file(&yaml, content, &config.for_path(&yaml));
        assert_eq!(rules(&diagnostics), vec!["KR-WF-003"]);
        assert_eq!(diagnostics[0].level, DiagnosticLevel::Warning);
        // The JSON file itself is the one Kiro loads.
        let json_path = dir.join("plan.workflow.json");
        let content = r#"{"name": "plan", "steps": []}"#;
        assert!(
            KiroWorkflowValidator
                .validate_per_file(&json_path, content, &config.for_path(&json_path))
                .is_empty()
        );
    }

    #[test]
    fn kr_wf_004_schema_errors() {
        for (content, expected) in [
            ("{", "JSON"),
            ("[]", "object"),
            (r#"{"steps": []}"#, "name"),
            (r#"{"name": "x"}"#, "steps"),
            (r#"{"name": "x", "steps": {}}"#, "array"),
            (r#"{"name": "x", "steps": [{"id": "a"}]}"#, "type"),
            (
                r#"{"name": "x", "steps": [{"type": "task", "id": "a"}]}"#,
                "unknown type",
            ),
            (
                r#"{"name": "x", "steps": [{"type": "step", "id": "a", "prompt": "p"}]}"#,
                "agent",
            ),
            (
                r#"{"name": "x", "steps": [{"type": "step", "id": "a", "agent": "x"}]}"#,
                "prompt",
            ),
            (
                r#"{"name": "x", "steps": [{"type": "step", "agent": "x", "prompt": "p"}]}"#,
                "id",
            ),
            (
                r#"{"name": "x", "steps": [{"type": "step", "id": "a", "agent": "x", "prompt": "p", "input": "d"}]}"#,
                "removed",
            ),
            (
                r#"{"name": "x", "steps": [{"type": "parallel", "id": "p", "branches": []}]}"#,
                "joinPolicy",
            ),
            (
                r#"{"name": "x", "steps": [{"type": "parallel", "id": "p", "joinPolicy": "first", "branches": []}]}"#,
                "joinPolicy",
            ),
            (
                r#"{"name": "x", "inputs": {"task": 1}, "steps": []}"#,
                "type hint",
            ),
            (
                r#"{"name": "x", "steps": [{"type": "include", "id": "i", "path": "review.workflow.json"}]}"#,
                "relative",
            ),
            (
                r#"{"name": "x", "steps": [{"type": "include", "id": "i", "path": "bundled://goal", "extra": 1}]}"#,
                "unrecognized",
            ),
        ] {
            let diagnostics = validate(".kiro/workflows/x.workflow.json", content);
            assert!(
                diagnostics
                    .iter()
                    .any(|d| d.rule == "KR-WF-004" && d.message.contains(expected)),
                "{content}: {diagnostics:#?}"
            );
        }
        // YAML syntax errors carry the parser's location.
        let diagnostics = validate(".kiro/workflows/x.workflow.yaml", "name: x\nsteps: [\n");
        assert_eq!(rules(&diagnostics), vec!["KR-WF-004"]);
        assert!(diagnostics[0].message.contains("YAML"));
        // Includes of bundled, generated and absolute recipes are accepted.
        for path in [
            "bundled://goal",
            "generated://gen_0123456789abcdef",
            "/repo/.kiro/workflows/review.workflow.json",
        ] {
            let content = serde_json::json!({"name": "x", "steps": [{"type": "include", "id": "inc", "path": path}]});
            assert!(json("x", content).is_empty(), "{path}");
        }
    }

    #[test]
    fn kr_wf_005_interactive_step_in_parallel() {
        let interactive = serde_json::json!({"type": "step", "id": "chat", "agent": "wf-coder", "prompt": "p",
            "completion": {"completionSignal": "success"}});
        let parallel = serde_json::json!({"type": "parallel", "id": "p", "joinPolicy": "all", "branches": [interactive.clone()]});
        let diagnostics = json("x", serde_json::json!({"name": "x", "steps": [parallel]}));
        assert_eq!(rules(&diagnostics), vec!["KR-WF-005"]);
        assert!(
            json(
                "x",
                serde_json::json!({"name": "x", "steps": [interactive]})
            )
            .is_empty()
        );
    }

    #[test]
    fn kr_wf_006_stop_conditions() {
        let body = [step("a", "write status.json")];
        let repeat = |condition: serde_json::Value| {
            serde_json::json!({"name": "x", "steps": [{"type": "repeat", "id": "r", "maxIterations": 2,
                "onMaxIterations": "abort", "stopCondition": condition, "steps": body}]})
        };
        for (condition, expected) in [
            (serde_json::json!({}), "without containsText"),
            (
                serde_json::json!({"completionSignal": "done"}),
                "completionSignal",
            ),
            (
                serde_json::json!({"fileCheck": {"path": "status.json", "jsonPath": "done"}}),
                "value",
            ),
            (
                serde_json::json!({"fileCheck": {"path": "status.json", "jsonPath": "done", "value": true, "equals": 1}}),
                "unrecognized",
            ),
            (
                serde_json::json!({"fileCheck": {"path": "status.json", "jsonPath": "done", "value": []}}),
                "empty array",
            ),
            (
                serde_json::json!({"fileCheck": {"path": "status.json", "jsonPath": "items[0]", "value": 1}}),
                "[",
            ),
        ] {
            let diagnostics = json("x", repeat(condition.clone()));
            assert!(
                diagnostics
                    .iter()
                    .any(|d| d.rule == "KR-WF-006" && d.message.contains(expected)),
                "{condition}: {diagnostics:#?}"
            );
        }
        for (stop_when, expected) in [
            ("done", "'<watchId>.terminal'"),
            ("{{a.output}}contains DONE", "' contains '"),
            ("a.output contains DONE", "left side"),
            ("missing.terminal", "no watch node"),
        ] {
            let content = serde_json::json!({"name": "x", "steps": [{"type": "repeat", "id": "r", "maxIterations": 2,
                "onMaxIterations": "abort", "stopWhen": stop_when, "steps": body}]});
            let diagnostics = json("x", content);
            assert!(
                diagnostics
                    .iter()
                    .any(|d| d.rule == "KR-WF-006" && d.message.contains(expected)),
                "{stop_when}: {diagnostics:#?}"
            );
        }
        // `{{a.output}} contains DONE` from a step inside the loop is valid.
        let content = serde_json::json!({"name": "x", "steps": [{"type": "repeat", "id": "r", "maxIterations": 2,
            "onMaxIterations": "abort", "stopWhen": "{{a.output}} contains DONE", "steps": body}]});
        assert!(json("x", content).is_empty());
    }

    #[test]
    fn kr_wf_007_template_references() {
        let first_previous =
            serde_json::json!({"name": "x", "steps": [step("a", "{{previous.output}}")]});
        assert!(
            json("x", first_previous)[0]
                .message
                .contains("no prior sibling")
        );

        let in_parallel = serde_json::json!({"name": "x", "steps": [step("a", "x"),
            {"type": "parallel", "id": "p", "joinPolicy": "all", "branches": [step("b", "{{previous.output}}")]}]});
        assert!(json("x", in_parallel)[0].message.contains("parallel"));

        let unknown = serde_json::json!({"name": "x", "steps": [step("a", "{{nope.output}} {{steps.nope2.output}}")]});
        assert_eq!(json("x", unknown).len(), 2);

        let later =
            serde_json::json!({"name": "x", "steps": [step("a", "{{b.output}}"), step("b", "x")]});
        assert!(json("x", later)[0].message.contains("does not run before"));

        let sibling_branch = serde_json::json!({"name": "x", "steps": [{"type": "parallel", "id": "p", "joinPolicy": "all",
            "branches": [step("a", "x"), step("b", "{{a.output}}")]}]});
        assert!(
            json("x", sibling_branch)[0]
                .message
                .contains("does not run before")
        );

        let no_capture = serde_json::json!({"name": "x", "steps": [
            {"type": "step", "id": "a", "agent": "x", "prompt": "p", "captureOutput": false}, step("b", "{{a.output}}")]});
        assert!(
            json("x", no_capture)[0]
                .message
                .contains("captures no output")
        );

        let artifact = serde_json::json!({"name": "x", "steps": [step("a", "{{artifacts.plan}}")]});
        assert!(json("x", artifact)[0].message.contains("artifact"));

        // An undeclared bare input is literal text: a warning, as in Kiro.
        let bare = serde_json::json!({"name": "x", "inputs": {"task": "prompt"}, "steps": [step("a", "{{task}} {{taks}}")]});
        let diagnostics = json("x", bare);
        assert_eq!(rules(&diagnostics), vec!["KR-WF-007"]);
        assert_eq!(diagnostics[0].level, DiagnosticLevel::Warning);
        assert!(diagnostics[0].message.contains("taks"));

        // References into included recipes cannot be checked.
        let include = serde_json::json!({"name": "x", "steps": [
            {"type": "include", "id": "setup", "path": "bundled://goal"}, step("a", "{{from_include.output}} {{undeclared}}")]});
        assert!(json("x", include).is_empty());

        // A stop condition may reference a step inside its own loop.
        let loop_ref = serde_json::json!({"name": "x", "steps": [{"type": "repeat", "id": "r", "maxIterations": 2,
            "onMaxIterations": "abort", "stopWhen": "{{inner.output}} contains OK", "steps": [step("inner", "x")]}]});
        assert!(json("x", loop_ref).is_empty());
    }

    #[test]
    fn kr_wf_008_watch_handlers() {
        for (watch, expected) in [
            (
                serde_json::json!({"type": "watch", "id": "w", "handler": "command", "config": {}}),
                "config.command",
            ),
            (
                serde_json::json!({"type": "watch", "id": "w", "handler": "command", "config": {"command": "node w.mjs", "args": ["x"]}}),
                "args",
            ),
            (
                serde_json::json!({"type": "watch", "id": "w", "handler": "command", "config": {"command": "node {{script}}"}}),
                "template",
            ),
            (
                serde_json::json!({"type": "watch", "id": "w", "handler": "command", "config": {"command": "node w.mjs", "pollIntervalSec": 5}}),
                "10",
            ),
            (
                serde_json::json!({"type": "watch", "id": "w", "handler": "github-pr", "config": {"pollIntervalSec": 60}}),
                "prRef",
            ),
            (
                serde_json::json!({"type": "watch", "id": "w", "handler": "github-pr", "config": {"url": "https://github.com/o/r/pull/1", "pollIntervalSec": 10}}),
                "30",
            ),
        ] {
            let diagnostics = json(
                "x",
                serde_json::json!({"name": "x", "steps": [watch.clone()]}),
            );
            assert!(
                diagnostics
                    .iter()
                    .any(|d| d.rule == "KR-WF-008" && d.message.contains(expected)),
                "{watch}: {diagnostics:#?}"
            );
        }
        // Custom handlers are validated by Kiro at run creation.
        let custom = serde_json::json!({"type": "watch", "id": "w", "handler": "jira", "config": {"pollIntervalSec": 1}});
        assert!(json("x", serde_json::json!({"name": "x", "steps": [custom]})).is_empty());
    }

    #[test]
    fn kr_wf_009_stop_file_never_written() {
        let content = serde_json::json!({"name": "x", "steps": [{"type": "repeat", "id": "r", "maxIterations": 2,
            "onMaxIterations": "abort", "stopCondition": {"fileCheck": {"path": "status.json", "jsonPath": "done", "value": true}},
            "steps": [step("a", "Do the work")]}]});
        let diagnostics = json("x", content);
        assert_eq!(rules(&diagnostics), vec!["KR-WF-009"]);
        assert_eq!(diagnostics[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn rules_can_be_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KR-WF-004".to_string()];
        let path = Path::new(".kiro/workflows/x.workflow.json");
        assert!(
            KiroWorkflowValidator
                .validate_per_file(path, "{", &config.for_path(path))
                .is_empty()
        );
    }

    #[test]
    fn json_path_and_stop_when_grammar() {
        assert_eq!(unsupported_json_path("verdict"), None);
        assert_eq!(unsupported_json_path("$.verdict"), None);
        assert_eq!(unsupported_json_path("a.b.c"), None);
        assert_eq!(unsupported_json_path("a..b"), Some(".."));
        assert_eq!(unsupported_json_path("a.*"), Some("*"));
        assert_eq!(unsupported_json_path("a.$"), Some("$"));
        assert!(
            matches!(parse_stop_when("w.terminal"), Ok(StopWhen::WatchTerminal(w)) if w == "w")
        );
        assert!(matches!(
            parse_stop_when("{{x.output}} contains OK"),
            Ok(StopWhen::Contains(_))
        ));
        assert!(parse_stop_when("{{x.output}} contains ").is_err());
        assert!(parse_stop_when("a.b.terminal").is_err());
    }

    #[test]
    fn runs_before_follows_the_engine() {
        let o = |i| (Kind::Ordered, i);
        let c = |i| (Kind::Concurrent, i);
        assert!(runs_before(&vec![o(0)], &vec![o(1)]));
        assert!(!runs_before(&vec![o(1)], &vec![o(0)]));
        assert!(runs_before(&vec![o(0), o(0)], &vec![o(1)]));
        assert!(!runs_before(&vec![o(0), c(0)], &vec![o(0), c(1)]));
        assert!(!runs_before(&vec![o(0)], &vec![o(0), o(1)]));
    }
}
