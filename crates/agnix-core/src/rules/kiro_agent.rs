//! Kiro custom agent validation (KR-AG-*, KR-HK-005, KR-HK-006).
//!
//! Kiro CLI 2.x ships two agent engines. V2, the 2.x default, loads the
//! top-level `*.json` files of `.kiro/agents/` with a strict serde loader;
//! V3, the default from CLI 3.0, loads `*.json` (JSONC allowed) and `*.md`
//! agents recursively and validates them with zod schemas. The same file can
//! load in one engine and be skipped by the other, so every check here knows
//! which engines skip the agent, drop a value, or ignore it, and each file is
//! checked against the engines it targets ([`super::kiro_engine`]): a
//! `[tool_versions] kiro` pin wins, otherwise the file's own V2 and V3 fields
//! decide. Markdown and nested JSON agents load only in V3.
//!
//! Severity follows what Kiro does: a condition that skips the agent in every
//! targeted engine is an error, one that skips it in only one of them is a
//! warning naming that engine, and dropped or ignored values are warnings or
//! info. For an unpinned file with only CLI 2.x fields, conditions that will
//! make V3 skip the agent are reported as migration warnings, like KR-AG-015.
//!
//! - KR-AG-001: field unknown to, or ignored by, the targeted engines.
//! - KR-AG-002: invalid `resources` entry.
//! - KR-AG-003: `allowedTools` trusts a tool `tools` does not make available.
//! - KR-AG-004: `model` is a Claude Code alias or not a string.
//! - KR-AG-005: tools reference MCP servers CLI 2.x will not load.
//! - KR-AG-006: sub-agent access that cannot work (no subagent tool, or an
//!   agent list naming no known agent).
//! - KR-AG-008: explicit `name` empty, blank, or not a string.
//! - KR-AG-009: `prompt` empty or not a string.
//! - KR-AG-010: duplicate `tools` entries.
//! - KR-AG-011: empty `tools` array.
//! - KR-AG-012: `toolAliases` renames a tool `tools` does not include.
//! - KR-AG-013: secrets in the prompt.
//! - KR-AG-014: invalid `permissions` block.
//! - KR-AG-015: CLI 2.x trust fields without `permissions` (V3 skips the file).
//! - KR-AG-016: agent JSON that does not parse in a targeted engine.
//! - KR-AG-017: Markdown agent without valid frontmatter.
//! - KR-AG-018: known field with the wrong type, or a duplicate field.
//! - KR-AG-019: JSON agent without `name` (CLI 2.x requires it).
//! - KR-AG-020: JSON file in `.kiro/agents` with no agent fields.
//! - KR-AG-021: `file://` prompt that cannot be loaded.
//! - KR-AG-022: agent id reserved by V3 or shared with another agent file.
//! - KR-AG-023: invalid inline MCP server.
//! - KR-AG-024: tool name that selects or trusts no tool in an engine.
//! - KR-AG-025: hook matcher that never matches in an engine.
//! - KR-AG-026: hook timeout in the wrong field or unit.
//! - KR-AG-027: hook entry field the engines ignore.
//! - KR-AG-028: agent file Kiro CLI 2.x does not load (when 2.x is pinned).
//! - KR-AG-029: `resources` glob that loads the whole workspace.
//! - KR-AG-030: agent description missing or too short for sub-agent selection.
//! - KR-HK-005: unknown agent hook trigger.
//! - KR-HK-006: invalid agent hook entry.

use std::cell::OnceCell;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use rust_i18n::t;
use serde_json::{Map, Value};

use crate::config::PerFileLintConfig;
use crate::diagnostics::{Diagnostic, DiagnosticLevel, Fix};
use crate::parsers::frontmatter::{check_yaml_depth, split_frontmatter};
use crate::rules::kiro_engine::{KiroEngine, KiroEngines, engines_for, pinned_engine};
use crate::rules::{Validator, ValidatorMetadata, json_type_name, line_col_at_offset};
use crate::schemas::kiro_agent::{
    ARRAY_HOOK_ENTRY_FIELDS, CLAUDE_CODE_MODEL_ALIASES, DISPATCH_KINDS, FieldOrigin,
    FrontmatterIndex, JsonNode, KNOWN_BUILT_IN_AGENTS, MCP_SERVER_FIELDS, McpFieldShape,
    PERMISSION_POLICY_PRESETS, PathSegment, V2_HOOK_ENTRY_FIELDS, V2_HOOK_TRIGGERS,
    V2_MARKER_FIELDS, V3_MARKER_FIELDS, V3_RESERVED_AGENT_IDS, V3Replacement,
    case_variant_of_known_tool, claude_code_tool_replacement, field_origin, strip_jsonc,
    v2_equivalent_for_v3_tool, v2_tool_group, v2_trigger_for, v3_replacement_for_v2_tool,
};
use crate::schemas::kiro_hook_v3::{
    MatcherSubject, foreign_tool_names, is_tool_selector, matches_everything, resolve_trigger,
    tool_matcher_matches_builtin, trigger_case_insensitive_match,
};
use crate::schemas::kiro_permissions::VALID_PERMISSION_CAPABILITIES;

use DiagnosticLevel::{Error, Info, Warning};
use PathSegment::{Index as I, Key as K};

const RULE_IDS: &[&str] = &[
    "KR-AG-001",
    "KR-AG-002",
    "KR-AG-003",
    "KR-AG-004",
    "KR-AG-005",
    "KR-AG-006",
    "KR-AG-008",
    "KR-AG-009",
    "KR-AG-010",
    "KR-AG-011",
    "KR-AG-012",
    "KR-AG-013",
    "KR-AG-014",
    "KR-AG-015",
    "KR-AG-016",
    "KR-AG-017",
    "KR-AG-018",
    "KR-AG-019",
    "KR-AG-020",
    "KR-AG-021",
    "KR-AG-022",
    "KR-AG-023",
    "KR-AG-024",
    "KR-AG-025",
    "KR-AG-026",
    "KR-AG-027",
    "KR-AG-028",
    "KR-AG-029",
    "KR-AG-030",
    "KR-HK-005",
    "KR-HK-006",
];

/// Directory levels searched below `.kiro/agents` for sibling agents.
const MAX_AGENT_DIR_DEPTH: usize = 10;

/// Descriptions shorter than this many words get the KR-AG-030 hint.
const MIN_DESCRIPTION_WORDS: usize = 4;

/// Engines that skip the agent when a condition holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fatal {
    Both,
    V2,
    V3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Anchor {
    Key,
    Value,
}

/// A parsed agent file: the config object plus what is needed to locate values.
struct AgentDoc<'a> {
    path: &'a Path,
    content: &'a str,
    markdown: bool,
    root: Map<String, Value>,
    json: Option<JsonNode>,
    frontmatter: Option<FrontmatterIndex>,
    /// Markdown body (the V3 prompt): offset of its first byte and its text.
    body: Option<(usize, String)>,
    /// Offset diagnostics fall back to: the `{` or the frontmatter start.
    root_offset: usize,
}

impl AgentDoc<'_> {
    fn offset(&self, path: &[PathSegment<'_>], anchor: Anchor) -> usize {
        if let Some(node) = &self.json {
            return json_offset(node, path, anchor);
        }
        if let Some(index) = &self.frontmatter {
            return index.locate(path).unwrap_or(self.root_offset);
        }
        self.root_offset
    }

    /// The exact JSON node at `path`, if this is a JSON document.
    fn node(&self, path: &[PathSegment<'_>]) -> Option<&JsonNode> {
        let mut current = self.json.as_ref()?;
        for segment in path {
            current = match segment {
                PathSegment::Key(key) => &current.member(key)?.value,
                PathSegment::Index(index) => current.items().get(*index)?,
            };
        }
        Some(current)
    }

    fn format(&self) -> String {
        if self.markdown {
            t!("rules.kr_ag_engine.format_frontmatter").to_string()
        } else {
            t!("rules.kr_ag_engine.format_json").to_string()
        }
    }
}

fn json_offset(node: &JsonNode, path: &[PathSegment<'_>], anchor: Anchor) -> usize {
    let mut current = node;
    let mut best = node.start;
    for (position, segment) in path.iter().enumerate() {
        match segment {
            PathSegment::Key(key) => {
                let Some(member) = current.member(key) else {
                    return best;
                };
                if position + 1 == path.len() && anchor == Anchor::Key {
                    return member.key_start;
                }
                best = member.value.start;
                current = &member.value;
            }
            PathSegment::Index(index) => {
                let Some(item) = current.items().get(*index) else {
                    return best;
                };
                best = item.start;
                current = item;
            }
        }
    }
    best
}

/// Why an agent file could not be read as an agent.
enum LoadFailure {
    /// JSON that neither engine parses.
    Syntax {
        line: usize,
        column: usize,
        error: String,
    },
    /// JSON whose top level is not an object.
    NotObject { found: &'static str },
    /// Markdown without a frontmatter block.
    NoFrontmatter,
    /// Markdown with an empty frontmatter block.
    EmptyFrontmatter { offset: usize },
    /// Frontmatter that is not valid YAML, or not a mapping of string keys.
    BadFrontmatter { offset: usize, error: String },
}

struct Loaded<'a> {
    doc: AgentDoc<'a>,
    /// Offset of the first comment or trailing comma, for JSON that only V3 parses.
    jsonc_offset: Option<usize>,
}

fn load_json<'a>(path: &'a Path, content: &'a str) -> Result<Loaded<'a>, LoadFailure> {
    let (value, span_text, jsonc_offset) = match serde_json::from_str::<Value>(content) {
        Ok(value) => (value, None, None),
        Err(strict_error) => {
            let (stripped, first_change) = strip_jsonc(content);
            match serde_json::from_str::<Value>(&stripped) {
                Ok(value) if first_change.is_some() => (value, Some(stripped), first_change),
                Ok(_) | Err(_) => {
                    // Report where the JSON really breaks, not the first comment.
                    let error = serde_json::from_str::<Value>(&stripped)
                        .err()
                        .unwrap_or(strict_error);
                    return Err(LoadFailure::Syntax {
                        line: error.line().max(1),
                        column: error.column().max(1),
                        error: error.to_string(),
                    });
                }
            }
        }
    };
    let Value::Object(root) = value else {
        return Err(LoadFailure::NotObject {
            found: json_type_name(&value),
        });
    };
    let json = JsonNode::parse(span_text.as_deref().unwrap_or(content));
    let root_offset = json.as_ref().map_or(0, |node| node.start);
    Ok(Loaded {
        doc: AgentDoc {
            path,
            content,
            markdown: false,
            root,
            json,
            frontmatter: None,
            body: None,
            root_offset,
        },
        jsonc_offset,
    })
}

fn load_markdown<'a>(path: &'a Path, content: &'a str) -> Result<Loaded<'a>, LoadFailure> {
    let parts = split_frontmatter(content);
    if !parts.has_frontmatter || !parts.has_closing {
        return Err(LoadFailure::NoFrontmatter);
    }
    let start = parts.frontmatter_start;
    let end = start + parts.frontmatter.len();
    if parts.frontmatter.trim().is_empty() {
        return Err(LoadFailure::EmptyFrontmatter { offset: start });
    }
    if let Err(error) = check_yaml_depth(&parts.frontmatter) {
        return Err(LoadFailure::BadFrontmatter {
            offset: start,
            error: error.to_string(),
        });
    }
    let yaml = serde_yaml::from_str::<serde_yaml::Value>(&parts.frontmatter).map_err(|error| {
        let offset = error
            .location()
            .map_or(start, |location| (start + location.index()).min(end));
        LoadFailure::BadFrontmatter {
            offset,
            error: error.to_string(),
        }
    })?;
    let root = match serde_json::to_value(&yaml) {
        Ok(Value::Object(root)) if !root.is_empty() => root,
        Ok(Value::Object(_)) | Ok(Value::Null) => {
            return Err(LoadFailure::EmptyFrontmatter { offset: start });
        }
        Ok(other) => {
            return Err(LoadFailure::BadFrontmatter {
                offset: start,
                error: t!(
                    "rules.kr_ag_017.not_mapping",
                    found = json_type_name(&other)
                )
                .to_string(),
            });
        }
        Err(error) => {
            return Err(LoadFailure::BadFrontmatter {
                offset: start,
                error: error.to_string(),
            });
        }
    };
    let raw_body = content.get(parts.body_start..).unwrap_or("");
    let leading = raw_body.len() - raw_body.trim_start().len();
    let body = raw_body.trim().to_string();
    Ok(Loaded {
        doc: AgentDoc {
            path,
            content,
            markdown: true,
            root,
            json: None,
            frontmatter: Some(FrontmatterIndex::new(content, start, end)),
            body: Some((parts.body_start + leading, body)),
            root_offset: start,
        },
        jsonc_offset: None,
    })
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

/// The `.kiro/agents` directory holding `path`, and the path below it.
fn agents_location(path: &Path) -> Option<(PathBuf, Vec<String>)> {
    let components: Vec<Component<'_>> = path.components().collect();
    let position = components.windows(2).rposition(|pair| {
        matches!(
            (pair[0], pair[1]),
            (Component::Normal(kiro), Component::Normal(agents))
                if kiro.eq_ignore_ascii_case(".kiro") && agents.eq_ignore_ascii_case("agents")
        )
    })?;
    let dir: PathBuf = components[..position + 2].iter().collect();
    let rest: Vec<String> = components[position + 2..]
        .iter()
        .filter_map(|component| match component {
            Component::Normal(value) => value.to_str().map(ToString::to_string),
            _ => None,
        })
        .collect();
    (!rest.is_empty()).then_some((dir, rest))
}

/// V3's path-derived agent id: the path below `.kiro/agents`, `/`-separated,
/// without a final `.md` or `.json`.
fn path_agent_id(rest: &[String]) -> String {
    let mut parts = rest.to_vec();
    if let Some(last) = parts.last_mut() {
        for extension in [".md", ".json"] {
            if let Some(stem) = last.strip_suffix(extension) {
                *last = stem.to_string();
                break;
            }
        }
    }
    parts.join("/")
}

/// The workspace (or home) directory that contains `.kiro/agents`, which V3
/// uses as the containment root for `file://` prompts.
fn containment_root(path: &Path) -> Option<PathBuf> {
    let (agents_dir, _) = agents_location(path)?;
    Some(agents_dir.parent()?.parent()?.to_path_buf())
}

/// Lexically normalize a path: drop `.`, fold `..` into the preceding
/// component, and keep leading `..` that cannot be folded.
fn normalize_components(path: &Path) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.last().is_some_and(|last| last != ".." && last != "/") {
                    parts.pop();
                } else if parts.last().map(String::as_str) != Some("/") {
                    parts.push("..".to_string());
                }
            }
            Component::RootDir => parts.push("/".to_string()),
            Component::Prefix(prefix) => {
                parts.push(prefix.as_os_str().to_string_lossy().into_owned())
            }
            Component::Normal(value) => parts.push(value.to_string_lossy().into_owned()),
        }
    }
    parts
}

fn escapes_root(target: &Path, root: &Path) -> bool {
    let target = normalize_components(target);
    let root = normalize_components(root);
    target.len() < root.len()
        || target[..root.len()] != root[..]
        || target[root.len()..].iter().any(|part| part == "..")
}

fn is_absolute_like(path: &str) -> bool {
    let bytes = path.as_bytes();
    path.starts_with('/')
        || path.starts_with('\\')
        || (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
}

fn has_glob(value: &str) -> bool {
    value.contains(['*', '?'])
}

/// `*` and `?` glob match, case-sensitive like Kiro's tool selectors.
fn glob_matches(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let (mut p, mut t) = (0, 0);
    let (mut star, mut mark) = (None, 0);
    while t < text.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            mark = t;
            p += 1;
        } else if let Some(position) = star {
            p = position + 1;
            mark += 1;
            t = mark;
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|ch| *ch == '*')
}

/// V3 tags that, in CLI 2.x, cover a tool name (the V2 TUI tool groups,
/// spec 2.6), used generously when checking whether `tools` covers a name.
fn v2_group_tag_covers(tag: &str, name: &str) -> bool {
    match tag {
        "read" => matches!(
            name,
            "fs_read" | "fsRead" | "grep" | "glob" | "code" | "introspect" | "tool_search"
        ),
        "write" => matches!(name, "fs_write" | "fsWrite" | "code"),
        "web" => matches!(name, "web_fetch" | "web_search"),
        _ => false,
    }
}

/// Whether a CLI 2.x `tools` list makes `entry` available, with Kiro's
/// selector semantics: `*`, `@builtin`, aliases of the same tool, the
/// `@server` / `@server/tool` forms, and globs. Unknown selectors count as
/// covering when they match exactly, ignoring case.
fn v2_tools_cover(tools: &[String], entry: &str) -> bool {
    if tools.iter().any(|tool| tool == "*") || has_glob(entry) || entry == "@builtin" {
        return true;
    }
    let entry = entry.strip_prefix("@builtin/").unwrap_or(entry);
    if let Some(reference) = entry.strip_prefix('@') {
        let server = reference.split('/').next().unwrap_or(reference);
        let whole_server = !reference.contains('/');
        return tools.iter().any(|tool| {
            tool == entry
                || tool.strip_prefix('@') == Some(server)
                || (whole_server && tool.starts_with(&format!("@{server}/")))
                || (has_glob(tool) && glob_matches(tool, entry))
        });
    }
    let group = v2_tool_group(entry);
    tools.iter().any(|tool| {
        let tool = tool.as_str();
        tool == "@builtin"
            || tool.eq_ignore_ascii_case(entry)
            || group.is_some_and(|group| group.contains(&tool))
            || v2_tool_group(tool).is_some_and(|tool_group| tool_group.contains(&entry))
            || v2_group_tag_covers(tool, entry)
            || (has_glob(tool) && glob_matches(tool, entry))
    })
}

/// Whether a V3 `tools` list selects `selector` (exactly, or through `*` or
/// `@builtin`).
fn v3_tools_select(tools: &[String], selector: &str) -> bool {
    tools
        .iter()
        .any(|tool| tool == "*" || tool == "@builtin" || tool == selector)
}

/// Whether `kiro-cli agent upgrade` leaves a V3 selector for the CLI 2.x tool
/// `name` (spec 1.8): it rewrites fs_read, fsRead, fs_write, execute_bash,
/// execute_cmd, executeCmd and agent_crew, adds `web` next to web_fetch and
/// web_search, and keeps grep, glob and use_subagent, which then select
/// nothing in V3 unless a read or subagent selector survives the rewrite.
/// Names whose rewrite is not documented (fsWrite, executeBash, agentCrew)
/// count as covered.
fn upgrade_covers(tools: &[String], name: &str) -> bool {
    let has = |names: &[&str]| tools.iter().any(|tool| names.contains(&tool.as_str()));
    match name {
        "grep" | "glob" => has(&["read", "fs_read", "fsRead", "*", "@builtin"]),
        "use_subagent" => has(&["subagent", "agent_crew", "*", "@builtin"]),
        _ => true,
    }
}

/// Whether a CLI 2.x `tools` list selects the tool named `name`.
fn v2_tools_select(tools: &[String], name: &str) -> bool {
    let group = v2_tool_group(name).unwrap_or(&[]);
    tools.iter().any(|tool| {
        tool == "*" || tool == "@builtin" || tool == name || group.contains(&tool.as_str())
    })
}

fn json_string_array(value: Option<&Value>) -> Vec<(usize, String)> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    item.as_str().map(|text| (index, text.trim().to_string()))
                })
                .filter(|(_, text)| !text.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn short_json(value: &Value) -> String {
    let text = value.to_string();
    if text.chars().count() > 60 {
        format!("{}...", text.chars().take(57).collect::<String>())
    } else {
        text
    }
}

/// The check context for one agent file.
struct Ctx<'a> {
    doc: AgentDoc<'a>,
    config: &'a PerFileLintConfig<'a>,
    engines: KiroEngines,
    pinned: Option<KiroEngine>,
    /// An unpinned top-level JSON file with only CLI 2.x fields: conditions
    /// that will make V3 skip it are reported as migration warnings.
    migration: bool,
    /// The other agent files in the same `.kiro/agents` directory, read once.
    siblings: OnceCell<Vec<Sibling>>,
}

/// Another agent file in the same `.kiro/agents` directory.
struct Sibling {
    /// Path components below `.kiro/agents`.
    rel: Vec<String>,
    markdown: bool,
    identity: SiblingIdentity,
}

impl Sibling {
    /// The id V3 uses: a non-empty `name`, else the path.
    fn v3_id(&self) -> String {
        self.identity
            .name
            .clone()
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| path_agent_id(&self.rel))
    }
}

impl Ctx<'_> {
    fn enabled(&self, rule: &str) -> bool {
        self.config.is_rule_enabled(rule)
    }

    fn diag(
        &self,
        level: DiagnosticLevel,
        rule: &str,
        offset: usize,
        message: impl Into<String>,
    ) -> Diagnostic {
        let (line, column) = line_col_at_offset(self.doc.content, offset);
        let file = self.doc.path.to_path_buf();
        match level {
            Error => Diagnostic::error(file, line, column, rule, message),
            Warning => Diagnostic::warning(file, line, column, rule, message),
            Info => Diagnostic::info(file, line, column, rule, message),
        }
    }

    fn at(&self, path: &[PathSegment<'_>], anchor: Anchor) -> usize {
        self.doc.offset(path, anchor)
    }

    /// Note the engine assumption on findings whose level depends on which
    /// engines an unpinned file targets.
    fn assume(&self, diagnostic: Diagnostic) -> Diagnostic {
        if self.pinned.is_some() || self.doc.markdown {
            return diagnostic;
        }
        let engines = if self.engines.v2 && self.engines.v3 {
            t!("rules.kr_ag_engine.engines_both")
        } else if self.engines.v2 {
            t!("rules.kr_ag_engine.engines_v2")
        } else {
            t!("rules.kr_ag_engine.engines_v3")
        };
        diagnostic.with_assumption(t!("rules.kr_ag_engine.assumption", engines = engines))
    }

    /// The level and consequence of a condition that skips the agent in
    /// `fatal`, or `None` when no targeted engine skips it.
    fn skip_effect(&self, fatal: Fatal) -> Option<(DiagnosticLevel, String)> {
        let KiroEngines { v2, v3 } = self.engines;
        let (level, key) = match fatal {
            Fatal::Both if v2 && v3 => (Error, "rules.kr_ag_engine.skip_both"),
            Fatal::Both if v2 => (Error, "rules.kr_ag_engine.skip_v2"),
            Fatal::Both => (Error, "rules.kr_ag_engine.skip_v3"),
            Fatal::V2 if v2 && v3 => (Warning, "rules.kr_ag_engine.skip_v2_partial"),
            Fatal::V2 if v2 => (Error, "rules.kr_ag_engine.skip_v2"),
            Fatal::V3 if v2 && v3 => (Warning, "rules.kr_ag_engine.skip_v3_partial"),
            Fatal::V3 if v3 => (Error, "rules.kr_ag_engine.skip_v3"),
            Fatal::V3 if self.migration => (Warning, "rules.kr_ag_engine.skip_v3_migration"),
            Fatal::V2 | Fatal::V3 => return None,
        };
        Some((level, t!(key).to_string()))
    }

    /// Push a finding for a condition that skips the agent in `fatal`.
    fn push_skip(
        &self,
        out: &mut Vec<Diagnostic>,
        rule: &str,
        fatal: Fatal,
        offset: usize,
        message: impl FnOnce(&str) -> String,
        suggestion: impl Into<String>,
    ) {
        let Some((level, effect)) = self.skip_effect(fatal) else {
            return;
        };
        let diagnostic = self
            .diag(level, rule, offset, message(&effect))
            .with_suggestion(suggestion);
        let diagnostic = if fatal == Fatal::Both {
            diagnostic
        } else {
            self.assume(diagnostic)
        };
        out.push(diagnostic);
    }

    fn root(&self) -> &Map<String, Value> {
        &self.doc.root
    }

    /// `tools` entries as strings (Markdown also allows a comma string).
    fn tools_list(&self) -> Option<Vec<String>> {
        match self.root().get("tools")? {
            Value::Array(items) => Some(
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|tool| tool.trim().to_string())
                    .filter(|tool| !tool.is_empty())
                    .collect(),
            ),
            Value::String(text) if text.trim() == "*" => Some(vec!["*".to_string()]),
            Value::String(text) if self.doc.markdown => Some(
                text.split(',')
                    .map(|tool| tool.trim().to_string())
                    .filter(|tool| !tool.is_empty())
                    .collect(),
            ),
            _ => None,
        }
    }

    fn run(&self, out: &mut Vec<Diagnostic>) {
        if !self.doc.markdown
            && !self.root().is_empty()
            && !self
                .root()
                .keys()
                .any(|key| field_origin(key).is_some_and(|origin| origin != FieldOrigin::Marker))
        {
            self.check_not_an_agent(out);
            return;
        }
        self.check_duplicate_keys(out);
        self.check_field_types(out);
        self.check_fields(out);
        self.check_name(out);
        self.check_duplicate_names(out);
        self.check_subagents(out);
        self.check_description(out);
        self.check_prompt(out);
        self.check_model(out);
        self.check_resources(out);
        self.check_resource_breadth(out);
        self.check_tools(out);
        self.check_allowed_tools(out);
        self.check_tool_aliases(out);
        self.check_mcp_access(out);
        self.check_mcp_servers(out);
        self.check_permissions(out);
        self.check_cli_only(out);
        self.check_hooks(out);
    }

    // ----- KR-AG-016 ---------------------------------------------------------

    fn check_jsonc(&self, offset: usize, out: &mut Vec<Diagnostic>) {
        if !self.enabled("KR-AG-016") {
            return;
        }
        self.push_skip(
            out,
            "KR-AG-016",
            Fatal::V2,
            offset,
            |effect| t!("rules.kr_ag_016.jsonc", effect = effect).to_string(),
            t!("rules.kr_ag_016.jsonc_suggestion"),
        );
    }

    // ----- KR-AG-020 ---------------------------------------------------------

    fn check_not_an_agent(&self, out: &mut Vec<Diagnostic>) {
        if !self.enabled("KR-AG-020") {
            return;
        }
        let keys: Vec<&str> = self.root().keys().map(String::as_str).take(5).collect();
        let id = agents_location(self.doc.path)
            .map(|(_, rest)| path_agent_id(&rest))
            .unwrap_or_default();
        let KiroEngines { v2, v3 } = self.engines;
        let (level, key) = match (v2, v3) {
            (true, true) => (Warning, "rules.kr_ag_020.both"),
            (true, false) => (Error, "rules.kr_ag_020.v2"),
            _ => (Warning, "rules.kr_ag_020.v3"),
        };
        let diagnostic = self
            .diag(
                level,
                "KR-AG-020",
                self.doc.root_offset,
                t!(key, fields = keys.join(", "), id = id),
            )
            .with_suggestion(t!("rules.kr_ag_020.suggestion"));
        out.push(self.assume(diagnostic));
    }

    // ----- KR-AG-018 ---------------------------------------------------------

    fn check_duplicate_keys(&self, out: &mut Vec<Diagnostic>) {
        if !self.enabled("KR-AG-018") {
            return;
        }
        let Some(node) = &self.doc.json else {
            return;
        };
        let mut seen = HashSet::new();
        for member in node.members() {
            let key = member.key.as_str();
            let read_by_v2 = matches!(
                field_origin(key),
                Some(FieldOrigin::Shared | FieldOrigin::V2)
            ) || key == "$schema";
            if read_by_v2 && !seen.insert(key) {
                self.push_skip(
                    out,
                    "KR-AG-018",
                    Fatal::V2,
                    member.key_start,
                    |effect| {
                        t!("rules.kr_ag_018.duplicate", field = key, effect = effect).to_string()
                    },
                    t!("rules.kr_ag_018.duplicate_suggestion"),
                );
            }
        }
        if let Some(legacy) = node.member("useLegacyMcpJson")
            && node.member("includeMcpJson").is_some()
        {
            self.push_skip(
                out,
                "KR-AG-018",
                Fatal::V2,
                legacy.key_start,
                |effect| t!("rules.kr_ag_018.legacy_alias", effect = effect).to_string(),
                t!("rules.kr_ag_018.legacy_alias_suggestion"),
            );
        }
    }

    fn push_type(
        &self,
        out: &mut Vec<Diagnostic>,
        fatal: Fatal,
        path: &[PathSegment<'_>],
        field: &str,
        expected: &str,
        found: &str,
    ) {
        self.push_skip(
            out,
            "KR-AG-018",
            fatal,
            self.at(path, Anchor::Value),
            |effect| {
                t!(
                    "rules.kr_ag_018.message",
                    field = field,
                    expected = expected,
                    found = found,
                    effect = effect
                )
                .to_string()
            },
            t!("rules.kr_ag_018.suggestion"),
        );
    }

    /// `null` is fatal in `null_fatal` (if any); any other wrong type in `fatal`.
    #[allow(clippy::too_many_arguments)]
    fn expect_shape(
        &self,
        out: &mut Vec<Diagnostic>,
        key: &str,
        value: &Value,
        expected: &str,
        matches: impl Fn(&Value) -> bool,
        fatal: Fatal,
        null_fatal: Option<Fatal>,
    ) {
        if value.is_null() {
            if let Some(null_fatal) = null_fatal {
                self.push_type(out, null_fatal, &[K(key)], key, expected, "null");
            }
            return;
        }
        if !matches(value) {
            self.push_type(out, fatal, &[K(key)], key, expected, json_type_name(value));
        }
    }

    fn check_field_types(&self, out: &mut Vec<Diagnostic>) {
        if !self.enabled("KR-AG-018") {
            return;
        }
        let markdown = self.doc.markdown;
        let is_string = |value: &Value| value.is_string();
        let is_bool = |value: &Value| value.is_boolean();
        for (key, value) in self.root() {
            let key = key.as_str();
            match key {
                // V3 Markdown frontmatter does not accept null for these.
                "description" | "welcomeMessage" => self.expect_shape(
                    out,
                    key,
                    value,
                    "a string",
                    is_string,
                    Fatal::Both,
                    markdown.then_some(Fatal::V3),
                ),
                "tools" => self.check_tools_type(out, value),
                "excludedTools" => {
                    self.expect_string_array(out, key, value, Fatal::V3, Some(Fatal::V3))
                }
                "allowedTools" => self.expect_string_array(out, key, value, Fatal::V2, None),
                "toolAliases" => {
                    self.expect_shape(
                        out,
                        key,
                        value,
                        "an object",
                        Value::is_object,
                        Fatal::V2,
                        None,
                    );
                    for (alias_key, alias) in value.as_object().into_iter().flatten() {
                        if !alias.is_string() {
                            self.push_type(
                                out,
                                Fatal::V2,
                                &[K(key), K(alias_key)],
                                &format!("toolAliases.{alias_key}"),
                                "a string",
                                json_type_name(alias),
                            );
                        }
                    }
                }
                "toolsSettings" => {
                    self.expect_shape(
                        out,
                        key,
                        value,
                        "an object",
                        Value::is_object,
                        Fatal::V2,
                        None,
                    );
                    self.check_tools_settings_values(out, value);
                }
                "includeMcpJson" => {
                    self.expect_shape(
                        out,
                        key,
                        value,
                        "a boolean",
                        is_bool,
                        Fatal::Both,
                        Some(Fatal::V3),
                    );
                }
                "useLegacyMcpJson" => {
                    self.expect_shape(out, key, value, "a boolean", is_bool, Fatal::V2, None);
                }
                "includePowers" => {
                    self.expect_shape(
                        out,
                        key,
                        value,
                        "a boolean",
                        is_bool,
                        Fatal::V3,
                        Some(Fatal::V3),
                    );
                }
                "keyboardShortcut" | "$schema" => {
                    self.expect_shape(out, key, value, "a string", is_string, Fatal::V2, None);
                }
                "effortLevel" => {
                    self.expect_shape(
                        out,
                        key,
                        value,
                        "a string",
                        is_string,
                        Fatal::V3,
                        Some(Fatal::V3),
                    );
                }
                "dispatchKind" => self.expect_shape(
                    out,
                    key,
                    value,
                    "one of sub-agent, custom-agent, spec",
                    |value| {
                        value
                            .as_str()
                            .is_some_and(|kind| DISPATCH_KINDS.contains(&kind))
                    },
                    Fatal::V3,
                    Some(Fatal::V3),
                ),
                "resources" => self.expect_shape(
                    out,
                    key,
                    value,
                    "an array",
                    Value::is_array,
                    Fatal::Both,
                    Some(Fatal::V3),
                ),
                "hooks" => self.expect_shape(
                    out,
                    key,
                    value,
                    "an array of hooks or an object of trigger arrays",
                    |value| value.is_array() || value.is_object(),
                    Fatal::Both,
                    Some(Fatal::V3),
                ),
                _ => {}
            }
        }
    }

    fn expect_string_array(
        &self,
        out: &mut Vec<Diagnostic>,
        key: &str,
        value: &Value,
        fatal: Fatal,
        null_fatal: Option<Fatal>,
    ) {
        self.expect_shape(
            out,
            key,
            value,
            "an array of strings",
            Value::is_array,
            fatal,
            null_fatal,
        );
        for (index, item) in value.as_array().into_iter().flatten().enumerate() {
            if !item.is_string() {
                self.push_type(
                    out,
                    fatal,
                    &[K(key), I(index)],
                    &format!("{key}[{index}]"),
                    "a string",
                    json_type_name(item),
                );
            }
        }
    }

    fn check_tools_type(&self, out: &mut Vec<Diagnostic>, value: &Value) {
        let expected = if self.doc.markdown {
            "a string or an array of strings"
        } else {
            "\"*\" or an array of strings"
        };
        match value {
            Value::Array(_) => self.expect_string_array(out, "tools", value, Fatal::Both, None),
            // V3 Markdown splits a string on commas; V3 JSON accepts only "*".
            Value::String(_) if self.doc.markdown => {}
            Value::String(text) if text == "*" => {
                self.push_type(
                    out,
                    Fatal::V2,
                    &[K("tools")],
                    "tools",
                    "an array of strings",
                    "the string \"*\"",
                );
            }
            Value::Null => self.push_type(out, Fatal::V3, &[K("tools")], "tools", expected, "null"),
            other => self.push_type(
                out,
                Fatal::Both,
                &[K("tools")],
                "tools",
                expected,
                json_type_name(other),
            ),
        }
    }

    /// CLI 2.x only warns when a `toolsSettings` entry is not an object.
    fn check_tools_settings_values(&self, out: &mut Vec<Diagnostic>, value: &Value) {
        if !self.engines.v2 {
            return;
        }
        for (tool, settings) in value.as_object().into_iter().flatten() {
            if settings.is_object() {
                continue;
            }
            out.push(
                self.diag(
                    Warning,
                    "KR-AG-018",
                    self.at(&[K("toolsSettings"), K(tool)], Anchor::Value),
                    t!(
                        "rules.kr_ag_018.tools_settings",
                        tool = tool.as_str(),
                        found = json_type_name(settings)
                    ),
                )
                .with_suggestion(t!("rules.kr_ag_018.suggestion")),
            );
        }
    }

    // ----- KR-AG-001 ---------------------------------------------------------

    fn check_fields(&self, out: &mut Vec<Diagnostic>) {
        if !self.enabled("KR-AG-001") {
            return;
        }
        let format = self.doc.format();
        let has_permissions = self
            .root()
            .get("permissions")
            .is_some_and(|value| !value.is_null());
        for key in self.root().keys() {
            let offset = self.at(&[K(key)], Anchor::Key);
            let field = key.as_str();
            let finding = match field_origin(field) {
                None => Some((
                    Warning,
                    t!(
                        "rules.kr_ag_001.unknown",
                        field = field,
                        format = format.as_str()
                    ),
                    t!("rules.kr_ag_001.suggestion"),
                )),
                Some(FieldOrigin::Shared) if self.doc.markdown && field == "prompt" => Some((
                    Warning,
                    t!("rules.kr_ag_001.markdown_prompt"),
                    t!("rules.kr_ag_001.markdown_prompt_suggestion"),
                )),
                Some(FieldOrigin::V2)
                    if !self.doc.markdown
                        && (self.engines.v3 || self.migration)
                        && self.engines.v2
                        && matches!(
                            field,
                            "toolAliases" | "keyboardShortcut" | "useLegacyMcpJson"
                        ) =>
                {
                    let legacy_include = field == "useLegacyMcpJson"
                        && self.root().get(field) == Some(&Value::Bool(true))
                        && !self.root().contains_key("includeMcpJson");
                    // useLegacyMcpJson: false (or alongside includeMcpJson,
                    // a KR-AG-018 duplicate) changes nothing in either engine.
                    if field == "useLegacyMcpJson" && !legacy_include {
                        None
                    } else if legacy_include {
                        Some((
                            if self.engines.v3 { Warning } else { Info },
                            t!("rules.kr_ag_001.legacy_mcp_json"),
                            t!("rules.kr_ag_001.legacy_mcp_json_suggestion"),
                        ))
                    } else {
                        Some((
                            Info,
                            t!("rules.kr_ag_001.v2_field_universal", field = field),
                            t!("rules.kr_ag_001.v2_field_universal_suggestion"),
                        ))
                    }
                }
                Some(FieldOrigin::V2) if !self.engines.v2 => {
                    let trust = matches!(field, "allowedTools" | "toolsSettings");
                    if self.doc.markdown {
                        Some((
                            Warning,
                            t!("rules.kr_ag_001.v2_field_markdown", field = field),
                            if trust {
                                t!("rules.kr_ag_001.trust_suggestion")
                            } else {
                                t!("rules.kr_ag_001.remove_suggestion")
                            },
                        ))
                    } else if trust && !has_permissions {
                        // KR-AG-015 reports that V3 skips this profile.
                        None
                    } else {
                        Some((
                            Info,
                            t!("rules.kr_ag_001.v2_field", field = field),
                            t!("rules.kr_ag_001.remove_suggestion"),
                        ))
                    }
                }
                Some(FieldOrigin::V3) if !self.engines.v3 => Some((
                    Info,
                    t!("rules.kr_ag_001.v3_field", field = field),
                    t!("rules.kr_ag_001.remove_suggestion"),
                )),
                _ => None,
            };
            if let Some((level, message, suggestion)) = finding {
                out.push(
                    self.diag(level, "KR-AG-001", offset, message)
                        .with_suggestion(suggestion),
                );
            }
        }
    }

    // ----- KR-AG-008, KR-AG-019, KR-AG-022 (reserved ids) --------------------

    fn v3_agent_id(&self) -> Option<String> {
        match self.root().get("name") {
            Some(Value::String(name)) if !name.is_empty() => Some(name.clone()),
            _ => agents_location(self.doc.path).map(|(_, rest)| path_agent_id(&rest)),
        }
    }

    fn check_name(&self, out: &mut Vec<Diagnostic>) {
        let name = self.root().get("name");
        let offset = self.at(&[K("name")], Anchor::Value);
        match name {
            None if !self.doc.markdown && self.enabled("KR-AG-019") => {
                let id = self.v3_agent_id().unwrap_or_default();
                self.push_skip(
                    out,
                    "KR-AG-019",
                    Fatal::V2,
                    self.doc.root_offset,
                    |effect| {
                        if self.engines.v3 {
                            t!(
                                "rules.kr_ag_019.message_v3",
                                effect = effect,
                                id = id.as_str()
                            )
                            .to_string()
                        } else {
                            t!("rules.kr_ag_019.message", effect = effect).to_string()
                        }
                    },
                    t!("rules.kr_ag_019.suggestion"),
                );
            }
            Some(Value::String(text)) if self.enabled("KR-AG-008") => {
                if text.is_empty() {
                    if self.skip_effect(Fatal::V3).is_some() {
                        self.push_skip(
                            out,
                            "KR-AG-008",
                            Fatal::V3,
                            offset,
                            |effect| t!("rules.kr_ag_008.empty", effect = effect).to_string(),
                            t!("rules.kr_ag_008.suggestion"),
                        );
                    } else {
                        out.push(
                            self.diag(Warning, "KR-AG-008", offset, t!("rules.kr_ag_008.empty_v2"))
                                .with_suggestion(t!("rules.kr_ag_008.suggestion")),
                        );
                    }
                } else if text.trim().is_empty() {
                    out.push(
                        self.diag(Warning, "KR-AG-008", offset, t!("rules.kr_ag_008.blank"))
                            .with_suggestion(t!("rules.kr_ag_008.suggestion")),
                    );
                }
            }
            Some(other) if !other.is_string() && self.enabled("KR-AG-008") => {
                let found = json_type_name(other);
                self.push_skip(
                    out,
                    "KR-AG-008",
                    Fatal::Both,
                    offset,
                    |effect| t!("rules.kr_ag_008.type", found = found, effect = effect).to_string(),
                    t!("rules.kr_ag_008.suggestion"),
                );
            }
            _ => {}
        }

        if self.enabled("KR-AG-022")
            && let Some(id) = self.v3_agent_id()
            && V3_RESERVED_AGENT_IDS.contains(&id.as_str())
        {
            let offset = if name.is_some() {
                offset
            } else {
                self.doc.root_offset
            };
            self.push_skip(
                out,
                "KR-AG-022",
                Fatal::V3,
                offset,
                |effect| {
                    t!(
                        "rules.kr_ag_022.reserved",
                        id = id.as_str(),
                        effect = effect
                    )
                    .to_string()
                },
                t!("rules.kr_ag_022.reserved_suggestion"),
            );
        }
    }

    // ----- KR-AG-022 (duplicates) --------------------------------------------

    /// The other agent files below this file's `.kiro/agents` directory.
    fn siblings(&self) -> &[Sibling] {
        self.siblings.get_or_init(|| {
            let Some((agents_dir, rest)) = agents_location(self.doc.path) else {
                return Vec::new();
            };
            let own_rel = rest.join("/");
            let fs = self.config.fs();
            let mut siblings = Vec::new();
            let mut pending = vec![(agents_dir, Vec::<String>::new())];
            while let Some((directory, prefix)) = pending.pop() {
                if prefix.len() > MAX_AGENT_DIR_DEPTH {
                    continue;
                }
                let Ok(entries) = fs.read_dir(&directory) else {
                    continue;
                };
                for entry in entries {
                    let Some(file_name) = entry.path.file_name().and_then(|name| name.to_str())
                    else {
                        continue;
                    };
                    let mut rel = prefix.clone();
                    rel.push(file_name.to_string());
                    if entry.metadata.is_dir {
                        pending.push((entry.path.clone(), rel));
                        continue;
                    }
                    let markdown = file_name.ends_with(".md");
                    if !entry.metadata.is_file
                        || rel.join("/") == own_rel
                        || !(markdown || file_name.ends_with(".json"))
                    {
                        continue;
                    }
                    let Ok(text) = fs.read_to_string(&entry.path) else {
                        continue;
                    };
                    if let Some(identity) = sibling_identity(&text, markdown) {
                        siblings.push(Sibling {
                            rel,
                            markdown,
                            identity,
                        });
                    }
                }
            }
            siblings.sort_by(|a, b| a.rel.cmp(&b.rel));
            siblings
        })
    }

    /// A JSON profile V3 skips (KR-AG-015) never occupies its id there.
    fn v3_skips_self(&self) -> bool {
        !self.doc.markdown
            && (self.root().contains_key("allowedTools")
                || self.root().contains_key("toolsSettings"))
            && self.root().get("permissions").is_none_or(Value::is_null)
    }

    fn check_duplicate_names(&self, out: &mut Vec<Diagnostic>) {
        if !self.enabled("KR-AG-022") {
            return;
        }
        let Some((_, rest)) = agents_location(self.doc.path) else {
            return;
        };
        let v2_name = self
            .root()
            .get("name")
            .and_then(Value::as_str)
            .filter(|_| !self.doc.markdown && rest.len() == 1)
            .map(ToString::to_string);
        let v3_id = self.v3_agent_id().filter(|_| !self.v3_skips_self());
        let offset = if self.root().contains_key("name") {
            self.at(&[K("name")], Anchor::Value)
        } else {
            self.doc.root_offset
        };
        for sibling in self.siblings() {
            let v2 = self.engines.v2
                && !sibling.markdown
                && sibling.rel.len() == 1
                && v2_name.is_some()
                && sibling.identity.name.as_deref() == v2_name.as_deref();
            let v3 = self.engines.v3
                && !sibling.identity.v3_skipped
                && v3_id.as_deref() == Some(sibling.v3_id().as_str());
            let (key, name) = match (v2, v3) {
                (true, true) => ("rules.kr_ag_022.duplicate_both", v2_name.clone()),
                (true, false) => ("rules.kr_ag_022.duplicate_v2", v2_name.clone()),
                (false, true) => ("rules.kr_ag_022.duplicate_v3", v3_id.clone()),
                (false, false) => continue,
            };
            let other = sibling.rel.join("/");
            let diagnostic = self
                .diag(
                    Warning,
                    "KR-AG-022",
                    offset,
                    t!(
                        key,
                        name = name.unwrap_or_default().as_str(),
                        other = other.as_str()
                    ),
                )
                .with_suggestion(t!("rules.kr_ag_022.duplicate_suggestion"));
            out.push(self.assume(diagnostic));
        }
    }

    // ----- KR-AG-006 -----------------------------------------------------------

    /// Names an agent can spawn: the agent files of this `.kiro/agents`
    /// directory (by `name` and by V3 id), this agent, and Kiro's built-ins.
    fn known_agent_names(&self) -> HashSet<String> {
        let mut known: HashSet<String> = KNOWN_BUILT_IN_AGENTS
            .iter()
            .chain(V3_RESERVED_AGENT_IDS)
            .map(|name| name.to_string())
            .collect();
        for sibling in self.siblings() {
            known.extend(sibling.identity.name.clone());
            known.insert(sibling.v3_id());
        }
        known.extend(
            self.root()
                .get("name")
                .and_then(Value::as_str)
                .map(ToString::to_string),
        );
        known.extend(self.v3_agent_id());
        known
    }

    /// Sub-agent access (spec 1.4 toolsSettings.subagent, 7.2 `subagent/<agent>`):
    /// an orchestrator needs a subagent tool, and its agent lists should name
    /// agents that exist.
    fn check_subagents(&self, out: &mut Vec<Diagnostic>) {
        if !self.enabled("KR-AG-006") || self.doc.markdown {
            return;
        }
        let settings = self.root().get("toolsSettings").and_then(Value::as_object);
        let subagent_key = settings.and_then(|settings| {
            settings
                .keys()
                .find(|key| {
                    matches!(
                        key.as_str(),
                        "subagent" | "use_subagent" | "agent_crew" | "agentCrew"
                    )
                })
                .cloned()
        });
        let tools = self.tools_list().unwrap_or_default();
        if self.engines.v2
            && let Some(key) = &subagent_key
            && !tools.iter().any(|tool| {
                matches!(
                    tool.as_str(),
                    "*" | "@builtin" | "subagent" | "use_subagent" | "agent_crew" | "agentCrew"
                ) || tool.starts_with("subagent/")
            })
        {
            let diagnostic = self
                .diag(
                    Warning,
                    "KR-AG-006",
                    self.at(&[K("toolsSettings"), K(key)], Anchor::Key),
                    t!("rules.kr_ag_006.no_tool", key = key.as_str()),
                )
                .with_suggestion(t!("rules.kr_ag_006.no_tool_suggestion"));
            out.push(self.assume(diagnostic));
        }

        if agents_location(self.doc.path).is_none() {
            return;
        }
        let mut references: Vec<(Vec<PathSegment<'_>>, String, String)> = Vec::new();
        if self.engines.v2
            && let (Some(settings), Some(key)) = (settings, &subagent_key)
            && let Some(lists) = settings.get(key).and_then(Value::as_object)
        {
            for list in ["availableAgents", "trustedAgents"] {
                for (index, name) in json_string_array(lists.get(list)) {
                    references.push((
                        vec![K("toolsSettings"), K(key), K(list), I(index)],
                        format!("toolsSettings.{key}.{list}"),
                        name,
                    ));
                }
            }
        }
        if self.engines.v3 {
            for (index, tool) in json_string_array(self.root().get("tools")) {
                if let Some(name) = tool.strip_prefix("subagent/") {
                    references.push((
                        vec![K("tools"), I(index)],
                        "tools".to_string(),
                        name.to_string(),
                    ));
                }
            }
        }
        if references.is_empty() {
            return;
        }
        let known = self.known_agent_names();
        for (path, field, name) in references {
            let matched = if has_glob(&name) {
                known.iter().any(|agent| glob_matches(&name, agent))
            } else {
                known.contains(&name)
            };
            if matched {
                continue;
            }
            let path: Vec<PathSegment<'_>> = path;
            out.push(
                self.diag(
                    Info,
                    "KR-AG-006",
                    self.at(&path, Anchor::Value),
                    t!(
                        "rules.kr_ag_006.unknown_agent",
                        field = field.as_str(),
                        agent = name.as_str()
                    ),
                )
                .with_suggestion(t!("rules.kr_ag_006.unknown_agent_suggestion")),
            );
        }
    }

    // ----- KR-AG-029, KR-AG-030 -------------------------------------------------

    fn check_resource_breadth(&self, out: &mut Vec<Diagnostic>) {
        if !self.enabled("KR-AG-029") {
            return;
        }
        for (index, resource) in json_string_array(self.root().get("resources")) {
            let Some(path) = resource.strip_prefix("file://") else {
                continue;
            };
            let path = path.trim_start_matches("./");
            if matches!(path, "**" | "*" | "**/*" | "**/**" | "**/*.*") {
                out.push(
                    self.diag(
                        Info,
                        "KR-AG-029",
                        self.at(&[K("resources"), I(index)], Anchor::Value),
                        t!("rules.kr_ag_029.message", resource = resource.as_str()),
                    )
                    .with_suggestion(t!("rules.kr_ag_029.suggestion")),
                );
            }
        }
    }

    fn check_description(&self, out: &mut Vec<Diagnostic>) {
        if !self.enabled("KR-AG-030") {
            return;
        }
        // Only agents another agent lists as a sub-agent: the orchestrator
        // picks among them by description (the docs also call the field
        // "primarily for human readability", so standalone agents are fine).
        let names: Vec<String> = self
            .root()
            .get("name")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .into_iter()
            .chain(self.v3_agent_id())
            .collect();
        let Some(orchestrator) = self.siblings().iter().find(|sibling| {
            sibling.identity.subagent_refs.iter().any(|reference| {
                names.iter().any(|name| {
                    if has_glob(reference) {
                        glob_matches(reference, name)
                    } else {
                        reference == name
                    }
                })
            })
        }) else {
            return;
        };
        let orchestrator = orchestrator.rel.join("/");
        let (key, offset) = match self.root().get("description") {
            None | Some(Value::Null) => ("rules.kr_ag_030.missing", self.doc.root_offset),
            Some(Value::String(text))
                if text.split_whitespace().count() < MIN_DESCRIPTION_WORDS =>
            {
                (
                    "rules.kr_ag_030.short",
                    self.at(&[K("description")], Anchor::Value),
                )
            }
            _ => return,
        };
        out.push(
            self.diag(
                Info,
                "KR-AG-030",
                offset,
                t!(key, orchestrator = orchestrator.as_str()),
            )
            .with_suggestion(t!("rules.kr_ag_030.suggestion")),
        );
    }

    // ----- KR-AG-009, KR-AG-021, KR-AG-013 -----------------------------------

    fn check_prompt(&self, out: &mut Vec<Diagnostic>) {
        if self.doc.markdown {
            let Some((body_offset, body)) = &self.doc.body else {
                return;
            };
            if body.is_empty() {
                if self.enabled("KR-AG-009") {
                    out.push(
                        self.diag(
                            Warning,
                            "KR-AG-009",
                            *body_offset,
                            t!("rules.kr_ag_009.empty_body"),
                        )
                        .with_suggestion(t!("rules.kr_ag_009.empty_body_suggestion")),
                    );
                }
                return;
            }
            self.check_secrets(out, body, Some(*body_offset));
            return;
        }

        let Some(prompt) = self.root().get("prompt") else {
            return;
        };
        let offset = self.at(&[K("prompt")], Anchor::Value);
        match prompt {
            Value::Null => {}
            Value::String(text) if text.trim().is_empty() => {
                if self.enabled("KR-AG-009") {
                    out.push(
                        self.diag(Warning, "KR-AG-009", offset, t!("rules.kr_ag_009.empty"))
                            .with_suggestion(t!("rules.kr_ag_009.suggestion")),
                    );
                }
            }
            Value::String(text) => {
                if let Some(target) = text.strip_prefix("file://") {
                    self.check_file_prompt(out, target, offset);
                } else {
                    self.check_secrets(out, text, None);
                }
            }
            other => {
                if self.enabled("KR-AG-009") {
                    let found = json_type_name(other);
                    self.push_skip(
                        out,
                        "KR-AG-009",
                        Fatal::Both,
                        offset,
                        |effect| {
                            t!("rules.kr_ag_009.type", found = found, effect = effect).to_string()
                        },
                        t!("rules.kr_ag_009.suggestion"),
                    );
                }
            }
        }
    }

    fn check_file_prompt(&self, out: &mut Vec<Diagnostic>, target: &str, offset: usize) {
        if !self.enabled("KR-AG-021") {
            return;
        }
        if target.is_empty() {
            self.push_skip(
                out,
                "KR-AG-021",
                Fatal::Both,
                offset,
                |effect| t!("rules.kr_ag_021.empty", effect = effect).to_string(),
                t!("rules.kr_ag_021.suggestion"),
            );
            return;
        }
        if target.starts_with('~') || is_absolute_like(target) {
            self.push_skip(
                out,
                "KR-AG-021",
                Fatal::V3,
                offset,
                |effect| {
                    t!("rules.kr_ag_021.absolute", target = target, effect = effect).to_string()
                },
                t!("rules.kr_ag_021.suggestion"),
            );
            return;
        }
        let agent_dir = self.doc.path.parent().unwrap_or_else(|| Path::new(""));
        let resolved = agent_dir.join(target);
        let normalized: PathBuf = normalize_components(&resolved).iter().collect();
        if !self.config.fs().is_file(&normalized) {
            let shown = normalized.display().to_string();
            self.push_skip(
                out,
                "KR-AG-021",
                Fatal::Both,
                offset,
                |effect| {
                    t!(
                        "rules.kr_ag_021.missing",
                        target = target,
                        resolved = shown.as_str(),
                        effect = effect
                    )
                    .to_string()
                },
                t!("rules.kr_ag_021.missing_suggestion"),
            );
            return;
        }
        if let Some(root) = containment_root(self.doc.path)
            && escapes_root(&resolved, &root)
        {
            self.push_skip(
                out,
                "KR-AG-021",
                Fatal::V3,
                offset,
                |effect| {
                    t!("rules.kr_ag_021.escapes", target = target, effect = effect).to_string()
                },
                t!("rules.kr_ag_021.suggestion"),
            );
        }
    }

    fn check_secrets(&self, out: &mut Vec<Diagnostic>, prompt: &str, body_offset: Option<usize>) {
        if !self.enabled("KR-AG-013") {
            return;
        }
        let Some(found) = agent_secret_pattern().find(prompt) else {
            return;
        };
        let offset = match body_offset {
            Some(start) => start + found.start(),
            None => self.at(&[K("prompt")], Anchor::Value),
        };
        out.push(
            self.diag(Error, "KR-AG-013", offset, t!("rules.kr_ag_013.message"))
                .with_suggestion(t!("rules.kr_ag_013.suggestion")),
        );
    }

    // ----- KR-AG-004 ---------------------------------------------------------

    fn check_model(&self, out: &mut Vec<Diagnostic>) {
        if !self.enabled("KR-AG-004") {
            return;
        }
        let Some(model) = self.root().get("model") else {
            return;
        };
        let offset = self.at(&[K("model")], Anchor::Value);
        match model {
            // JSON accepts null (unset); V3 Markdown frontmatter does not.
            Value::Null if self.doc.markdown => self.push_skip(
                out,
                "KR-AG-004",
                Fatal::V3,
                offset,
                |effect| t!("rules.kr_ag_004.type", found = "null", effect = effect).to_string(),
                t!("rules.kr_ag_004.type_suggestion"),
            ),
            Value::Null => {}
            Value::String(text) => {
                let trimmed = text.trim();
                let lowered = trimmed.to_ascii_lowercase();
                let base = lowered.strip_suffix("[1m]").unwrap_or(&lowered);
                let key = if CLAUDE_CODE_MODEL_ALIASES.contains(&base) {
                    "rules.kr_ag_004.alias"
                } else if trimmed.contains(char::is_whitespace) {
                    "rules.kr_ag_004.display_name"
                } else {
                    return;
                };
                out.push(
                    self.diag(Warning, "KR-AG-004", offset, t!(key, model = text.as_str()))
                        .with_suggestion(t!("rules.kr_ag_004.alias_suggestion")),
                );
            }
            other => {
                let found = json_type_name(other);
                self.push_skip(
                    out,
                    "KR-AG-004",
                    Fatal::Both,
                    offset,
                    |effect| t!("rules.kr_ag_004.type", found = found, effect = effect).to_string(),
                    t!("rules.kr_ag_004.type_suggestion"),
                );
            }
        }
    }

    // ----- KR-AG-002 ---------------------------------------------------------

    fn check_resources(&self, out: &mut Vec<Diagnostic>) {
        if !self.enabled("KR-AG-002") {
            return;
        }
        let Some(items) = self.root().get("resources").and_then(Value::as_array) else {
            return;
        };
        for (index, item) in items.iter().enumerate() {
            let offset = self.at(&[K("resources"), I(index)], Anchor::Value);
            let resource = short_json(item);
            let problem = match item {
                Value::String(uri) => (!(uri.starts_with("file://")
                    || uri.starts_with("skill://")))
                .then(|| t!("rules.kr_ag_002.bad_scheme").to_string()),
                Value::Object(object) => match object.get("type").and_then(Value::as_str) {
                    Some("knowledgeBase") => {
                        self.check_knowledge_base(out, object, offset, &resource);
                        None
                    }
                    Some(kind) => Some(t!("rules.kr_ag_002.bad_type", kind = kind).to_string()),
                    None => Some(t!("rules.kr_ag_002.missing_type").to_string()),
                },
                other => {
                    Some(t!("rules.kr_ag_002.bad_value", found = json_type_name(other)).to_string())
                }
            };
            let Some(problem) = problem else {
                continue;
            };
            if self.engines.v2 {
                let v3 = self.engines.v3;
                self.push_skip(
                    out,
                    "KR-AG-002",
                    Fatal::V2,
                    offset,
                    |effect| {
                        let effect = if v3 {
                            format!("{effect}; {}", t!("rules.kr_ag_002.v3_drops"))
                        } else {
                            effect.to_string()
                        };
                        t!(
                            "rules.kr_ag_002.message",
                            resource = resource.as_str(),
                            problem = problem.as_str(),
                            effect = effect
                        )
                        .to_string()
                    },
                    t!("rules.kr_ag_002.suggestion"),
                );
            } else {
                out.push(
                    self.diag(
                        Warning,
                        "KR-AG-002",
                        offset,
                        t!(
                            "rules.kr_ag_002.message",
                            resource = resource.as_str(),
                            problem = problem.as_str(),
                            effect = t!("rules.kr_ag_002.v3_drops_only")
                        ),
                    )
                    .with_suggestion(t!("rules.kr_ag_002.suggestion")),
                );
            }
        }
    }

    fn check_knowledge_base(
        &self,
        out: &mut Vec<Diagnostic>,
        object: &Map<String, Value>,
        offset: usize,
        resource: &str,
    ) {
        if !self.engines.v3 {
            return;
        }
        let source_ok = object
            .get("source")
            .and_then(Value::as_str)
            .and_then(|source| source.strip_prefix("file://"))
            .is_some_and(|path| !path.is_empty());
        let is_string_array = |value: &Value| {
            value
                .as_array()
                .is_some_and(|items| items.iter().all(Value::is_string))
        };
        let problem = if !source_ok {
            Some("source must be a file:// URI with a non-empty path".to_string())
        } else if let Some(index_type) = object.get("indexType")
            && !matches!(index_type.as_str(), Some("best" | "fast"))
        {
            Some(format!(
                "indexType must be best or fast, found {}",
                short_json(index_type)
            ))
        } else if let Some(auto_update) = object.get("autoUpdate")
            && !auto_update.is_boolean()
        {
            Some(format!(
                "autoUpdate must be a boolean, found {}",
                json_type_name(auto_update)
            ))
        } else if let Some((field, _)) = ["name", "description"]
            .iter()
            .filter_map(|field| object.get(*field).map(|value| (*field, value)))
            .find(|(_, value)| !value.is_string())
        {
            Some(format!("{field} must be a string"))
        } else {
            ["include", "exclude"]
                .iter()
                .find(|field| {
                    object
                        .get(**field)
                        .is_some_and(|value| !is_string_array(value))
                })
                .map(|field| format!("{field} must be an array of strings"))
        };
        if let Some(problem) = problem {
            out.push(
                self.diag(
                    Warning,
                    "KR-AG-002",
                    offset,
                    t!(
                        "rules.kr_ag_002.message",
                        resource = resource,
                        problem = problem.as_str(),
                        effect = t!("rules.kr_ag_002.v3_drops_only")
                    ),
                )
                .with_suggestion(t!("rules.kr_ag_002.kb_suggestion")),
            );
        }
    }

    // ----- KR-AG-010, KR-AG-011, KR-AG-024 (tools) ---------------------------

    fn check_tools(&self, out: &mut Vec<Diagnostic>) {
        let Some(tools_value) = self.root().get("tools") else {
            return;
        };
        if let Some(items) = tools_value.as_array()
            && items.is_empty()
            && self.enabled("KR-AG-011")
        {
            out.push(
                self.diag(
                    Info,
                    "KR-AG-011",
                    self.at(&[K("tools")], Anchor::Value),
                    t!("rules.kr_ag_011.message"),
                )
                .with_suggestion(t!("rules.kr_ag_011.suggestion")),
            );
        }
        let Some(tools) = self.tools_list() else {
            return;
        };
        // Array entries keep their index for locations; a Markdown string
        // points at the key.
        let entries: Vec<(Option<usize>, String)> = match tools_value {
            Value::Array(_) => json_string_array(Some(tools_value))
                .into_iter()
                .map(|(index, tool)| (Some(index), tool))
                .collect(),
            _ => tools.iter().map(|tool| (None, tool.clone())).collect(),
        };
        let locate = |index: Option<usize>| match index {
            Some(index) => self.at(&[K("tools"), I(index)], Anchor::Value),
            None => self.at(&[K("tools")], Anchor::Value),
        };

        if self.enabled("KR-AG-010") {
            let mut seen = HashSet::new();
            for (index, tool) in &entries {
                if !seen.insert(tool.to_ascii_lowercase()) {
                    out.push(
                        self.diag(
                            Warning,
                            "KR-AG-010",
                            locate(*index),
                            t!("rules.kr_ag_010.message", tool = tool.as_str()),
                        )
                        .with_suggestion(t!("rules.kr_ag_010.suggestion")),
                    );
                }
            }
        }

        if !self.enabled("KR-AG-024") {
            return;
        }
        for (index, tool) in &entries {
            let tool = tool.as_str();
            if tool == "*"
                || has_glob(tool)
                || tool.starts_with('@')
                || tool.starts_with("mcp_") && !tool.starts_with("mcp__")
                || tool.starts_with("subagent/")
                || tool.starts_with("c2s_")
            {
                continue;
            }
            let offset = locate(*index);
            if let Some(message) = foreign_tool_name(tool, "tools") {
                out.push(
                    self.diag(Warning, "KR-AG-024", offset, message)
                        .with_suggestion(t!("rules.kr_ag_024.suggestion")),
                );
                continue;
            }
            if self.engines.v3 || self.migration {
                let level = if self.engines.v3 { Warning } else { Info };
                match v3_replacement_for_v2_tool(tool) {
                    Some(V3Replacement::Selector(selector))
                        if !v3_tools_select(&tools, selector)
                            && (self.engines.v3 || !upgrade_covers(&tools, tool)) =>
                    {
                        let diagnostic = self
                            .diag(
                                level,
                                "KR-AG-024",
                                offset,
                                t!(
                                    "rules.kr_ag_024.v2_name_in_v3",
                                    tool = tool,
                                    selector = selector
                                ),
                            )
                            .with_suggestion(t!("rules.kr_ag_024.suggestion"));
                        out.push(self.assume(diagnostic));
                    }
                    Some(V3Replacement::McpServer) => {
                        let diagnostic = self
                            .diag(
                                level,
                                "KR-AG-024",
                                offset,
                                t!("rules.kr_ag_024.aws", tool = tool),
                            )
                            .with_suggestion(t!("rules.kr_ag_024.aws_suggestion"));
                        out.push(self.assume(diagnostic));
                    }
                    _ => {}
                }
            }
            if self.engines.v2
                && let Some(equivalent) = v2_equivalent_for_v3_tool(tool)
                && !v2_tools_select(&tools, equivalent)
            {
                let diagnostic = self
                    .diag(
                        Warning,
                        "KR-AG-024",
                        offset,
                        t!(
                            "rules.kr_ag_024.v3_id_in_v2",
                            tool = tool,
                            equivalent = equivalent
                        ),
                    )
                    .with_suggestion(t!("rules.kr_ag_024.suggestion"));
                out.push(self.assume(diagnostic));
            }
        }
    }

    // ----- KR-AG-003, KR-AG-024 (allowedTools) -------------------------------

    fn alias_targets(&self) -> Vec<(String, String)> {
        self.root()
            .get("toolAliases")
            .and_then(Value::as_object)
            .map(|aliases| {
                aliases
                    .iter()
                    .filter_map(|(original, alias)| {
                        alias
                            .as_str()
                            .map(|alias| (original.clone(), alias.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The upgrader's deprecated-aws-tool: AWS trust in `allowedTools` or
    /// `toolsSettings` has no V3 equivalent (V3 has no AWS tool, and the
    /// upgrader's `use_aws` capability is skipped as unknown). Reported once,
    /// unless `tools` already names the AWS tool (reported there).
    fn check_aws_trust(&self, out: &mut Vec<Diagnostic>) {
        if !self.enabled("KR-AG-024") || !(self.engines.v3 || self.migration) || self.doc.markdown {
            return;
        }
        let is_aws = |name: &str| matches!(name, "aws" | "use_aws");
        if self
            .tools_list()
            .is_some_and(|tools| tools.iter().any(|tool| is_aws(tool)))
        {
            return;
        }
        let path = json_string_array(self.root().get("allowedTools"))
            .into_iter()
            .find(|(_, entry)| is_aws(entry))
            .map(|(index, entry)| (vec![K("allowedTools"), I(index)], entry))
            .or_else(|| {
                self.root()
                    .get("toolsSettings")
                    .and_then(Value::as_object)
                    .and_then(|settings| settings.keys().find(|key| is_aws(key)))
                    .map(|key| (vec![K("toolsSettings"), K(key)], key.clone()))
            });
        let Some((path, name)) = path else {
            return;
        };
        let level = if self.engines.v3 { Warning } else { Info };
        let diagnostic = self
            .diag(
                level,
                "KR-AG-024",
                self.at(&path, Anchor::Key),
                t!("rules.kr_ag_024.aws_trust", tool = name.as_str()),
            )
            .with_suggestion(t!("rules.kr_ag_024.aws_suggestion"));
        out.push(self.assume(diagnostic));
    }

    fn check_allowed_tools(&self, out: &mut Vec<Diagnostic>) {
        self.check_aws_trust(out);
        if !self.engines.v2 {
            return;
        }
        let allowed = json_string_array(self.root().get("allowedTools"));
        if allowed.is_empty() {
            return;
        }
        let aliases = self.alias_targets();
        let tools = self.tools_list();
        for (index, entry) in &allowed {
            let offset = self.at(&[K("allowedTools"), I(*index)], Anchor::Value);
            let entry = entry.as_str();
            let original = aliases
                .iter()
                .find(|(_, alias)| alias == entry)
                .map(|(original, _)| original.as_str());
            let native = entry.strip_prefix("@builtin/").unwrap_or(entry);
            if entry == "*" {
                if self.enabled("KR-AG-024") {
                    out.push(
                        self.diag(
                            Warning,
                            "KR-AG-024",
                            offset,
                            t!("rules.kr_ag_024.allowed_star"),
                        )
                        .with_suggestion(t!("rules.kr_ag_024.allowed_star_suggestion")),
                    );
                }
                continue;
            }
            let names_a_tool = original.is_some()
                || has_glob(entry)
                || (entry.starts_with('@') && !entry.starts_with("@builtin/"))
                || v2_tool_group(native).is_some();
            if !names_a_tool {
                if self.enabled("KR-AG-024") {
                    let message = foreign_tool_name(native, "allowedTools").unwrap_or_else(|| {
                        match v2_equivalent_for_v3_tool(native) {
                            Some(equivalent) => t!(
                                "rules.kr_ag_024.v3_id_in_allowed",
                                tool = native,
                                equivalent = equivalent
                            )
                            .to_string(),
                            None => t!("rules.kr_ag_024.unmapped", tool = entry).to_string(),
                        }
                    });
                    out.push(
                        self.diag(Warning, "KR-AG-024", offset, message)
                            .with_suggestion(t!("rules.kr_ag_024.allowed_suggestion")),
                    );
                }
                continue;
            }
            if self.enabled("KR-AG-003")
                && let Some(tools) = &tools
                && !v2_tools_cover(tools, original.unwrap_or(entry))
            {
                out.push(
                    self.diag(
                        Warning,
                        "KR-AG-003",
                        offset,
                        t!("rules.kr_ag_003.message", tool = entry),
                    )
                    .with_suggestion(t!("rules.kr_ag_003.suggestion")),
                );
            }
        }
    }

    // ----- KR-AG-012 ---------------------------------------------------------

    fn check_tool_aliases(&self, out: &mut Vec<Diagnostic>) {
        if !self.enabled("KR-AG-012") || !self.engines.v2 {
            return;
        }
        let Some(tools) = self.tools_list() else {
            return;
        };
        for (original, alias) in self.alias_targets() {
            if v2_tools_cover(&tools, &original) {
                continue;
            }
            out.push(
                self.diag(
                    Warning,
                    "KR-AG-012",
                    self.at(&[K("toolAliases"), K(&original)], Anchor::Key),
                    t!(
                        "rules.kr_ag_012.renames",
                        tool = original.as_str(),
                        alias = alias.as_str()
                    ),
                )
                .with_suggestion(t!("rules.kr_ag_012.renames_suggestion")),
            );
        }
    }

    // ----- KR-AG-005 ---------------------------------------------------------

    fn check_mcp_access(&self, out: &mut Vec<Diagnostic>) {
        if !self.enabled("KR-AG-005") || !self.engines.v2 {
            return;
        }
        let include = self
            .root()
            .get("includeMcpJson")
            .and_then(Value::as_bool)
            .or_else(|| self.root().get("useLegacyMcpJson").and_then(Value::as_bool));
        if include != Some(false) {
            return;
        }
        let inline: HashSet<&str> = self
            .root()
            .get("mcpServers")
            .and_then(Value::as_object)
            .map(|servers| servers.keys().map(String::as_str).collect())
            .unwrap_or_default();
        let mut reported = HashSet::new();
        for field in ["tools", "allowedTools"] {
            for (index, entry) in json_string_array(self.root().get(field)) {
                let Some(reference) = entry.strip_prefix('@') else {
                    continue;
                };
                let server = reference.split('/').next().unwrap_or(reference);
                if server == "builtin"
                    || server == "mcp"
                    || server.is_empty()
                    || has_glob(server)
                    || inline.contains(server)
                {
                    continue;
                }
                if !reported.insert(server.to_string()) {
                    continue;
                }
                let key = if self.engines.v3 {
                    "rules.kr_ag_005.message_both"
                } else {
                    "rules.kr_ag_005.message_v2"
                };
                let diagnostic = self
                    .diag(
                        Warning,
                        "KR-AG-005",
                        self.at(&[K(field), I(index)], Anchor::Value),
                        t!(key, server = server),
                    )
                    .with_suggestion(t!("rules.kr_ag_005.suggestion"));
                out.push(self.assume(diagnostic));
            }
        }
    }

    // ----- KR-AG-023 ---------------------------------------------------------

    fn check_mcp_servers(&self, out: &mut Vec<Diagnostic>) {
        if !self.enabled("KR-AG-023") {
            return;
        }
        let Some(servers) = self.root().get("mcpServers") else {
            return;
        };
        let Some(servers) = servers.as_object() else {
            if servers.is_null() {
                return;
            }
            let found = json_type_name(servers);
            let offset = self.at(&[K("mcpServers")], Anchor::Value);
            if self.engines.v2 {
                let v3 = self.engines.v3;
                self.push_skip(
                    out,
                    "KR-AG-023",
                    Fatal::V2,
                    offset,
                    |effect| {
                        let key = if v3 {
                            "rules.kr_ag_023.not_object_both"
                        } else {
                            "rules.kr_ag_023.not_object"
                        };
                        t!(key, found = found, effect = effect).to_string()
                    },
                    t!("rules.kr_ag_023.suggestion"),
                );
            } else {
                out.push(
                    self.diag(
                        Warning,
                        "KR-AG-023",
                        offset,
                        t!("rules.kr_ag_023.not_object_v3", found = found),
                    )
                    .with_suggestion(t!("rules.kr_ag_023.suggestion")),
                );
            }
            return;
        };

        for (name, server) in servers {
            let path = [K("mcpServers"), K(name)];
            let Some(server) = server.as_object() else {
                self.push_server_problem(
                    out,
                    &path,
                    name,
                    true,
                    format!("must be an object, found {}", json_type_name(server)),
                );
                continue;
            };
            for (field, shape, v2_typed) in MCP_SERVER_FIELDS {
                let Some(value) = server.get(*field) else {
                    continue;
                };
                if value.is_null() {
                    continue;
                }
                let ok = match shape {
                    McpFieldShape::String => value.is_string(),
                    McpFieldShape::StringArray => value
                        .as_array()
                        .is_some_and(|items| items.iter().all(Value::is_string)),
                    McpFieldShape::StringMap => value
                        .as_object()
                        .is_some_and(|map| map.values().all(Value::is_string)),
                    McpFieldShape::Bool => value.is_boolean(),
                    McpFieldShape::Number => value.is_number(),
                    McpFieldShape::Object => value.is_object(),
                };
                if !ok {
                    let expected = match shape {
                        McpFieldShape::String => "a string",
                        McpFieldShape::StringArray => "an array of strings",
                        McpFieldShape::StringMap => "an object of strings",
                        McpFieldShape::Bool => "a boolean",
                        McpFieldShape::Number => "a number",
                        McpFieldShape::Object => "an object",
                    };
                    let field_path = [K("mcpServers"), K(name), K(field)];
                    self.push_server_problem(
                        out,
                        &field_path,
                        name,
                        *v2_typed,
                        format!(
                            "{field} must be {expected}, found {}",
                            json_type_name(value)
                        ),
                    );
                }
            }
            if let Some(timeout) = server.get("timeout").and_then(Value::as_f64) {
                let field_path = [K("mcpServers"), K(name), K("timeout")];
                let v2_bad = timeout < 0.0 || timeout.fract() != 0.0;
                if v2_bad || timeout < 1.0 {
                    self.push_server_problem(
                        out,
                        &field_path,
                        name,
                        v2_bad,
                        format!(
                            "timeout must be a whole number of milliseconds >= 1, found {timeout}"
                        ),
                    );
                }
            }
            if let Some(mode) = server.get("versionNegotiation")
                && !matches!(mode.as_str(), Some("auto" | "legacy"))
            {
                let field_path = [K("mcpServers"), K(name), K("versionNegotiation")];
                self.push_server_problem(
                    out,
                    &field_path,
                    name,
                    false,
                    format!(
                        "versionNegotiation must be auto or legacy, found {}",
                        short_json(mode)
                    ),
                );
            }
            self.check_server_transport(out, name, server);
        }
    }

    /// V3 picks stdio when `command` is set, else HTTP from `url`, and marks
    /// the server invalid when neither works (spec 3.2).
    fn check_server_transport(
        &self,
        out: &mut Vec<Diagnostic>,
        name: &str,
        server: &Map<String, Value>,
    ) {
        if !self.engines.v3
            || server.get("disabled").and_then(Value::as_bool) == Some(true)
            || server.get("type").and_then(Value::as_str) == Some("registry")
            || server
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(|command| !command.is_empty())
        {
            return;
        }
        let Some(url) = server
            .get("url")
            .and_then(Value::as_str)
            .filter(|url| !url.is_empty())
        else {
            if server.get("url").is_none_or(Value::is_string)
                && server.get("command").is_none_or(Value::is_string)
            {
                self.push_server_problem(
                    out,
                    &[K("mcpServers"), K(name)],
                    name,
                    false,
                    "specifies neither command nor url".to_string(),
                );
            }
            return;
        };
        let problem = match url::Url::parse(url) {
            Err(_) => Some(format!("url '{url}' is not a valid URL")),
            Ok(parsed) if parsed.scheme() == "https" => None,
            Ok(parsed) => {
                let loopback = matches!(
                    parsed.host_str(),
                    Some("localhost" | "127.0.0.1" | "::1" | "[::1]")
                );
                (!loopback)
                    .then(|| format!("url '{url}' must use https unless it points at localhost"))
            }
        };
        if let Some(problem) = problem {
            self.push_server_problem(
                out,
                &[K("mcpServers"), K(name), K("url")],
                name,
                false,
                problem,
            );
        }
    }

    /// A server problem: `v2_fatal` problems skip the agent in CLI 2.x; every
    /// problem makes V3 skip that server.
    fn push_server_problem(
        &self,
        out: &mut Vec<Diagnostic>,
        path: &[PathSegment<'_>],
        name: &str,
        v2_fatal: bool,
        problem: String,
    ) {
        let offset = self.at(path, Anchor::Value);
        if v2_fatal && self.engines.v2 {
            let v3 = self.engines.v3;
            self.push_skip(
                out,
                "KR-AG-023",
                Fatal::V2,
                offset,
                |effect| {
                    let effect = if v3 {
                        format!("{effect}; {}", t!("rules.kr_ag_023.v3_skips_server"))
                    } else {
                        effect.to_string()
                    };
                    t!(
                        "rules.kr_ag_023.message",
                        server = name,
                        problem = problem.as_str(),
                        effect = effect
                    )
                    .to_string()
                },
                t!("rules.kr_ag_023.suggestion"),
            );
        } else if self.engines.v3 {
            out.push(
                self.diag(
                    Warning,
                    "KR-AG-023",
                    offset,
                    t!(
                        "rules.kr_ag_023.message",
                        server = name,
                        problem = problem.as_str(),
                        effect = t!("rules.kr_ag_023.v3_skips_server_only")
                    ),
                )
                .with_suggestion(t!("rules.kr_ag_023.suggestion")),
            );
        }
    }

    // ----- KR-AG-014 ---------------------------------------------------------

    fn push_permission_skip(
        &self,
        out: &mut Vec<Diagnostic>,
        path: &[PathSegment<'_>],
        issue: String,
    ) {
        self.push_skip(
            out,
            "KR-AG-014",
            Fatal::V3,
            self.at(path, Anchor::Value),
            |effect| {
                t!(
                    "rules.kr_ag_014.fatal",
                    issue = issue.as_str(),
                    effect = effect
                )
                .to_string()
            },
            t!("rules.kr_ag_014.suggestion"),
        );
    }

    fn push_permission_note(
        &self,
        out: &mut Vec<Diagnostic>,
        path: &[PathSegment<'_>],
        anchor: Anchor,
        issue: String,
    ) {
        if !self.engines.v3 {
            return;
        }
        out.push(
            self.diag(
                Warning,
                "KR-AG-014",
                self.at(path, anchor),
                t!("rules.kr_ag_014.ignored", issue = issue.as_str()),
            )
            .with_suggestion(t!("rules.kr_ag_014.suggestion")),
        );
    }

    fn check_permissions(&self, out: &mut Vec<Diagnostic>) {
        if !self.enabled("KR-AG-014") {
            return;
        }
        let Some(permissions) = self.root().get("permissions") else {
            return;
        };
        if permissions.is_null() {
            // With CLI 2.x trust fields KR-AG-015 already reports the skip.
            let has_trust = ["allowedTools", "toolsSettings"]
                .iter()
                .any(|field| self.root().contains_key(*field));
            if self.doc.markdown || !has_trust {
                self.push_permission_skip(
                    out,
                    &[K("permissions")],
                    "permissions must be an object, found null".to_string(),
                );
            }
            return;
        }
        let Some(object) = permissions.as_object() else {
            self.push_permission_skip(
                out,
                &[K("permissions")],
                format!(
                    "permissions must be an object, found {}",
                    json_type_name(permissions)
                ),
            );
            return;
        };
        for key in object.keys() {
            if key != "rules" && key != "policies" {
                self.push_permission_note(
                    out,
                    &[K("permissions"), K(key)],
                    Anchor::Key,
                    format!("V3 ignores permissions.{key}"),
                );
            }
        }
        if let Some(policies) = object.get("policies") {
            match policies.as_array() {
                Some(items) => {
                    for (index, policy) in items.iter().enumerate() {
                        let path = [K("permissions"), K("policies"), I(index)];
                        match policy.as_str() {
                            Some(id) if PERMISSION_POLICY_PRESETS.contains(&id) => {}
                            Some(id) => self.push_permission_note(
                                out,
                                &path,
                                Anchor::Value,
                                format!("V3 skips unknown policy preset '{id}'"),
                            ),
                            None => self.push_permission_skip(
                                out,
                                &path,
                                format!("permissions.policies[{index}] must be a string"),
                            ),
                        }
                    }
                }
                None => self.push_permission_skip(
                    out,
                    &[K("permissions"), K("policies")],
                    "permissions.policies must be an array of strings".to_string(),
                ),
            }
        }
        let Some(rules) = object.get("rules").and_then(Value::as_array) else {
            let issue = if object.contains_key("rules") {
                "permissions.rules must be an array"
            } else {
                "permissions needs a rules array"
            };
            self.push_permission_skip(out, &[K("permissions"), K("rules")], issue.to_string());
            return;
        };
        for (index, rule) in rules.iter().enumerate() {
            let rule_path = [K("permissions"), K("rules"), I(index)];
            let Some(rule) = rule.as_object() else {
                self.push_permission_skip(
                    out,
                    &rule_path,
                    format!("permissions.rules[{index}] must be an object"),
                );
                continue;
            };
            let field_path = |field| [K("permissions"), K("rules"), I(index), K(field)];
            match rule.get("capability") {
                Some(Value::String(capability))
                    if VALID_PERMISSION_CAPABILITIES.contains(&capability.as_str()) || capability == "workflow" => {}
                Some(Value::String(capability)) => self.push_permission_note(
                    out,
                    &field_path("capability"),
                    Anchor::Value,
                    format!("permissions.rules[{index}].capability '{capability}' is unknown, so V3 skips this rule"),
                ),
                _ => self.push_permission_skip(
                    out,
                    &rule_path,
                    format!("permissions.rules[{index}].capability must be a string"),
                ),
            }
            match rule.get("effect").and_then(Value::as_str) {
                Some("allow" | "deny" | "ask") => {}
                Some("evaluate") => self.push_permission_note(
                    out,
                    &field_path("effect"),
                    Anchor::Value,
                    format!("permissions.rules[{index}].effect 'evaluate' is not supported yet, so V3 skips this rule"),
                ),
                Some(effect) => self.push_permission_skip(
                    out,
                    &field_path("effect"),
                    format!("permissions.rules[{index}].effect '{effect}' must be allow, ask, or deny"),
                ),
                None => self.push_permission_skip(
                    out,
                    &rule_path,
                    format!("permissions.rules[{index}].effect must be allow, ask, or deny"),
                ),
            }
            for field in ["match", "exclude"] {
                let Some(patterns) = rule.get(field) else {
                    continue;
                };
                if !patterns
                    .as_array()
                    .is_some_and(|items| items.iter().all(Value::is_string))
                {
                    self.push_permission_skip(
                        out,
                        &field_path(field),
                        format!("permissions.rules[{index}].{field} must be an array of strings"),
                    );
                }
            }
            for key in rule.keys() {
                if matches!(key.as_str(), "capability" | "effect" | "match" | "exclude") {
                    continue;
                }
                let issue = if rule.contains_key("match") {
                    format!("V3 ignores permissions.rules[{index}].{key}")
                } else {
                    format!(
                        "V3 ignores permissions.rules[{index}].{key}, so the rule has no match and applies to every resource"
                    )
                };
                self.push_permission_note(
                    out,
                    &[K("permissions"), K("rules"), I(index), K(key)],
                    Anchor::Key,
                    issue,
                );
            }
        }
    }

    // ----- KR-AG-015 ---------------------------------------------------------

    fn check_cli_only(&self, out: &mut Vec<Diagnostic>) {
        if !self.enabled("KR-AG-015") || self.doc.markdown || self.pinned == Some(KiroEngine::V2) {
            return;
        }
        if self
            .root()
            .get("permissions")
            .is_some_and(|value| !value.is_null())
        {
            return;
        }
        let present: Vec<&str> = ["allowedTools", "toolsSettings"]
            .into_iter()
            .filter(|field| self.root().contains_key(*field))
            .collect();
        let Some(first) = present.first() else {
            return;
        };
        let empty = present.iter().all(|field| match &self.root()[*field] {
            Value::Null => true,
            Value::Array(items) => items.is_empty(),
            Value::Object(map) => map.is_empty(),
            _ => false,
        });
        let level = if self.pinned == Some(KiroEngine::V3) {
            Error
        } else {
            Warning
        };
        let suggestion = if empty {
            t!("rules.kr_ag_015.empty_suggestion")
        } else {
            t!("rules.kr_ag_015.suggestion")
        };
        let mut diagnostic = self
            .diag(
                level,
                "KR-AG-015",
                self.at(&[K(first)], Anchor::Key),
                t!("rules.kr_ag_015.message", fields = present.join(", ")),
            )
            .with_suggestion(suggestion);
        if empty && let Some(ranges) = self.member_removals(&present) {
            let fixes = ranges.into_iter().map(|(start, end)| {
                Fix::delete(start, end, t!("rules.kr_ag_015.fix").to_string(), false)
            });
            diagnostic = diagnostic.with_fixes(fixes);
        }
        out.push(diagnostic);
    }

    /// Byte ranges that remove the top-level members `keys` (each run of
    /// adjacent members with one comma) and keep the object valid JSON. The
    /// ranges never overlap. `None` when a key is missing or duplicated.
    fn member_removals(&self, keys: &[&str]) -> Option<Vec<(usize, usize)>> {
        let members = self.doc.json.as_ref()?.members();
        if keys
            .iter()
            .any(|key| members.iter().filter(|member| member.key == *key).count() != 1)
        {
            return None;
        }
        let removed: Vec<bool> = members
            .iter()
            .map(|member| keys.contains(&member.key.as_str()))
            .collect();
        let mut ranges = Vec::new();
        let mut index = 0;
        while index < members.len() {
            if !removed[index] {
                index += 1;
                continue;
            }
            let first = index;
            while index + 1 < members.len() && removed[index + 1] {
                index += 1;
            }
            let last = index;
            ranges.push(if first > 0 {
                // `, "key": value` from the end of the previous kept value.
                (members[first - 1].value.end, members[last].value.end)
            } else if let Some(next) = members.get(last + 1) {
                (members[first].key_start, next.key_start)
            } else {
                (members[first].key_start, members[last].value.end)
            });
            index += 1;
        }
        Some(ranges)
    }

    // ----- Hooks: KR-HK-005, KR-HK-006, KR-AG-025..027 -----------------------

    fn check_hooks(&self, out: &mut Vec<Diagnostic>) {
        match self.root().get("hooks") {
            Some(Value::Object(map)) => self.check_object_hooks(out, map),
            Some(Value::Array(items)) => self.check_array_hooks(out, items),
            _ => {}
        }
    }

    fn push_hook_entry(
        &self,
        out: &mut Vec<Diagnostic>,
        fatal: Fatal,
        path: &[PathSegment<'_>],
        location: &str,
        issue: String,
    ) {
        if !self.enabled("KR-HK-006") {
            return;
        }
        self.push_skip(
            out,
            "KR-HK-006",
            fatal,
            self.at(path, Anchor::Value),
            |effect| {
                t!(
                    "rules.kr_hk_006.invalid",
                    hook = location,
                    issue = issue.as_str(),
                    effect = effect
                )
                .to_string()
            },
            t!("rules.kr_hk_006.suggestion"),
        );
    }

    fn check_object_hooks(&self, out: &mut Vec<Diagnostic>, map: &Map<String, Value>) {
        for (trigger, entries) in map {
            let key_path = [K("hooks"), K(trigger)];
            let v2_valid = V2_HOOK_TRIGGERS.contains(&trigger.as_str());
            let v3_trigger = resolve_trigger(trigger);
            if !v2_valid && self.enabled("KR-HK-005") {
                self.report_object_trigger(out, map, trigger, v3_trigger);
            }

            let Some(entries) = entries.as_array() else {
                self.push_hook_entry(
                    out,
                    Fatal::Both,
                    &key_path,
                    &format!("hooks.{trigger}"),
                    format!(
                        "must be an array of hook entries, found {}",
                        json_type_name(entries)
                    ),
                );
                continue;
            };
            let tool_trigger = matches!(v3_trigger, Some("PreToolUse" | "PostToolUse"));
            for (index, entry) in entries.iter().enumerate() {
                let entry_path = [K("hooks"), K(trigger), I(index)];
                let location = format!("hooks.{trigger}[{index}]");
                let Some(entry) = entry.as_object() else {
                    self.push_hook_entry(
                        out,
                        Fatal::Both,
                        &entry_path,
                        &location,
                        format!("must be an object, found {}", json_type_name(entry)),
                    );
                    continue;
                };
                let field = |name| [K("hooks"), K(trigger), I(index), K(name)];
                match entry.get("command") {
                    Some(Value::String(command)) if command.is_empty() => self.push_hook_entry(
                        out,
                        Fatal::V3,
                        &field("command"),
                        &location,
                        "has an empty command".to_string(),
                    ),
                    Some(Value::String(command))
                        if command.trim().is_empty() && self.enabled("KR-HK-006") =>
                    {
                        out.push(
                            self.diag(
                                Warning,
                                "KR-HK-006",
                                self.at(&field("command"), Anchor::Value),
                                t!("rules.kr_hk_006.blank", hook = location.as_str()),
                            )
                            .with_suggestion(t!("rules.kr_hk_006.suggestion")),
                        )
                    }
                    Some(Value::String(_)) => {}
                    Some(other) => self.push_hook_entry(
                        out,
                        Fatal::Both,
                        &field("command"),
                        &location,
                        format!("command must be a string, found {}", json_type_name(other)),
                    ),
                    None => self.push_hook_entry(
                        out,
                        Fatal::Both,
                        &entry_path,
                        &location,
                        "is missing the required command".to_string(),
                    ),
                }
                match entry.get("matcher") {
                    Some(Value::String(matcher)) => {
                        if tool_trigger {
                            self.check_tool_matcher(
                                out,
                                Some(matcher),
                                &entry_path,
                                index,
                                trigger,
                            );
                        }
                    }
                    None => {
                        if tool_trigger {
                            self.check_tool_matcher(out, None, &entry_path, index, trigger);
                        }
                    }
                    Some(other) => {
                        if self.engines.v2 {
                            self.push_hook_entry(
                                out,
                                Fatal::V2,
                                &field("matcher"),
                                &location,
                                format!(
                                    "matcher must be a string, found {} (V3 drops it, so the hook runs for every tool)",
                                    json_type_name(other)
                                ),
                            );
                        } else if self.enabled("KR-HK-006") {
                            out.push(
                                self.diag(
                                    Warning,
                                    "KR-HK-006",
                                    self.at(&field("matcher"), Anchor::Value),
                                    t!("rules.kr_hk_006.matcher_dropped", hook = location.as_str()),
                                )
                                .with_suggestion(t!("rules.kr_hk_006.suggestion")),
                            );
                        }
                    }
                }
                for name in ["timeout_ms", "max_output_size", "cache_ttl_seconds"] {
                    if let Some(value) = entry.get(name)
                        && !value.is_u64()
                        && self.engines.v2
                    {
                        self.push_hook_entry(
                            out,
                            Fatal::V2,
                            &field(name),
                            &location,
                            format!(
                                "{name} must be a non-negative integer, found {}",
                                short_json(value)
                            ),
                        );
                    }
                }
                self.check_object_hook_timeouts(
                    out,
                    entry,
                    &location,
                    &field("timeout"),
                    &field("timeout_ms"),
                );
                self.check_object_hook_fields(out, entry, &location, trigger, index);
            }
        }
    }

    fn report_object_trigger(
        &self,
        out: &mut Vec<Diagnostic>,
        map: &Map<String, Value>,
        trigger: &str,
        v3_trigger: Option<&'static str>,
    ) {
        let key_offset = self.at(&[K("hooks"), K(trigger)], Anchor::Key);
        let replacement = v2_trigger_for(trigger).filter(|candidate| !map.contains_key(*candidate));
        let fix = replacement.and_then(|replacement| {
            let node = self.doc.node(&[K("hooks")])?;
            let member = node.member(trigger)?;
            (node
                .members()
                .iter()
                .filter(|member| member.key == trigger)
                .count()
                == 1)
                .then(|| {
                    Fix::replace(
                        member.key_start,
                        member.key_end,
                        format!("\"{replacement}\""),
                        t!("rules.kr_hk_005.fix", trigger = replacement).to_string(),
                        false,
                    )
                })
        });
        let v3_note = match v3_trigger {
            Some(canonical) => t!("rules.kr_hk_005.v3_runs", trigger = canonical).to_string(),
            None => t!("rules.kr_hk_005.v3_skips_hook").to_string(),
        };
        let suggestion = match replacement {
            Some(replacement) => t!("rules.kr_hk_005.rename_suggestion", trigger = replacement),
            None => t!("rules.kr_hk_005.suggestion"),
        };
        if self.engines.v2 {
            let v3 = self.engines.v3;
            let Some((level, effect)) = self.skip_effect(Fatal::V2) else {
                return;
            };
            let effect = if v3 {
                format!("{effect}; {v3_note}")
            } else {
                effect
            };
            let mut diagnostic = self
                .diag(
                    level,
                    "KR-HK-005",
                    key_offset,
                    t!(
                        "rules.kr_hk_005.object_key",
                        trigger = trigger,
                        effect = effect
                    ),
                )
                .with_suggestion(suggestion);
            if let Some(fix) = fix {
                diagnostic = diagnostic.with_fix(fix);
            }
            out.push(self.assume(diagnostic));
        } else if v3_trigger.is_none() {
            out.push(
                self.diag(
                    Warning,
                    "KR-HK-005",
                    key_offset,
                    t!("rules.kr_hk_005.v3_unknown", trigger = trigger),
                )
                .with_suggestion(t!("rules.kr_hk_005.v3_suggestion")),
            );
        }
    }

    fn check_object_hook_timeouts(
        &self,
        out: &mut Vec<Diagnostic>,
        entry: &Map<String, Value>,
        location: &str,
        timeout_path: &[PathSegment<'_>],
        timeout_ms_path: &[PathSegment<'_>],
    ) {
        if !self.enabled("KR-AG-026") {
            return;
        }
        if entry.contains_key("timeout") && !entry.contains_key("timeout_ms") {
            out.push(
                self.diag(
                    Warning,
                    "KR-AG-026",
                    self.at(timeout_path, Anchor::Key),
                    t!("rules.kr_ag_026.timeout_in_object", hook = location),
                )
                .with_suggestion(t!("rules.kr_ag_026.timeout_in_object_suggestion")),
            );
        }
        if let Some(milliseconds) = entry.get("timeout_ms").and_then(Value::as_u64)
            && (1..100).contains(&milliseconds)
        {
            out.push(
                self.diag(
                    Warning,
                    "KR-AG-026",
                    self.at(timeout_ms_path, Anchor::Value),
                    t!(
                        "rules.kr_ag_026.ms_looks_like_seconds",
                        hook = location,
                        value = milliseconds
                    ),
                )
                .with_suggestion(t!("rules.kr_ag_026.ms_suggestion")),
            );
        }
    }

    fn check_object_hook_fields(
        &self,
        out: &mut Vec<Diagnostic>,
        entry: &Map<String, Value>,
        location: &str,
        trigger: &str,
        index: usize,
    ) {
        if !self.enabled("KR-AG-027") {
            return;
        }
        for key in entry.keys() {
            let key = key.as_str();
            let path = [K("hooks"), K(trigger), I(index), K(key)];
            if key == "timeout" || key == "name" {
                continue;
            }
            if !V2_HOOK_ENTRY_FIELDS.contains(&key) {
                out.push(
                    self.diag(
                        Warning,
                        "KR-AG-027",
                        self.at(&path, Anchor::Key),
                        t!(
                            "rules.kr_ag_027.object_unknown",
                            hook = location,
                            field = key
                        ),
                    )
                    .with_suggestion(t!("rules.kr_ag_027.object_suggestion")),
                );
            } else if !self.engines.v2
                && matches!(key, "timeout_ms" | "max_output_size" | "cache_ttl_seconds")
            {
                out.push(
                    self.diag(
                        Info,
                        "KR-AG-027",
                        self.at(&path, Anchor::Key),
                        t!("rules.kr_ag_027.v3_drops", hook = location, field = key),
                    )
                    .with_suggestion(t!("rules.kr_ag_027.v3_drops_suggestion")),
                );
            }
        }
    }

    fn check_array_hooks(&self, out: &mut Vec<Diagnostic>, items: &[Value]) {
        for (index, entry) in items.iter().enumerate() {
            let entry_path = [K("hooks"), I(index)];
            let location = format!("hooks[{index}]");
            let Some(entry) = entry.as_object() else {
                self.push_hook_entry(
                    out,
                    Fatal::Both,
                    &entry_path,
                    &location,
                    format!("must be an object, found {}", json_type_name(entry)),
                );
                continue;
            };
            let field = |name| [K("hooks"), I(index), K(name)];
            match entry.get("name") {
                None => self.push_hook_entry(
                    out,
                    Fatal::V3,
                    &entry_path,
                    &location,
                    "is missing the required name".to_string(),
                ),
                Some(Value::String(name)) if name.is_empty() => {
                    self.push_hook_entry(
                        out,
                        Fatal::V3,
                        &field("name"),
                        &location,
                        "has an empty name".to_string(),
                    );
                }
                Some(Value::String(_)) => {}
                Some(other) => self.push_hook_entry(
                    out,
                    Fatal::Both,
                    &field("name"),
                    &location,
                    format!("name must be a string, found {}", json_type_name(other)),
                ),
            }
            let canonical = match entry.get("trigger") {
                None => {
                    self.push_hook_entry(
                        out,
                        Fatal::Both,
                        &entry_path,
                        &location,
                        "is missing the required trigger".to_string(),
                    );
                    None
                }
                Some(Value::String(trigger)) if trigger.is_empty() => {
                    self.push_hook_entry(
                        out,
                        Fatal::V3,
                        &field("trigger"),
                        &location,
                        "has an empty trigger".to_string(),
                    );
                    None
                }
                Some(Value::String(trigger)) => {
                    let canonical = resolve_trigger(trigger);
                    if canonical.is_none() {
                        self.report_array_trigger(out, index, trigger, &location);
                    }
                    canonical
                }
                Some(other) => {
                    self.push_hook_entry(
                        out,
                        Fatal::Both,
                        &field("trigger"),
                        &location,
                        format!("trigger must be a string, found {}", json_type_name(other)),
                    );
                    None
                }
            };
            self.check_hook_action(out, entry, index, &location);
            let tool_trigger = matches!(canonical, Some("PreToolUse" | "PostToolUse"));
            match entry.get("matcher") {
                Some(Value::String(matcher)) => {
                    if tool_trigger {
                        let trigger = canonical.unwrap_or_default();
                        self.check_tool_matcher(out, Some(matcher), &entry_path, index, trigger);
                    }
                }
                Some(other) => self.push_hook_entry(
                    out,
                    Fatal::Both,
                    &field("matcher"),
                    &location,
                    format!("matcher must be a string, found {}", json_type_name(other)),
                ),
                None => {
                    if tool_trigger {
                        let trigger = canonical.unwrap_or_default();
                        self.check_tool_matcher(out, None, &entry_path, index, trigger);
                    }
                }
            }
            if let Some(timeout) = entry.get("timeout") {
                match timeout {
                    Value::Number(number) if number.is_u64() => {
                        let seconds = number.as_u64().unwrap_or_default();
                        if seconds >= 1000 && seconds % 1000 == 0 && self.enabled("KR-AG-026") {
                            out.push(
                                self.diag(
                                    Warning,
                                    "KR-AG-026",
                                    self.at(&field("timeout"), Anchor::Value),
                                    t!(
                                        "rules.kr_ag_026.seconds_looks_like_ms",
                                        hook = location.as_str(),
                                        value = seconds,
                                        seconds = seconds / 1000
                                    ),
                                )
                                .with_suggestion(t!("rules.kr_ag_026.seconds_suggestion")),
                            );
                        }
                    }
                    Value::Number(_) => self.push_hook_entry(
                        out,
                        Fatal::V3,
                        &field("timeout"),
                        &location,
                        format!("timeout must be a whole number of seconds >= 0, found {timeout}"),
                    ),
                    other => self.push_hook_entry(
                        out,
                        Fatal::Both,
                        &field("timeout"),
                        &location,
                        format!(
                            "timeout must be a number of seconds, found {}",
                            json_type_name(other)
                        ),
                    ),
                }
            }
            if let Some(enabled) = entry.get("enabled")
                && !enabled.is_boolean()
            {
                self.push_hook_entry(
                    out,
                    Fatal::V3,
                    &field("enabled"),
                    &location,
                    format!(
                        "enabled must be a boolean, found {}",
                        json_type_name(enabled)
                    ),
                );
            }
            if let Some(description) = entry.get("description")
                && !description.is_string()
            {
                self.push_hook_entry(
                    out,
                    Fatal::V3,
                    &field("description"),
                    &location,
                    format!(
                        "description must be a string, found {}",
                        json_type_name(description)
                    ),
                );
            }
            if let Some(confirm) = entry.get("confirm")
                && let Some(issue) = confirm_issue(confirm)
            {
                self.push_hook_entry(out, Fatal::V3, &field("confirm"), &location, issue);
            }
            if self.enabled("KR-AG-026") && entry.contains_key("timeout_ms") {
                out.push(
                    self.diag(
                        Warning,
                        "KR-AG-026",
                        self.at(&field("timeout_ms"), Anchor::Key),
                        t!(
                            "rules.kr_ag_026.timeout_ms_in_array",
                            hook = location.as_str()
                        ),
                    )
                    .with_suggestion(t!("rules.kr_ag_026.timeout_ms_in_array_suggestion")),
                );
            }
            if self.enabled("KR-AG-027") {
                for key in entry.keys() {
                    if !ARRAY_HOOK_ENTRY_FIELDS.contains(&key.as_str()) && key != "timeout_ms" {
                        out.push(
                            self.diag(
                                Warning,
                                "KR-AG-027",
                                self.at(&field(key), Anchor::Key),
                                t!(
                                    "rules.kr_ag_027.array_unknown",
                                    hook = location.as_str(),
                                    field = key.as_str()
                                ),
                            )
                            .with_suggestion(t!("rules.kr_ag_027.array_suggestion")),
                        );
                    }
                }
            }
        }
    }

    fn report_array_trigger(
        &self,
        out: &mut Vec<Diagnostic>,
        index: usize,
        trigger: &str,
        location: &str,
    ) {
        if !self.enabled("KR-HK-005") || !(self.engines.v3 || self.migration) {
            return;
        }
        let path = [K("hooks"), I(index), K("trigger")];
        let replacement = trigger_case_insensitive_match(trigger).or_else(|| {
            trigger
                .eq_ignore_ascii_case("agentSpawn")
                .then_some("SessionStart")
        });
        let mut diagnostic = self
            .diag(
                Warning,
                "KR-HK-005",
                self.at(&path, Anchor::Value),
                t!(
                    "rules.kr_hk_005.array_trigger",
                    hook = location,
                    trigger = trigger
                ),
            )
            .with_suggestion(match replacement {
                Some(replacement) => t!("rules.kr_hk_005.rename_suggestion", trigger = replacement),
                None => t!("rules.kr_hk_005.v3_suggestion"),
            });
        if let Some(replacement) = replacement
            && let Some(node) = self.doc.node(&[K("hooks"), I(index)])
            && node
                .members()
                .iter()
                .filter(|member| member.key == "trigger")
                .count()
                == 1
            && let Some(value) = self.doc.node(&path)
        {
            diagnostic = diagnostic.with_fix(Fix::replace(
                value.start,
                value.end,
                format!("\"{replacement}\""),
                t!("rules.kr_hk_005.fix", trigger = replacement).to_string(),
                false,
            ));
        }
        out.push(diagnostic);
    }

    fn check_hook_action(
        &self,
        out: &mut Vec<Diagnostic>,
        entry: &Map<String, Value>,
        index: usize,
        location: &str,
    ) {
        let action_path = [K("hooks"), I(index), K("action")];
        let field = |name| [K("hooks"), I(index), K("action"), K(name)];
        let Some(action) = entry.get("action") else {
            self.push_hook_entry(
                out,
                Fatal::Both,
                &[K("hooks"), I(index)],
                location,
                "is missing the required action".to_string(),
            );
            return;
        };
        let Some(action) = action.as_object() else {
            self.push_hook_entry(
                out,
                Fatal::Both,
                &action_path,
                location,
                format!("action must be an object, found {}", json_type_name(action)),
            );
            return;
        };
        let (payload, kind) = match action.get("type").and_then(Value::as_str) {
            Some("command") => ("command", "command"),
            Some("agent") => ("prompt", "agent"),
            Some(other) => {
                self.push_hook_entry(
                    out,
                    Fatal::Both,
                    &field("type"),
                    location,
                    format!("action.type '{other}' must be command or agent"),
                );
                return;
            }
            None => {
                self.push_hook_entry(
                    out,
                    Fatal::Both,
                    &action_path,
                    location,
                    "action needs a type of command or agent".to_string(),
                );
                return;
            }
        };
        match action.get(payload) {
            Some(Value::String(text)) if text.is_empty() => {
                self.push_hook_entry(
                    out,
                    Fatal::V3,
                    &field(payload),
                    location,
                    format!("action.{payload} is empty"),
                );
            }
            Some(Value::String(_)) => {}
            Some(other) => self.push_hook_entry(
                out,
                Fatal::Both,
                &field(payload),
                location,
                format!(
                    "action.{payload} must be a string, found {}",
                    json_type_name(other)
                ),
            ),
            None => {
                let fatal = if kind == "command" {
                    Fatal::Both
                } else {
                    Fatal::V3
                };
                self.push_hook_entry(
                    out,
                    fatal,
                    &action_path,
                    location,
                    format!("action is missing {payload}"),
                );
            }
        }
        if kind == "agent" && self.engines.v2 && self.enabled("KR-AG-027") {
            out.push(
                self.diag(
                    Info,
                    "KR-AG-027",
                    self.at(&field("type"), Anchor::Value),
                    t!("rules.kr_ag_027.agent_action_v2", hook = location),
                )
                .with_suggestion(t!("rules.kr_ag_027.agent_action_suggestion")),
            );
        }
    }

    /// Matcher checks for a tool-trigger hook (`matcher` is `None` when the
    /// key is absent). For V3 the hooks validator's rules apply: KR-HK-027
    /// (no specific matcher), KR-HK-021 (CLI 2.x or Claude Code names that
    /// match no V3 tool) and KR-HK-029 (unanchored regex alternation).
    /// KR-AG-025 covers the reverse case: a V3 tool id in a matcher Kiro
    /// CLI 2.x reads.
    fn check_tool_matcher(
        &self,
        out: &mut Vec<Diagnostic>,
        matcher: Option<&str>,
        entry_path: &[PathSegment<'_>],
        index: usize,
        trigger: &str,
    ) {
        let mut matcher_path = entry_path.to_vec();
        matcher_path.push(K("matcher"));
        let offset = match matcher {
            Some(_) => self.at(&matcher_path, Anchor::Value),
            None => self.at(entry_path, Anchor::Value),
        };
        if self.engines.v3
            && self.enabled("KR-HK-027")
            && matches_everything(matcher, MatcherSubject::Tool)
        {
            out.push(
                self.diag(
                    Info,
                    "KR-HK-027",
                    offset,
                    t!("rules.kr_hk_027.message", index = index, trigger = trigger),
                )
                .with_suggestion(t!("rules.kr_hk_027.suggestion")),
            );
        }
        let Some(matcher) = matcher else {
            return;
        };
        if (self.engines.v3 || self.migration) && self.enabled("KR-HK-021") {
            let names = if !matcher.contains('@')
                && !matcher.contains("mcp_")
                && tool_matcher_matches_builtin(matcher) == Some(false)
            {
                foreign_tool_names(matcher)
            } else {
                Vec::new()
            };
            if !names.is_empty() {
                let names = names
                    .iter()
                    .map(|(name, origin, replacement)| {
                        t!(
                            "rules.kr_hk_021.name",
                            name = *name,
                            origin = *origin,
                            replacement = *replacement
                        )
                        .to_string()
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                let level = if self.engines.v3 { Warning } else { Info };
                let diagnostic = self
                    .diag(
                        level,
                        "KR-HK-021",
                        offset,
                        t!(
                            "rules.kr_hk_021.message",
                            index = index,
                            matcher = matcher,
                            names = names
                        ),
                    )
                    .with_suggestion(t!("rules.kr_hk_021.suggestion"));
                out.push(self.assume(diagnostic));
                return;
            }
        }
        if self.engines.v3
            && self.enabled("KR-HK-029")
            && !is_tool_selector(matcher)
            && matcher.contains(['|', '('])
            && !(matcher.starts_with('^') && matcher.ends_with('$'))
        {
            out.push(
                self.diag(
                    Info,
                    "KR-HK-029",
                    offset,
                    t!("rules.kr_hk_029.message", index = index, matcher = matcher),
                )
                .with_suggestion(t!("rules.kr_hk_029.suggestion")),
            );
        }
        if !self.engines.v2 || !self.enabled("KR-AG-025") {
            return;
        }
        // CLI 2.x matchers are tool names, aliases or globs; split a plain
        // alternation so each name is checked.
        let is_name = |part: &str| {
            !part.is_empty()
                && part.chars().all(|ch| {
                    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '@' | '/' | '.')
                })
        };
        let parts: Vec<&str> = matcher.split('|').collect();
        if !parts.iter().all(|part| is_name(part)) {
            return;
        }
        for part in parts {
            if let Some(equivalent) = v2_equivalent_for_v3_tool(part) {
                let diagnostic = self
                    .diag(
                        Warning,
                        "KR-AG-025",
                        offset,
                        t!("rules.kr_ag_025.v2", matcher = part),
                    )
                    .with_suggestion(t!(
                        "rules.kr_ag_025.v2_suggestion",
                        replacement = equivalent
                    ));
                out.push(self.assume(diagnostic));
            }
        }
    }
}

/// The first problem with a V3 Stop-hook `confirm` object, if any.
fn confirm_issue(confirm: &Value) -> Option<String> {
    let non_empty = |value: Option<&Value>| {
        value
            .and_then(Value::as_str)
            .is_some_and(|text| !text.is_empty())
    };
    let Some(confirm) = confirm.as_object() else {
        return Some(format!(
            "confirm must be an object, found {}",
            json_type_name(confirm)
        ));
    };
    if !non_empty(confirm.get("question")) {
        return Some("confirm.question must be a non-empty string".to_string());
    }
    let Some(options) = confirm
        .get("options")
        .and_then(Value::as_array)
        .filter(|options| !options.is_empty())
    else {
        return Some("confirm.options must be a non-empty array".to_string());
    };
    for (index, option) in options.iter().enumerate() {
        let valid = option.as_object().is_some_and(|option| {
            non_empty(option.get("id"))
                && non_empty(option.get("label"))
                && option.get("run").is_some_and(Value::is_boolean)
                && option.get("continueReason").is_none_or(Value::is_string)
        });
        if !valid {
            return Some(format!(
                "confirm.options[{index}] needs a non-empty id and label and a boolean run"
            ));
        }
    }
    if let Some(command) = confirm.get("confirmCommand")
        && !non_empty(Some(command))
    {
        return Some("confirm.confirmCommand must be a non-empty string".to_string());
    }
    None
}

/// A message for a tool name from another tool's vocabulary (Claude Code
/// names, `mcp__server__tool`) or with the wrong case.
fn foreign_tool_name(tool: &str, field: &str) -> Option<String> {
    if let Some(rest) = tool.strip_prefix("mcp__") {
        let mut parts = rest.splitn(2, "__");
        let server = parts.next().unwrap_or_default();
        let replacement = match parts.next() {
            Some(name) => format!("@{server}/{name}"),
            None => format!("@{server}"),
        };
        return Some(
            t!(
                "rules.kr_ag_024.claude_mcp",
                tool = tool,
                field = field,
                replacement = replacement
            )
            .to_string(),
        );
    }
    if let Some(replacement) = claude_code_tool_replacement(tool) {
        return Some(
            t!(
                "rules.kr_ag_024.claude_name",
                tool = tool,
                field = field,
                replacement = replacement
            )
            .to_string(),
        );
    }
    case_variant_of_known_tool(tool).map(|replacement| {
        t!(
            "rules.kr_ag_024.case",
            tool = tool,
            field = field,
            replacement = replacement
        )
        .to_string()
    })
}

/// What a sibling agent file contributes to duplicate-name checks.
struct SiblingIdentity {
    name: Option<String>,
    /// A JSON profile V3 skips (CLI 2.x trust fields without permissions).
    v3_skipped: bool,
    /// Agents this file lists as sub-agents: `toolsSettings.subagent`
    /// `availableAgents` / `trustedAgents` entries (globs allowed) and V3
    /// `subagent/<agent>` tools.
    subagent_refs: Vec<String>,
}

/// The sub-agents an agent config lists (see [`SiblingIdentity::subagent_refs`]).
fn subagent_references(root: &Map<String, Value>) -> Vec<String> {
    let mut refs = Vec::new();
    if let Some(settings) = root.get("toolsSettings").and_then(Value::as_object) {
        for key in ["subagent", "use_subagent", "agent_crew", "agentCrew"] {
            let Some(lists) = settings.get(key).and_then(Value::as_object) else {
                continue;
            };
            for list in ["availableAgents", "trustedAgents"] {
                refs.extend(
                    json_string_array(lists.get(list))
                        .into_iter()
                        .map(|(_, name)| name),
                );
            }
        }
    }
    refs.extend(
        json_string_array(root.get("tools"))
            .into_iter()
            .filter_map(|(_, tool)| tool.strip_prefix("subagent/").map(ToString::to_string)),
    );
    refs
}

fn sibling_identity(text: &str, markdown: bool) -> Option<SiblingIdentity> {
    let root = if markdown {
        let parts = split_frontmatter(text);
        if !parts.has_frontmatter
            || !parts.has_closing
            || check_yaml_depth(&parts.frontmatter).is_err()
        {
            return None;
        }
        let yaml = serde_yaml::from_str::<serde_yaml::Value>(&parts.frontmatter).ok()?;
        serde_json::to_value(yaml).ok()?
    } else {
        serde_json::from_str::<Value>(text)
            .or_else(|_| serde_json::from_str::<Value>(&strip_jsonc(text).0))
            .ok()?
    };
    let root = root.as_object()?;
    if root.is_empty() {
        return None;
    }
    let has_trust = root.contains_key("allowedTools") || root.contains_key("toolsSettings");
    let has_permissions = root
        .get("permissions")
        .is_some_and(|value| !value.is_null());
    Some(SiblingIdentity {
        name: root
            .get("name")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        v3_skipped: !markdown && has_trust && !has_permissions,
        subagent_refs: subagent_references(root),
    })
}

fn agent_secret_pattern() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(
            r"(?i)\b(?:api[_-]?key|token|password|secret)\b\s*[:=]\s*(?:sk-|sk-ant-|sk-proj-|ghp_|gho_|AKIA|xoxb-|glpat-|[A-Za-z0-9+/]{20,})"
        )
        .expect("agent secret pattern must compile")
    })
}

/// Engine markers of an agent: does it carry CLI 2.x-only or V3-only content?
fn engine_markers(root: &Map<String, Value>) -> (bool, bool) {
    let hooks = root.get("hooks");
    let array_hooks_with_ms = hooks
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.get("timeout_ms").is_some()));
    let universal = root.contains_key("kiroUpgrade");
    let v2 = universal
        || V2_MARKER_FIELDS
            .iter()
            .any(|field| root.contains_key(*field))
        || hooks.is_some_and(Value::is_object)
        || array_hooks_with_ms;
    let v3 = universal
        || V3_MARKER_FIELDS
            .iter()
            .any(|field| root.contains_key(*field))
        || root
            .get("permissions")
            .is_some_and(|value| !value.is_null())
        || hooks.is_some_and(Value::is_array);
    (v2, v3)
}

pub struct KiroAgentValidator;

impl KiroAgentValidator {
    fn report_load_failure(
        path: &Path,
        content: &str,
        failure: LoadFailure,
        config: &PerFileLintConfig<'_>,
        out: &mut Vec<Diagnostic>,
    ) {
        let file = path.to_path_buf();
        match failure {
            LoadFailure::Syntax {
                line,
                column,
                error,
            } if config.is_rule_enabled("KR-AG-016") => out.push(
                Diagnostic::error(
                    file,
                    line,
                    column,
                    "KR-AG-016",
                    t!("rules.kr_ag_016.syntax", error = error),
                )
                .with_suggestion(t!("rules.kr_ag_016.suggestion")),
            ),
            LoadFailure::NotObject { found } if config.is_rule_enabled("KR-AG-016") => out.push(
                Diagnostic::error(
                    file,
                    1,
                    1,
                    "KR-AG-016",
                    t!("rules.kr_ag_016.not_object", found = found),
                )
                .with_suggestion(t!("rules.kr_ag_016.suggestion")),
            ),
            LoadFailure::NoFrontmatter if config.is_rule_enabled("KR-AG-017") => out.push(
                Diagnostic::warning(file, 1, 1, "KR-AG-017", t!("rules.kr_ag_017.missing"))
                    .with_suggestion(t!("rules.kr_ag_017.missing_suggestion")),
            ),
            LoadFailure::EmptyFrontmatter { offset } if config.is_rule_enabled("KR-AG-017") => {
                let (line, column) = line_col_at_offset(content, offset);
                out.push(
                    Diagnostic::error(file, line, column, "KR-AG-017", t!("rules.kr_ag_017.empty"))
                        .with_suggestion(t!("rules.kr_ag_017.suggestion")),
                );
            }
            LoadFailure::BadFrontmatter { offset, error }
                if config.is_rule_enabled("KR-AG-017") =>
            {
                let (line, column) = line_col_at_offset(content, offset);
                out.push(
                    Diagnostic::error(
                        file,
                        line,
                        column,
                        "KR-AG-017",
                        t!("rules.kr_ag_017.invalid", error = error),
                    )
                    .with_suggestion(t!("rules.kr_ag_017.suggestion")),
                );
            }
            _ => {}
        }
    }
}

impl Validator for KiroAgentValidator {
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
        let mut out = Vec::new();
        if !RULE_IDS.iter().any(|id| config.is_rule_enabled(id)) {
            return out;
        }
        let pinned = pinned_engine(config);
        let markdown = is_markdown(path);
        let nested = agents_location(path).is_some_and(|(_, rest)| rest.len() > 1);

        // CLI 2.x loads only top-level JSON agents.
        if pinned == Some(KiroEngine::V2) && (markdown || nested) {
            let looks_like_agent = if markdown {
                let parts = split_frontmatter(content);
                parts.has_frontmatter && parts.has_closing && !parts.frontmatter.trim().is_empty()
            } else {
                serde_json::from_str::<Value>(content).is_ok_and(|value| value.is_object())
            };
            if looks_like_agent && config.is_rule_enabled("KR-AG-028") {
                let key = if markdown {
                    "rules.kr_ag_028.markdown"
                } else {
                    "rules.kr_ag_028.nested"
                };
                out.push(
                    Diagnostic::warning(path.to_path_buf(), 1, 1, "KR-AG-028", t!(key))
                        .with_suggestion(t!("rules.kr_ag_028.suggestion")),
                );
            }
            return out;
        }

        let loaded = if markdown {
            load_markdown(path, content)
        } else {
            load_json(path, content)
        };
        let Loaded { doc, jsonc_offset } = match loaded {
            Ok(loaded) => loaded,
            Err(failure) => {
                Self::report_load_failure(path, content, failure, config, &mut out);
                return out;
            }
        };

        let engines = if markdown || nested {
            KiroEngines::V3_ONLY
        } else {
            let (v2, v3) = engine_markers(&doc.root);
            engines_for(config, v2, v3)
        };
        let ctx = Ctx {
            doc,
            config,
            engines,
            pinned,
            migration: pinned.is_none() && !markdown && !nested && engines == KiroEngines::V2_ONLY,
            siblings: OnceCell::new(),
        };
        if let Some(offset) = jsonc_offset {
            ctx.check_jsonc(offset, &mut out);
        }
        ctx.run(&mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LintConfig, ToolVersions};
    use crate::fixes::apply_fixes_with_fs;
    use crate::fs::{FileSystem, MockFileSystem};
    use std::sync::Arc;

    const AGENTS: &str = "/w/.kiro/agents";

    struct Run {
        diagnostics: Vec<Diagnostic>,
        fs: Arc<MockFileSystem>,
        path: PathBuf,
    }

    impl Run {
        fn rule(&self, rule: &str) -> Vec<&Diagnostic> {
            self.diagnostics.iter().filter(|d| d.rule == rule).collect()
        }

        fn one(&self, rule: &str) -> &Diagnostic {
            let found = self.rule(rule);
            assert_eq!(
                found.len(),
                1,
                "expected one {rule}: {:#?}",
                self.diagnostics
            );
            found[0]
        }

        fn none(&self, rule: &str) {
            assert!(
                self.rule(rule).is_empty(),
                "unexpected {rule}: {:#?}",
                self.diagnostics
            );
        }

        fn rules(&self) -> Vec<&str> {
            self.diagnostics.iter().map(|d| d.rule.as_str()).collect()
        }

        /// Apply every fix (safe and unsafe) and return the new content.
        fn apply_all(&self) -> String {
            apply_fixes_with_fs(&self.diagnostics, false, false, Some(self.fs.clone())).unwrap();
            self.fs.read_to_string(&self.path).unwrap()
        }
    }

    /// Validate `content` at `.kiro/agents/<name>`, with `extra` files (paths
    /// relative to the workspace `/w`) in a mock file system.
    fn check_with(name: &str, content: &str, kiro: Option<&str>, extra: &[(&str, &str)]) -> Run {
        let fs = Arc::new(MockFileSystem::new());
        fs.add_dir("/w");
        fs.add_dir("/w/.kiro");
        fs.add_dir(AGENTS);
        let path = PathBuf::from(format!("{AGENTS}/{name}"));
        for (rel, text) in
            std::iter::once((path.strip_prefix("/w").unwrap().to_str().unwrap(), content))
                .chain(extra.iter().copied())
        {
            let full = Path::new("/w").join(rel);
            let mut dir = full.parent();
            while let Some(parent) = dir {
                if parent == Path::new("/w") || parent.as_os_str().is_empty() {
                    break;
                }
                fs.add_dir(parent);
                dir = parent.parent();
            }
            fs.add_file(&full, text);
        }
        let config = LintConfig::builder()
            .fs(fs.clone())
            .tool_versions(ToolVersions {
                kiro: kiro.map(ToString::to_string),
                ..ToolVersions::default()
            })
            .build()
            .unwrap();
        let diagnostics = KiroAgentValidator.validate(&path, content, &config);
        Run {
            diagnostics,
            fs,
            path,
        }
    }

    fn check(name: &str, content: &str) -> Run {
        check_with(name, content, None, &[])
    }

    fn json(value: serde_json::Value) -> String {
        serde_json::to_string_pretty(&value).unwrap()
    }

    // ----- parsing and load failures ------------------------------------

    #[test]
    fn invalid_json_is_an_error_at_the_real_break() {
        let run = check("a.json", "{\n  \"name\": \"a\",\n  \"tools\": [\"read\"\n}");
        let diagnostic = run.one("KR-AG-016");
        assert_eq!(diagnostic.level, DiagnosticLevel::Error);
        assert_eq!(diagnostic.line, 4, "{diagnostic:?}");
        assert_eq!(run.diagnostics.len(), 1);
    }

    #[test]
    fn jsonc_is_fatal_only_for_v2() {
        let neutral = "{\n  // comment\n  \"name\": \"a\",\n  \"madeUp\": 1,\n}";
        let run = check("a.json", neutral);
        let diagnostic = run.one("KR-AG-016");
        assert_eq!(diagnostic.level, DiagnosticLevel::Warning);
        assert_eq!((diagnostic.line, diagnostic.column), (2, 3));
        assert!(diagnostic.message.contains("2.x"), "{}", diagnostic.message);
        assert!(diagnostic.assumption.is_some());
        // The rest of the file is still checked.
        assert_eq!(run.one("KR-AG-001").line, 4);

        let v2 = "{\"name\": \"a\", \"allowedTools\": [], /* x */ \"permissions\": null}";
        assert_eq!(
            check_with("a.json", v2, None, &[]).one("KR-AG-016").level,
            DiagnosticLevel::Error
        );
        let v3 = "{\"name\": \"a\", // c\n \"permissions\": {\"rules\": []}}";
        check("a.json", v3).none("KR-AG-016");
        check_with("a.json", neutral, Some("3.0.0"), &[]).none("KR-AG-016");
    }

    #[test]
    fn non_object_json_is_an_error() {
        let run = check("a.json", "[1, 2]");
        let diagnostic = run.one("KR-AG-016");
        assert_eq!(diagnostic.level, DiagnosticLevel::Error);
        assert!(diagnostic.message.contains("array"));
    }

    #[test]
    fn markdown_frontmatter_problems() {
        let missing = check("readme.md", "# Notes\n\nNot an agent.");
        assert_eq!(missing.one("KR-AG-017").level, DiagnosticLevel::Warning);
        assert_eq!(missing.diagnostics.len(), 1);

        let empty = check("a.md", "---\n---\nBody");
        assert_eq!(empty.one("KR-AG-017").level, DiagnosticLevel::Error);

        let invalid = check("a.md", "---\nname: a\ntools: [read\n---\nBody");
        let diagnostic = invalid.one("KR-AG-017");
        assert_eq!(diagnostic.level, DiagnosticLevel::Error);
        assert!(diagnostic.line >= 2, "{diagnostic:?}");

        let list = check("a.md", "---\n- a\n- b\n---\nBody");
        assert_eq!(list.one("KR-AG-017").level, DiagnosticLevel::Error);
    }

    #[test]
    fn pinned_v2_reports_only_that_markdown_and_nested_agents_do_not_load() {
        let markdown = check_with(
            "a.md",
            "---\ndescription: x\nmodel: opus\n---\nBody",
            Some("2.32.1"),
            &[],
        );
        assert_eq!(markdown.rules(), ["KR-AG-028"]);
        check_with("readme.md", "# notes", Some("2.32.1"), &[]).none("KR-AG-028");
        let nested = check_with(
            "team/a.json",
            r#"{"description": "x"}"#,
            Some("2.32.1"),
            &[],
        );
        assert_eq!(nested.rules(), ["KR-AG-028"]);
        // Unpinned, a nested JSON agent targets V3 only, so a missing name is fine.
        check("team/a.json", r#"{"description": "x"}"#).none("KR-AG-019");
    }

    #[test]
    fn non_agent_json_gets_one_finding() {
        let content = json(serde_json::json!({"artifact": ".kiro/agents/ada.json", "spans": []}));
        let run = check("ada.json.attribution.json", &content);
        let diagnostic = run.one("KR-AG-020");
        assert_eq!(diagnostic.level, DiagnosticLevel::Warning);
        assert!(
            diagnostic.message.contains("ada.json.attribution"),
            "{}",
            diagnostic.message
        );
        assert_eq!(run.diagnostics.len(), 1, "{:#?}", run.diagnostics);
        let pinned = check_with("ada.json.attribution.json", &content, Some("2.x"), &[]);
        assert_eq!(pinned.one("KR-AG-020").level, DiagnosticLevel::Error);
    }

    #[test]
    fn empty_object_is_an_agent_without_name() {
        let run = check("a.json", "{}");
        run.none("KR-AG-020");
        assert_eq!(run.one("KR-AG-019").level, DiagnosticLevel::Warning);
    }

    // ----- names --------------------------------------------------------

    #[test]
    fn missing_name_is_fatal_in_v2_only() {
        let v2 = check(
            "a.json",
            r#"{"description": "x", "allowedTools": ["read"]}"#,
        );
        assert_eq!(v2.one("KR-AG-019").level, DiagnosticLevel::Error);
        let neutral = check("a.json", r#"{"description": "x"}"#);
        let diagnostic = neutral.one("KR-AG-019");
        assert_eq!(diagnostic.level, DiagnosticLevel::Warning);
        assert!(diagnostic.message.contains("'a'"), "{}", diagnostic.message);
        check(
            "a.json",
            r#"{"description": "x", "permissions": {"rules": []}}"#,
        )
        .none("KR-AG-019");
        check("a.md", "---\ndescription: x\n---\nBody").none("KR-AG-019");
    }

    #[test]
    fn explicit_name_checks() {
        assert_eq!(
            check("a.json", r#"{"name": "", "permissions": {"rules": []}}"#)
                .one("KR-AG-008")
                .level,
            DiagnosticLevel::Error
        );
        assert_eq!(
            check("a.json", r#"{"name": ""}"#).one("KR-AG-008").level,
            DiagnosticLevel::Warning
        );
        assert_eq!(
            check_with("a.json", r#"{"name": ""}"#, Some("2.x"), &[])
                .one("KR-AG-008")
                .level,
            DiagnosticLevel::Warning
        );
        assert_eq!(
            check("a.json", r#"{"name": 5}"#).one("KR-AG-008").level,
            DiagnosticLevel::Error
        );
        assert_eq!(
            check("a.json", r#"{"name": null}"#).one("KR-AG-008").level,
            DiagnosticLevel::Error
        );
        assert_eq!(
            check("a.json", r#"{"name": "  "}"#).one("KR-AG-008").level,
            DiagnosticLevel::Warning
        );
        check("a.json", r#"{"name": "ok"}"#).none("KR-AG-008");
    }

    #[test]
    fn reserved_v3_ids() {
        let run = check("plan.md", "---\ndescription: x\n---\nBody");
        assert_eq!(run.one("KR-AG-022").level, DiagnosticLevel::Error);
        let named = check_with(
            "a.json",
            r#"{"name": "spec", "tools": ["read"]}"#,
            None,
            &[],
        );
        assert_eq!(named.one("KR-AG-022").level, DiagnosticLevel::Warning);
        check_with("a.json", r#"{"name": "spec"}"#, Some("2.x"), &[]).none("KR-AG-022");
    }

    #[test]
    fn duplicate_agent_names_across_files() {
        let run = check_with(
            "a.json",
            r#"{"name": "reviewer", "description": "x"}"#,
            None,
            &[(".kiro/agents/b.json", r#"{"name": "reviewer"}"#)],
        );
        let diagnostic = run.one("KR-AG-022");
        assert!(
            diagnostic.message.contains("b.json"),
            "{}",
            diagnostic.message
        );
        assert!(diagnostic.message.contains("2.x") && diagnostic.message.contains("V3"));

        // V3 ids: a Markdown agent with the same path-derived id.
        let md = check_with(
            "reviewer.md",
            "---\ndescription: x\n---\nBody",
            None,
            &[(
                ".kiro/agents/reviewer.json",
                r#"{"description": "json twin"}"#,
            )],
        );
        assert!(md.one("KR-AG-022").message.contains("reviewer.json"));

        // A JSON sibling that V3 skips (CLI 2.x trust fields) does not collide in V3.
        check_with(
            "reviewer.md",
            "---\ndescription: x\n---\nBody",
            None,
            &[(
                ".kiro/agents/reviewer.json",
                r#"{"name": "reviewer", "allowedTools": []}"#,
            )],
        )
        .none("KR-AG-022");
    }

    // ----- field types and fields -----------------------------------------

    #[test]
    fn tools_shapes() {
        let star = check("a.json", r#"{"name": "a", "tools": "*"}"#);
        assert_eq!(star.one("KR-AG-018").level, DiagnosticLevel::Warning);
        check(
            "a.json",
            r#"{"name": "a", "tools": "*", "permissions": {"rules": []}}"#,
        )
        .none("KR-AG-018");
        assert_eq!(
            check("a.json", r#"{"name": "a", "tools": "read"}"#)
                .one("KR-AG-018")
                .level,
            DiagnosticLevel::Error
        );
        let item = check("a.json", "{\"name\": \"a\",\n \"tools\": [\"read\", 7]}");
        let diagnostic = item.one("KR-AG-018");
        assert_eq!(diagnostic.level, DiagnosticLevel::Error);
        assert_eq!((diagnostic.line, diagnostic.column), (2, 20));
        check("a.md", "---\ndescription: x\ntools: read, shell\n---\nBody").none("KR-AG-018");
    }

    #[test]
    fn field_type_levels_follow_the_engines() {
        let v3 = r#"{"name": "a", "excludedTools": "knowledge", "includePowers": "yes", "dispatchKind": "x"}"#;
        let run_v3 = check("a.json", v3);
        assert_eq!(run_v3.rule("KR-AG-018").len(), 3);
        assert!(
            run_v3
                .rule("KR-AG-018")
                .iter()
                .all(|d| d.level == DiagnosticLevel::Error)
        );
        assert_eq!(
            check("a.json", r#"{"name": "a", "includeMcpJson": "yes"}"#)
                .one("KR-AG-018")
                .level,
            DiagnosticLevel::Error
        );
        let alias = check(
            "a.json",
            r#"{"name": "a", "includeMcpJson": true, "useLegacyMcpJson": false}"#,
        );
        assert_eq!(alias.one("KR-AG-018").level, DiagnosticLevel::Error);
        let duplicate = check(
            "a.json",
            r#"{"name": "a", "tools": ["read"], "tools": ["write"]}"#,
        );
        assert_eq!(duplicate.one("KR-AG-018").level, DiagnosticLevel::Warning);
        let settings = check(
            "a.json",
            r#"{"name": "a", "toolsSettings": {"shell": 5}, "allowedTools": []}"#,
        );
        assert!(
            settings
                .one("KR-AG-018")
                .message
                .contains("toolsSettings.shell")
        );
        assert_eq!(
            check("a.md", "---\ndescription:\n---\nBody")
                .one("KR-AG-018")
                .level,
            DiagnosticLevel::Error
        );
        check(
            "a.json",
            r#"{"name": "a", "description": null, "welcomeMessage": null, "prompt": null}"#,
        )
        .none("KR-AG-018");
        assert_eq!(
            check("a.json", r#"{"name": "a", "resources": "file://x"}"#)
                .one("KR-AG-018")
                .level,
            DiagnosticLevel::Error
        );
    }

    #[test]
    fn documented_cli_2x_fields_are_not_unknown() {
        let content = r#"{"name": "a", "useLegacyMcpJson": false, "$schema": "x", "kiroUpgrade": {}, "keyboardShortcut": "ctrl+a"}"#;
        // Known fields: no "unknown field" warning. V3 does not apply the CLI
        // 2.x fields, which this universal profile (kiroUpgrade) gets as info.
        let run = check("a.json", content);
        assert!(
            run.rule("KR-AG-001")
                .iter()
                .all(|d| d.level == DiagnosticLevel::Info && d.message.contains("does not apply")),
            "{:#?}",
            run.diagnostics
        );
        check_with("a.json", content, Some("2.x"), &[]).none("KR-AG-001");
    }

    #[test]
    fn unknown_fields_say_json_or_frontmatter_and_point_at_the_key() {
        let json_run = check("a.json", "{\n  \"name\": \"a\",\n  \"notes\": 1\n}");
        let diagnostic = json_run.one("KR-AG-001");
        assert!(diagnostic.message.contains("JSON"));
        assert_eq!((diagnostic.line, diagnostic.column), (3, 3));
        let md = check("a.md", "---\ndescription: x\nmaxTurns: 3\n---\nBody");
        let diagnostic = md.one("KR-AG-001");
        assert!(
            diagnostic.message.contains("frontmatter"),
            "{}",
            diagnostic.message
        );
        assert_eq!((diagnostic.line, diagnostic.column), (3, 1));
    }

    #[test]
    fn fields_ignored_by_the_targeted_engine() {
        let md = check(
            "a.md",
            "---\ndescription: x\nallowedTools: [read]\nprompt: hi\n---\nBody",
        );
        let messages: Vec<_> = md
            .rule("KR-AG-001")
            .iter()
            .map(|d| d.message.clone())
            .collect();
        assert_eq!(messages.len(), 2, "{messages:?}");
        assert!(
            messages
                .iter()
                .any(|m| m.contains("allowedTools") && m.contains("Markdown"))
        );
        assert!(messages.iter().any(|m| m.contains("body")));
        md.none("KR-AG-015");

        let pinned_v3 = check_with(
            "a.json",
            r#"{"name": "a", "toolAliases": {}, "permissions": {"rules": []}}"#,
            Some("3.0"),
            &[],
        );
        assert_eq!(pinned_v3.one("KR-AG-001").level, DiagnosticLevel::Info);
        let pinned_v2 = check_with(
            "a.json",
            r#"{"name": "a", "excludedTools": []}"#,
            Some("2.32"),
            &[],
        );
        assert_eq!(pinned_v2.one("KR-AG-001").level, DiagnosticLevel::Info);
    }

    // ----- prompt -----------------------------------------------------------

    #[test]
    fn prompt_values() {
        check("a.json", r#"{"name": "a", "prompt": null}"#).none("KR-AG-009");
        assert_eq!(
            check("a.json", r#"{"name": "a", "prompt": ["x"]}"#)
                .one("KR-AG-009")
                .level,
            DiagnosticLevel::Error
        );
        assert_eq!(
            check("a.json", r#"{"name": "a", "prompt": " "}"#)
                .one("KR-AG-009")
                .level,
            DiagnosticLevel::Warning
        );
        assert_eq!(
            check("a.md", "---\ndescription: x\n---\n\n")
                .one("KR-AG-009")
                .level,
            DiagnosticLevel::Warning
        );
        check("a.md", "---\ndescription: x\n---\nDo work.").none("KR-AG-009");
    }

    #[test]
    fn file_prompt_targets() {
        let missing = check(
            "a.json",
            r#"{"name": "a", "prompt": "file://./prompts/a.md"}"#,
        );
        let diagnostic = missing.one("KR-AG-021");
        assert_eq!(diagnostic.level, DiagnosticLevel::Error);
        assert!(
            diagnostic.message.contains("/w/.kiro/agents/prompts/a.md"),
            "{}",
            diagnostic.message
        );

        let present = check_with(
            "a.json",
            r#"{"name": "a", "prompt": "file://./prompts/a.md"}"#,
            None,
            &[(".kiro/agents/prompts/a.md", "Prompt")],
        );
        present.none("KR-AG-021");

        let up = check_with(
            "a.json",
            r#"{"name": "a", "prompt": "file://../../docs/p.md"}"#,
            None,
            &[("docs/p.md", "Prompt")],
        );
        up.none("KR-AG-021");

        let escape = r#"{"name": "a", "prompt": "file://../../../outside.md", "permissions": {"rules": []}}"#;
        let fs_files = [("/outside.md", "Prompt")];
        let escaped = check_with("a.json", escape, None, &fs_files);
        assert_eq!(escaped.one("KR-AG-021").level, DiagnosticLevel::Error);
        assert!(
            escaped
                .one("KR-AG-021")
                .message
                .contains("outside the workspace")
        );

        assert_eq!(
            check("a.json", r#"{"name": "a", "prompt": "file://"}"#)
                .one("KR-AG-021")
                .level,
            DiagnosticLevel::Error
        );
    }

    #[test]
    fn absolute_file_prompts_fail_only_in_v3() {
        let v3 = r#"{"name": "a", "prompt": "file://~/p.md", "permissions": {"rules": []}}"#;
        assert_eq!(
            check("a.json", v3).one("KR-AG-021").level,
            DiagnosticLevel::Error
        );
        let neutral = r#"{"name": "a", "prompt": "file:///etc/p.md"}"#;
        assert_eq!(
            check("a.json", neutral).one("KR-AG-021").level,
            DiagnosticLevel::Warning
        );
        let v2 = r#"{"name": "a", "prompt": "file://~/p.md", "allowedTools": ["read"], "tools": ["read"]}"#;
        let migration = check("a.json", v2);
        let diagnostic = migration.one("KR-AG-021");
        assert_eq!(diagnostic.level, DiagnosticLevel::Warning);
        assert!(diagnostic.message.contains("3.0"), "{}", diagnostic.message);
        check_with("a.json", v2, Some("2.x"), &[]).none("KR-AG-021");
    }

    #[test]
    fn secrets_in_prompts() {
        let content =
            r#"{"name": "a", "prompt": "Use api_key= sk-test_PLACEHOLDER_NOT_REAL_KEY_000"}"#;
        assert_eq!(
            check("a.json", content).one("KR-AG-013").level,
            DiagnosticLevel::Error
        );
        check(
            "a.json",
            r#"{"name": "a", "prompt": "Use api_key=${MY_SECRET}"}"#,
        )
        .none("KR-AG-013");
        let md = check(
            "a.md",
            "---\ndescription: x\n---\nLine one\ntoken: ghp_abcdefghijklmnopqrstuvwxyz0123",
        );
        assert_eq!(md.one("KR-AG-013").line, 5);
    }

    // ----- model ------------------------------------------------------------

    #[test]
    fn model_values() {
        for model in [
            "claude-sonnet-4.6",
            "claude-opus-5.5",
            "gpt-5.6-terra",
            "auto",
            "glm-5",
        ] {
            check(
                "a.json",
                &json(serde_json::json!({"name": "a", "model": model})),
            )
            .none("KR-AG-004");
        }
        check("a.json", r#"{"name": "a", "model": null}"#).none("KR-AG-004");
        for model in ["sonnet", "Opus", "inherit", "haiku", "sonnet[1m]"] {
            let run = check(
                "a.json",
                &json(serde_json::json!({"name": "a", "model": model})),
            );
            assert_eq!(
                run.one("KR-AG-004").level,
                DiagnosticLevel::Warning,
                "{model}"
            );
        }
        assert_eq!(
            check("a.json", r#"{"name": "a", "model": "Claude Sonnet 4.6"}"#)
                .one("KR-AG-004")
                .level,
            DiagnosticLevel::Warning
        );
        assert_eq!(
            check("a.json", r#"{"name": "a", "model": 4}"#)
                .one("KR-AG-004")
                .level,
            DiagnosticLevel::Error
        );
        assert_eq!(
            check("a.md", "---\ndescription: x\nmodel:\n---\nBody")
                .one("KR-AG-004")
                .level,
            DiagnosticLevel::Error
        );
    }

    // ----- resources --------------------------------------------------------

    #[test]
    fn resource_entries() {
        let v2 = r#"{"name": "a", "allowedTools": [], "resources": [".kiro/steering/x.md"]}"#;
        assert_eq!(
            check("a.json", v2).one("KR-AG-002").level,
            DiagnosticLevel::Error
        );
        let universal = r#"{"name": "a", "allowedTools": [], "permissions": {"rules": []}, "resources": ["https://x"]}"#;
        let diagnostic_run = check("a.json", universal);
        let diagnostic = diagnostic_run.one("KR-AG-002");
        assert_eq!(diagnostic.level, DiagnosticLevel::Warning);
        assert!(
            diagnostic.message.contains("V3 drops"),
            "{}",
            diagnostic.message
        );
        let v3 = "---\ndescription: x\nresources:\n  - file://a.md\n  - README.md\n---\nBody";
        let md = check("a.md", v3);
        assert_eq!(md.one("KR-AG-002").level, DiagnosticLevel::Warning);
        assert_eq!(md.one("KR-AG-002").line, 5);
        let kb = r#"{"name": "a", "permissions": {"rules": []}, "resources": [{"type": "knowledgeBase", "source": "file://docs", "indexType": "deep"}]}"#;
        assert!(
            check("a.json", kb)
                .one("KR-AG-002")
                .message
                .contains("indexType")
        );
        let ok = r#"{"name": "a", "resources": ["file://a.md", "skill://.kiro/skills/*/SKILL.md", {"type": "knowledgeBase", "source": "file://docs"}]}"#;
        check("a.json", ok).none("KR-AG-002");
    }

    // ----- tools, allowedTools, toolAliases ----------------------------------

    #[test]
    fn allowed_tools_selector_semantics() {
        let covered = [
            (
                serde_json::json!(["*"]),
                serde_json::json!(["fs_read", "@git/status"]),
            ),
            (
                serde_json::json!(["@builtin"]),
                serde_json::json!(["read", "shell", "@builtin/fs_write"]),
            ),
            (
                serde_json::json!(["@git"]),
                serde_json::json!(["@git/git_status", "@git"]),
            ),
            (
                serde_json::json!(["read", "shell"]),
                serde_json::json!(["fs_read", "execute_bash", "grep"]),
            ),
            (
                serde_json::json!(["@server/read_a"]),
                serde_json::json!(["@server/read_*"]),
            ),
            (
                serde_json::json!(["@sap/*"]),
                serde_json::json!(["@sap/get"]),
            ),
        ];
        for (tools, allowed) in covered {
            let content =
                json(serde_json::json!({"name": "a", "tools": tools, "allowedTools": allowed}));
            check("a.json", &content).none("KR-AG-003");
        }
        let content = json(
            serde_json::json!({"name": "a", "tools": ["read", "grep"], "allowedTools": ["read", "@jj", "write"]}),
        );
        let run = check("a.json", &content);
        let flagged: Vec<_> = run
            .rule("KR-AG-003")
            .iter()
            .map(|d| d.message.clone())
            .collect();
        assert_eq!(flagged.len(), 2, "{flagged:?}");
        assert!(flagged[0].contains("@jj") && flagged[1].contains("write"));
        // V3-only files have no allowedTools semantics.
        let pinned = json(
            serde_json::json!({"name": "a", "tools": ["read"], "allowedTools": ["write"], "permissions": {"rules": []}}),
        );
        check_with("a.json", &pinned, Some("3.0"), &[]).none("KR-AG-003");
    }

    #[test]
    fn allowed_tools_that_trust_nothing() {
        let content = json(serde_json::json!({
            "name": "a",
            "tools": ["*"],
            "allowedTools": ["Read", "browser_navigate", "mcp__jira__get", "read_file", "github_issues", "fs_read"],
            "toolAliases": {"@github/get_issues": "github_issues"}
        }));
        let run = check("a.json", &content);
        let messages: Vec<_> = run
            .rule("KR-AG-024")
            .iter()
            .map(|d| d.message.clone())
            .collect();
        assert_eq!(messages.len(), 4, "{messages:?}");
        assert!(messages[0].contains("Claude Code") && messages[0].contains("'read'"));
        assert!(messages[1].contains("browser_navigate"));
        assert!(messages[2].contains("@jira/get"));
        assert!(messages[3].contains("V3 tool id"));
        run.none("KR-AG-003");
    }

    #[test]
    fn tools_that_select_nothing_in_an_engine() {
        let v3 = r#"{"name": "a", "tools": ["fs_read", "grep", "use_aws", "todo_list"], "permissions": {"rules": []}}"#;
        let run_v3 = check("a.json", v3);
        let messages: Vec<_> = run_v3
            .rule("KR-AG-024")
            .iter()
            .map(|d| (d.level, d.message.clone()))
            .collect();
        assert_eq!(messages.len(), 3, "{messages:?}");
        assert!(
            messages
                .iter()
                .all(|(level, _)| *level == DiagnosticLevel::Warning)
        );
        assert!(messages[2].1.contains("AWS"));
        // Covered by the V3 tag in a universal profile.
        let universal = r#"{"name": "a", "tools": ["read", "fs_read", "grep"], "allowedTools": ["fs_read"], "permissions": {"rules": []}}"#;
        check("a.json", universal).none("KR-AG-024");
        // Migration hints for a CLI 2.x-only profile cover only what
        // `kiro-cli agent upgrade` keeps: it rewrites fs_read to read.
        let v2 = r#"{"name": "a", "tools": ["fs_read", "grep", "aws", "use_subagent"], "allowedTools": ["fs_read"]}"#;
        let run_v2 = check("a.json", v2);
        let hints: Vec<_> = run_v2
            .rule("KR-AG-024")
            .iter()
            .map(|d| d.message.clone())
            .collect();
        assert_eq!(hints.len(), 2, "{hints:?}");
        assert!(hints[0].contains("'aws'") && hints[1].contains("'use_subagent'"));
        assert!(
            run_v2
                .rule("KR-AG-024")
                .iter()
                .all(|d| d.level == DiagnosticLevel::Info)
        );
        let grep_only = r#"{"name": "a", "tools": ["grep"], "allowedTools": []}"#;
        assert!(
            check("a.json", grep_only)
                .one("KR-AG-024")
                .message
                .contains("'grep'")
        );
        // AWS trust without the tool in tools (tools "*").
        let aws_trust = r#"{"name": "a", "tools": ["*"], "allowedTools": ["use_aws"], "toolsSettings": {"use_aws": {}}}"#;
        let aws_run = check("a.json", aws_trust);
        let diagnostic = aws_run.one("KR-AG-024");
        assert_eq!(diagnostic.level, DiagnosticLevel::Info);
        assert!(diagnostic.message.contains("AWS"), "{}", diagnostic.message);
        // V3 ids in a CLI 2.x-only file.
        let pinned = r#"{"name": "a", "tools": ["read_file"]}"#;
        assert!(
            check_with("a.json", pinned, Some("2.x"), &[])
                .one("KR-AG-024")
                .message
                .contains("'read'")
        );
        // Claude Code names select nothing in either engine.
        let claude = check(
            "a.md",
            "---\ndescription: x\ntools: [Read, Bash]\n---\nBody",
        );
        assert_eq!(claude.rule("KR-AG-024").len(), 2);
    }

    #[test]
    fn duplicate_and_empty_tools() {
        let dup = check(
            "a.json",
            "{\"name\": \"a\",\n\"tools\": [\"read\", \"READ\"]}",
        );
        let diagnostic = dup.one("KR-AG-010");
        assert_eq!((diagnostic.line, diagnostic.column), (2, 19));
        assert_eq!(
            check("a.json", r#"{"name": "a", "tools": []}"#)
                .one("KR-AG-011")
                .level,
            DiagnosticLevel::Info
        );
    }

    #[test]
    fn tool_aliases_check_the_original_tool() {
        let docs = r#"{"name": "a", "tools": ["@github-mcp"], "toolAliases": {"@github-mcp/get_issues": "github_issues"}}"#;
        check("a.json", docs).none("KR-AG-012");
        let missing = r#"{"name": "a", "tools": ["read"], "toolAliases": {"@github-mcp/get_issues": "github_issues"}}"#;
        assert!(
            check("a.json", missing)
                .one("KR-AG-012")
                .message
                .contains("@github-mcp/get_issues")
        );
        check("a.json", r#"{"name": "a", "toolAliases": {"@x/y": "z"}}"#).none("KR-AG-012");
    }

    // ----- MCP ---------------------------------------------------------------

    #[test]
    fn mcp_access_only_for_referenced_servers() {
        check("a.json", r#"{"name": "a", "includeMcpJson": false}"#).none("KR-AG-005");
        let referenced = r#"{"name": "a", "includeMcpJson": false, "tools": ["read", "@link"], "allowedTools": ["@link/check"]}"#;
        let run_ref = check("a.json", referenced);
        assert_eq!(run_ref.one("KR-AG-005").level, DiagnosticLevel::Warning);
        let inline = r#"{"name": "a", "includeMcpJson": false, "tools": ["@link"], "mcpServers": {"link": {"command": "x"}}}"#;
        check("a.json", inline).none("KR-AG-005");
        let v3 = r#"{"name": "a", "includeMcpJson": false, "tools": ["@link"], "permissions": {"rules": []}}"#;
        check("a.json", v3).none("KR-AG-005");
    }

    #[test]
    fn inline_mcp_servers() {
        let array = r#"{"name": "a", "allowedTools": [], "mcpServers": ["filesystem"]}"#;
        assert_eq!(
            check("a.json", array).one("KR-AG-023").level,
            DiagnosticLevel::Error
        );
        let typed = r#"{"name": "a", "mcpServers": {"git": {"command": 5}}}"#;
        let typed_run = check("a.json", typed);
        let diagnostic = typed_run.one("KR-AG-023");
        assert_eq!(diagnostic.level, DiagnosticLevel::Warning);
        assert!(diagnostic.message.contains("V3 skips only this server"));
        let v3 = r#"{"name": "a", "permissions": {"rules": []}, "mcpServers": {
            "remote": {"url": "http://example.com/mcp"},
            "local": {"url": "http://localhost:3000/mcp"},
            "nothing": {"args": []},
            "off": {"disabled": true},
            "zero": {"command": "x", "timeout": 0}
        }}"#;
        let v3_run = check("a.json", v3);
        let messages: Vec<_> = v3_run
            .rule("KR-AG-023")
            .iter()
            .map(|d| d.message.clone())
            .collect();
        assert_eq!(messages.len(), 3, "{messages:?}");
        assert!(
            messages
                .iter()
                .any(|m| m.contains("'remote'") && m.contains("https"))
        );
        assert!(messages.iter().any(|m| m.contains("'nothing'")));
        assert!(messages.iter().any(|m| m.contains("'zero'")));
        check("a.json", r#"{"name": "a", "mcpServers": {"git": {"command": "git-mcp", "args": ["x"], "env": {"A": "b"}}}}"#)
            .none("KR-AG-023");
    }

    // ----- permissions ---------------------------------------------------------

    #[test]
    fn permissions_shape_errors_skip_the_agent_in_v3() {
        let cases = [
            r#"{"permissions": {}}"#,
            r#"{"permissions": {"rules": {}}}"#,
            r#"{"permissions": {"rules": [{"effect": "allow"}]}}"#,
            r#"{"permissions": {"rules": [{"capability": "shell", "effect": "sometimes"}]}}"#,
            r#"{"permissions": {"rules": [{"capability": "shell", "effect": "allow", "match": "git *"}]}}"#,
            r#"{"permissions": {"rules": [], "policies": "read-all"}}"#,
        ];
        for case in cases {
            let run = check("a.json", case);
            assert_eq!(run.one("KR-AG-014").level, DiagnosticLevel::Error, "{case}");
        }
        let universal =
            r#"{"allowedTools": [], "permissions": {"rules": [{"capability": "shell"}]}}"#;
        assert_eq!(
            check("a.json", universal).one("KR-AG-014").level,
            DiagnosticLevel::Warning
        );
    }

    #[test]
    fn permissions_values_v3_skips_or_ignores() {
        let content = json(serde_json::json!({"permissions": {
            "rules": [
                {"capability": "execute_bash", "effect": "allow"},
                {"capability": "shell", "effect": "evaluate"},
                {"capability": "shell", "effect": "allow", "matches": ["git *"]},
                {"capability": "workflow", "effect": "ask"},
                {"capability": "fs_read", "effect": "allow", "match": ["./**"]}
            ],
            "policies": ["read-only-shell", "everything"],
            "defaultEffect": "deny"
        }}));
        let run = check("a.json", &content);
        let messages: Vec<_> = run
            .rule("KR-AG-014")
            .iter()
            .map(|d| (d.level, d.message.clone()))
            .collect();
        assert_eq!(messages.len(), 5, "{messages:#?}");
        assert!(
            messages
                .iter()
                .all(|(level, _)| *level == DiagnosticLevel::Warning)
        );
        assert!(messages.iter().any(|(_, m)| m.contains("every resource")));
        assert!(messages.iter().any(|(_, m)| m.contains("everything")));
        assert!(messages.iter().any(|(_, m)| m.contains("defaultEffect")));
    }

    #[test]
    fn permissions_null() {
        assert_eq!(
            check(
                "a.json",
                r#"{"name": "a", "permissions": null, "includePowers": true}"#
            )
            .one("KR-AG-014")
            .level,
            DiagnosticLevel::Error
        );
        // CLI 2.x-only file: V2 ignores permissions; KR-AG-015 covers the V3 skip.
        let v2 = check(
            "a.json",
            r#"{"name": "a", "permissions": null, "allowedTools": []}"#,
        );
        v2.none("KR-AG-014");
        v2.one("KR-AG-015");
    }

    // ----- KR-AG-015 -----------------------------------------------------------

    #[test]
    fn cli_only_profiles() {
        let content = r#"{"name": "a", "tools": ["read"], "allowedTools": ["read"]}"#;
        let unpinned = check("a.json", content);
        assert_eq!(unpinned.one("KR-AG-015").level, DiagnosticLevel::Warning);
        assert!(unpinned.one("KR-AG-015").fixes.is_empty());
        assert_eq!(
            check_with("a.json", content, Some("3.0.0"), &[])
                .one("KR-AG-015")
                .level,
            DiagnosticLevel::Error
        );
        check_with("a.json", content, Some("2.32.1"), &[]).none("KR-AG-015");
        check(
            "a.json",
            r#"{"name": "a", "allowedTools": ["read"], "permissions": {"rules": []}}"#,
        )
        .none("KR-AG-015");
        check(
            "a.md",
            "---\ndescription: x\nallowedTools: [read]\n---\nBody",
        )
        .none("KR-AG-015");
        // Key presence counts, even when the value is null or empty.
        check("a.json", r#"{"name": "a", "allowedTools": null}"#).one("KR-AG-015");
    }

    #[test]
    fn empty_cli_only_fields_have_an_unsafe_removal_fix() {
        let content = "{\n  \"name\": \"a\",\n  \"toolsSettings\": {},\n  \"includePowers\": true,\n  \"allowedTools\": []\n}";
        let run = check("a.json", content);
        let diagnostic = run.one("KR-AG-015");
        assert_eq!(diagnostic.fixes.len(), 2);
        assert!(diagnostic.fixes.iter().all(|fix| !fix.is_safe()));
        let fixed = run.apply_all();
        let value: serde_json::Value = serde_json::from_str(&fixed).unwrap();
        assert_eq!(
            value,
            serde_json::json!({"name": "a", "includePowers": true})
        );

        for (content, expected) in [
            (
                r#"{"name": "a", "toolsSettings": {}, "allowedTools": []}"#,
                serde_json::json!({"name": "a"}),
            ),
            (
                r#"{"toolsSettings": {}, "allowedTools": null, "name": "a"}"#,
                serde_json::json!({"name": "a"}),
            ),
            (
                r#"{"allowedTools": [], "name": "a", "toolsSettings": {}}"#,
                serde_json::json!({"name": "a"}),
            ),
        ] {
            let adjacent = check_with("a.json", content, None, &[]);
            let fixed = adjacent.apply_all();
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&fixed).unwrap(),
                expected,
                "{content}"
            );
        }

        let only = "{\"toolsSettings\": {}}";
        let only_run = check_with("a.json", only, None, &[]);
        let fixed = only_run.apply_all();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&fixed).unwrap(),
            serde_json::json!({})
        );
    }

    // ----- hooks ---------------------------------------------------------------

    #[test]
    fn object_hook_trigger_keys() {
        let content = "{\n  \"name\": \"a\",\n  \"hooks\": {\n    \"PreToolUse\": [{\"command\": \"echo\"}]\n  }\n}";
        let run = check("a.json", content);
        let diagnostic = run.one("KR-HK-005");
        assert_eq!(diagnostic.level, DiagnosticLevel::Error);
        assert_eq!((diagnostic.line, diagnostic.column), (4, 5));
        assert_eq!(diagnostic.fixes.len(), 1);
        assert!(!diagnostic.fixes[0].is_safe());
        let fixed = run.apply_all();
        let value: serde_json::Value = serde_json::from_str(&fixed).unwrap();
        assert_eq!(value["hooks"]["preToolUse"][0]["command"], "echo");

        let universal = r#"{"name": "a", "permissions": {"rules": []}, "hooks": {"SessionStart": [{"command": "x"}]}}"#;
        let universal_run = check_with("a.json", universal, None, &[]);
        let diagnostic = universal_run.one("KR-HK-005");
        assert_eq!(diagnostic.level, DiagnosticLevel::Warning);
        assert!(
            diagnostic.message.contains("V3 runs it as SessionStart"),
            "{}",
            diagnostic.message
        );

        let pinned = r#"{"name": "a", "hooks": {"PreToolUse": [{"command": "x"}], "beforePrompt": [{"command": "y"}]}}"#;
        let pinned_run = check_with("a.json", pinned, Some("3.x"), &[]);
        assert!(pinned_run.one("KR-HK-005").message.contains("beforePrompt"));

        // No fix when the camelCase key already exists.
        let clash =
            r#"{"name": "a", "hooks": {"stop": [{"command": "x"}], "Stop": [{"command": "y"}]}}"#;
        assert!(check("a.json", clash).one("KR-HK-005").fixes.is_empty());
    }

    #[test]
    fn object_hook_entries() {
        let content = json(serde_json::json!({"name": "a", "hooks": {
            "agentSpawn": {"command": "x"},
            "preToolUse": [{"matcher": "read"}, {"command": "", "timeout_ms": "5"}, "x"]
        }}));
        let run = check("a.json", &content);
        let issues: Vec<_> = run
            .rule("KR-HK-006")
            .iter()
            .map(|d| (d.level, d.message.clone()))
            .collect();
        assert_eq!(issues.len(), 5, "{issues:#?}");
        assert!(issues[0].1.contains("hooks.agentSpawn") && issues[0].0 == DiagnosticLevel::Error);
        assert!(
            issues
                .iter()
                .any(|(_, m)| m.contains("missing the required command"))
        );
        assert!(issues.iter().any(|(_, m)| m.contains("timeout_ms")));
        assert!(issues.iter().any(|(_, m)| m.contains("empty command")));
        check_with(
            "a.json",
            r#"{"name": "a", "hooks": {"preToolUse": [{"command": "echo", "matcher": "fs_write", "timeout_ms": 5000}]}}"#,
            None,
            &[],
        )
        .none("KR-HK-006");
    }

    #[test]
    fn hook_timeouts_and_ignored_fields() {
        let object = json(serde_json::json!({"name": "a", "hooks": {"stop": [
            {"command": "a", "timeout": 30},
            {"command": "b", "timeout_ms": 30},
            {"command": "c", "toolTypes": ["read"], "description": "kept"}
        ]}}));
        let run = check("a.json", &object);
        assert_eq!(run.rule("KR-AG-026").len(), 2, "{:#?}", run.diagnostics);
        assert!(run.one("KR-AG-027").message.contains("toolTypes"));

        let array = json(serde_json::json!({"name": "a", "hooks": [
            {"name": "a", "trigger": "Stop", "action": {"type": "command", "command": "x"}, "timeout": 30000},
            {"name": "b", "trigger": "Stop", "action": {"type": "command", "command": "x"}, "timeout_ms": 5000},
            {"name": "c", "trigger": "Stop", "action": {"type": "command", "command": "x"}, "when": {}}
        ]}));
        let run = check("a.json", &array);
        let timeouts: Vec<_> = run
            .rule("KR-AG-026")
            .iter()
            .map(|d| d.message.clone())
            .collect();
        assert_eq!(timeouts.len(), 2, "{timeouts:?}");
        assert!(timeouts[0].contains("30 s"));
        assert!(run.one("KR-AG-027").message.contains("when"));

        let pinned = r#"{"name": "a", "hooks": {"stop": [{"command": "x", "timeout_ms": 5000}]}}"#;
        assert_eq!(
            check_with("a.json", pinned, Some("3.0"), &[])
                .one("KR-AG-027")
                .level,
            DiagnosticLevel::Info
        );
    }

    #[test]
    fn array_hook_entries() {
        let content = json(serde_json::json!({"name": "a", "hooks": [
            {"trigger": "Stop", "action": {"type": "command", "command": "x"}},
            {"name": "b", "trigger": "Stop"},
            {"name": "c", "trigger": "Stop", "action": {"type": "script", "command": "x"}},
            {"name": "d", "trigger": "Stop", "action": {"type": "command", "command": ""}},
            {"name": "e", "trigger": "Stop", "action": {"type": "command", "command": "x"}, "timeout": -1},
            {"name": "f", "trigger": "Stop", "action": {"type": "agent", "prompt": "p"}, "confirm": {"question": ""}},
            {"name": "g", "trigger": "PreToolUse", "matcher": "read", "action": {"type": "command", "command": "x"}, "enabled": false}
        ]}));
        let run = check("a.json", &content);
        let issues: Vec<_> = run
            .rule("KR-HK-006")
            .iter()
            .map(|d| (d.level, d.message.clone()))
            .collect();
        assert_eq!(issues.len(), 6, "{issues:#?}");
        assert!(
            issues
                .iter()
                .all(|(level, _)| *level == DiagnosticLevel::Error)
        );
        run.none("KR-HK-005");
    }

    #[test]
    fn array_hook_triggers() {
        let content = "{\"name\": \"a\", \"hooks\": [\n  {\"name\": \"s\", \"trigger\": \"AgentSpawn\", \"action\": {\"type\": \"command\", \"command\": \"x\"}},\n  {\"name\": \"t\", \"trigger\": \"agentSpawn\", \"action\": {\"type\": \"command\", \"command\": \"x\"}},\n  {\"name\": \"u\", \"trigger\": \"onSave\", \"action\": {\"type\": \"command\", \"command\": \"x\"}}\n]}";
        let run = check("a.json", content);
        let triggers = run.rule("KR-HK-005");
        assert_eq!(triggers.len(), 2, "{triggers:#?}");
        assert!(triggers.iter().all(|d| d.level == DiagnosticLevel::Warning));
        assert_eq!(triggers[0].line, 2);
        assert_eq!(triggers[0].fixes.len(), 1);
        assert!(triggers[1].fixes.is_empty());
        let fixed = run.apply_all();
        let value: serde_json::Value = serde_json::from_str(&fixed).unwrap();
        assert_eq!(value["hooks"][0]["trigger"], "SessionStart");
        assert_eq!(value["hooks"][1]["trigger"], "agentSpawn");
    }

    #[test]
    fn hook_matchers_that_never_fire() {
        // V3-targeted agent hooks get the hooks validator's matcher rules.
        let universal = json(
            serde_json::json!({"name": "a", "allowedTools": [], "permissions": {"rules": []}, "hooks": {
                "preToolUse": [
                    {"command": "x", "matcher": "fs_read"},
                    {"command": "y", "matcher": "fs_write|glob"},
                    {"command": "z", "matcher": "grep"},
                    {"command": "w"},
                    {"command": "v", "matcher": "*"}
                ]
            }}),
        );
        let run = check("a.json", &universal);
        let foreign = run.one("KR-HK-021");
        assert_eq!(foreign.level, DiagnosticLevel::Warning);
        assert!(foreign.message.contains("fs_read"), "{}", foreign.message);
        assert_eq!(run.one("KR-HK-029").level, DiagnosticLevel::Info);
        assert_eq!(run.rule("KR-HK-027").len(), 2, "{:#?}", run.diagnostics);
        run.none("KR-AG-025");
        // The array form too.
        let array = r#"{"name": "a", "hooks": [{"name": "h", "trigger": "PreToolUse", "matcher": "Bash", "action": {"type": "command", "command": "x"}}]}"#;
        assert!(
            check("a.json", array)
                .one("KR-HK-021")
                .message
                .contains("Bash")
        );
        // CLI 2.x-only profile: a migration note, and no V3 scope notes.
        let v2 = r#"{"name": "a", "hooks": {"preToolUse": [{"command": "x", "matcher": "use_aws"}, {"command": "y"}]}}"#;
        let v2_run = check_with("a.json", v2, None, &[]);
        assert_eq!(v2_run.one("KR-HK-021").level, DiagnosticLevel::Info);
        v2_run.none("KR-HK-027");
        check_with("a.json", v2, Some("2.x"), &[]).none("KR-HK-021");
        // A V3 tool id in a matcher CLI 2.x reads.
        let v3_ids =
            r#"{"name": "a", "hooks": {"preToolUse": [{"command": "x", "matcher": "read_file"}]}}"#;
        assert!(
            check_with("a.json", v3_ids, Some("2.x"), &[])
                .one("KR-AG-025")
                .message
                .contains("2.x")
        );
        // Not a tool trigger: matchers are ignored there.
        let stop = check(
            "a.json",
            r#"{"name": "a", "hooks": {"stop": [{"command": "x", "matcher": "fs_read"}]}}"#,
        );
        stop.none("KR-AG-025");
        stop.none("KR-HK-021");
    }

    // ----- survey additions: KR-AG-006, KR-AG-029, KR-AG-030, legacy fields ----

    #[test]
    fn orchestrators_need_a_subagent_tool() {
        let missing = r#"{"name": "a", "description": "Coordinates the release work", "tools": ["read"], "toolsSettings": {"subagent": {"availableAgents": ["reviewer"]}}}"#;
        let sibling = [(".kiro/agents/reviewer.json", r#"{"name": "reviewer"}"#)];
        let run = check_with("a.json", missing, None, &sibling);
        let diagnostic = run.one("KR-AG-006");
        assert_eq!(diagnostic.level, DiagnosticLevel::Warning);
        assert!(diagnostic.message.contains("cannot delegate"));
        for tools in [
            r#"["read", "subagent"]"#,
            r#"["@builtin"]"#,
            r#"["read", "subagent/reviewer"]"#,
        ] {
            let content = missing.replace(r#"["read"]"#, tools);
            check_with("a.json", &content, None, &sibling).none("KR-AG-006");
        }
    }

    #[test]
    fn subagent_lists_name_known_agents() {
        let content = json(serde_json::json!({
            "name": "lead",
            "tools": ["subagent", "subagent/helper", "subagent/ghost"],
            "toolsSettings": {"subagent": {
                "availableAgents": ["review-*", "kiro_default", "lead", "missing-one"],
                "trustedAgents": ["review-bot"]
            }},
            "permissions": {"rules": []}
        }));
        let extra = [
            (".kiro/agents/review-bot.json", r#"{"name": "review-bot"}"#),
            (
                ".kiro/agents/team/helper.md",
                "---\nname: helper\ndescription: Helps with things\n---\nHelp.",
            ),
        ];
        let run = check_with("lead.json", &content, None, &extra);
        let unknown: Vec<_> = run
            .rule("KR-AG-006")
            .iter()
            .map(|d| (d.level, d.message.clone()))
            .collect();
        assert_eq!(unknown.len(), 2, "{unknown:#?}");
        assert!(
            unknown
                .iter()
                .all(|(level, _)| *level == DiagnosticLevel::Info)
        );
        assert!(unknown[0].1.contains("missing-one") && unknown[1].1.contains("ghost"));
    }

    #[test]
    fn broad_resource_globs() {
        for resource in ["file://**", "file://./**/*", "file://*"] {
            let content = json(serde_json::json!({"name": "a", "resources": [resource]}));
            assert_eq!(
                check("a.json", &content).one("KR-AG-029").level,
                DiagnosticLevel::Info,
                "{resource}"
            );
        }
        check(
            "a.json",
            r#"{"name": "a", "resources": ["file://docs/**/*.md", "file://README.md"]}"#,
        )
        .none("KR-AG-029");
    }

    #[test]
    fn descriptions_for_sub_agent_selection() {
        let lead = [(
            ".kiro/agents/lead.json",
            r#"{"name": "lead", "tools": ["subagent"], "toolsSettings": {"subagent": {"availableAgents": ["review*"]}}}"#,
        )];
        let missing = check_with("reviewer.json", r#"{"name": "reviewer"}"#, None, &lead);
        let diagnostic = missing.one("KR-AG-030");
        assert_eq!(diagnostic.level, DiagnosticLevel::Info);
        assert!(
            diagnostic.message.contains("lead.json"),
            "{}",
            diagnostic.message
        );
        let short = check_with(
            "reviewer.json",
            r#"{"name": "reviewer", "description": "Dev agent"}"#,
            None,
            &lead,
        );
        assert!(short.one("KR-AG-030").message.contains("short"));
        check_with(
            "reviewer.json",
            r#"{"name": "reviewer", "description": "Reviews Rust changes for safety issues"}"#,
            None,
            &lead,
        )
        .none("KR-AG-030");
        // A V3 subagent/<id> selector also makes the agent a sub-agent.
        let v3_lead = [(
            ".kiro/agents/lead.json",
            r#"{"name": "lead", "tools": ["subagent/team/helper"], "permissions": {"rules": []}}"#,
        )];
        assert_eq!(
            check_with(
                "team/helper.md",
                "---\nmodel: claude-opus-5.5\n---\nBody",
                None,
                &v3_lead
            )
            .one("KR-AG-030")
            .level,
            DiagnosticLevel::Info
        );
        // Agents nobody lists as a sub-agent are not reported.
        check("a.json", r#"{"name": "a"}"#).none("KR-AG-030");
    }

    #[test]
    fn allowed_tools_star_trusts_nothing() {
        let run = check(
            "a.json",
            r#"{"name": "a", "tools": ["*"], "allowedTools": ["*", "read"]}"#,
        );
        let diagnostic = run.one("KR-AG-024");
        assert!(
            diagnostic.message.contains("\"*\""),
            "{}",
            diagnostic.message
        );
        run.none("KR-AG-003");
        check_with(
            "a.json",
            r#"{"name": "a", "allowedTools": ["*"]}"#,
            Some("3.0"),
            &[],
        )
        .none("KR-AG-024");
    }

    #[test]
    fn cli_2x_fields_that_v3_does_not_apply() {
        let universal = r#"{"name": "a", "useLegacyMcpJson": true, "keyboardShortcut": "ctrl+r", "permissions": {"rules": []}}"#;
        let run = check("a.json", universal);
        let notes: Vec<_> = run
            .rule("KR-AG-001")
            .iter()
            .map(|d| (d.level, d.message.clone()))
            .collect();
        assert_eq!(notes.len(), 2, "{notes:#?}");
        assert!(
            notes
                .iter()
                .any(|(level, m)| *level == DiagnosticLevel::Warning && m.contains("mcp.json"))
        );
        assert!(notes.iter().any(|(level, m)| *level == DiagnosticLevel::Info && m.contains("keyboardShortcut")));
        let v2 = r#"{"name": "a", "useLegacyMcpJson": true, "allowedTools": []}"#;
        assert_eq!(
            check("a.json", v2).one("KR-AG-001").level,
            DiagnosticLevel::Info
        );
        check_with("a.json", v2, Some("2.x"), &[]).none("KR-AG-001");
        // With includeMcpJson set, useLegacyMcpJson is only a CLI 2.x duplicate.
        let both = r#"{"name": "a", "useLegacyMcpJson": true, "includeMcpJson": true, "permissions": {"rules": []}}"#;
        check("a.json", both).none("KR-AG-001");
        check(
            "a.json",
            r#"{"name": "a", "useLegacyMcpJson": false, "permissions": {"rules": []}}"#,
        )
        .none("KR-AG-001");
    }

    // ----- misc ------------------------------------------------------------------

    #[test]
    fn valid_agents_have_no_findings() {
        let v2 = json(serde_json::json!({
            "$schema": "https://example.com/agent.json",
            "name": "reviewer",
            "description": "Reviews pull requests for correctness and style",
            "prompt": null,
            "model": "claude-sonnet-4.6",
            "tools": ["read", "@builtin", "@git"],
            "allowedTools": ["fs_read", "@git/git_status"],
            "toolAliases": {"@git/git_status": "status"},
            "toolsSettings": {"shell": {"allowedCommands": ["git status"]}},
            "resources": ["file://README.md"],
            "includeMcpJson": true,
            "useLegacyMcpJson": true,
            "keyboardShortcut": "ctrl+r",
            "hooks": {"preToolUse": [{"matcher": "execute_bash", "command": "echo", "timeout_ms": 5000}]}
        }));
        let run_v2 = check("reviewer.json", &v2);
        assert!(
            run_v2
                .diagnostics
                .iter()
                // CLI 2.x-only: V3 migration notes (KR-AG-015, KR-AG-001 info) only.
                .all(|d| d.rule == "KR-AG-015"
                    || d.rule == "KR-AG-018"
                    || (d.rule == "KR-AG-001" && d.level == DiagnosticLevel::Info)),
            "{:#?}",
            run_v2.diagnostics
        );
        // useLegacyMcpJson duplicates includeMcpJson in CLI 2.x.
        run_v2.one("KR-AG-018");

        let v3 = "---\ndescription: Implements and reviews backend changes\nmodel: claude-opus-5.5\ntools: [read, write, shell, web]\nresources:\n  - file://ARCHITECTURE.md\npermissions:\n  rules:\n    - capability: shell\n      effect: allow\n      match: [\"npm *\"]\nhooks:\n  - name: lint\n    trigger: PostFileSave\n    matcher: \"\\\\.rs$\"\n    action:\n      type: command\n      command: cargo fmt\n    timeout: 30\n---\nImplement backend changes.";
        let run_v3 = check("backend.md", v3);
        assert!(run_v3.diagnostics.is_empty(), "{:#?}", run_v3.diagnostics);
    }

    #[test]
    fn unpinned_engine_dependent_findings_carry_an_assumption() {
        let run_neutral = check("a.json", r#"{"description": "x"}"#);
        assert!(
            run_neutral
                .one("KR-AG-019")
                .assumption
                .as_deref()
                .is_some_and(|a| a.contains("tool_versions"))
        );
        let pinned = check_with("a.json", r#"{"description": "x"}"#, Some("2.x"), &[]);
        assert!(pinned.one("KR-AG-019").assumption.is_none());
    }

    #[test]
    fn rules_can_be_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = ["KR-AG-019", "KR-AG-016", "KR-AG-030"]
            .map(str::to_string)
            .to_vec();
        let path = Path::new("/w/.kiro/agents/a.json");
        assert!(
            KiroAgentValidator
                .validate(path, r#"{"description": "x"}"#, &config)
                .is_empty()
        );
        assert!(KiroAgentValidator.validate(path, "{", &config).is_empty());
        config.rules_mut().kiro_agents = false;
        assert!(
            KiroAgentValidator
                .validate(path, r#"{"name": 5}"#, &config)
                .is_empty()
        );
    }

    #[test]
    fn metadata_lists_rule_ids() {
        let metadata = KiroAgentValidator.metadata();
        assert_eq!(metadata.name, "KiroAgentValidator");
        assert_eq!(metadata.rule_ids, RULE_IDS);
        // KR-AG-007 is retired; KR-AG-006 checks the documented sub-agent lists.
        assert!(RULE_IDS.contains(&"KR-AG-006") && !RULE_IDS.contains(&"KR-AG-007"));
    }

    #[test]
    fn dynamic_locale_keys_resolve() {
        let keys = [
            "rules.kr_ag_engine.skip_both",
            "rules.kr_ag_engine.skip_v2",
            "rules.kr_ag_engine.skip_v2_partial",
            "rules.kr_ag_engine.skip_v3",
            "rules.kr_ag_engine.skip_v3_partial",
            "rules.kr_ag_engine.skip_v3_migration",
            "rules.kr_ag_020.both",
            "rules.kr_ag_020.v2",
            "rules.kr_ag_020.v3",
            "rules.kr_ag_022.duplicate_both",
            "rules.kr_ag_022.duplicate_v2",
            "rules.kr_ag_022.duplicate_v3",
            "rules.kr_ag_023.not_object_both",
            "rules.kr_ag_023.not_object",
            "rules.kr_ag_004.alias",
            "rules.kr_ag_004.display_name",
            "rules.kr_ag_005.message_both",
            "rules.kr_ag_005.message_v2",
            "rules.kr_ag_028.markdown",
            "rules.kr_ag_028.nested",
        ];
        for key in keys {
            let text = t!(key, locale = "en");
            assert!(!text.starts_with("rules."), "{key} does not resolve");
        }
    }

    #[test]
    fn markers_select_engines() {
        let markers = |value: serde_json::Value| engine_markers(value.as_object().unwrap());
        assert_eq!(markers(serde_json::json!({"name": "a"})), (false, false));
        assert_eq!(
            markers(serde_json::json!({"allowedTools": []})),
            (true, false)
        );
        assert_eq!(markers(serde_json::json!({"hooks": {}})), (true, false));
        assert_eq!(markers(serde_json::json!({"hooks": []})), (false, true));
        assert_eq!(
            markers(serde_json::json!({"permissions": null})),
            (false, false)
        );
        assert_eq!(
            markers(serde_json::json!({"includePowers": false})),
            (false, true)
        );
        assert_eq!(
            markers(serde_json::json!({"kiroUpgrade": {}})),
            (true, true)
        );
    }

    #[test]
    fn path_helpers() {
        assert!(glob_matches("@server/read_*", "@server/read_x"));
        assert!(glob_matches("r?ad", "read"));
        assert!(!glob_matches("r*", "write"));
        let (dir, rest) = agents_location(Path::new("/w/.kiro/agents/team/a.md")).unwrap();
        assert_eq!(dir, PathBuf::from("/w/.kiro/agents"));
        assert_eq!(path_agent_id(&rest), "team/a");
        assert!(escapes_root(
            Path::new("/w/.kiro/agents/../../../x"),
            Path::new("/w")
        ));
        assert!(!escapes_root(
            Path::new("/w/.kiro/agents/../../docs/x"),
            Path::new("/w")
        ));
        assert!(!escapes_root(
            Path::new(".kiro/agents/../../x"),
            Path::new("")
        ));
        assert!(escapes_root(
            Path::new(".kiro/agents/../../../x"),
            Path::new("")
        ));
    }
}
