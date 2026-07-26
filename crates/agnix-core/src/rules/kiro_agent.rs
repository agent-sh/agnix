//! Kiro agent validation rules (KR-AG-001 to KR-AG-014, KR-HK-005 to KR-HK-006).
//!
//! Validates legacy JSON and universal Markdown/JSON profiles under `.kiro/agents/`:
//! - KR-AG-001: Unknown top-level field in agent JSON.
//! - KR-AG-002: Invalid resource protocol/type.
//! - KR-AG-003: allowedTools contains tool not present in tools.
//! - KR-AG-004: Invalid model value.
//! - KR-AG-005: includeMcpJson disabled with no inline mcpServers.
//! - KR-AG-006: Prompt references a non-existent subagent.
//! - KR-AG-007: Invoking agent has a broader tool scope than referenced subagent.
//! - KR-AG-008: Agent missing name.
//! - KR-AG-009: Agent missing prompt.
//! - KR-AG-010: Duplicate tool entries.
//! - KR-AG-011: Empty tools array.
//! - KR-AG-012: toolAliases references unknown tool.
//! - KR-AG-013: Secrets in agent prompt.
//! - KR-AG-014: Invalid universal permissions rule.
//! - KR-HK-005: Invalid CLI hook event key.
//! - KR-HK-006: CLI hook entry missing required command.

use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    parsers::frontmatter::split_frontmatter,
    rules::{Validator, ValidatorMetadata, line_col_at_offset},
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
    "KR-AG-008",
    "KR-AG-009",
    "KR-AG-010",
    "KR-AG-011",
    "KR-AG-012",
    "KR-AG-013",
    "KR-AG-014",
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
    "permissions",
];
const VALID_CLI_HOOK_EVENTS: &[&str] = &[
    "agentSpawn",
    "userPromptSubmit",
    "preToolUse",
    "postToolUse",
    "stop",
];
const VALID_PERMISSION_CAPABILITIES: &[&str] = &[
    "fs_read",
    "fs_write",
    "filesystem",
    "shell",
    "web_fetch",
    "web_search",
    "mcp",
    "subagent",
    "skill",
    "diagnostics",
    "context",
    "all",
    "builtin",
];
const VALID_PERMISSION_EFFECTS: &[&str] = &["deny", "ask", "allow"];

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
        regex::Regex::new(
            r"(^|[^A-Za-z0-9_@])@([A-Za-z][A-Za-z0-9_-]{0,63}(?:/[A-Za-z][A-Za-z0-9_-]{0,63})*)",
        )
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

fn collect_mentions(
    text: &str,
    base_offset: usize,
    seen: &mut HashSet<String>,
    mentions: &mut Vec<AgentMention>,
) {
    for captures in mention_regex().captures_iter(text) {
        let Some(name_match) = captures.get(2) else {
            continue;
        };
        let normalized = normalize_agent_name(name_match.as_str());
        if normalized.is_empty() || !seen.insert(normalized.clone()) {
            continue;
        }
        mentions.push(AgentMention {
            name: normalized,
            byte_offset: base_offset + name_match.start().saturating_sub(1),
        });
    }
}

fn extract_prompt_agent_mentions(
    content: &str,
    markdown_body_offset: Option<usize>,
) -> Vec<AgentMention> {
    let mut seen = HashSet::new();
    let mut mentions = Vec::new();

    if let Some(body_offset) = markdown_body_offset {
        if let Some(body) = content.get(body_offset..) {
            collect_mentions(body, body_offset, &mut seen, &mut mentions);
        }
        return mentions;
    }

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

        collect_mentions(
            value_match.as_str(),
            value_match.start(),
            &mut seen,
            &mut mentions,
        );
    }

    mentions
}

fn kiro_agent_name_from_path(path: &Path) -> Option<String> {
    let components: Vec<String> = path
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => value.to_str().map(ToString::to_string),
            _ => None,
        })
        .collect();
    let agents_index = components.windows(2).position(|pair| {
        pair[0].eq_ignore_ascii_case(".kiro") && pair[1].eq_ignore_ascii_case("agents")
    });

    let mut name_parts = agents_index
        .map(|index| components[index + 2..].to_vec())
        .unwrap_or_default();
    if name_parts.is_empty() {
        return path
            .file_stem()
            .and_then(|value| value.to_str())
            .map(ToString::to_string);
    }
    let last = name_parts.pop()?;
    let stem = Path::new(&last).file_stem()?.to_str()?.to_string();
    name_parts.push(stem);
    Some(name_parts.join("/"))
}

fn parse_kiro_agent_document(path: &Path, content: &str) -> Option<(Value, Option<usize>)> {
    let is_markdown = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"));

    let (mut value, body_offset) = if is_markdown {
        let parts = split_frontmatter(content);
        if !parts.has_frontmatter || !parts.has_closing {
            return None;
        }
        let yaml = serde_yaml::from_str::<serde_yaml::Value>(&parts.frontmatter).ok()?;
        let mut value = serde_json::to_value(yaml).ok()?;
        let object = value.as_object_mut()?;
        let body = content.get(parts.body_start..)?.trim();
        object.insert("prompt".to_string(), Value::String(body.to_string()));
        (value, Some(parts.body_start))
    } else {
        (serde_json::from_str::<Value>(content).ok()?, None)
    };

    if let Some(object) = value.as_object_mut()
        && !object.contains_key("name")
        && let Some(name) = kiro_agent_name_from_path(path)
    {
        object.insert("name".to_string(), Value::String(name));
    }
    Some((value, body_offset))
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
    config: &PerFileLintConfig<'_>,
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

fn push_permissions_diagnostic(path: &Path, issue: &str, diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.push(
        Diagnostic::error(
            path.to_path_buf(),
            1,
            0,
            "KR-AG-014",
            t!("rules.kr_ag_014.message", issue = issue),
        )
        .with_suggestion(t!("rules.kr_ag_014.suggestion")),
    );
}

fn validate_permissions_rules(
    path: &Path,
    current_agent: &Value,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !config.is_rule_enabled("KR-AG-014") {
        return;
    }
    let Some(permissions) = current_agent.get("permissions") else {
        return;
    };
    if permissions.is_null() {
        return;
    }
    let Some(permissions) = permissions.as_object() else {
        push_permissions_diagnostic(path, "permissions must be an object", diagnostics);
        return;
    };
    let Some(rules) = permissions.get("rules").and_then(Value::as_array) else {
        push_permissions_diagnostic(path, "permissions.rules must be an array", diagnostics);
        return;
    };

    for (index, rule) in rules.iter().enumerate() {
        let Some(rule) = rule.as_object() else {
            push_permissions_diagnostic(
                path,
                &format!("permissions.rules[{index}] must be an object"),
                diagnostics,
            );
            continue;
        };

        match rule.get("capability").and_then(Value::as_str) {
            Some(capability) if VALID_PERMISSION_CAPABILITIES.contains(&capability) => {}
            Some(capability) => push_permissions_diagnostic(
                path,
                &format!("permissions.rules[{index}].capability has unknown value '{capability}'"),
                diagnostics,
            ),
            None => push_permissions_diagnostic(
                path,
                &format!("permissions.rules[{index}].capability must be a string"),
                diagnostics,
            ),
        }

        match rule.get("effect").and_then(Value::as_str) {
            Some(effect) if VALID_PERMISSION_EFFECTS.contains(&effect) => {}
            Some(effect) => push_permissions_diagnostic(
                path,
                &format!("permissions.rules[{index}].effect has unknown value '{effect}'"),
                diagnostics,
            ),
            None => push_permissions_diagnostic(
                path,
                &format!("permissions.rules[{index}].effect must be deny, ask, or allow"),
                diagnostics,
            ),
        }

        for field in ["match", "exclude"] {
            let Some(patterns) = rule.get(field) else {
                continue;
            };
            let valid = patterns
                .as_array()
                .is_some_and(|patterns| patterns.iter().all(Value::is_string));
            if !valid {
                push_permissions_diagnostic(
                    path,
                    &format!("permissions.rules[{index}].{field} must be an array of strings"),
                    diagnostics,
                );
            }
        }
    }
}

fn validate_agent_schema_rules(
    path: &Path,
    current_agent: &Value,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let check_unknown_fields = config.is_rule_enabled("KR-AG-001");
    let check_resource_protocols = config.is_rule_enabled("KR-AG-002");
    let check_allowed_tools_subset = config.is_rule_enabled("KR-AG-003");
    let check_model = config.is_rule_enabled("KR-AG-004");
    let check_no_mcp_access = config.is_rule_enabled("KR-AG-005");
    let any_schema_rule = check_unknown_fields
        || check_resource_protocols
        || check_allowed_tools_subset
        || check_model
        || check_no_mcp_access
        || config.is_rule_enabled("KR-AG-008")
        || config.is_rule_enabled("KR-AG-009")
        || config.is_rule_enabled("KR-AG-010")
        || config.is_rule_enabled("KR-AG-011")
        || config.is_rule_enabled("KR-AG-012")
        || config.is_rule_enabled("KR-AG-013")
        || config.is_rule_enabled("KR-AG-014");
    if !any_schema_rule {
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

    // KR-AG-008: Agent missing name
    if config.is_rule_enabled("KR-AG-008") {
        let name_present = obj
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|n| !n.trim().is_empty());
        if !name_present {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "KR-AG-008",
                    t!("rules.kr_ag_008.message"),
                )
                .with_suggestion(t!("rules.kr_ag_008.suggestion")),
            );
        }
    }

    // KR-AG-009: Agent missing prompt
    if config.is_rule_enabled("KR-AG-009") {
        let prompt_present = obj
            .get("prompt")
            .and_then(Value::as_str)
            .is_some_and(|p| !p.trim().is_empty());
        if !prompt_present {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "KR-AG-009",
                    t!("rules.kr_ag_009.message"),
                )
                .with_suggestion(t!("rules.kr_ag_009.suggestion")),
            );
        }
    }

    // KR-AG-010: Duplicate tool entries
    if config.is_rule_enabled("KR-AG-010")
        && let Some(tools) = current_agent.get("tools").and_then(Value::as_array)
    {
        let mut seen = HashSet::new();
        for tool in tools {
            if let Some(tool_str) = tool.as_str() {
                if !seen.insert(tool_str.to_ascii_lowercase()) {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            1,
                            0,
                            "KR-AG-010",
                            t!("rules.kr_ag_010.message", tool = tool_str),
                        )
                        .with_suggestion(t!("rules.kr_ag_010.suggestion")),
                    );
                }
            }
        }
    }

    // KR-AG-011: Empty tools array
    if config.is_rule_enabled("KR-AG-011")
        && let Some(tools) = current_agent.get("tools").and_then(Value::as_array)
        && tools.is_empty()
    {
        diagnostics.push(
            Diagnostic::info(
                path.to_path_buf(),
                1,
                0,
                "KR-AG-011",
                t!("rules.kr_ag_011.message"),
            )
            .with_suggestion(t!("rules.kr_ag_011.suggestion")),
        );
    }

    // KR-AG-012: toolAliases references unknown tool
    if config.is_rule_enabled("KR-AG-012")
        && let Some(aliases) = current_agent.get("toolAliases").and_then(Value::as_object)
    {
        let tools_set: HashSet<String> = current_agent
            .get("tools")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(|s| s.to_ascii_lowercase())
                    .collect()
            })
            .unwrap_or_default();

        for (alias, target) in aliases {
            if let Some(target_str) = target.as_str() {
                if !tools_set.contains(&target_str.to_ascii_lowercase()) {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            1,
                            0,
                            "KR-AG-012",
                            t!(
                                "rules.kr_ag_012.message",
                                alias = alias.as_str(),
                                tool = target_str
                            ),
                        )
                        .with_suggestion(t!("rules.kr_ag_012.suggestion")),
                    );
                }
            }
        }
    }

    // KR-AG-013: Secrets in agent prompt
    // The regex already excludes env var patterns because ${...} doesn't match
    // the required value prefixes (sk-, ghp_, AKIA, etc.) or base64 patterns.
    if config.is_rule_enabled("KR-AG-013")
        && let Some(prompt) = obj.get("prompt").and_then(Value::as_str)
    {
        let secret_re = agent_secret_pattern();
        if secret_re.is_match(prompt) {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "KR-AG-013",
                    t!("rules.kr_ag_013.message"),
                )
                .with_suggestion(t!("rules.kr_ag_013.suggestion")),
            );
        }
    }

    validate_permissions_rules(path, current_agent, config, diagnostics);
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

fn is_reserved_kiro_agent_filename(filename: &str) -> bool {
    let lowered = filename.to_ascii_lowercase();
    matches!(
        lowered.as_str(),
        "plugin.json" | "mcp.json" | "settings.json" | "settings.local.json"
    ) || lowered.starts_with("mcp-")
        || lowered.ends_with(".mcp.json")
}

fn find_kiro_agents_dir(path: &Path, config: &PerFileLintConfig<'_>) -> Option<PathBuf> {
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

fn load_agent_index(
    agents_dir: &Path,
    config: &PerFileLintConfig<'_>,
) -> HashMap<String, AgentInfo> {
    let fs = config.fs();
    let mut index: HashMap<String, AgentInfo> = HashMap::new();
    let mut pending = vec![(agents_dir.to_path_buf(), 0usize)];

    while let Some((directory, depth)) = pending.pop() {
        if depth > MAX_PROJECT_SEARCH_DEPTH {
            continue;
        }
        let Ok(mut entries) = fs.read_dir(&directory) else {
            continue;
        };
        entries.sort_by(|a, b| a.path.cmp(&b.path));

        for entry in entries {
            if entry.metadata.is_dir {
                pending.push((entry.path, depth + 1));
                continue;
            }
            if !entry.metadata.is_file {
                continue;
            }

            let Some(filename) = entry.path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if is_reserved_kiro_agent_filename(filename) {
                continue;
            }

            let is_agent_file = entry
                .path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("json") || ext.eq_ignore_ascii_case("md")
                });
            if !is_agent_file {
                continue;
            }

            let Ok(raw) = fs.read_to_string(&entry.path) else {
                continue;
            };
            let Some((value, _)) = parse_kiro_agent_document(&entry.path, &raw) else {
                continue;
            };

            let info = AgentInfo {
                tools: extract_tools(&value),
                has_explicit_tool_scope: has_explicit_tool_scope(&value),
            };
            let names = [
                kiro_agent_name_from_path(&entry.path),
                value
                    .get("name")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
            ];
            for name in names.into_iter().flatten() {
                let normalized_name = normalize_agent_name(&name);
                if normalized_name.is_empty() {
                    continue;
                }

                // The relative path is canonical in 2.14; keep an explicit
                // legacy name as an alias for upgraded JSON compatibility.
                index.entry(normalized_name).or_insert_with(|| info.clone());
            }
        }
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

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let check_missing_reference = config.is_rule_enabled("KR-AG-006");
        let check_tool_scope = config.is_rule_enabled("KR-AG-007");

        let any_enabled = RULE_IDS.iter().any(|id| config.is_rule_enabled(id));
        if !any_enabled {
            return diagnostics;
        }

        let Some((current_agent, markdown_body_offset)) = parse_kiro_agent_document(path, content)
        else {
            return diagnostics;
        };

        validate_agent_schema_rules(path, &current_agent, config, &mut diagnostics);
        validate_cli_hook_rules(path, &current_agent, config, &mut diagnostics);

        if !check_missing_reference && !check_tool_scope {
            return diagnostics;
        }

        let mentions = extract_prompt_agent_mentions(content, markdown_body_offset);
        if mentions.is_empty() {
            return diagnostics;
        }

        let current_names: HashSet<String> = [
            kiro_agent_name_from_path(path),
            current_agent
                .get("name")
                .and_then(Value::as_str)
                .map(ToString::to_string),
        ]
        .into_iter()
        .flatten()
        .map(|name| normalize_agent_name(&name))
        .filter(|name| !name.is_empty())
        .collect();
        let current_tools = extract_tools(&current_agent);

        let Some(agents_dir) = find_kiro_agents_dir(path, config) else {
            return diagnostics;
        };

        let known_agents = load_agent_index(&agents_dir, config);
        if known_agents.is_empty() {
            return diagnostics;
        }

        for mention in mentions {
            if current_names.contains(&mention.name) {
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
    use crate::config::LintConfig;
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
    fn test_kr_ag_008_name_is_derived_from_filename() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("no-name.json");
        write_agent(
            &agent,
            r#"{
  "prompt": "Do something"
}"#,
        );

        let diagnostics = validate(&agent);
        assert!(diagnostics.iter().all(|d| d.rule != "KR-AG-008"));
    }

    #[test]
    fn test_kr_ag_008_has_name_no_diagnostic() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("named.json");
        write_agent(
            &agent,
            r#"{
  "name": "named-agent",
  "prompt": "Do something"
}"#,
        );

        let diagnostics = validate(&agent);
        assert!(diagnostics.iter().all(|d| d.rule != "KR-AG-008"));
    }

    #[test]
    fn test_kr_ag_009_missing_prompt() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("no-prompt.json");
        write_agent(
            &agent,
            r#"{
  "name": "no-prompt"
}"#,
        );

        let diagnostics = validate(&agent);
        assert!(diagnostics.iter().any(|d| d.rule == "KR-AG-009"));
    }

    #[test]
    fn test_universal_markdown_agent_is_valid() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents").join("team");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("backend.md");
        write_agent(
            &agent,
            r#"---
description: Backend development agent
model: claude-sonnet-4
tools: [read, write, shell, web]
resources:
  - file://./ARCHITECTURE.md
permissions:
  rules:
    - capability: shell
      effect: allow
      match: ["npm *", "node *"]
welcomeMessage: Ready to work.
---
Review and implement backend changes."#,
        );

        let diagnostics = validate(&agent);
        assert!(
            diagnostics.iter().all(|diagnostic| !matches!(
                diagnostic.rule.as_str(),
                "KR-AG-001" | "KR-AG-008" | "KR-AG-009" | "KR-AG-014"
            )),
            "universal Markdown agent should validate cleanly: {diagnostics:?}"
        );
    }

    #[test]
    fn test_kr_ag_014_rejects_invalid_permissions_rule() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("invalid-permissions.md");
        write_agent(
            &agent,
            "---\ndescription: Invalid permissions\npermissions:\n  rules:\n    - capability: launch_missiles\n      effect: sometimes\n      match: npm *\n---\nDo work.",
        );

        let diagnostics = validate(&agent);
        assert!(diagnostics.iter().any(|d| d.rule == "KR-AG-014"));
    }

    #[test]
    fn test_markdown_agent_resolves_nested_subagent_reference() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        let team_dir = agents_dir.join("team");
        fs::create_dir_all(&team_dir).unwrap();

        let reviewer = team_dir.join("reviewer.md");
        write_agent(
            &reviewer,
            "---\ndescription: Reviews code\ntools: [read]\n---\nReview code.",
        );
        let orchestrator = agents_dir.join("orchestrator.md");
        write_agent(
            &orchestrator,
            "---\ndescription: Delegates reviews\ntools: [read]\n---\nAsk @team/reviewer to review the change.",
        );

        let diagnostics = validate(&orchestrator);
        assert!(
            diagnostics.iter().all(|d| d.rule != "KR-AG-006"),
            "nested Markdown agent should resolve: {diagnostics:?}"
        );
    }

    #[test]
    fn test_nested_json_agent_keeps_path_name_with_legacy_explicit_name() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        let team_dir = agents_dir.join("team");
        fs::create_dir_all(&team_dir).unwrap();

        let reviewer = team_dir.join("reviewer.json");
        write_agent(
            &reviewer,
            r#"{"name":"reviewer","prompt":"Review code","tools":["read"]}"#,
        );
        let orchestrator = agents_dir.join("orchestrator.md");
        write_agent(
            &orchestrator,
            "---\ndescription: Delegates reviews\ntools: [read]\n---\nAsk @team/reviewer to review the change.",
        );

        let diagnostics = validate(&orchestrator);
        assert!(
            diagnostics.iter().all(|d| d.rule != "KR-AG-006"),
            "path-derived names must remain canonical after upgrade: {diagnostics:?}"
        );
    }

    #[test]
    fn test_kr_ag_009_has_prompt_no_diagnostic() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("with-prompt.json");
        write_agent(
            &agent,
            r#"{
  "name": "with-prompt",
  "prompt": "Do the work"
}"#,
        );

        let diagnostics = validate(&agent);
        assert!(diagnostics.iter().all(|d| d.rule != "KR-AG-009"));
    }

    #[test]
    fn test_kr_ag_010_duplicate_tools() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("dup-tools.json");
        write_agent(
            &agent,
            r#"{
  "name": "dup-tools",
  "prompt": "Work",
  "tools": ["readFiles", "readFiles"]
}"#,
        );

        let diagnostics = validate(&agent);
        assert!(diagnostics.iter().any(|d| d.rule == "KR-AG-010"));
    }

    #[test]
    fn test_kr_ag_010_unique_tools_no_diagnostic() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("unique-tools.json");
        write_agent(
            &agent,
            r#"{
  "name": "unique-tools",
  "prompt": "Work",
  "tools": ["readFiles", "writeFiles"]
}"#,
        );

        let diagnostics = validate(&agent);
        assert!(diagnostics.iter().all(|d| d.rule != "KR-AG-010"));
    }

    #[test]
    fn test_kr_ag_011_empty_tools_array() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("empty-tools.json");
        write_agent(
            &agent,
            r#"{
  "name": "empty-tools",
  "prompt": "Work",
  "tools": []
}"#,
        );

        let diagnostics = validate(&agent);
        assert!(diagnostics.iter().any(|d| d.rule == "KR-AG-011"));
    }

    #[test]
    fn test_kr_ag_012_tool_alias_references_unknown_tool() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("bad-alias.json");
        write_agent(
            &agent,
            r#"{
  "name": "bad-alias",
  "prompt": "Work",
  "tools": ["readFiles"],
  "toolAliases": {
    "write": "writeFiles"
  }
}"#,
        );

        let diagnostics = validate(&agent);
        assert!(diagnostics.iter().any(|d| d.rule == "KR-AG-012"));
    }

    #[test]
    fn test_kr_ag_012_valid_alias_no_diagnostic() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("good-alias.json");
        write_agent(
            &agent,
            r#"{
  "name": "good-alias",
  "prompt": "Work",
  "tools": ["readFiles"],
  "toolAliases": {
    "read": "readFiles"
  }
}"#,
        );

        let diagnostics = validate(&agent);
        assert!(diagnostics.iter().all(|d| d.rule != "KR-AG-012"));
    }

    // L5: KR-AG-011 non-empty tools should NOT fire
    #[test]
    fn test_kr_ag_011_non_empty_tools_no_diagnostic() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("has-tools.json");
        write_agent(
            &agent,
            r#"{
  "name": "has-tools",
  "prompt": "Work",
  "tools": ["readFiles"]
}"#,
        );

        let diagnostics = validate(&agent);
        assert!(diagnostics.iter().all(|d| d.rule != "KR-AG-011"));
    }

    #[test]
    fn test_kr_ag_013_secrets_in_prompt() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("secret-prompt.json");
        write_agent(
            &agent,
            r#"{
  "name": "secret-prompt",
  "prompt": "Use api_key= sk-test_PLACEHOLDER_NOT_REAL_KEY_000"
}"#,
        );

        let diagnostics = validate(&agent);
        assert!(diagnostics.iter().any(|d| d.rule == "KR-AG-013"));
    }

    #[test]
    fn test_kr_ag_013_no_secrets_no_diagnostic() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("clean-prompt.json");
        write_agent(
            &agent,
            r#"{
  "name": "clean-prompt",
  "prompt": "Do normal work without secrets"
}"#,
        );

        let diagnostics = validate(&agent);
        assert!(diagnostics.iter().all(|d| d.rule != "KR-AG-013"));
    }

    #[test]
    fn test_kr_ag_013_env_var_not_flagged() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("env-prompt.json");
        write_agent(
            &agent,
            r#"{
  "name": "env-prompt",
  "prompt": "Use api_key=${MY_SECRET} and token=$(get_token)"
}"#,
        );

        let diagnostics = validate(&agent);
        assert!(
            diagnostics.iter().all(|d| d.rule != "KR-AG-013"),
            "Env var references should not trigger KR-AG-013"
        );
    }

    #[test]
    fn test_kr_ag_008_blank_name() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("blank-name.json");
        write_agent(
            &agent,
            r#"{
  "name": "   ",
  "prompt": "Do something"
}"#,
        );

        let diagnostics = validate(&agent);
        assert!(diagnostics.iter().any(|d| d.rule == "KR-AG-008"));
    }

    #[test]
    fn test_kr_ag_009_blank_prompt() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("blank-prompt.json");
        write_agent(
            &agent,
            r#"{
  "name": "blank-prompt",
  "prompt": "   "
}"#,
        );

        let diagnostics = validate(&agent);
        assert!(diagnostics.iter().any(|d| d.rule == "KR-AG-009"));
    }

    #[test]
    fn test_kr_ag_010_case_insensitive_duplicate_tools() {
        let temp = tempfile::TempDir::new().unwrap();
        let agents_dir = temp.path().join(".kiro").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        let agent = agents_dir.join("case-dup-tools.json");
        write_agent(
            &agent,
            r#"{
  "name": "case-dup-tools",
  "prompt": "Work",
  "tools": ["readFiles", "READFILES"]
}"#,
        );

        let diagnostics = validate(&agent);
        assert!(diagnostics.iter().any(|d| d.rule == "KR-AG-010"));
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
                "KR-AG-008",
                "KR-AG-009",
                "KR-AG-010",
                "KR-AG-011",
                "KR-AG-012",
                "KR-AG-013",
                "KR-AG-014",
                "KR-HK-005",
                "KR-HK-006",
            ]
        );
    }
}
