//! Shared model and checks for the OpenCode validators.
//!
//! OpenCode reads agents, permissions, commands and MCP servers from JSON or
//! JSONC config (`opencode.json(c)`) and from Markdown frontmatter
//! (`.opencode/agents/*.md`, `.opencode/commands/*.md`). The checks run on
//! [`Node`], a small value tree with the source position of every key, built
//! from either format, so one implementation serves both.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, Fix},
    fs::FileSystem,
    rules::config_span::yaml_key_pos,
    rules::opencode_version::{OpenCodeLines, lines_for},
    schemas::kiro_json::{JsonNode, JsonValue, LineIndex, closest_name, parse_json_document},
};
use rust_i18n::t;
use std::path::{Path, PathBuf};

/// A 1-based (line, column) source position.
pub(crate) type Pos = (usize, usize);

/// A configuration value with the position of its source text.
#[derive(Debug, Clone)]
pub(crate) struct Node {
    /// Start of the value (JSON) or of its key (YAML).
    pub pos: Pos,
    /// Byte span of the value in the file (JSON only).
    pub span: Option<(usize, usize)>,
    pub val: Val,
}

#[derive(Debug, Clone)]
pub(crate) enum Val {
    Obj(Vec<Member>),
    Arr(Vec<Node>),
    Str(String),
    Num(serde_json::Number),
    Bool(bool),
    Null,
}

/// One `key: value` member, in source order.
#[derive(Debug, Clone)]
pub(crate) struct Member {
    pub key: String,
    /// Position of the key.
    pub pos: Pos,
    /// Byte span of the key including quotes (JSON only).
    pub key_span: Option<(usize, usize)>,
    pub value: Node,
}

impl Node {
    pub fn obj(&self) -> Option<&[Member]> {
        match &self.val {
            Val::Obj(members) => Some(members),
            _ => None,
        }
    }

    pub fn arr(&self) -> Option<&[Node]> {
        match &self.val {
            Val::Arr(items) => Some(items),
            _ => None,
        }
    }

    pub fn str(&self) -> Option<&str> {
        match &self.val {
            Val::Str(value) => Some(value),
            _ => None,
        }
    }

    pub fn bool(&self) -> Option<bool> {
        match self.val {
            Val::Bool(value) => Some(value),
            _ => None,
        }
    }

    pub fn i64(&self) -> Option<i64> {
        match &self.val {
            Val::Num(number) => number.as_i64(),
            _ => None,
        }
    }

    pub fn f64(&self) -> Option<f64> {
        match &self.val {
            Val::Num(number) => number.as_f64(),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self.val, Val::Null)
    }

    pub fn is_num(&self) -> bool {
        matches!(self.val, Val::Num(_))
    }

    pub fn type_name(&self) -> &'static str {
        match self.val {
            Val::Obj(_) => "object",
            Val::Arr(_) => "array",
            Val::Str(_) => "string",
            Val::Num(_) => "number",
            Val::Bool(_) => "boolean",
            Val::Null => "null",
        }
    }

    /// The last member named `key` (the one JSON parsers keep).
    pub fn get(&self, key: &str) -> Option<&Member> {
        self.obj()?.iter().rev().find(|member| member.key == key)
    }

    /// The value of the member named `key`.
    pub fn at(&self, key: &str) -> Option<&Node> {
        self.get(key).map(|member| &member.value)
    }

    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }
}

/// Convert a span-preserving JSON tree.
pub(crate) fn from_json(node: &JsonNode, index: &LineIndex<'_>) -> Node {
    let val = match &node.value {
        JsonValue::Object(members) => Val::Obj(
            members
                .iter()
                .map(|member| Member {
                    key: member.key.clone(),
                    pos: index.line_col(member.key_start),
                    key_span: Some((member.key_start, member.key_end)),
                    value: from_json(&member.value, index),
                })
                .collect(),
        ),
        JsonValue::Array(items) => {
            Val::Arr(items.iter().map(|item| from_json(item, index)).collect())
        }
        JsonValue::String(value) => Val::Str(value.clone()),
        JsonValue::Number(number) => Val::Num(number.clone()),
        JsonValue::Bool(value) => Val::Bool(*value),
        JsonValue::Null => Val::Null,
    };
    Node {
        pos: index.line_col(node.start),
        span: Some((node.start, node.end)),
        val,
    }
}

/// Parse OpenCode JSON or JSONC (comments and trailing commas), as OpenCode
/// does for every config file.
pub(crate) fn parse_config(content: &str) -> Result<Node, (String, Pos)> {
    match parse_json_document(content, true) {
        Ok(document) => Ok(from_json(&document.root, &LineIndex::new(content))),
        Err(error) => Err((error.message, (error.line, error.column.max(1)))),
    }
}

/// Convert YAML frontmatter. `first_line` is the file line of the first
/// frontmatter line; key positions come from the frontmatter text.
pub(crate) fn from_yaml(value: &serde_yaml::Value, frontmatter: &str, first_line: usize) -> Node {
    fn convert(
        value: &serde_yaml::Value,
        frontmatter: &str,
        first_line: usize,
        path: &mut Vec<String>,
        pos: Pos,
    ) -> Node {
        let val = match value {
            serde_yaml::Value::Mapping(mapping) => Val::Obj(
                mapping
                    .iter()
                    .map(|(key, value)| {
                        let key = match key {
                            serde_yaml::Value::String(key) => key.clone(),
                            other => serde_yaml::to_string(other)
                                .unwrap_or_default()
                                .trim()
                                .to_string(),
                        };
                        path.push(key.clone());
                        let segments: Vec<&str> = path.iter().map(String::as_str).collect();
                        let key_pos = yaml_key_pos(frontmatter, &segments)
                            .map_or(pos, |(line, column)| (first_line + line - 1, column));
                        let value = convert(value, frontmatter, first_line, path, key_pos);
                        path.pop();
                        Member {
                            key,
                            pos: key_pos,
                            key_span: None,
                            value,
                        }
                    })
                    .collect(),
            ),
            serde_yaml::Value::Sequence(items) => Val::Arr(
                items
                    .iter()
                    .map(|item| {
                        // Sequence items keep the position of their key.
                        let mut nested = Vec::new();
                        convert(item, "", first_line, &mut nested, pos)
                    })
                    .collect(),
            ),
            serde_yaml::Value::String(value) => Val::Str(value.clone()),
            serde_yaml::Value::Number(number) => Val::Num(
                number
                    .as_i64()
                    .map(serde_json::Number::from)
                    .or_else(|| number.as_u64().map(serde_json::Number::from))
                    .or_else(|| number.as_f64().and_then(serde_json::Number::from_f64))
                    .unwrap_or_else(|| serde_json::Number::from(0)),
            ),
            serde_yaml::Value::Bool(value) => Val::Bool(*value),
            serde_yaml::Value::Null => Val::Null,
            serde_yaml::Value::Tagged(tagged) => {
                return convert(&tagged.value, frontmatter, first_line, path, pos);
            }
        };
        Node {
            pos,
            span: None,
            val,
        }
    }
    convert(
        value,
        frontmatter,
        first_line,
        &mut Vec::new(),
        (first_line, 1),
    )
}

// ============================================================================
// Diagnostics
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    Error,
    Warning,
    Info,
}

/// Collects diagnostics for one file, honoring enabled rules.
pub(crate) struct Out<'a, 'c> {
    pub path: &'a Path,
    pub config: &'a PerFileLintConfig<'c>,
    pub diags: Vec<Diagnostic>,
}

impl<'a, 'c> Out<'a, 'c> {
    pub fn new(path: &'a Path, config: &'a PerFileLintConfig<'c>) -> Self {
        Self {
            path,
            config,
            diags: Vec::new(),
        }
    }

    pub fn on(&self, rule: &str) -> bool {
        self.config.is_rule_enabled(rule)
    }

    /// Push a diagnostic when `rule` is enabled and return it for fixes.
    pub fn emit(
        &mut self,
        level: Level,
        rule: &str,
        pos: Pos,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Option<&mut Diagnostic> {
        if !self.on(rule) {
            return None;
        }
        let file = self.path.to_path_buf();
        let (line, column) = pos;
        let diagnostic = match level {
            Level::Error => Diagnostic::error(file, line, column, rule, message),
            Level::Warning => Diagnostic::warning(file, line, column, rule, message),
            Level::Info => Diagnostic::info(file, line, column, rule, message),
        }
        .with_suggestion(suggestion);
        self.diags.push(diagnostic);
        self.diags.last_mut()
    }

    pub fn error(
        &mut self,
        rule: &str,
        pos: Pos,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Option<&mut Diagnostic> {
        self.emit(Level::Error, rule, pos, message, suggestion)
    }

    pub fn warn(
        &mut self,
        rule: &str,
        pos: Pos,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Option<&mut Diagnostic> {
        self.emit(Level::Warning, rule, pos, message, suggestion)
    }

    pub fn info(
        &mut self,
        rule: &str,
        pos: Pos,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Option<&mut Diagnostic> {
        self.emit(Level::Info, rule, pos, message, suggestion)
    }
}

/// Replace the contents of the JSON string `node` (between its quotes) with
/// `replacement`, a plain value that needs no escaping. Unsafe: OpenCode
/// reads the new value.
pub(crate) fn string_fix(node: &Node, replacement: &str, description: String) -> Option<Fix> {
    let (start, end) = node.span?;
    node.str()?;
    if end < start + 2 || replacement.contains(['"', '\\']) {
        return None;
    }
    Some(Fix::replace(
        start + 1,
        end - 1,
        replacement,
        description,
        false,
    ))
}

/// Rename the JSON key of `member` (between its quotes). Unsafe: OpenCode
/// reads the old and the new key.
pub(crate) fn rename_fix(member: &Member, new_key: &str, description: String) -> Option<Fix> {
    let (start, end) = member.key_span?;
    if end < start + 2 {
        return None;
    }
    Some(Fix::replace(
        start + 1,
        end - 1,
        new_key,
        description,
        false,
    ))
}

// ============================================================================
// Agents
// ============================================================================

/// Built-in agents of either line. OpenCode 1 also ships `scout`.
pub(crate) const BUILTIN_AGENTS: &[&str] = &[
    "build",
    "plan",
    "general",
    "explore",
    "scout",
    "compaction",
    "title",
    "summary",
];

/// OpenCode 1 theme color names for agents.
pub(crate) const THEME_COLORS: &[&str] = &[
    "primary",
    "secondary",
    "accent",
    "success",
    "warning",
    "error",
    "info",
];

pub(crate) const AGENT_MODES: &[&str] = &["subagent", "primary", "all"];

/// OpenCode 1 agent members that OpenCode 2 moved or renamed.
const V1_AGENT_MEMBERS: &[&str] = &[
    "prompt",
    "permission",
    "tools",
    "disable",
    "maxSteps",
    "variant",
    "temperature",
    "top_p",
    "options",
];
/// Native OpenCode 2 agent members.
const V2_AGENT_MEMBERS: &[&str] = &["system", "permissions", "disabled", "request"];

/// Where an agent definition comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AgentSource {
    /// `agent.<id>` in JSON (OpenCode 1 shape, read by both lines).
    V1Map,
    /// `agents.<id>` in JSON (native OpenCode 2 shape).
    V2Map,
    /// A Markdown agent file's frontmatter.
    Markdown,
}

/// Run the per-agent checks on one agent definition.
pub(crate) fn check_agent(
    out: &mut Out<'_, '_>,
    id: &str,
    agent: &Node,
    source: AgentSource,
    lines: OpenCodeLines,
) {
    let Some(_) = agent.obj() else {
        return;
    };
    let mode = agent.at("mode").and_then(Node::str);

    // OC-AG-001: mode value.
    if let Some(member) = agent.get("mode") {
        match member.value.str() {
            Some(value) if AGENT_MODES.contains(&value) => {}
            Some(value) => {
                out.error(
                    "OC-AG-001",
                    member.value.pos,
                    t!("rules.oc_ag_001.message", value = value, agent = id),
                    t!("rules.oc_ag_001.suggestion"),
                );
            }
            None if member.value.is_null() => {}
            None => {
                out.error(
                    "OC-AG-001",
                    member.value.pos,
                    t!("rules.oc_ag_001.type_error", agent = id),
                    t!("rules.oc_ag_001.suggestion"),
                );
            }
        }
    }

    // OC-AG-002 / OC-AG-006: color.
    if let Some(color) = agent.at("color").and_then(Node::str) {
        let color_node = agent.at("color").expect("color present");
        if color.starts_with('#') {
            if !is_hex6(color) {
                out.error(
                    "OC-AG-002",
                    color_node.pos,
                    t!("rules.oc_ag_002.message", value = color),
                    t!("rules.oc_ag_002.suggestion"),
                );
            }
        } else if source == AgentSource::V2Map {
            out.error(
                "OC-AG-002",
                color_node.pos,
                t!("rules.oc_ag_002.v2_named", value = color),
                t!("rules.oc_ag_002.suggestion"),
            );
        } else if !THEME_COLORS.contains(&color) {
            let suggested = crate::rules::find_closest_value(color, THEME_COLORS);
            let fix = suggested.and_then(|value| {
                string_fix(
                    color_node,
                    value,
                    t!("rules.oc_ag_006.fix", value = value).to_string(),
                )
            });
            if let Some(diagnostic) = out.warn(
                "OC-AG-006",
                color_node.pos,
                t!(
                    "rules.oc_ag_006.message",
                    value = color,
                    colors = THEME_COLORS.join(", ")
                ),
                t!("rules.oc_ag_006.suggestion"),
            ) && let Some(fix) = fix
            {
                diagnostic.fixes.push(fix);
            }
        }
    }

    // OC-AG-003: temperature (agnix heuristic range), OpenCode 1 field.
    if source != AgentSource::V2Map
        && let Some(node) = agent.at("temperature")
    {
        match node.f64() {
            Some(value) if (0.0..=2.0).contains(&value) => {}
            Some(value) => {
                out.warn(
                    "OC-AG-003",
                    node.pos,
                    t!("rules.oc_ag_003.message", value = value.to_string()),
                    t!("rules.oc_ag_003.suggestion"),
                );
            }
            None if node.is_null() => {}
            None => {
                out.warn(
                    "OC-AG-003",
                    node.pos,
                    t!("rules.oc_ag_003.type_error"),
                    t!("rules.oc_ag_003.suggestion"),
                );
            }
        }
    }

    // OC-AG-004: steps.
    if let Some(node) = agent.at("steps")
        && !node.is_null()
        && !node.i64().is_some_and(|steps| steps > 0)
    {
        out.error(
            "OC-AG-004",
            node.pos,
            t!("rules.oc_ag_004.message", agent = id),
            t!("rules.oc_ag_004.suggestion"),
        );
    }

    // OC-AG-005: top_p, OpenCode 1 field.
    if source != AgentSource::V2Map
        && let Some(node) = agent.at("top_p")
        && !node.is_null()
        && !node.f64().is_some_and(|value| (0.0..=1.0).contains(&value))
    {
        out.error(
            "OC-AG-005",
            node.pos,
            t!("rules.oc_ag_005.message", agent = id),
            t!("rules.oc_ag_005.suggestion"),
        );
    }

    // OC-AG-007 / OC-DEP-006: maxSteps.
    if let Some(member) = agent.get("maxSteps") {
        if agent.has("steps") {
            out.warn(
                "OC-AG-007",
                member.pos,
                t!("rules.oc_ag_007.message", agent = id),
                t!("rules.oc_ag_007.suggestion"),
            );
        } else {
            let fix = rename_fix(member, "steps", t!("rules.oc_dep_006.fix").to_string());
            if let Some(diagnostic) = out.warn(
                "OC-DEP-006",
                member.pos,
                t!("rules.oc_dep_006.message", agent = id),
                t!("rules.oc_dep_006.suggestion"),
            ) && let Some(fix) = fix
            {
                diagnostic.fixes.push(fix);
            }
        }
    }

    // OC-AG-008: hidden.
    if let Some(node) = agent.at("hidden")
        && !node.is_null()
        && node.bool().is_none()
    {
        out.error(
            "OC-AG-008",
            node.pos,
            t!("rules.oc_ag_008.message", agent = id),
            t!("rules.oc_ag_008.suggestion"),
        );
    }

    // OC-AG-009: disable / disabled.
    for field in ["disable", "disabled"] {
        if let Some(node) = agent.at(field)
            && !node.is_null()
            && node.bool().is_none()
        {
            out.error(
                "OC-AG-009",
                node.pos,
                t!("rules.oc_ag_009.message", agent = id, field = field),
                t!("rules.oc_ag_009.suggestion", field = field),
            );
        }
    }

    // OC-AG-011: hidden only applies to subagents in OpenCode 1.
    if lines.v1
        && source != AgentSource::V2Map
        && agent.at("hidden").and_then(Node::bool) == Some(true)
        && mode != Some("subagent")
    {
        out.info(
            "OC-AG-011",
            agent.get("hidden").expect("hidden present").pos,
            t!(
                "rules.oc_ag_011.message",
                agent = id,
                mode = mode.unwrap_or("all")
            ),
            t!("rules.oc_ag_011.suggestion"),
        );
    }

    // OC-DEP-002: agent-level tools.
    if let Some(member) = agent.get("tools") {
        out.warn(
            "OC-DEP-002",
            member.pos,
            t!("rules.oc_dep_002.agent_message", agent = id),
            t!("rules.oc_dep_002.migration_suggestion"),
        );
    }

    // OC-CFG-001: model reference.
    if let Some(node) = agent.at("model") {
        check_model(out, node, &format!("agent '{id}' model"));
    }

    // Permissions.
    if let Some(node) = agent.at("permission") {
        check_permission_v1(out, node);
    }
    if let Some(node) = agent.at("permissions") {
        check_permissions_v2(out, node);
    }

    // OC-CFG-017: one entry mixing both formats.
    check_mixed_entry(
        out,
        agent,
        &t!("rules.oc_cfg_017.agent", id = id),
        V1_AGENT_MEMBERS,
        V2_AGENT_MEMBERS,
    );
}

/// Whether an agent definition needs a description: OpenCode 1 calls it "a
/// required config option" for custom agents (default mode `all`), and
/// OpenCode 2 asks for it on subagents (default mode `primary`).
pub(crate) fn agent_needs_description(
    id: &str,
    agent: &Node,
    source: AgentSource,
    lines: OpenCodeLines,
) -> bool {
    if BUILTIN_AGENTS.contains(&id) || agent.has("description") {
        return false;
    }
    if [agent.at("disable"), agent.at("disabled")]
        .into_iter()
        .flatten()
        .any(|node| node.bool() == Some(true))
    {
        return false;
    }
    let mode = agent.at("mode").and_then(Node::str);
    if matches!(mode, Some("subagent" | "all")) {
        return true;
    }
    // Mode omitted or primary: only OpenCode 1 requires it for every custom
    // agent, and only for OpenCode 1 shapes.
    lines.v1 && source != AgentSource::V2Map && mode.is_none()
}

fn is_hex6(value: &str) -> bool {
    value.len() == 7 && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// OC-CFG-001: `provider/model`, optionally `#variant`, an `{env:...}`
/// reference, or (OpenCode 2) `{ providerID, model, variant? }`.
pub(crate) fn check_model(out: &mut Out<'_, '_>, node: &Node, field: &str) {
    match &node.val {
        Val::Str(value) => {
            if value.contains("{env:") || value.contains("{file:") {
                return;
            }
            let (base, variant) = match value.split_once('#') {
                Some((base, variant)) => (base, Some(variant)),
                None => (value.as_str(), None),
            };
            let valid = base
                .split_once('/')
                .is_some_and(|(provider, model)| !provider.is_empty() && !model.is_empty())
                && variant.is_none_or(|variant| !variant.is_empty() && !variant.contains('#'));
            if !valid {
                out.error(
                    "OC-CFG-001",
                    node.pos,
                    t!(
                        "rules.oc_cfg_001.message",
                        value = value.as_str(),
                        field = field
                    ),
                    t!("rules.oc_cfg_001.suggestion"),
                );
            }
        }
        Val::Obj(_) => {
            let ok = node.at("providerID").and_then(Node::str).is_some()
                && node.at("model").and_then(Node::str).is_some();
            if !ok {
                out.error(
                    "OC-CFG-001",
                    node.pos,
                    t!("rules.oc_cfg_001.object_error", field = field),
                    t!("rules.oc_cfg_001.suggestion"),
                );
            }
        }
        Val::Null => {}
        _ => {
            out.error(
                "OC-CFG-001",
                node.pos,
                t!("rules.oc_cfg_001.type_error", field = field),
                t!("rules.oc_cfg_001.suggestion"),
            );
        }
    }
}

/// OC-CFG-017: an agent, command, provider or model entry that uses
/// members of both formats. OpenCode "does not recursively infer formats
/// inside individual agents, providers, commands, or models".
pub(crate) fn check_mixed_entry(
    out: &mut Out<'_, '_>,
    entry: &Node,
    label: &str,
    v1_members: &[&str],
    v2_members: &[&str],
) {
    let Some(members) = entry.obj() else {
        return;
    };
    let v1 = members
        .iter()
        .find(|member| v1_members.contains(&member.key.as_str()));
    let v2 = members
        .iter()
        .find(|member| v2_members.contains(&member.key.as_str()));
    if let (Some(v1), Some(v2)) = (v1, v2) {
        let later = if v2.pos >= v1.pos { v2 } else { v1 };
        out.warn(
            "OC-CFG-017",
            later.pos,
            t!(
                "rules.oc_cfg_017.message",
                entry = label,
                v1 = v1.key.as_str(),
                v2 = v2.key.as_str()
            ),
            t!("rules.oc_cfg_017.suggestion"),
        );
    }
}

// ============================================================================
// Permissions
// ============================================================================

pub(crate) const PERMISSION_ACTIONS: &[&str] = &["allow", "ask", "deny"];

/// OpenCode 1 permission keys (built-in tool names and guards).
const V1_PERMISSION_KEYS: &[&str] = &[
    "read",
    "edit",
    "glob",
    "grep",
    "list",
    "bash",
    "task",
    "external_directory",
    "todowrite",
    "todoread",
    "question",
    "webfetch",
    "websearch",
    "codesearch",
    "lsp",
    "doom_loop",
    "skill",
];

/// OpenCode 1 action names that OpenCode 2 renamed or dropped.
const V1_ONLY_ACTIONS: &[(&str, Option<&str>)] = &[
    ("bash", Some("shell")),
    ("task", Some("subagent")),
    ("write", Some("edit")),
    ("patch", Some("edit")),
    ("doom_loop", None),
    ("lsp", None),
];

/// Patterns that cover every path or the whole home directory.
const BROAD_DIRECTORY_PATTERNS: &[&str] = &[
    "*", "**", "/", "/*", "/**", "~", "~/", "~/*", "~/**", "$HOME", "$HOME/", "$HOME/*", "$HOME/**",
];

fn has_wildcard(pattern: &str) -> bool {
    pattern.contains('*') || pattern.contains('?')
}

/// The OpenCode 1 `permission` value: a shorthand action or an object of
/// tool patterns, each an action or an object of input patterns.
pub(crate) fn check_permission_v1(out: &mut Out<'_, '_>, node: &Node) {
    match &node.val {
        Val::Str(value) => check_action(out, node, value, "*"),
        Val::Obj(members) => {
            check_shadowed_object(out, members, None);
            for member in members {
                let tool = member.key.as_str();
                // OC-PM-002: a likely misspelling of a built-in key. Other
                // keys are wildcard patterns over tool names (custom tools,
                // MCP tools), which OpenCode documents.
                if !has_wildcard(tool)
                    && !V1_PERMISSION_KEYS.contains(&tool)
                    && let Some(close) = closest_name(tool, V1_PERMISSION_KEYS)
                {
                    out.info(
                        "OC-PM-002",
                        member.pos,
                        t!("rules.oc_pm_002.message", key = tool, suggestion = close),
                        t!("rules.oc_pm_002.suggestion"),
                    );
                }
                match &member.value.val {
                    Val::Str(value) => check_action(out, &member.value, value, tool),
                    Val::Obj(patterns) => {
                        check_shadowed_object(out, patterns, Some(tool));
                        for pattern in patterns {
                            match pattern.value.str() {
                                Some(value) => check_action(out, &pattern.value, value, tool),
                                None if pattern.value.is_null() => {}
                                None => {
                                    out.error(
                                        "OC-008",
                                        pattern.value.pos,
                                        t!("rules.oc_008.type_error"),
                                        t!("rules.oc_008.suggestion"),
                                    );
                                }
                            }
                        }
                    }
                    Val::Null => {}
                    _ => {
                        out.error(
                            "OC-008",
                            member.value.pos,
                            t!("rules.oc_008.type_error"),
                            t!("rules.oc_008.suggestion"),
                        );
                    }
                }
                if tool == "external_directory" {
                    check_external_directory_v1(out, member);
                }
                if tool == "bash" {
                    check_shell_patterns_v1(out, &member.value);
                }
            }
        }
        Val::Null => {}
        _ => {
            out.error(
                "OC-008",
                node.pos,
                t!("rules.oc_008.type_error"),
                t!("rules.oc_008.suggestion"),
            );
        }
    }
}

/// OC-008: one permission action value.
fn check_action(out: &mut Out<'_, '_>, node: &Node, value: &str, tool: &str) {
    if PERMISSION_ACTIONS.contains(&value) {
        return;
    }
    let fix = crate::rules::find_closest_value(value, PERMISSION_ACTIONS).and_then(|suggested| {
        string_fix(
            node,
            suggested,
            t!("rules.oc_008.fix", value = suggested).to_string(),
        )
    });
    if let Some(diagnostic) = out.error(
        "OC-008",
        node.pos,
        t!("rules.oc_008.message", value = value, tool = tool),
        t!("rules.oc_008.suggestion"),
    ) && let Some(fix) = fix
    {
        diagnostic.fixes.push(fix);
    }
}

/// OC-PM-003 for an OpenCode 1 permission object: OpenCode applies the last
/// matching pattern, so a `"*"` after other patterns overrides all of them.
fn check_shadowed_object(out: &mut Out<'_, '_>, members: &[Member], tool: Option<&str>) {
    for (index, member) in members.iter().enumerate().skip(1) {
        if member.key != "*" {
            continue;
        }
        let catch_all = member.value.str();
        let shadowed: Vec<&str> = members[..index]
            .iter()
            .filter(|earlier| earlier.key != "*")
            .filter(|earlier| earlier.value.str().is_none() || earlier.value.str() != catch_all)
            .map(|earlier| earlier.key.as_str())
            .collect();
        if shadowed.is_empty() {
            continue;
        }
        let scope = tool.map_or_else(
            || t!("rules.oc_pm_003.scope_tools").to_string(),
            |tool| t!("rules.oc_pm_003.scope_tool", tool = tool).to_string(),
        );
        out.warn(
            "OC-PM-003",
            member.pos,
            t!(
                "rules.oc_pm_003.message",
                rule = format!("\"*\": {}", describe_action(&member.value)),
                scope = scope,
                count = shadowed.len(),
                first = shadowed[0]
            ),
            t!("rules.oc_pm_003.suggestion"),
        );
    }
}

fn describe_action(node: &Node) -> String {
    node.str().map_or_else(
        || node.type_name().to_string(),
        |value| format!("\"{value}\""),
    )
}

/// OC-PM-004 for `permission.external_directory` in OpenCode 1.
fn check_external_directory_v1(out: &mut Out<'_, '_>, member: &Member) {
    match &member.value.val {
        Val::Str(value) if value == "allow" => {
            out.warn(
                "OC-PM-004",
                member.value.pos,
                t!("rules.oc_pm_004.message", pattern = "*"),
                t!("rules.oc_pm_004.suggestion"),
            );
        }
        Val::Obj(patterns) => {
            for pattern in patterns {
                if pattern.value.str() == Some("allow")
                    && BROAD_DIRECTORY_PATTERNS.contains(&pattern.key.as_str())
                {
                    out.warn(
                        "OC-PM-004",
                        pattern.pos,
                        t!("rules.oc_pm_004.message", pattern = pattern.key.as_str()),
                        t!("rules.oc_pm_004.suggestion"),
                    );
                }
            }
        }
        _ => {}
    }
}

/// OC-PM-006 and OC-PM-007 for `permission.bash` in OpenCode 1.
fn check_shell_patterns_v1(out: &mut Out<'_, '_>, node: &Node) {
    let Some(patterns) = node.obj() else {
        return;
    };
    if let Some(allow_all) = patterns
        .iter()
        .find(|member| member.key == "*" && member.value.str() == Some("allow"))
    {
        let denies = patterns
            .iter()
            .filter(|member| member.key != "*" && member.value.str() == Some("deny"))
            .count();
        if denies > 0 {
            out.info(
                "OC-PM-006",
                allow_all.pos,
                t!("rules.oc_pm_006.message", tool = "bash", count = denies),
                t!("rules.oc_pm_006.suggestion"),
            );
        }
    }
    for member in patterns {
        if member.value.str() == Some("allow") && !has_wildcard(&member.key) {
            out.info(
                "OC-PM-007",
                member.pos,
                t!("rules.oc_pm_007.message", pattern = member.key.as_str()),
                t!("rules.oc_pm_007.suggestion", pattern = member.key.as_str()),
            );
        }
    }
}

/// A rule of an OpenCode 2 `permissions` array or a policy statement.
struct Rule<'n> {
    item: &'n Node,
    action: &'n str,
    resource: &'n str,
    effect: &'n str,
}

/// The OpenCode 2 `permissions` array: ordered `{action, resource, effect}`
/// rules, the last match wins.
pub(crate) fn check_permissions_v2(out: &mut Out<'_, '_>, node: &Node) {
    let Some(items) = node.arr() else {
        if !node.is_null() {
            out.error(
                "OC-008",
                node.pos,
                t!("rules.oc_008.v2_type_error"),
                t!("rules.oc_008.v2_suggestion"),
            );
        }
        return;
    };
    let mut rules = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let Some(members) = item.obj() else {
            out.error(
                "OC-008",
                item.pos,
                t!("rules.oc_008.v2_rule_type", index = index + 1),
                t!("rules.oc_008.v2_suggestion"),
            );
            continue;
        };
        let mut valid = true;
        for member in members {
            if !["action", "resource", "effect"].contains(&member.key.as_str()) {
                valid = false;
                out.error(
                    "OC-008",
                    member.pos,
                    t!(
                        "rules.oc_008.v2_unknown_field",
                        index = index + 1,
                        field = member.key.as_str()
                    ),
                    t!("rules.oc_008.v2_suggestion"),
                );
            }
        }
        let field = |name: &str| item.at(name).and_then(Node::str);
        for name in ["action", "resource", "effect"] {
            if field(name).is_none() {
                valid = false;
                out.error(
                    "OC-008",
                    item.at(name).map_or(item.pos, |value| value.pos),
                    t!("rules.oc_008.v2_missing", index = index + 1, field = name),
                    t!("rules.oc_008.v2_suggestion"),
                );
            }
        }
        if let Some(effect) = field("effect")
            && !PERMISSION_ACTIONS.contains(&effect)
        {
            valid = false;
            let effect_node = item.at("effect").expect("effect present");
            check_action(out, effect_node, effect, field("action").unwrap_or("*"));
        }
        if let Some(action) = field("action")
            && let Some((_, replacement)) = V1_ONLY_ACTIONS.iter().find(|(old, _)| *old == action)
        {
            let pos = item.at("action").expect("action present").pos;
            match replacement {
                Some(new) => out.warn(
                    "OC-PM-005",
                    pos,
                    t!(
                        "rules.oc_pm_005.renamed",
                        action = action,
                        replacement = *new
                    ),
                    t!("rules.oc_pm_005.suggestion_renamed", replacement = *new),
                ),
                None => out.warn(
                    "OC-PM-005",
                    pos,
                    t!("rules.oc_pm_005.removed", action = action),
                    t!("rules.oc_pm_005.suggestion_removed"),
                ),
            };
        }
        if valid {
            rules.push(Rule {
                item,
                action: field("action").unwrap_or_default(),
                resource: field("resource").unwrap_or_default(),
                effect: field("effect").unwrap_or_default(),
            });
        }
    }

    for rule in &rules {
        if rule.action == "external_directory"
            && rule.effect == "allow"
            && BROAD_DIRECTORY_PATTERNS.contains(&rule.resource)
        {
            out.warn(
                "OC-PM-004",
                rule.item
                    .at("resource")
                    .map_or(rule.item.pos, |value| value.pos),
                t!("rules.oc_pm_004.message", pattern = rule.resource),
                t!("rules.oc_pm_004.suggestion"),
            );
        }
        if rule.action == "shell" && rule.effect == "allow" && !has_wildcard(rule.resource) {
            out.info(
                "OC-PM-007",
                rule.item
                    .at("resource")
                    .map_or(rule.item.pos, |value| value.pos),
                t!("rules.oc_pm_007.message", pattern = rule.resource),
                t!("rules.oc_pm_007.suggestion", pattern = rule.resource),
            );
        }
    }
    for (index, rule) in rules.iter().enumerate() {
        if rule.action == "shell" && rule.resource == "*" && rule.effect == "allow" {
            let denies = rules[index + 1..]
                .iter()
                .filter(|later| later.action == "shell" && later.effect == "deny")
                .count();
            if denies > 0 {
                out.info(
                    "OC-PM-006",
                    rule.item.pos,
                    t!("rules.oc_pm_006.message", tool = "shell", count = denies),
                    t!("rules.oc_pm_006.suggestion"),
                );
            }
        }
    }
    check_shadowed_rules(out, &rules, "OC-PM-003");
}

/// OC-PM-003 for ordered rule arrays: a rule whose resource is `*` (and
/// whose action is `*` or the same action) overrides every earlier rule of
/// that action with a different effect.
fn check_shadowed_rules(out: &mut Out<'_, '_>, rules: &[Rule<'_>], rule_id: &str) {
    for (index, rule) in rules.iter().enumerate() {
        if rule.resource != "*" {
            continue;
        }
        let shadowed: Vec<&Rule<'_>> = rules[..index]
            .iter()
            .filter(|earlier| rule.action == "*" || earlier.action == rule.action)
            .filter(|earlier| earlier.effect != rule.effect)
            .collect();
        let Some(first) = shadowed.first() else {
            continue;
        };
        out.warn(
            rule_id,
            rule.item.pos,
            t!(
                "rules.oc_pm_003.message",
                rule = format!(
                    "{{ action: \"{}\", resource: \"*\", effect: \"{}\" }}",
                    rule.action, rule.effect
                ),
                scope = t!("rules.oc_pm_003.scope_rules").to_string(),
                count = shadowed.len(),
                first = format!("{} {}", first.action, first.resource)
            ),
            t!("rules.oc_pm_003.suggestion"),
        );
    }
}

/// OC-POL-001 (and OC-PM-003) for `policies` / `experimental.policies`.
pub(crate) fn check_policies(
    out: &mut Out<'_, '_>,
    node: &Node,
    field: &str,
    lines: OpenCodeLines,
) {
    const ACTIONS: &[&str] = &["provider.use", "tool.use", "integration.use"];
    let Some(items) = node.arr() else {
        if !node.is_null() {
            out.error(
                "OC-POL-001",
                node.pos,
                t!("rules.oc_pol_001.type_error", field = field),
                t!("rules.oc_pol_001.suggestion"),
            );
        }
        return;
    };
    let mut rules = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let number = index + 1;
        let Some(members) = item.obj() else {
            out.error(
                "OC-POL-001",
                item.pos,
                t!(
                    "rules.oc_pol_001.dropped",
                    field = field,
                    index = number,
                    reason = "it is not an object"
                ),
                t!("rules.oc_pol_001.suggestion"),
            );
            continue;
        };
        let mut problems = Vec::new();
        for member in members {
            if !["action", "resource", "effect"].contains(&member.key.as_str()) {
                problems.push(format!("unknown field '{}'", member.key));
            }
        }
        let text = |name: &str| item.at(name).and_then(Node::str);
        match text("action") {
            Some(action) if ACTIONS.contains(&action) => {}
            Some(action) => problems.push(format!(
                "action '{action}' is not provider.use, tool.use or integration.use"
            )),
            None => problems.push("action is missing or not a string".to_string()),
        }
        if text("resource").is_none() {
            problems.push("resource is missing or not a string".to_string());
        }
        match text("effect") {
            Some("allow" | "deny") => {}
            Some(effect) => problems.push(format!("effect '{effect}' is not allow or deny")),
            None => problems.push("effect is missing or not a string".to_string()),
        }
        if !problems.is_empty() {
            out.error(
                "OC-POL-001",
                item.pos,
                t!(
                    "rules.oc_pol_001.dropped",
                    field = field,
                    index = number,
                    reason = problems.join("; ")
                ),
                t!("rules.oc_pol_001.suggestion"),
            );
            continue;
        }
        let action = text("action").unwrap_or_default();
        let resource = text("resource").unwrap_or_default();
        let resource_pos = item.at("resource").map_or(item.pos, |value| value.pos);
        if lines.v1_only() && field == "experimental.policies" && action != "provider.use" {
            out.error(
                "OC-POL-001",
                item.at("action").map_or(item.pos, |value| value.pos),
                t!(
                    "rules.oc_pol_001.v1_action",
                    index = number,
                    action = action
                ),
                t!("rules.oc_pol_001.suggestion"),
            );
        }
        let shape_problem = match action {
            "tool.use" => {
                (resource != "*" && !resource.contains(':')).then_some("<action>:<value>")
            }
            "integration.use" => (resource != "*"
                && !["mcp:", "skill:", "plugin:"]
                    .iter()
                    .any(|prefix| resource.starts_with(prefix)))
            .then_some("mcp:<server>, skill:<id> or plugin:<target>"),
            _ => None,
        };
        if let Some(expected) = shape_problem {
            out.warn(
                "OC-POL-001",
                resource_pos,
                t!(
                    "rules.oc_pol_001.resource_shape",
                    index = number,
                    resource = resource,
                    action = action,
                    expected = expected
                ),
                t!("rules.oc_pol_001.suggestion"),
            );
        }
        rules.push(Rule {
            item,
            action,
            resource,
            effect: text("effect").unwrap_or_default(),
        });
    }
    check_shadowed_rules(out, &rules, "OC-PM-003");
}

// ============================================================================
// Commands
// ============================================================================

/// Shell blocks (``!`cmd` ``) that interpolate `$ARGUMENTS` or `$N`.
pub(crate) fn shell_blocks_with_arguments(template: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut rest = template;
    let mut offset = 0;
    while let Some(start) = rest.find("!`") {
        let block_start = start + 2;
        let Some(len) = rest[block_start..].find('`') else {
            break;
        };
        let block = &rest[block_start..block_start + len];
        if let Some(placeholder) = argument_placeholder(block) {
            found.push((offset + start, placeholder));
        }
        let consumed = block_start + len + 1;
        offset += consumed;
        rest = &rest[consumed..];
    }
    found
}

fn argument_placeholder(text: &str) -> Option<String> {
    if text.contains("$ARGUMENTS") {
        return Some("$ARGUMENTS".to_string());
    }
    let bytes = text.as_bytes();
    bytes.iter().enumerate().find_map(|(index, byte)| {
        (*byte == b'$' && bytes.get(index + 1).is_some_and(u8::is_ascii_digit)).then(|| {
            let digits: String = text[index + 1..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            format!("${digits}")
        })
    })
}

/// Checks shared by JSON command entries and Markdown commands.
pub(crate) fn check_command_entry(
    out: &mut Out<'_, '_>,
    id: &str,
    entry: &Node,
    from_markdown: bool,
) {
    if entry.obj().is_none() {
        return;
    }
    if !from_markdown {
        match entry.at("template") {
            Some(template) if template.str().is_some() => {
                for (_, placeholder) in
                    shell_blocks_with_arguments(template.str().unwrap_or_default())
                {
                    out.warn(
                        "OC-CMD-001",
                        template.pos,
                        t!(
                            "rules.oc_cmd_001.message",
                            command = id,
                            placeholder = placeholder.as_str()
                        ),
                        t!("rules.oc_cmd_001.suggestion"),
                    );
                }
            }
            Some(template) => {
                out.error(
                    "OC-CMD-002",
                    template.pos,
                    t!("rules.oc_cmd_002.type_error", command = id),
                    t!("rules.oc_cmd_002.suggestion"),
                );
            }
            None => {
                out.error(
                    "OC-CMD-002",
                    entry.pos,
                    t!("rules.oc_cmd_002.message", command = id),
                    t!("rules.oc_cmd_002.suggestion"),
                );
            }
        }
    }
    if let Some(model) = entry.at("model") {
        check_model(out, model, &format!("command '{id}' model"));
    }
    check_mixed_entry(
        out,
        entry,
        &t!("rules.oc_cfg_017.command", id = id),
        &["variant"],
        &["subagent"],
    );
}

// ============================================================================
// Secrets and variables
// ============================================================================

/// Whether a header or variable name names a credential.
pub(crate) fn is_credential_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "authorization"
        || lower == "cookie"
        || [
            "token",
            "secret",
            "password",
            "passwd",
            "api_key",
            "api-key",
            "apikey",
            "access_key",
            "private_key",
            "credential",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
}

/// Whether a string is a literal credential rather than a reference or a
/// placeholder: not `{env:...}` / `{file:...}`, not a shell variable, and
/// not an all-caps placeholder such as `Bearer MY_API_KEY`.
pub(crate) fn is_literal_secret(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty()
        || value.contains("{env:")
        || value.contains("{file:")
        || value.contains('$')
    {
        return false;
    }
    let secret = value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
        .or_else(|| value.strip_prefix("Basic "))
        .or_else(|| value.strip_prefix("Token "))
        .unwrap_or(value)
        .trim();
    let placeholder = secret
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' || c == '-')
        || secret.starts_with('<')
        || secret.contains("...")
        || secret.chars().all(|c| c == 'x' || c == 'X' || c == '*');
    if placeholder {
        return false;
    }
    let known_prefix = [
        "sk-",
        "sk_",
        "ghp_",
        "gho_",
        "ghs_",
        "github_pat_",
        "xoxb-",
        "xoxp-",
        "AKIA",
        "AIza",
        "glpat-",
    ]
    .iter()
    .any(|prefix| secret.starts_with(prefix));
    known_prefix || secret.chars().count() >= 16
}

/// Shell-style variable references (`$NAME`, `${NAME}`) with an all-caps
/// name, which OpenCode passes through literally.
pub(crate) fn shell_variable(value: &str) -> Option<(String, String)> {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'$' {
            let braced = bytes.get(index + 1) == Some(&b'{');
            let start = index + 1 + usize::from(braced);
            let name: String = value[start..]
                .chars()
                .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
                .collect();
            let first_ok = name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase() || c == '_');
            let closed = !braced || value[start + name.len()..].starts_with('}');
            if first_ok && closed && name.len() >= 2 {
                let written = if braced {
                    format!("${{{name}}}")
                } else {
                    format!("${name}")
                };
                return Some((written, name));
            }
        }
        index += 1;
    }
    None
}

// ============================================================================
// Project and install locations
// ============================================================================

#[cfg(feature = "filesystem")]
fn user_home() -> Option<PathBuf> {
    dirs::home_dir()
}

#[cfg(not(feature = "filesystem"))]
fn user_home() -> Option<PathBuf> {
    None
}

/// The global OpenCode config directory (`$XDG_CONFIG_HOME/opencode`, else
/// `~/.config/opencode`).
pub(crate) fn opencode_global_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(xdg).join("opencode"));
    }
    user_home().map(|home| home.join(".config").join("opencode"))
}

/// Whether `dir` is the global or a system OpenCode config directory, whose
/// config applies to every project.
pub(crate) fn is_user_or_system_dir(dir: &Path) -> bool {
    let system = [
        Path::new("/etc/opencode"),
        Path::new("/Library/Application Support/opencode"),
    ];
    if system.contains(&dir) {
        return true;
    }
    if let Some(program_data) = std::env::var_os("ProgramData")
        && dir == Path::new(&program_data).join("opencode")
    {
        return true;
    }
    opencode_global_dir().is_some_and(|global| {
        dir == global
            || match (std::fs::canonicalize(dir), std::fs::canonicalize(&global)) {
                (Ok(a), Ok(b)) => a == b,
                _ => false,
            }
    })
}

/// Top-level keys that only exist in the native OpenCode 2 shape.
pub(crate) const V2_TOP_LEVEL_KEYS: &[&str] = &[
    "agents",
    "commands",
    "permissions",
    "policies",
    "plugins",
    "providers",
    "snapshots",
    "update",
    "media",
    "websearch",
    "worktree",
    "warming",
];

/// Whether a parsed opencode.json uses native OpenCode 2 fields.
pub(crate) fn config_has_v2_markers(root: &Node) -> bool {
    let Some(members) = root.obj() else {
        return false;
    };
    members.iter().any(|member| {
        V2_TOP_LEVEL_KEYS.contains(&member.key.as_str())
            || (member.key == "mcp"
                && (member.value.has("servers")
                    || member
                        .value
                        .at("timeout")
                        .is_some_and(|t| t.obj().is_some())))
            || (member.key == "compaction"
                && (member.value.has("keep") || member.value.has("buffer")))
    })
}

/// Evidence that a directory tree uses OpenCode: the nearest directory, from
/// `start` up to the filesystem root (at most 16 levels), that has an
/// `opencode.json(c)` or a `.opencode/` directory. Returns that directory and
/// the release lines its config targets.
pub(crate) fn find_opencode_project(
    start: &Path,
    config: &PerFileLintConfig<'_>,
) -> Option<(PathBuf, OpenCodeLines)> {
    let fs: &dyn FileSystem = config.fs().as_ref();
    for dir in start.ancestors().take(16) {
        let configs: Vec<PathBuf> = [
            dir.join("opencode.json"),
            dir.join("opencode.jsonc"),
            dir.join(".opencode").join("opencode.json"),
            dir.join(".opencode").join("opencode.jsonc"),
        ]
        .into_iter()
        .filter(|candidate| fs.is_file(candidate))
        .collect();
        if configs.is_empty() && !fs.is_dir(&dir.join(".opencode")) {
            continue;
        }
        let v2 = configs.iter().any(|path| {
            fs.read_to_string(path)
                .ok()
                .and_then(|content| parse_config(&content).ok())
                .is_some_and(|root| config_has_v2_markers(&root))
        });
        return Some((dir.to_path_buf(), lines_for(config, v2)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonc_with_trailing_commas_parses_with_positions() {
        let content = "{\n  // c\n  \"model\": \"a/b\",\n  \"server\": { \"port\": 4096, },\n}\n";
        let root = parse_config(content).unwrap();
        let model = root.get("model").unwrap();
        assert_eq!(model.pos, (3, 3));
        assert_eq!(model.value.str(), Some("a/b"));
        assert_eq!(
            root.at("server").unwrap().at("port").unwrap().i64(),
            Some(4096)
        );
        assert!(parse_config("{ \"a\": }").is_err());
    }

    #[test]
    fn yaml_frontmatter_positions() {
        let fm = "description: x\nmode: subagent\npermission:\n  bash:\n    \"*\": ask\n";
        let value: serde_yaml::Value = serde_yaml::from_str(fm).unwrap();
        let node = from_yaml(&value, fm, 2);
        assert_eq!(node.get("mode").unwrap().pos, (3, 1));
        let bash = node.at("permission").unwrap().get("bash").unwrap();
        assert_eq!(bash.pos, (5, 3));
        assert_eq!(bash.value.get("*").unwrap().pos, (6, 5));
    }

    #[test]
    fn secrets_and_shell_variables() {
        assert!(is_literal_secret("Bearer sk-live-1234567890abcdef"));
        assert!(!is_literal_secret("Bearer MY_API_KEY"));
        assert!(!is_literal_secret("Bearer {env:MY_API_KEY}"));
        assert!(!is_literal_secret("{file:~/.secrets/openai-key}"));
        assert!(!is_literal_secret("short"));
        assert_eq!(
            shell_variable("Bearer ${MY_API_KEY}").map(|(written, _)| written),
            Some("${MY_API_KEY}".to_string())
        );
        assert_eq!(
            shell_variable("$HOME/x").map(|(_, name)| name),
            Some("HOME".to_string())
        );
        assert!(shell_variable("costs $5").is_none());
        assert!(shell_variable("{env:API_KEY}").is_none());
    }

    #[test]
    fn shell_blocks_find_argument_placeholders() {
        let found = shell_blocks_with_arguments(
            "Log:\n!`git log --oneline -$1`\n!`git diff`\nUse $ARGUMENTS",
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, "$1");
        assert!(shell_blocks_with_arguments("!`echo $ARGUMENTS`")[0].1 == "$ARGUMENTS");
    }
}
