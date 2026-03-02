//! Kiro agent validation rules (KR-AG-001 to KR-AG-007, KR-HK-005 to KR-HK-006).
//!
//! Validates cross-agent invocation references in `.kiro/agents/*.json`:
//! - KR-AG-001: Unknown top-level field in agent JSON.
//! - KR-AG-002: Invalid resource protocol/type.
//! - KR-AG-003: allowedTools contains tool not present in tools.
//! - KR-AG-004: Invalid model value.
//! - KR-AG-005: includeMcpJson disabled with no inline mcpServers.
//! - KR-AG-006: Prompt references a non-existent subagent.
//! - KR-AG-007: Invoking agent has a broader tool scope than referenced subagent.
//! - KR-HK-005: Invalid CLI hook event key.
//! - KR-HK-006: CLI hook entry missing required command.

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
    schemas::kiro_agent::VALID_KIRO_AGENT_MODELS,
};
use rust_i18n::t;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const RULE_IDS: &[&str] = &[
    "KR-AG-001",
    "KR-AG-002",
    "KR-AG-003",
    "KR-AG-004",
    "KR-AG-005",
    "KR-AG-006",
    "KR-AG-007",
    "KR-HK-005",
    "KR-HK-006",
];
const MAX_PROJECT_SEARCH_DEPTH: usize = 10;
const VALID_AGENT_FIELDS: &[&str] = &[
    "name",
    "description",
    "prompt",
    "model",
    "tools",
    "allowedTools",
    "toolAliases",
    "toolsSettings",
    "resources",
    "mcpServers",
    "includeMcpJson",
    "hooks",
    "keyboardShortcut",
    "welcomeMessage",
];
const VALID_CLI_HOOK_EVENTS: &[&str] = &[
    "agentSpawn",
    "userPromptSubmit",
    "preToolUse",
    "postToolUse",
    "stop",
];

#[derive(Debug, Clone)]
struct AgentInfo {
    tools: HashSet<String>,
    has_explicit_tool_scope: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AgentMention {
    name: String,
    byte_offset: usize,
}

fn normalize_agent_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn mention_regex() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"(^|[^A-Za-z0-9_@])@([A-Za-z][A-Za-z0-9_-]{0,63})")
            .expect("mention regex must compile")
    })
}

fn prompt_field_regex() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r#"(?is)"(?P<key>[A-Za-z0-9_]+)"\s*:\s*"(?P<value>(?:\\.|[^"\\])*)""#)
            .expect("prompt field regex must compile")
    })
}

fn is_prompt_field(key: &str) -> bool {
    let lowered = key.to_ascii_lowercase();
    lowered == "prompt" || lowered.ends_with("prompt")
}

fn extract_prompt_agent_mentions(content: &str) -> Vec<AgentMention> {
    let mut seen = HashSet::new();
    let mut mentions = Vec::new();

    for captures in prompt_field_regex().captures_iter(content) {
        let Some(key_match) = captures.name("key") else {
            continue;
        };
        if !is_prompt_field(key_match.as_str()) {
            continue;
        }

        let Some(value_match) = captures.name("value") else {
            continue;
        };

        for mention_captures in mention_regex().captures_iter(value_match.as_str()) {
            let Some(name_match) = mention_captures.get(2) else {
                continue;
            };

            let normalized = normalize_agent_name(name_match.as_str());
            if normalized.is_empty() {
                continue;
            }

            // Keep the first occurrence for stable diagnostics.
            if seen.insert(normalized.clone()) {
                mentions.push(AgentMention {
                    name: normalized,
                    byte_offset: value_match.start() + name_match.start().saturating_sub(1), // include '@'
                });
            }
        }
    }

    mentions
}

fn extract_tools(value: &Value) -> HashSet<String> {
    fn parse_tool_array(value: Option<&Value>) -> HashSet<String> {
        let mut tools = HashSet::new();
        let Some(array) = value.and_then(Value::as_array) else {
            return tools;
        };

        for item in array {
            if let Some(tool) = item.as_str() {
                let normalized = tool.trim().to_ascii_lowercase();
                if !normalized.is_empty() {
                    tools.insert(normalized);
                }
            }
        }
        tools
    }

    // Presence of allowedTools is authoritative, even when explicitly empty.
    if value.get("allowedTools").is_some() {
        return parse_tool_array(value.get("allowedTools"));
    }

    parse_tool_array(value.get("tools"))
}

fn has_explicit_tool_scope(value: &Value) -> bool {
    value.get("allowedTools").is_some() || value.get("tools").is_some()
}

fn extract_string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn normalize_tool_set(tools: &[String]) -> HashSet<String> {
    tools.iter().map(|tool| tool.to_ascii_lowercase()).collect()
}

fn has_inline_mcp_servers(value: &Value) -> bool {
    value
        .get("mcpServers")
        .and_then(Value::as_object)
        .is_some_and(|entries| !entries.is_empty())
}

fn is_valid_resource_entry(resource: &Value) -> bool {
    match resource {
        Value::String(uri) => {
            let normalized = uri.trim().to_ascii_lowercase();
            normalized.starts_with("file://") || normalized.starts_with("skill://")
        }
        Value::Object(obj) => obj
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|kind| kind == "knowledgeBase"),
        _ => false,
    }
}

fn validate_cli_hook_rules(
    path: &Path,
    current_agent: &Value,
    config: &LintConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let check_invalid_event = config.is_rule_enabled("KR-HK-005");
    let check_missing_command = config.is_rule_enabled("KR-HK-006");
    if !check_invalid_event && !check_missing_command {
        return;
    }

    let Some(hooks_obj) = current_agent.get("hooks").and_then(Value::as_object) else {
        return;
    };

    for (event, entries) in hooks_obj {
        let event_valid = VALID_CLI_HOOK_EVENTS.contains(&event.as_str());

        if check_invalid_event && !event_valid {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "KR-HK-005",
                    t!("rules.kr_hk_005.message", event = event.as_str()),
                )
                .with_suggestion(t!("rules.kr_hk_005.suggestion")),
            );
        }

        if !check_missing_command || !event_valid {
            continue;
        }

        let Some(entries_array) = entries.as_array() else {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "KR-HK-006",
                    t!("rules.kr_hk_006.message", event = event.as_str(), index = 0),
                )
                .with_suggestion(t!("rules.kr_hk_006.suggestion")),
            );
            continue;
        };

        for (index, entry) in entries_array.iter().enumerate() {
            let has_command = entry
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(|command| !command.trim().is_empty());
            if !has_command {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "KR-HK-006",
                        t!(
                            "rules.kr_hk_006.message",
                            event = event.as_str(),
                            index = index
                        ),
                    )
                    .with_suggestion(t!("rules.kr_hk_006.suggestion")),
                );
            }
        }
    }
}

fn validate_agent_schema_rules(
    path: &Path,
    current_agent: &Value,
    config: &LintConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let check_unknown_fields = config.is_rule_enabled("KR-AG-001");
    let check_resource_protocols = config.is_rule_enabled("KR-AG-002");
    let check_allowed_tools_subset = config.is_rule_enabled("KR-AG-003");
    let check_model = config.is_rule_enabled("KR-AG-004");
    let check_no_mcp_access = config.is_rule_enabled("KR-AG-005");
    if !check_unknown_fields
        && !check_resource_protocols
        && !check_allowed_tools_subset
        && !check_model
        && !check_no_mcp_access
    {
        return;
    }

    let Some(obj) = current_agent.as_object() else {
        return;
    };

    if check_unknown_fields {
        for key in obj.keys() {
            if VALID_AGENT_FIELDS.contains(&key.as_str()) {
                continue;
            }
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "KR-AG-001",
                    t!("rules.kr_ag_001.message", field = key.as_str()),
                )
                .with_suggestion(t!("rules.kr_ag_001.suggestion")),
            );
        }
    }

    if check_resource_protocols
        && let Some(resources) = current_agent.get("resources").and_then(Value::as_array)
    {
        for resource in resources {
            if is_valid_resource_entry(resource) {
                continue;
            }
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "KR-AG-002",
                    t!("rules.kr_ag_002.message", resource = resource.to_string()),
                )
                .with_suggestion(t!("rules.kr_ag_002.suggestion")),
            );
        }
    }

    if check_allowed_tools_subset
        && current_agent.get("allowedTools").is_some()
        && current_agent
            .get("tools")
            .and_then(Value::as_array)
            .is_some()
    {
        let tools = extract_string_array(current_agent.get("tools"));
        let tools_set = normalize_tool_set(&tools);
        let allowed_tools = extract_string_array(current_agent.get("allowedTools"));
        for allowed in allowed_tools {
            if !tools_set.contains(&allowed.to_ascii_lowercase()) {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "KR-AG-003",
                        t!("rules.kr_ag_003.message", tool = allowed.as_str()),
                    )
                    .with_suggestion(t!("rules.kr_ag_003.suggestion")),
                );
            }
        }
    }

    if check_model && let Some(model_value) = current_agent.get("model") {
        let is_valid_model = model_value
            .as_str()
            .is_some_and(|model| VALID_KIRO_AGENT_MODELS.contains(&model));
        if !is_valid_model {
            let model = model_value
                .as_str()
                .map(ToString::to_string)
                .unwrap_or_else(|| model_value.to_string());
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "KR-AG-004",
                    t!("rules.kr_ag_004.message", model = model.as_str()),
                )
                .with_suggestion(t!("rules.kr_ag_004.suggestion")),
            );
        }
    }

    if check_no_mcp_access
        && current_agent
            .get("includeMcpJson")
            .and_then(Value::as_bool)
            .is_some_and(|enabled| !enabled)
        && !has_inline_mcp_servers(current_agent)
    {
        diagnostics.push(
            Diagnostic::info(
                path.to_path_buf(),
                1,
                0,
                "KR-AG-005",
                t!("rules.kr_ag_005.message"),
            )
            .with_suggestion(t!("rules.kr_ag_005.suggestion")),
        );
    }
}

fn is_reserved_kiro_agent_filename(filename: &str) -> bool {
    let lowered = filename.to_ascii_lowercase();
    matches!(
        lowered.as_str(),
        "plugin.json" | "mcp.json" | "settings.json" | "settings.local.json"
    ) || lowered.starts_with("mcp-")
        || lowered.ends_with(".mcp.json")
}

fn line_col_at_offset(content: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;

    for (idx, ch) in content.char_indices() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}

fn find_kiro_agents_dir(path: &Path, config: &LintConfig) -> Option<PathBuf> {
    let fs = config.fs();

    if let Some(parent) = path.parent() {
        let parent_name = parent.file_name().and_then(|n| n.to_str());
        let grandparent_name = parent
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str());
        if let (Some(parent_name), Some(grandparent_name)) = (parent_name, grandparent_name) {
            if parent_name.eq_ignore_ascii_case("agents")
                && grandparent_name.eq_ignore_ascii_case(".kiro")
            {
                return Some(parent.to_path_buf());
            }
        }
    }

    let find_child_dir_case_insensitive = |parent: &Path, expected: &str| -> Option<PathBuf> {
        let Ok(entries) = fs.read_dir(parent) else {
            return None;
        };

        for entry in entries {
            if !entry.metadata.is_dir {
                continue;
            }

            let Some(name) = entry.path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            if name.eq_ignore_ascii_case(expected) {
                return Some(entry.path);
            }
        }

        None
    };

    let mut current = path.parent();
    let mut depth = 0usize;

    while let Some(dir) = current {
        if depth >= MAX_PROJECT_SEARCH_DEPTH {
            break;
        }

        let Some(kiro_dir) = find_child_dir_case_insensitive(dir, ".kiro") else {
            current = dir.parent();
            depth += 1;
            continue;
        };

        if let Some(agents_dir) = find_child_dir_case_insensitive(&kiro_dir, "agents") {
            return Some(agents_dir);
        }

        current = dir.parent();
        depth += 1;
    }

    None
}

fn load_agent_index(agents_dir: &Path, config: &LintConfig) -> HashMap<String, AgentInfo> {
    let fs = config.fs();
    let Ok(mut entries) = fs.read_dir(agents_dir) else {
        return HashMap::new();
    };

    entries.sort_by(|a, b| a.path.cmp(&b.path));

    let mut index: HashMap<String, AgentInfo> = HashMap::new();

    for entry in entries {
        if !entry.metadata.is_file {
            continue;
        }

        let Some(filename) = entry.path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if is_reserved_kiro_agent_filename(filename) {
            continue;
        }

        let is_json = entry
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));
        if !is_json {
            continue;
        }

        let Ok(raw) = fs.read_to_string(&entry.path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };

        let explicit_name = value.get("name").and_then(Value::as_str);
        let fallback_name = entry.path.file_stem().and_then(|stem| stem.to_str());
        let Some(name) = explicit_name.or(fallback_name) else {
            continue;
        };

        let normalized_name = normalize_agent_name(name);
        if normalized_name.is_empty() {
            continue;
        }

        // Keep first observed definition for deterministic conflict handling.
        index.entry(normalized_name).or_insert_with(|| AgentInfo {
            tools: extract_tools(&value),
            has_explicit_tool_scope: has_explicit_tool_scope(&value),
        });
    }

    index
}

pub struct KiroAgentValidator;

impl Validator for KiroAgentValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let check_unknown_fields = config.is_rule_enabled("KR-AG-001");
        let check_resource_protocols = config.is_rule_enabled("KR-AG-002");
        let check_allowed_tools_subset = config.is_rule_enabled("KR-AG-003");
        let check_model = config.is_rule_enabled("KR-AG-004");
        let check_no_mcp_access = config.is_rule_enabled("KR-AG-005");
        let check_missing_reference = config.is_rule_enabled("KR-AG-006");
        let check_tool_scope = config.is_rule_enabled("KR-AG-007");
        let check_cli_hook_event = config.is_rule_enabled("KR-HK-005");
        let check_cli_hook_command = config.is_rule_enabled("KR-HK-006");

        if !check_unknown_fields
            && !check_resource_protocols
            && !check_allowed_tools_subset
            && !check_model
            && !check_no_mcp_access
            && !check_missing_reference
            && !check_tool_scope
            && !check_cli_hook_event
            && !check_cli_hook_command
        {
            return diagnostics;
        }

        let Ok(current_agent) = serde_json::from_str::<Value>(content) else {
            return diagnostics;
        };

        validate_agent_schema_rules(path, &current_agent, config, &mut diagnostics);
        validate_cli_hook_rules(path, &current_agent, config, &mut diagnostics);

        if !check_missing_reference && !check_tool_scope {
            return diagnostics;
        }

        let mentions = extract_prompt_agent_mentions(content);
        if mentions.is_empty() {
            return diagnostics;
        }

        let current_name = current_agent
            .get("name")
            .and_then(Value::as_str)
            .map(normalize_agent_name)
            .or_else(|| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(normalize_agent_name)
            });
        let current_tools = extract_tools(&current_agent);

        let Some(agents_dir) = find_kiro_agents_dir(path, config) else {
            return diagnostics;
        };

        let known_agents = load_agent_index(&agents_dir, config);
        if known_agents.is_empty() {
            return diagnostics;
        }

        for mention in mentions {
            if current_name.as_ref() == Some(&mention.name) {
                continue;
            }

            let (line, col) = line_col_at_offset(content, mention.byte_offset);
            let display_name = mention.name.as_str();

            let Some(referenced_agent) = known_agents.get(&mention.name) else {
                if check_missing_reference {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            line,
                            col,
                            "KR-AG-006",
                            t!("rules.kr_ag_006.message", agent = display_name),
                        )
                        .with_suggestion(t!("rules.kr_ag_006.suggestion", agent = display_name)),
                    );
                }
                continue;
            };

            if !check_tool_scope || current_tools.is_empty() {
                continue;
            }
            if !referenced_agent.has_explicit_tool_scope {
                continue;
            }

            let mut extra_tools: Vec<String> = current_tools
                .difference(&referenced_agent.tools)
                .cloned()
                .collect();
            if extra_tools.is_empty() {
                continue;
            }

            extra_tools.sort();
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "KR-AG-007",
                    t!(
                        "rules.kr_ag_007.message",
                        agent = display_name,
                        extra_tools = extra_tools.join(", ")
                    ),
                )
                .with_suggestion(t!("rules.kr_ag_007.suggestion", agent = display_name)),
            );
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_agent(path: &Path, content: &str) {
        fs::write(path, content).unwrap_or_else(|e| {
            panic!("Failed writing {}: {}", path.display(), e);
        });
    }

    fn validate(path: &Path) -> Vec<Diagnostic> {
        let validator = KiroAgentValidator;
        let content = fs::read_to_string(path).unwrap();
        validator.validate(path, &content, &LintConfig::default())
    }

    #[test]
    fn test_kr_ag_001_unknown_fields() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let unknown = agents_dir.join("unknown-fields.json");
        write_agent(
            &unknown,
            include_str!("../../../../tests/fixtures/kiro-agents/.kiro/agents/unknown-fields.json"),
        );

        let diagnostics = validate(&unknown);
        let kr_ag_001: Vec<_> = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.rule == "KR-AG-001")
            .collect();
        assert_eq!(kr_ag_001.len(), 2);
    }

    #[test]
    fn test_kr_ag_002_invalid_resource_protocol() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let invalid = agents_dir.join("invalid-resource.json");
        write_agent(
            &invalid,
            include_str!(
                "../../../../tests/fixtures/kiro-agents/.kiro/agents/invalid-resource.json"
            ),
        );

        let diagnostics = validate(&invalid);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule == "KR-AG-002")
        );
    }

    #[test]
    fn test_kr_ag_003_allowed_tools_subset() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let invalid = agents_dir.join("mismatched-tools.json");
        write_agent(
            &invalid,
            include_str!(
                "../../../../tests/fixtures/kiro-agents/.kiro/agents/mismatched-tools.json"
            ),
        );

        let diagnostics = validate(&invalid);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule == "KR-AG-003")
        );
    }

    #[test]
    fn test_kr_ag_003_skips_when_tools_missing() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let missing_tools = agents_dir.join("missing-tools.json");
        write_agent(
            &missing_tools,
            r#"{
  "name": "missing-tools",
  "allowedTools": ["readFiles"]
}"#,
        );

        let diagnostics = validate(&missing_tools);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.rule != "KR-AG-003"),
            "KR-AG-003 should skip when tools is absent: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_kr_ag_004_invalid_model() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let invalid = agents_dir.join("invalid-model.json");
        write_agent(
            &invalid,
            include_str!("../../../../tests/fixtures/kiro-agents/.kiro/agents/invalid-model.json"),
        );

        let diagnostics = validate(&invalid);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule == "KR-AG-004")
        );
    }

    #[test]
    fn test_kr_ag_004_non_string_model() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let invalid = agents_dir.join("non-string-model.json");
        write_agent(
            &invalid,
            r#"{
  "name": "non-string-model",
  "model": 123
}"#,
        );

        let diagnostics = validate(&invalid);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule == "KR-AG-004")
        );
    }

    #[test]
    fn test_kr_ag_005_no_mcp_access_info() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let no_mcp = agents_dir.join("no-mcp-access.json");
        write_agent(
            &no_mcp,
            include_str!("../../../../tests/fixtures/kiro-agents/.kiro/agents/no-mcp-access.json"),
        );

        let diagnostics = validate(&no_mcp);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule == "KR-AG-005")
        );
    }

    #[test]
    fn test_kr_ag_005_mcp_server_references_do_not_count_as_inline() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let by_reference = agents_dir.join("mcp-references.json");
        write_agent(
            &by_reference,
            r#"{
  "name": "mcp-references",
  "includeMcpJson": false,
  "mcpServers": ["github"]
}"#,
        );

        let diagnostics = validate(&by_reference);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule == "KR-AG-005")
        );
    }

    #[test]
    fn test_kr_hk_005_invalid_cli_hook_event() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let invalid = agents_dir.join("invalid-hook-event.json");
        write_agent(
            &invalid,
            include_str!(
                "../../../../tests/fixtures/kiro-agents/.kiro/agents/invalid-hook-event.json"
            ),
        );

        let diagnostics = validate(&invalid);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule == "KR-HK-005")
        );
    }

    #[test]
    fn test_kr_hk_006_missing_cli_hook_command() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let invalid = agents_dir.join("missing-hook-command.json");
        write_agent(
            &invalid,
            include_str!(
                "../../../../tests/fixtures/kiro-agents/.kiro/agents/missing-hook-command.json"
            ),
        );

        let diagnostics = validate(&invalid);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule == "KR-HK-006")
        );
    }

    #[test]
    fn test_kr_ag_006_reports_unknown_subagent_reference() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let orchestrator = agents_dir.join("orchestrator.json");
        write_agent(
            &orchestrator,
            r#"{
  "name": "orchestrator",
  "prompt": "Delegate this to @research-agent"
}"#,
        );

        let diagnostics = validate(&orchestrator);
        let kr_ag_006: Vec<_> = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.rule == "KR-AG-006")
            .collect();

        assert_eq!(kr_ag_006.len(), 1);
        assert!(!kr_ag_006[0].message.trim().is_empty());
    }

    #[test]
    fn test_kr_ag_006_skips_when_reference_exists() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let orchestrator = agents_dir.join("orchestrator.json");
        let worker = agents_dir.join("research-agent.json");

        write_agent(
            &orchestrator,
            r#"{
  "name": "orchestrator",
  "prompt": "Delegate this to @research-agent"
}"#,
        );
        write_agent(
            &worker,
            r#"{
  "name": "research-agent",
  "tools": ["readFiles"]
}"#,
        );

        let diagnostics = validate(&orchestrator);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.rule != "KR-AG-006"),
            "KR-AG-006 should not fire when subagent exists: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_reserved_kiro_json_files_are_not_indexed_as_subagents() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let orchestrator = agents_dir.join("orchestrator.json");
        let plugin = agents_dir.join("plugin.json");

        write_agent(
            &orchestrator,
            r#"{
  "name": "orchestrator",
  "prompt": "Delegate this to @plugin"
}"#,
        );
        write_agent(
            &plugin,
            r#"{
  "name": "plugin",
  "tools": ["readFiles"]
}"#,
        );

        let diagnostics = validate(&orchestrator);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule == "KR-AG-006"),
            "Reserved files should not be indexed as subagents: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_kr_ag_mentions_only_counted_from_prompt_fields() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let orchestrator = agents_dir.join("orchestrator.json");
        write_agent(
            &orchestrator,
            r#"{
  "name": "orchestrator",
  "description": "Contact @missing-agent for docs",
  "prompt": "Run local checks only"
}"#,
        );

        let diagnostics = validate(&orchestrator);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.rule != "KR-AG-006"),
            "KR-AG-006 should ignore mentions outside prompt fields: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_kr_ag_mentions_detected_in_prompt_suffix_fields() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let orchestrator = agents_dir.join("orchestrator.json");
        write_agent(
            &orchestrator,
            r#"{
  "name": "orchestrator",
  "systemPrompt": "Delegate this to @missing-agent"
}"#,
        );

        let diagnostics = validate(&orchestrator);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule == "KR-AG-006")
        );
    }

    #[test]
    fn test_kr_ag_007_reports_broader_parent_tools() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let orchestrator = agents_dir.join("orchestrator.json");
        let reviewer = agents_dir.join("reviewer.json");

        write_agent(
            &orchestrator,
            r#"{
  "name": "orchestrator",
  "allowedTools": ["readFiles", "runShellCommand"],
  "prompt": "Use @reviewer for checks"
}"#,
        );
        write_agent(
            &reviewer,
            r#"{
  "name": "reviewer",
  "allowedTools": ["readFiles"]
}"#,
        );

        let diagnostics = validate(&orchestrator);
        let kr_ag_007: Vec<_> = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.rule == "KR-AG-007")
            .collect();

        assert_eq!(kr_ag_007.len(), 1);
        assert!(!kr_ag_007[0].message.trim().is_empty());
    }

    #[test]
    fn test_kr_ag_007_skips_when_tool_scope_not_broader() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let orchestrator = agents_dir.join("orchestrator.json");
        let reviewer = agents_dir.join("reviewer.json");

        write_agent(
            &orchestrator,
            r#"{
  "name": "orchestrator",
  "tools": ["readFiles"],
  "prompt": "Use @reviewer for checks"
}"#,
        );
        write_agent(
            &reviewer,
            r#"{
  "name": "reviewer",
  "tools": ["readFiles", "listDirectory"]
}"#,
        );

        let diagnostics = validate(&orchestrator);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.rule != "KR-AG-007"),
            "KR-AG-007 should not fire when parent tools are not broader: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_kr_ag_007_reports_when_referenced_scope_is_explicitly_empty() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let orchestrator = agents_dir.join("orchestrator.json");
        let reviewer = agents_dir.join("reviewer.json");

        write_agent(
            &orchestrator,
            r#"{
  "name": "orchestrator",
  "allowedTools": ["readFiles"],
  "prompt": "Use @reviewer for checks"
}"#,
        );
        write_agent(
            &reviewer,
            r#"{
  "name": "reviewer",
  "allowedTools": []
}"#,
        );

        let diagnostics = validate(&orchestrator);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule == "KR-AG-007"),
            "Explicitly empty referenced scope should still trigger KR-AG-007: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_kr_ag_007_skips_when_referenced_scope_is_missing() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let orchestrator = agents_dir.join("orchestrator.json");
        let reviewer = agents_dir.join("reviewer.json");

        write_agent(
            &orchestrator,
            r#"{
  "name": "orchestrator",
  "allowedTools": ["readFiles"],
  "prompt": "Use @reviewer for checks"
}"#,
        );
        write_agent(
            &reviewer,
            r#"{
  "name": "reviewer"
}"#,
        );

        let diagnostics = validate(&orchestrator);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.rule != "KR-AG-007"),
            "Missing referenced scope should be treated as unknown and skipped: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_allowed_tools_empty_is_authoritative() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let orchestrator = agents_dir.join("orchestrator.json");
        let reviewer = agents_dir.join("reviewer.json");

        write_agent(
            &orchestrator,
            r#"{
  "name": "orchestrator",
  "allowedTools": [],
  "tools": ["readFiles", "runShellCommand"],
  "prompt": "Use @reviewer for checks"
}"#,
        );
        write_agent(
            &reviewer,
            r#"{
  "name": "reviewer",
  "tools": ["readFiles"]
}"#,
        );

        let diagnostics = validate(&orchestrator);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.rule != "KR-AG-007"),
            "KR-AG-007 should not fall back to tools when allowedTools is present: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_case_insensitive_kiro_agents_directory_discovery() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".KIRO").join("AGENTS");
        fs::create_dir_all(&agents_dir).unwrap();

        let orchestrator = agents_dir.join("orchestrator.json");
        let reviewer = agents_dir.join("reviewer.json");

        write_agent(
            &orchestrator,
            r#"{
  "name": "orchestrator",
  "allowedTools": ["readFiles", "runShellCommand"],
  "prompt": "Use @reviewer for checks"
}"#,
        );
        write_agent(
            &reviewer,
            r#"{
  "name": "reviewer",
  "allowedTools": ["readFiles"]
}"#,
        );

        let diagnostics = validate(&orchestrator);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule == "KR-AG-007"),
            "Expected KR-AG-007 when using case-variant .kiro/agents path: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_line_col_at_offset_is_one_based() {
        assert_eq!(line_col_at_offset("@agent", 0), (1, 1));
        assert_eq!(line_col_at_offset("x\n@agent", 2), (2, 1));
    }

    #[test]
    fn test_rules_can_be_disabled_individually() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let orchestrator = agents_dir.join("orchestrator.json");
        let reviewer = agents_dir.join("reviewer.json");

        write_agent(
            &orchestrator,
            r#"{
  "name": "orchestrator",
  "allowedTools": ["readFiles", "runShellCommand"],
  "prompt": "Use @missing-agent and @reviewer"
}"#,
        );
        write_agent(
            &reviewer,
            r#"{
  "name": "reviewer",
  "allowedTools": ["readFiles"]
}"#,
        );

        let validator = KiroAgentValidator;
        let content = fs::read_to_string(&orchestrator).unwrap();

        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KR-AG-006".to_string()];
        let diagnostics = validator.validate(&orchestrator, &content, &config);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.rule != "KR-AG-006")
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule == "KR-AG-007")
        );
    }

    #[test]
    fn test_metadata_lists_kr_ag_rules() {
        let validator = KiroAgentValidator;
        let metadata = validator.metadata();

        assert_eq!(metadata.name, "KiroAgentValidator");
        assert_eq!(
            metadata.rule_ids,
            &[
                "KR-AG-001",
                "KR-AG-002",
                "KR-AG-003",
                "KR-AG-004",
                "KR-AG-005",
                "KR-AG-006",
                "KR-AG-007",
                "KR-HK-005",
                "KR-HK-006",
            ]
        );
    }
}
