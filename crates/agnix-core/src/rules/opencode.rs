//! OpenCode configuration validation rules (OC-001 to OC-009)
//!
//! Validates:
//! - OC-001: Invalid share mode (HIGH) - must be "manual", "auto", or "disabled"
//! - OC-002: Invalid instruction path (HIGH) - paths must exist or be valid globs
//! - OC-003: opencode.json parse error (HIGH) - must be valid JSON/JSONC
//! - OC-004: Unknown config key (MEDIUM) - unrecognized key in opencode.json
//! - OC-006: Remote URL in instructions (LOW) - may slow startup
//! - OC-007: Invalid agent definition (MEDIUM/HIGH) - agents must have description
//! - OC-008: Invalid permission config (HIGH) - must be allow/ask/deny
//! - OC-009: Invalid variable substitution (MEDIUM) - must use {env:...} or {file:...}

use crate::{
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::{Validator, ValidatorMetadata},
    schemas::opencode::{
        VALID_PERMISSION_MODES, VALID_SHARE_MODES, is_glob_pattern, parse_opencode_json,
        validate_glob_pattern,
    },
};
use rust_i18n::t;
use std::path::Path;

use crate::rules::{find_closest_value, find_unique_json_string_value_span};

const RULE_IDS: &[&str] = &[
    "OC-001",
    "OC-002",
    "OC-003",
    "OC-004",
    "OC-006",
    "OC-007",
    "OC-008",
    "OC-009",
    "OC-CFG-001",
    "OC-CFG-002",
    "OC-CFG-003",
    "OC-CFG-004",
    "OC-CFG-005",
    "OC-CFG-006",
    "OC-CFG-007",
    "OC-AG-001",
    "OC-AG-002",
    "OC-AG-003",
    "OC-AG-004",
    "OC-PM-001",
    "OC-PM-002",
];

pub struct OpenCodeValidator;

impl Validator for OpenCodeValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // OC-003: Parse error (ERROR)
        let parsed = parse_opencode_json(content);
        if let Some(ref error) = parsed.parse_error {
            if config.is_rule_enabled("OC-003") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        error.line,
                        error.column,
                        "OC-003",
                        t!("rules.oc_003.message", error = error.message.as_str()),
                    )
                    .with_suggestion(t!("rules.oc_003.suggestion")),
                );
            }
            // Can't continue if JSON is broken
            return diagnostics;
        }

        // OC-004: Unknown config keys (WARNING)
        // Runs on unknown_keys which are populated whenever JSON parses successfully,
        // even when schema extraction fails.
        if config.is_rule_enabled("OC-004") {
            for unknown in &parsed.unknown_keys {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        unknown.line,
                        unknown.column,
                        "OC-004",
                        t!("rules.oc_004.message", key = unknown.key.as_str()),
                    )
                    .with_suggestion(t!("rules.oc_004.suggestion")),
                );
            }
        }

        let schema = match parsed.schema {
            Some(s) => s,
            None => return diagnostics,
        };

        // OC-001: Invalid share mode (ERROR)
        if config.is_rule_enabled("OC-001") {
            if parsed.share_wrong_type {
                let line = find_key_line(content, "share").unwrap_or(1);
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        0,
                        "OC-001",
                        t!("rules.oc_001.type_error"),
                    )
                    .with_suggestion(t!("rules.oc_001.suggestion")),
                );
            } else if let Some(ref share_value) = schema.share {
                if !VALID_SHARE_MODES.contains(&share_value.as_str()) {
                    let line = find_key_line(content, "share").unwrap_or(1);
                    let mut diagnostic = Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        0,
                        "OC-001",
                        t!("rules.oc_001.message", value = share_value.as_str()),
                    )
                    .with_suggestion(t!("rules.oc_001.suggestion"));

                    // Unsafe auto-fix: replace with closest valid share mode.
                    if let Some(suggested) = find_closest_value(share_value, VALID_SHARE_MODES) {
                        if let Some((start, end)) =
                            find_unique_json_string_value_span(content, "share", share_value)
                        {
                            diagnostic = diagnostic.with_fix(Fix::replace(
                                start,
                                end,
                                suggested,
                                t!("rules.oc_001.fix", fixed = suggested),
                                false,
                            ));
                        }
                    }

                    diagnostics.push(diagnostic);
                }
            }
        }

        // OC-002: Invalid instruction path (ERROR)
        if config.is_rule_enabled("OC-002") {
            if parsed.instructions_wrong_type {
                let instructions_line = find_key_line(content, "instructions").unwrap_or(1);
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        instructions_line,
                        0,
                        "OC-002",
                        t!("rules.oc_002.type_error"),
                    )
                    .with_suggestion(t!("rules.oc_002.suggestion")),
                );
            }
            if let Some(ref instructions) = schema.instructions {
                let config_dir = path.parent().unwrap_or(Path::new("."));
                let instructions_line = find_key_line(content, "instructions").unwrap_or(1);
                let fs = config.fs();

                for instruction_path in instructions {
                    if instruction_path.trim().is_empty() {
                        continue;
                    }

                    // OC-006: Remote URL in instructions (INFO)
                    if instruction_path.starts_with("http://")
                        || instruction_path.starts_with("https://")
                    {
                        if config.is_rule_enabled("OC-006") {
                            diagnostics.push(
                                Diagnostic::info(
                                    path.to_path_buf(),
                                    instructions_line,
                                    0,
                                    "OC-006",
                                    t!("rules.oc_006.message", url = instruction_path.as_str()),
                                )
                                .with_suggestion(t!("rules.oc_006.suggestion")),
                            );
                        }
                        continue; // Don't check URL as file path
                    }

                    // Reject absolute paths and path traversal attempts
                    let p = Path::new(instruction_path);
                    if p.is_absolute()
                        || p.components().any(|c| c == std::path::Component::ParentDir)
                    {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                instructions_line,
                                0,
                                "OC-002",
                                t!("rules.oc_002.traversal", path = instruction_path.as_str()),
                            )
                            .with_suggestion(t!("rules.oc_002.suggestion")),
                        );
                        continue;
                    }

                    // If it's a glob pattern, validate the pattern syntax
                    if is_glob_pattern(instruction_path) {
                        if !validate_glob_pattern(instruction_path) {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    instructions_line,
                                    0,
                                    "OC-002",
                                    t!(
                                        "rules.oc_002.invalid_glob",
                                        path = instruction_path.as_str()
                                    ),
                                )
                                .with_suggestion(t!("rules.oc_002.suggestion")),
                            );
                        }
                        // Valid glob patterns are allowed even if no files match yet
                        continue;
                    }

                    // For non-glob paths, check if the file exists relative to config dir
                    let resolved = config_dir.join(instruction_path);
                    if !fs.exists(&resolved) {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                instructions_line,
                                0,
                                "OC-002",
                                t!("rules.oc_002.not_found", path = instruction_path.as_str()),
                            )
                            .with_suggestion(t!("rules.oc_002.suggestion")),
                        );
                    }
                }
            }
        }

        // OC-007: Agent validation (WARNING for missing description, ERROR for wrong type)
        if config.is_rule_enabled("OC-007") {
            if parsed.agent_wrong_type {
                let line = find_key_line(content, "agent").unwrap_or(1);
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        0,
                        "OC-007",
                        t!("rules.oc_007.type_error"),
                    )
                    .with_suggestion(t!("rules.oc_007.suggestion")),
                );
            } else if let Some(ref agent_value) = schema.agent {
                if let Some(agents) = agent_value.as_object() {
                    let agent_line = find_key_line(content, "agent").unwrap_or(1);
                    for (name, config_val) in agents {
                        if let Some(obj) = config_val.as_object() {
                            if !obj.contains_key("description") {
                                diagnostics.push(
                                    Diagnostic::warning(
                                        path.to_path_buf(),
                                        agent_line,
                                        0,
                                        "OC-007",
                                        t!("rules.oc_007.message", name = name.as_str()),
                                    )
                                    .with_suggestion(t!("rules.oc_007.suggestion")),
                                );
                            }
                        } else if !config_val.is_null() {
                            diagnostics.push(
                                Diagnostic::warning(
                                    path.to_path_buf(),
                                    agent_line,
                                    0,
                                    "OC-007",
                                    t!("rules.oc_007.message", name = name.as_str()),
                                )
                                .with_suggestion(t!("rules.oc_007.suggestion")),
                            );
                        }
                    }
                }
            }
        }

        // OC-008: Permission validation (ERROR)
        if config.is_rule_enabled("OC-008") {
            if parsed.permission_wrong_type {
                let line = find_key_line(content, "permission").unwrap_or(1);
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        0,
                        "OC-008",
                        t!("rules.oc_008.type_error"),
                    )
                    .with_suggestion(t!("rules.oc_008.suggestion")),
                );
            } else if let Some(ref perm_value) = schema.permission {
                let perm_line = find_key_line(content, "permission").unwrap_or(1);
                if let Some(perm_str) = perm_value.as_str() {
                    // Global string shorthand
                    if !VALID_PERMISSION_MODES.contains(&perm_str) {
                        let mut diagnostic = Diagnostic::error(
                            path.to_path_buf(),
                            perm_line,
                            0,
                            "OC-008",
                            t!("rules.oc_008.message", value = perm_str, tool = "*"),
                        )
                        .with_suggestion(t!("rules.oc_008.suggestion"));

                        if let Some(suggested) =
                            find_closest_value(perm_str, VALID_PERMISSION_MODES)
                        {
                            if let Some((start, end)) =
                                find_unique_json_string_value_span(content, "permission", perm_str)
                            {
                                diagnostic = diagnostic.with_fix(Fix::replace(
                                    start,
                                    end,
                                    suggested,
                                    format!("Replace permission with '{}'", suggested),
                                    false,
                                ));
                            }
                        }

                        diagnostics.push(diagnostic);
                    }
                } else if let Some(perm_obj) = perm_value.as_object() {
                    for (tool, mode_value) in perm_obj {
                        if let Some(mode_str) = mode_value.as_str() {
                            if !VALID_PERMISSION_MODES.contains(&mode_str) {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        perm_line,
                                        0,
                                        "OC-008",
                                        t!(
                                            "rules.oc_008.message",
                                            value = mode_str,
                                            tool = tool.as_str()
                                        ),
                                    )
                                    .with_suggestion(t!("rules.oc_008.suggestion")),
                                );
                            }
                        } else if let Some(mode_obj) = mode_value.as_object() {
                            // Nested permission objects (with patterns)
                            for (_, pattern_mode) in mode_obj {
                                if let Some(pm) = pattern_mode.as_str() {
                                    if !VALID_PERMISSION_MODES.contains(&pm) {
                                        diagnostics.push(
                                            Diagnostic::error(
                                                path.to_path_buf(),
                                                perm_line,
                                                0,
                                                "OC-008",
                                                t!(
                                                    "rules.oc_008.message",
                                                    value = pm,
                                                    tool = tool.as_str()
                                                ),
                                            )
                                            .with_suggestion(t!("rules.oc_008.suggestion")),
                                        );
                                    }
                                } else if !pattern_mode.is_null() {
                                    diagnostics.push(
                                        Diagnostic::error(
                                            path.to_path_buf(),
                                            perm_line,
                                            0,
                                            "OC-008",
                                            t!("rules.oc_008.type_error"),
                                        )
                                        .with_suggestion(t!("rules.oc_008.suggestion")),
                                    );
                                }
                            }
                        } else if !mode_value.is_null() {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    perm_line,
                                    0,
                                    "OC-008",
                                    t!("rules.oc_008.type_error"),
                                )
                                .with_suggestion(t!("rules.oc_008.suggestion")),
                            );
                        }
                    }
                }
            }
        }

        // OC-009: Variable substitution validation (WARNING)
        if config.is_rule_enabled("OC-009") {
            if let Some(ref raw_value) = parsed.raw_value {
                validate_substitutions(raw_value, path, content, &mut diagnostics);
            }
        }

        // New OpenCode Rules

        if let Some(ref raw_value) = parsed.raw_value {
            if let Some(obj) = raw_value.as_object() {
                // OC-CFG-001: Invalid Model Format
                if config.is_rule_enabled("OC-CFG-001") {
                    for key in &["model", "small_model"] {
                        if let Some(model_val) = obj.get(*key) {
                            if let Some(model_str) = model_val.as_str() {
                                if !model_str.contains('/') && !model_str.contains("{env:") {
                                    diagnostics.push(
                                        Diagnostic::error(
                                            path.to_path_buf(),
                                            find_key_line(content, key).unwrap_or(1),
                                            0,
                                            "OC-CFG-001",
                                            t!("rules.oc_cfg_001.message").to_string(),
                                        )
                                        .with_suggestion(
                                            t!("rules.oc_cfg_001.suggestion").to_string(),
                                        ),
                                    );
                                }
                            } else {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        find_key_line(content, key).unwrap_or(1),
                                        0,
                                        "OC-CFG-001",
                                        t!("rules.oc_cfg_001.type_error").to_string(),
                                    )
                                    .with_suggestion(t!("rules.oc_cfg_001.suggestion").to_string()),
                                );
                            }
                        }
                    }
                }

                // OC-CFG-002: Invalid autoupdate value/type
                if config.is_rule_enabled("OC-CFG-002")
                    && let Some(autoupdate_val) = obj.get("autoupdate")
                {
                    let is_valid = autoupdate_val.is_boolean()
                        || autoupdate_val
                            .as_str()
                            .is_some_and(|s| s.eq_ignore_ascii_case("notify"));

                    if !is_valid {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                find_key_line(content, "autoupdate").unwrap_or(1),
                                0,
                                "OC-CFG-002",
                                t!("rules.oc_cfg_002.message").to_string(),
                            )
                            .with_suggestion(t!("rules.oc_cfg_002.suggestion").to_string()),
                        );
                    }
                }

                // OC-CFG-003: Unknown top-level config field
                if config.is_rule_enabled("OC-CFG-003") {
                    for unknown in &parsed.unknown_keys {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                unknown.line,
                                unknown.column,
                                "OC-CFG-003",
                                t!("rules.oc_cfg_003.message", key = unknown.key.as_str())
                                    .to_string(),
                            )
                            .with_suggestion(t!("rules.oc_cfg_003.suggestion").to_string()),
                        );
                    }
                }

                // OC-CFG-004: Invalid Default Agent
                if config.is_rule_enabled("OC-CFG-004") {
                    if let Some(agent_val) = obj.get("default_agent") {
                        if let Some(agent_str) = agent_val.as_str() {
                            let mut known_agents = std::collections::HashSet::new();
                            known_agents.insert("build");
                            known_agents.insert("plan");
                            known_agents.insert("general");
                            known_agents.insert("explore");

                            if let Some(agents_obj) = obj.get("agent").and_then(|a| a.as_object()) {
                                for k in agents_obj.keys() {
                                    known_agents.insert(k.as_str());
                                }
                            }

                            if !known_agents.contains(agent_str) {
                                diagnostics.push(
                                    Diagnostic::warning(
                                        path.to_path_buf(),
                                        find_key_line(content, "default_agent").unwrap_or(1),
                                        0,
                                        "OC-CFG-004",
                                        format!("Invalid default_agent '{}'. Must be 'build' or a defined custom agent", agent_str),
                                    )
                                );
                            }
                        } else if !agent_val.is_null() {
                            diagnostics.push(
                                Diagnostic::warning(
                                    path.to_path_buf(),
                                    find_key_line(content, "default_agent").unwrap_or(1),
                                    0,
                                    "OC-CFG-004",
                                    "Invalid default_agent type. Must be a string referring to a valid agent".to_string(),
                                )
                                .with_suggestion(
                                    "Use a string value such as 'build' or a defined custom agent name"
                                        .to_string(),
                                ),
                            );
                        }
                    }
                }

                // OC-CFG-005: Hardcoded API Key
                if config.is_rule_enabled("OC-CFG-005") {
                    if let Some(provider_obj) = obj.get("provider").and_then(|p| p.as_object()) {
                        // Case 1: provider.options.apiKey
                        if let Some(p_opts) =
                            provider_obj.get("options").and_then(|o| o.as_object())
                        {
                            if let Some(api_key) = p_opts.get("apiKey").and_then(|k| k.as_str()) {
                                if !api_key.starts_with("{env:") {
                                    diagnostics.push(
                                        Diagnostic::error(
                                            path.to_path_buf(),
                                            find_key_line(content, "apiKey").unwrap_or(1),
                                            0,
                                            "OC-CFG-005",
                                            t!("rules.oc_cfg_005.message", name = "provider")
                                                .to_string(),
                                        )
                                        .with_suggestion(
                                            t!("rules.oc_cfg_005.suggestion").to_string(),
                                        ),
                                    );
                                }
                            }
                        }

                        // Case 2: provider.<providerName>.options.apiKey
                        for (p_name, p_val) in provider_obj {
                            if p_name == "options" {
                                continue;
                            }
                            if let Some(p_opts) = p_val.get("options").and_then(|o| o.as_object()) {
                                if let Some(api_key) = p_opts.get("apiKey").and_then(|k| k.as_str())
                                {
                                    if !api_key.starts_with("{env:") {
                                        diagnostics.push(
                                            Diagnostic::error(
                                                path.to_path_buf(),
                                                find_key_line(content, "apiKey").unwrap_or(1),
                                                0,
                                                "OC-CFG-005",
                                                t!("rules.oc_cfg_005.message", name = p_name)
                                                    .to_string(),
                                            )
                                            .with_suggestion(
                                                t!("rules.oc_cfg_005.suggestion").to_string(),
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                // OC-CFG-006 & OC-CFG-007: MCP Server Structure & Requirements
                let check_mcp =
                    config.is_rule_enabled("OC-CFG-006") || config.is_rule_enabled("OC-CFG-007");
                if check_mcp {
                    if let Some(mcp_val) = obj.get("mcp") {
                        if let Some(mcp_obj) = mcp_val.as_object() {
                            for (srv_name, srv_val) in mcp_obj {
                                if let Some(srv) = srv_val.as_object() {
                                    let srv_type =
                                        srv.get("type").and_then(|t| t.as_str()).unwrap_or("");
                                    if srv_type != "local" && srv_type != "remote" {
                                        if config.is_rule_enabled("OC-CFG-006") {
                                            diagnostics.push(
                                                Diagnostic::error(
                                                    path.to_path_buf(),
                                                    find_key_line(content, srv_name).unwrap_or(1),
                                                    0,
                                                    "OC-CFG-006",
                                                    t!("rules.oc_cfg_006.message", typ = srv_type)
                                                        .to_string(),
                                                )
                                                .with_suggestion(
                                                    t!("rules.oc_cfg_006.suggestion").to_string(),
                                                ),
                                            );
                                        }
                                    } else if config.is_rule_enabled("OC-CFG-007") {
                                        if srv_type == "local" && !srv.contains_key("command") {
                                            diagnostics.push(
                                                Diagnostic::error(
                                                    path.to_path_buf(),
                                                    find_key_line(content, srv_name).unwrap_or(1),
                                                    0,
                                                    "OC-CFG-007",
                                                    t!("rules.oc_cfg_007.local_missing")
                                                        .to_string(),
                                                )
                                                .with_suggestion(
                                                    t!("rules.oc_cfg_007.suggestion_local")
                                                        .to_string(),
                                                ),
                                            );
                                        } else if srv_type == "local"
                                            && let Some(command_val) = srv.get("command")
                                        {
                                            let valid_command =
                                                command_val.as_array().is_some_and(|arr| {
                                                    !arr.is_empty()
                                                        && arr.iter().all(|v| {
                                                            v.as_str().is_some_and(|s| {
                                                                !s.trim().is_empty()
                                                            })
                                                        })
                                                });

                                            if !valid_command {
                                                diagnostics.push(
                                                    Diagnostic::error(
                                                        path.to_path_buf(),
                                                        find_key_line(content, srv_name).unwrap_or(1),
                                                        0,
                                                        "OC-CFG-007",
                                                        "Local MCP server 'command' must be a non-empty array of non-empty strings".to_string(),
                                                    )
                                                    .with_suggestion(
                                                        "Use command like [\"node\", \"server.js\"]"
                                                            .to_string(),
                                                    ),
                                                );
                                            }
                                        } else if srv_type == "remote" && !srv.contains_key("url") {
                                            diagnostics.push(
                                                Diagnostic::error(
                                                    path.to_path_buf(),
                                                    find_key_line(content, srv_name).unwrap_or(1),
                                                    0,
                                                    "OC-CFG-007",
                                                    t!("rules.oc_cfg_007.remote_missing")
                                                        .to_string(),
                                                )
                                                .with_suggestion(
                                                    t!("rules.oc_cfg_007.suggestion_remote")
                                                        .to_string(),
                                                ),
                                            );
                                        } else if srv_type == "remote"
                                            && let Some(url_val) = srv.get("url")
                                        {
                                            let valid_url = url_val.as_str().is_some_and(|url| {
                                                let trimmed = url.trim();
                                                !trimmed.is_empty()
                                                    && (trimmed.starts_with("http://")
                                                        || trimmed.starts_with("https://"))
                                            });

                                            if !valid_url {
                                                diagnostics.push(
                                                    Diagnostic::error(
                                                        path.to_path_buf(),
                                                        find_key_line(content, srv_name).unwrap_or(1),
                                                        0,
                                                        "OC-CFG-007",
                                                        "Remote MCP server 'url' must be a non-empty http:// or https:// URL".to_string(),
                                                    )
                                                    .with_suggestion(
                                                        "Set a valid URL such as \"https://example.com/mcp\""
                                                            .to_string(),
                                                    ),
                                                );
                                            }
                                        }
                                    }
                                } else if config.is_rule_enabled("OC-CFG-006") {
                                    diagnostics.push(
                                        Diagnostic::error(
                                            path.to_path_buf(),
                                            find_key_line(content, srv_name).unwrap_or(1),
                                            0,
                                            "OC-CFG-006",
                                            t!("rules.oc_cfg_006.type_error", name = srv_name)
                                                .to_string(),
                                        )
                                        .with_suggestion(
                                            t!("rules.oc_cfg_006.suggestion_type").to_string(),
                                        ),
                                    );
                                }
                            }
                        } else if config.is_rule_enabled("OC-CFG-006") {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    find_key_line(content, "mcp").unwrap_or(1),
                                    0,
                                    "OC-CFG-006",
                                    "Invalid mcp config type. Expected an object of named servers"
                                        .to_string(),
                                )
                                .with_suggestion(
                                    "Use object format: \"mcp\": { \"server-name\": { ... } }"
                                        .to_string(),
                                ),
                            );
                        }
                    }
                }

                // Agent Validation (OC-AG-*)
                if let Some(agent_obj) = obj.get("agent").and_then(|a| a.as_object()) {
                    for (ag_name, ag_val) in agent_obj {
                        if let Some(ag) = ag_val.as_object() {
                            // OC-AG-001
                            if config.is_rule_enabled("OC-AG-001") {
                                if let Some(mode_val) = ag.get("mode").and_then(|m| m.as_str()) {
                                    if mode_val != "subagent"
                                        && mode_val != "primary"
                                        && mode_val != "all"
                                    {
                                        diagnostics.push(
                                            Diagnostic::error(
                                                path.to_path_buf(),
                                                find_key_line(content, ag_name).unwrap_or(1),
                                                0,
                                                "OC-AG-001",
                                                t!("rules.oc_ag_001.message", mode = mode_val)
                                                    .to_string(),
                                            )
                                            .with_suggestion(
                                                t!("rules.oc_ag_001.suggestion").to_string(),
                                            ),
                                        );
                                    }
                                }
                            }

                            // OC-AG-002
                            if config.is_rule_enabled("OC-AG-002") {
                                if let Some(color_val) = ag.get("color").and_then(|c| c.as_str()) {
                                    let valid_theme_colors = [
                                        "accent", "blue", "cyan", "gray", "green", "indigo",
                                        "orange", "pink", "purple", "red", "teal", "yellow",
                                    ];
                                    if !is_valid_hex_color(color_val)
                                        && !valid_theme_colors.contains(&color_val)
                                    {
                                        diagnostics.push(
                                            Diagnostic::error(
                                                path.to_path_buf(),
                                                find_key_line(content, "color").unwrap_or(1),
                                                0,
                                                "OC-AG-002",
                                                t!("rules.oc_ag_002.message", color = color_val)
                                                    .to_string(),
                                            )
                                            .with_suggestion(
                                                t!("rules.oc_ag_002.suggestion").to_string(),
                                            ),
                                        );
                                    }
                                }
                            }

                            // OC-AG-003
                            if config.is_rule_enabled("OC-AG-003") {
                                if let Some(temp_raw) = ag.get("temperature") {
                                    if let Some(temp_val) = temp_raw.as_f64() {
                                        if !(0.0..=2.0).contains(&temp_val) {
                                            diagnostics.push(Diagnostic::error(
                                                path.to_path_buf(),
                                                find_key_line(content, "temperature").unwrap_or(1),
                                                0,
                                                "OC-AG-003",
                                                "Temperature out of range (must be 0-2)"
                                                    .to_string(),
                                            ));
                                        }
                                    } else if !temp_raw.is_null() {
                                        diagnostics.push(
                                            Diagnostic::error(
                                                path.to_path_buf(),
                                                find_key_line(content, "temperature").unwrap_or(1),
                                                0,
                                                "OC-AG-003",
                                                "Temperature must be a number between 0 and 2"
                                                    .to_string(),
                                            )
                                            .with_suggestion(
                                                "Set temperature to a numeric value such as 0.7"
                                                    .to_string(),
                                            ),
                                        );
                                    }
                                }
                            }

                            // OC-AG-004
                            if config.is_rule_enabled("OC-AG-004") {
                                if let Some(steps_raw) = ag.get("steps") {
                                    if let Some(steps_val) = steps_raw.as_i64() {
                                        if steps_val <= 0 {
                                            diagnostics.push(Diagnostic::error(
                                                path.to_path_buf(),
                                                find_key_line(content, "steps").unwrap_or(1),
                                                0,
                                                "OC-AG-004",
                                                "Steps must be a positive integer".to_string(),
                                            ));
                                        }
                                    } else if !steps_raw.is_null() {
                                        diagnostics.push(
                                            Diagnostic::error(
                                                path.to_path_buf(),
                                                find_key_line(content, "steps").unwrap_or(1),
                                                0,
                                                "OC-AG-004",
                                                "Steps must be a positive integer".to_string(),
                                            )
                                            .with_suggestion(
                                                "Use an integer greater than zero, such as 20"
                                                    .to_string(),
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                // OC-PM-002: Unknown Permission Key
                if config.is_rule_enabled("OC-PM-002") {
                    if let Some(perm_obj) = obj.get("permission").and_then(|p| p.as_object()) {
                        let known_perms = [
                            "read",
                            "edit",
                            "glob",
                            "grep",
                            "list",
                            "bash",
                            "task",
                            "lsp",
                            "skill",
                            "todowrite",
                            "todoread",
                            "question",
                            "webfetch",
                            "websearch",
                            "external_directory",
                            "doom_loop",
                        ];
                        for key in perm_obj.keys() {
                            if !known_perms.contains(&key.as_str()) {
                                diagnostics.push(Diagnostic::warning(
                                    path.to_path_buf(),
                                    find_key_line(content, key).unwrap_or(1),
                                    0,
                                    "OC-PM-002",
                                    format!("Unknown permission key '{}'", key),
                                ));
                            }
                        }
                    }
                }

                // OC-PM-001: Invalid permission action value
                if config.is_rule_enabled("OC-PM-001")
                    && let Some(permission_val) = obj.get("permission")
                {
                    let perm_line = find_key_line(content, "permission").unwrap_or(1);
                    match permission_val {
                        serde_json::Value::String(action) => {
                            if !VALID_PERMISSION_MODES.contains(&action.as_str()) {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        perm_line,
                                        0,
                                        "OC-PM-001",
                                        t!(
                                            "rules.oc_pm_001.message",
                                            value = action.as_str(),
                                            tool = "*"
                                        )
                                        .to_string(),
                                    )
                                    .with_suggestion(t!("rules.oc_pm_001.suggestion").to_string()),
                                );
                            }
                        }
                        serde_json::Value::Object(perm_obj) => {
                            for (tool, mode_value) in perm_obj {
                                if let Some(mode_str) = mode_value.as_str() {
                                    if !VALID_PERMISSION_MODES.contains(&mode_str) {
                                        diagnostics.push(
                                            Diagnostic::error(
                                                path.to_path_buf(),
                                                perm_line,
                                                0,
                                                "OC-PM-001",
                                                t!(
                                                    "rules.oc_pm_001.message",
                                                    value = mode_str,
                                                    tool = tool.as_str()
                                                )
                                                .to_string(),
                                            )
                                            .with_suggestion(
                                                t!("rules.oc_pm_001.suggestion").to_string(),
                                            ),
                                        );
                                    }
                                } else if let Some(mode_obj) = mode_value.as_object() {
                                    for nested_mode in mode_obj.values() {
                                        if let Some(pm) = nested_mode.as_str() {
                                            if !VALID_PERMISSION_MODES.contains(&pm) {
                                                diagnostics.push(
                                                    Diagnostic::error(
                                                        path.to_path_buf(),
                                                        perm_line,
                                                        0,
                                                        "OC-PM-001",
                                                        t!(
                                                            "rules.oc_pm_001.message",
                                                            value = pm,
                                                            tool = tool.as_str()
                                                        )
                                                        .to_string(),
                                                    )
                                                    .with_suggestion(
                                                        t!("rules.oc_pm_001.suggestion")
                                                            .to_string(),
                                                    ),
                                                );
                                            }
                                        } else if !nested_mode.is_null() {
                                            diagnostics.push(
                                                Diagnostic::error(
                                                    path.to_path_buf(),
                                                    perm_line,
                                                    0,
                                                    "OC-PM-001",
                                                    t!("rules.oc_pm_001.type_error").to_string(),
                                                )
                                                .with_suggestion(
                                                    t!("rules.oc_pm_001.suggestion").to_string(),
                                                ),
                                            );
                                        }
                                    }
                                } else if !mode_value.is_null() {
                                    diagnostics.push(
                                        Diagnostic::error(
                                            path.to_path_buf(),
                                            perm_line,
                                            0,
                                            "OC-PM-001",
                                            t!("rules.oc_pm_001.type_error").to_string(),
                                        )
                                        .with_suggestion(
                                            t!("rules.oc_pm_001.suggestion").to_string(),
                                        ),
                                    );
                                }
                            }
                        }
                        serde_json::Value::Null => {}
                        _ => {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    perm_line,
                                    0,
                                    "OC-PM-001",
                                    t!("rules.oc_pm_001.type_error").to_string(),
                                )
                                .with_suggestion(t!("rules.oc_pm_001.suggestion").to_string()),
                            );
                        }
                    }
                }
            }
        }

        diagnostics
    }
}

/// Recursively walk the JSON value tree and validate any string containing
/// variable substitution patterns like `{env:...}` or `{file:...}`.
///
/// Depth is bounded to prevent stack overflow on pathologically nested JSON.
/// In practice, `file_utils::safe_read_file` enforces a 1 MiB limit upstream,
/// but the depth guard is an additional safety layer.
fn validate_substitutions(
    value: &serde_json::Value,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_substitutions_inner(value, path, content, diagnostics, 0);
}

/// Maximum recursion depth for JSON tree traversal (OC-009).
const MAX_SUBSTITUTION_DEPTH: usize = 64;

fn validate_substitutions_inner(
    value: &serde_json::Value,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
    depth: usize,
) {
    if depth > MAX_SUBSTITUTION_DEPTH {
        return;
    }
    match value {
        serde_json::Value::String(s) => {
            validate_substitution_string(s, path, content, diagnostics);
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                validate_substitutions_inner(item, path, content, diagnostics, depth + 1);
            }
        }
        serde_json::Value::Object(obj) => {
            for (_, v) in obj {
                validate_substitutions_inner(v, path, content, diagnostics, depth + 1);
            }
        }
        _ => {}
    }
}

/// Validate substitution patterns in a single string value.
///
/// Valid patterns: `{env:VARIABLE_NAME}`, `{file:path/to/file}`
/// Flags: unknown prefix (not env or file), empty value part
fn validate_substitution_string(
    s: &str,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Match patterns like {word:...}
    let mut start = 0;
    while let Some(open_pos) = s[start..].find('{') {
        let abs_open = start + open_pos;
        if let Some(close_pos) = s[abs_open..].find('}') {
            let abs_close = abs_open + close_pos;
            let inner = &s[abs_open + 1..abs_close];

            if let Some(colon_pos) = inner.find(':') {
                let prefix = &inner[..colon_pos];
                let value_part = &inner[colon_pos + 1..];

                // Only flag patterns that look like substitutions (word:something)
                if !prefix.is_empty()
                    && prefix
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    let reason = if prefix != "env" && prefix != "file" {
                        Some(format!(
                            "unknown prefix '{}'. Valid prefixes: 'env', 'file'",
                            prefix
                        ))
                    } else if value_part.is_empty() {
                        Some(format!("empty value after '{}:'", prefix))
                    } else {
                        None
                    };

                    if let Some(reason_str) = reason {
                        let pattern = format!("{{{}}}", inner);
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                find_string_line(content, &pattern).unwrap_or(1),
                                0,
                                "OC-009",
                                t!(
                                    "rules.oc_009.message",
                                    pattern = pattern.as_str(),
                                    reason = reason_str.as_str()
                                ),
                            )
                            .with_suggestion(t!("rules.oc_009.suggestion")),
                        );
                    }
                }
            }

            start = abs_close + 1;
        } else {
            break;
        }
    }
}

/// Find the 1-indexed line number where a string pattern appears in content.
fn find_string_line(content: &str, pattern: &str) -> Option<usize> {
    for (i, line) in content.lines().enumerate() {
        if line.contains(pattern) {
            return Some(i + 1);
        }
    }
    None
}

/// Find the 1-indexed line number of a JSON key in the content.
///
/// Looks for `"key"` followed by `:` to avoid matching the key name
/// when it appears as a string value rather than an object key.
fn find_key_line(content: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{}\"", key);
    for (i, line) in content.lines().enumerate() {
        if let Some(pos) = line.find(&needle) {
            // Check that a colon follows the key (possibly with whitespace)
            let after = &line[pos + needle.len()..];
            if after.trim_start().starts_with(':') {
                return Some(i + 1);
            }
        }
    }
    None
}

fn is_valid_hex_color(value: &str) -> bool {
    if !value.starts_with('#') {
        return false;
    }
    let hex = &value[1..];
    (hex.len() == 3 || hex.len() == 6) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = OpenCodeValidator;
        validator.validate(Path::new("opencode.json"), content, &LintConfig::default())
    }

    fn validate_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = OpenCodeValidator;
        validator.validate(Path::new("opencode.json"), content, config)
    }

    // ===== OC-003: Parse Error =====

    #[test]
    fn test_oc_003_invalid_json() {
        let diagnostics = validate("{ invalid json }");
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert_eq!(oc_003.len(), 1);
        assert_eq!(oc_003[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_oc_003_empty_content() {
        let diagnostics = validate("");
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert_eq!(oc_003.len(), 1);
    }

    #[test]
    fn test_oc_003_trailing_comma() {
        let diagnostics = validate(r#"{"share": "manual",}"#);
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert_eq!(oc_003.len(), 1);
    }

    #[test]
    fn test_oc_003_valid_json() {
        let diagnostics = validate(r#"{"share": "manual"}"#);
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert!(oc_003.is_empty());
    }

    #[test]
    fn test_oc_003_jsonc_comments_allowed() {
        let content = r#"{
  // This is a JSONC comment
  "share": "manual"
}"#;
        let diagnostics = validate(content);
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert!(oc_003.is_empty());
    }

    #[test]
    fn test_oc_003_blocks_further_rules() {
        // When JSON is invalid, no OC-001/OC-002 should fire
        let diagnostics = validate("{ invalid }");
        assert!(diagnostics.iter().all(|d| d.rule == "OC-003"));
    }

    // ===== OC-001: Invalid Share Mode =====

    #[test]
    fn test_oc_001_invalid_share_mode() {
        let diagnostics = validate(r#"{"share": "public"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1);
        assert_eq!(oc_001[0].level, DiagnosticLevel::Error);
        assert!(oc_001[0].message.contains("public"));
    }

    #[test]
    fn test_oc_001_valid_manual() {
        let diagnostics = validate(r#"{"share": "manual"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert!(oc_001.is_empty());
    }

    #[test]
    fn test_oc_001_valid_auto() {
        let diagnostics = validate(r#"{"share": "auto"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert!(oc_001.is_empty());
    }

    #[test]
    fn test_oc_001_valid_disabled() {
        let diagnostics = validate(r#"{"share": "disabled"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert!(oc_001.is_empty());
    }

    #[test]
    fn test_oc_001_autofix_case_insensitive() {
        // "Manual" is a case-insensitive match to "manual"
        let diagnostics = validate(r#"{"share": "Manual"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1);
        assert!(
            oc_001[0].has_fixes(),
            "OC-001 should have auto-fix for case mismatch"
        );
        let fix = &oc_001[0].fixes[0];
        assert!(!fix.safe, "OC-001 fix should be unsafe");
        assert_eq!(fix.replacement, "manual", "Fix should suggest 'manual'");
    }

    #[test]
    fn test_oc_001_no_autofix_when_duplicate() {
        // JSON with two "share" keys (duplicate keys are technically valid JSON
        // but our regex uniqueness guard should catch this and suppress autofix).
        let content = r#"{"share": "Manual", "nested": {"share": "Manual"}}"#;
        let diagnostics = validate(content);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1);
        assert!(
            !oc_001[0].has_fixes(),
            "OC-001 should not have auto-fix when share value appears multiple times"
        );
    }

    #[test]
    fn test_oc_001_no_autofix_nonsense() {
        let diagnostics = validate(r#"{"share": "public"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1);
        // "public" has no close match - should NOT get a fix
        assert!(
            !oc_001[0].has_fixes(),
            "OC-001 should not auto-fix nonsense values"
        );
    }

    #[test]
    fn test_oc_001_autofix_targets_correct_bytes() {
        let content = r#"{"share": "Manual"}"#;
        let diagnostics = validate(content);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1);
        assert!(oc_001[0].has_fixes());
        let fix = &oc_001[0].fixes[0];
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "Manual", "Fix should target the inner value");
    }

    #[test]
    fn test_oc_001_absent_share() {
        // No share field at all should not trigger OC-001
        let diagnostics = validate(r#"{}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert!(oc_001.is_empty());
    }

    #[test]
    fn test_oc_001_empty_string() {
        let diagnostics = validate(r#"{"share": ""}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1);
    }

    #[test]
    fn test_oc_001_case_sensitive() {
        let diagnostics = validate(r#"{"share": "Manual"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1, "Share mode should be case-sensitive");
    }

    #[test]
    fn test_oc_001_line_number() {
        let content = "{\n  \"share\": \"invalid\"\n}";
        let diagnostics = validate(content);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1);
        assert_eq!(oc_001[0].line, 2);
    }

    // ===== OC-002: Invalid Instruction Path =====

    #[test]
    fn test_oc_002_nonexistent_path() {
        let diagnostics =
            validate(r#"{"instructions": ["nonexistent-file-that-does-not-exist.md"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(oc_002.len(), 1);
        assert_eq!(oc_002[0].level, DiagnosticLevel::Error);
        assert!(oc_002[0].message.contains("nonexistent-file"));
    }

    #[test]
    fn test_oc_002_valid_glob_pattern() {
        // Valid glob patterns should pass even if no files match
        let diagnostics = validate(r#"{"instructions": ["**/*.md"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert!(oc_002.is_empty());
    }

    #[test]
    fn test_oc_002_invalid_glob_pattern() {
        let diagnostics = validate(r#"{"instructions": ["[unclosed"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(oc_002.len(), 1);
    }

    #[test]
    fn test_oc_002_absent_instructions() {
        // No instructions field should not trigger OC-002
        let diagnostics = validate(r#"{}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert!(oc_002.is_empty());
    }

    #[test]
    fn test_oc_002_empty_instructions_array() {
        let diagnostics = validate(r#"{"instructions": []}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert!(oc_002.is_empty());
    }

    #[test]
    fn test_oc_002_multiple_invalid_paths() {
        let diagnostics = validate(r#"{"instructions": ["nonexistent1.md", "nonexistent2.md"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(oc_002.len(), 2);
    }

    #[test]
    fn test_oc_002_mixed_valid_invalid() {
        // Glob patterns pass, nonexistent literal paths fail
        let diagnostics = validate(r#"{"instructions": ["**/*.md", "nonexistent.md"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(oc_002.len(), 1);
        assert!(oc_002[0].message.contains("nonexistent.md"));
    }

    #[test]
    fn test_oc_002_empty_path_skipped() {
        let diagnostics = validate(r#"{"instructions": [""]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert!(oc_002.is_empty());
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_opencode_category() {
        let mut config = LintConfig::default();
        config.rules_mut().opencode = false;

        let diagnostics = validate_with_config(r#"{"share": "invalid"}"#, &config);
        let oc_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("OC-"))
            .collect();
        assert!(oc_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["OC-001".to_string()];

        let diagnostics = validate_with_config(r#"{"share": "invalid"}"#, &config);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert!(oc_001.is_empty());
    }

    #[test]
    fn test_all_oc_rules_can_be_disabled() {
        let rules = [
            "OC-001", "OC-002", "OC-003", "OC-004", "OC-006", "OC-007", "OC-008", "OC-009",
        ];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];

            let content = match rule {
                "OC-001" => r#"{"share": "invalid"}"#,
                "OC-002" => r#"{"instructions": ["nonexistent.md"]}"#,
                "OC-003" => "{ invalid }",
                "OC-004" => r#"{"totally_unknown": true}"#,
                "OC-006" => r#"{"instructions": ["https://example.com/rules.md"]}"#,
                "OC-007" => r#"{"agent": {"test": {}}}"#,
                "OC-008" => r#"{"permission": {"read": "bogus"}}"#,
                "OC-009" => r#"{"model": "{bad:value}"}"#,
                _ => unreachable!(),
            };

            let diagnostics = validate_with_config(content, &config);
            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    // ===== Valid Config =====

    #[test]
    fn test_valid_config_no_issues() {
        let content = r#"{
  "share": "manual",
  "instructions": ["**/*.md"]
}"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_empty_object_no_issues() {
        let diagnostics = validate("{}");
        assert!(diagnostics.is_empty());
    }

    // ===== Path Traversal Prevention =====

    #[test]
    fn test_oc_002_absolute_path_rejected() {
        let diagnostics = validate(r#"{"instructions": ["/etc/passwd"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(oc_002.len(), 1);
    }

    #[test]
    fn test_oc_002_parent_dir_traversal_rejected() {
        let diagnostics = validate(r#"{"instructions": ["../../etc/shadow"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(oc_002.len(), 1);
    }

    // ===== Type Mismatch Handling =====

    #[test]
    fn test_type_mismatch_share_not_string() {
        // "share": true is valid JSON but wrong type; should not be OC-003
        let diagnostics = validate(r#"{"share": true}"#);
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert!(
            oc_003.is_empty(),
            "Type mismatch should not be a parse error"
        );
        // Should emit OC-001 for wrong type
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1, "Wrong type share should trigger OC-001");
        assert!(oc_001[0].message.contains("string"));
    }

    #[test]
    fn test_type_mismatch_share_number() {
        let diagnostics = validate(r#"{"share": 123}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1, "Numeric share should trigger OC-001");
    }

    #[test]
    fn test_type_mismatch_instructions_not_array() {
        // "instructions": "README.md" is valid JSON but wrong type
        let diagnostics = validate(r#"{"instructions": "README.md"}"#);
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert!(
            oc_003.is_empty(),
            "Type mismatch should not be a parse error"
        );
        // Should emit OC-002 for wrong type
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(
            oc_002.len(),
            1,
            "Non-array instructions should trigger OC-002"
        );
        assert!(oc_002[0].message.contains("array"));
    }

    #[test]
    fn test_type_mismatch_instructions_with_non_string_elements() {
        let diagnostics = validate(r#"{"instructions": [123, true]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert!(
            !oc_002.is_empty(),
            "Non-string array elements should trigger OC-002"
        );
    }

    // ===== OC-004: Unknown config keys =====

    #[test]
    fn test_oc_004_unknown_key() {
        let diagnostics = validate(r#"{"totally_unknown": true}"#);
        let oc_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-004").collect();
        assert_eq!(oc_004.len(), 1);
        assert_eq!(oc_004[0].level, DiagnosticLevel::Warning);
        assert!(oc_004[0].message.contains("totally_unknown"));
    }

    #[test]
    fn test_oc_004_known_keys_no_warning() {
        let content = r#"{
  "share": "manual",
  "instructions": ["**/*.md"],
  "model": "claude-sonnet-4-5",
  "agent": {},
  "permission": {},
  "autoshare": "manual",
  "enterprise": {},
  "layout": "stretch",
  "logLevel": "INFO",
  "lsp": false,
  "mode": "agent",
  "skills": [],
  "snapshot": false,
  "username": "dev"
}"#;
        let diagnostics = validate(content);
        let oc_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-004").collect();
        assert!(oc_004.is_empty(), "Known keys should not trigger OC-004");
    }

    #[test]
    fn test_oc_004_multiple_unknown_keys() {
        let content = r#"{"unknown_a": true, "unknown_b": false, "share": "manual"}"#;
        let diagnostics = validate(content);
        let oc_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-004").collect();
        assert_eq!(oc_004.len(), 2);
    }

    #[test]
    fn test_oc_004_has_suggestion() {
        let diagnostics = validate(r#"{"bogus_setting": 42}"#);
        let oc_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-004").collect();
        assert_eq!(oc_004.len(), 1);
        assert!(
            oc_004[0].suggestion.is_some(),
            "OC-004 should have a suggestion"
        );
    }

    // ===== OC-006: Remote URL in instructions =====

    #[test]
    fn test_oc_006_https_url() {
        let diagnostics = validate(r#"{"instructions": ["https://example.com/rules.md"]}"#);
        let oc_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-006").collect();
        assert_eq!(oc_006.len(), 1);
        assert_eq!(oc_006[0].level, DiagnosticLevel::Info);
        assert!(oc_006[0].message.contains("https://example.com"));
    }

    #[test]
    fn test_oc_006_http_url() {
        let diagnostics = validate(r#"{"instructions": ["http://example.com/rules.md"]}"#);
        let oc_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-006").collect();
        assert_eq!(oc_006.len(), 1);
    }

    #[test]
    fn test_oc_006_local_path_no_warning() {
        let diagnostics = validate(r#"{"instructions": ["**/*.md"]}"#);
        let oc_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-006").collect();
        assert!(oc_006.is_empty());
    }

    #[test]
    fn test_oc_006_url_not_checked_as_path() {
        // URLs should trigger OC-006 but NOT OC-002 (not-found)
        let diagnostics = validate(r#"{"instructions": ["https://example.com/rules.md"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert!(
            oc_002.is_empty(),
            "URLs should not be checked as file paths"
        );
    }

    // ===== OC-007: Agent validation =====

    #[test]
    fn test_oc_007_missing_description() {
        let diagnostics = validate(r#"{"agent": {"my-agent": {"model": "gpt-4"}}}"#);
        let oc_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-007").collect();
        assert_eq!(oc_007.len(), 1);
        assert_eq!(oc_007[0].level, DiagnosticLevel::Warning);
        assert!(oc_007[0].message.contains("my-agent"));
    }

    #[test]
    fn test_oc_007_with_description() {
        let content =
            r#"{"agent": {"my-agent": {"description": "A test agent", "model": "gpt-4"}}}"#;
        let diagnostics = validate(content);
        let oc_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-007").collect();
        assert!(oc_007.is_empty());
    }

    #[test]
    fn test_oc_007_wrong_type() {
        let diagnostics = validate(r#"{"agent": "not an object"}"#);
        let oc_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-007").collect();
        assert_eq!(oc_007.len(), 1);
        assert_eq!(oc_007[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_oc_007_absent() {
        let diagnostics = validate(r#"{"share": "manual"}"#);
        let oc_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-007").collect();
        assert!(oc_007.is_empty());
    }

    #[test]
    fn test_oc_007_multiple_agents() {
        let content = r#"{"agent": {"agent-a": {}, "agent-b": {"description": "ok"}}}"#;
        let diagnostics = validate(content);
        let oc_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-007").collect();
        assert_eq!(oc_007.len(), 1, "Only agent-a should trigger OC-007");
        assert!(oc_007[0].message.contains("agent-a"));
    }

    #[test]
    fn test_oc_007_non_object_agent_entry() {
        // Agent entry that's a string instead of an object should trigger OC-007
        let diagnostics = validate(r#"{"agent": {"my-agent": "oops"}}"#);
        let oc_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-007").collect();
        assert_eq!(
            oc_007.len(),
            1,
            "Non-object agent entry should trigger OC-007"
        );
        assert!(oc_007[0].message.contains("my-agent"));
    }

    // ===== OC-008: Permission validation =====

    #[test]
    fn test_oc_008_valid_permissions() {
        let content = r#"{"permission": {"read": "allow", "edit": "ask", "bash": "deny"}}"#;
        let diagnostics = validate(content);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert!(oc_008.is_empty());
    }

    #[test]
    fn test_oc_008_invalid_permission_value() {
        let diagnostics = validate(r#"{"permission": {"read": "yes"}}"#);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert_eq!(oc_008.len(), 1);
        assert_eq!(oc_008[0].level, DiagnosticLevel::Error);
        assert!(oc_008[0].message.contains("yes"));
        assert!(oc_008[0].message.contains("read"));
    }

    #[test]
    fn test_oc_008_has_fix() {
        // Use a case-insensitive mismatch that find_closest_value can match
        let content = r#"{"permission": "Allow"}"#;
        let diagnostics = validate(content);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert_eq!(oc_008.len(), 1);
        assert!(
            oc_008[0].has_fixes(),
            "OC-008 should have auto-fix for case-mismatched permission mode"
        );
        let fix = &oc_008[0].fixes[0];
        assert!(!fix.safe, "OC-008 fix should be unsafe");
        assert_eq!(
            fix.replacement, "allow",
            "Fix should suggest 'allow' as closest match"
        );
    }

    #[test]
    fn test_oc_008_global_string_valid() {
        let diagnostics = validate(r#"{"permission": "allow"}"#);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert!(oc_008.is_empty());
    }

    #[test]
    fn test_oc_008_global_string_invalid() {
        let diagnostics = validate(r#"{"permission": "bogus"}"#);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert_eq!(oc_008.len(), 1);
        assert!(oc_008[0].message.contains("bogus"));
    }

    #[test]
    fn test_oc_008_wrong_type() {
        let diagnostics = validate(r#"{"permission": 42}"#);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert_eq!(oc_008.len(), 1);
        assert_eq!(oc_008[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_oc_008_absent() {
        let diagnostics = validate(r#"{"share": "manual"}"#);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert!(oc_008.is_empty());
    }

    #[test]
    fn test_oc_008_nested_pattern_permissions() {
        let content = r#"{"permission": {"bash": {"*.sh": "allow", "*.py": "invalid"}}}"#;
        let diagnostics = validate(content);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert_eq!(oc_008.len(), 1, "Only 'invalid' should trigger OC-008");
    }

    #[test]
    fn test_oc_008_non_string_permission_value() {
        // Permission value that's a number instead of a string should trigger OC-008
        let diagnostics = validate(r#"{"permission": {"read": 123}}"#);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert_eq!(
            oc_008.len(),
            1,
            "Non-string permission value should trigger OC-008"
        );
    }

    #[test]
    fn test_oc_008_non_string_nested_permission_value() {
        // Nested permission value that's not a string should trigger OC-008
        let diagnostics = validate(r#"{"permission": {"bash": {"*.sh": true}}}"#);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert_eq!(
            oc_008.len(),
            1,
            "Non-string nested permission should trigger OC-008"
        );
    }

    // ===== OC-009: Variable substitution validation =====

    #[test]
    fn test_oc_009_valid_env_substitution() {
        let diagnostics = validate(r#"{"model": "{env:OPENAI_MODEL}"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert!(oc_009.is_empty());
    }

    #[test]
    fn test_oc_009_valid_file_substitution() {
        let diagnostics = validate(r#"{"model": "{file:model.txt}"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert!(oc_009.is_empty());
    }

    #[test]
    fn test_oc_009_unknown_prefix() {
        let diagnostics = validate(r#"{"model": "{bad:value}"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert_eq!(oc_009.len(), 1);
        assert_eq!(oc_009[0].level, DiagnosticLevel::Warning);
        assert!(oc_009[0].message.contains("bad"));
    }

    #[test]
    fn test_oc_009_empty_env_value() {
        let diagnostics = validate(r#"{"model": "{env:}"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert_eq!(oc_009.len(), 1);
        assert!(oc_009[0].message.contains("empty"));
    }

    #[test]
    fn test_oc_009_empty_file_value() {
        let diagnostics = validate(r#"{"model": "{file:}"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert_eq!(oc_009.len(), 1);
    }

    #[test]
    fn test_oc_009_no_substitution_no_warning() {
        let diagnostics = validate(r#"{"model": "gpt-4"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert!(oc_009.is_empty());
    }

    #[test]
    fn test_oc_009_nested_value() {
        // Substitution in a nested value should be found
        let diagnostics = validate(r#"{"tui": {"prompt": "{bogus:test}"}}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert_eq!(oc_009.len(), 1);
    }

    #[test]
    fn test_oc_009_multiple_substitutions_in_one_string() {
        let diagnostics = validate(r#"{"model": "{env:MODEL} and {file:path.txt} and {bad:x}"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert_eq!(
            oc_009.len(),
            1,
            "Only {{bad:x}} should flag, not {{env:MODEL}} or {{file:path.txt}}"
        );
    }

    #[test]
    fn test_oc_009_colon_in_value_part() {
        // {file:C:/path/to/file} has a colon in the value part - should still be valid
        let diagnostics = validate(r#"{"model": "{file:C:/path/to/file}"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert!(
            oc_009.is_empty(),
            "Colons after the first should be part of the value"
        );
    }

    #[test]
    fn test_oc_009_unmatched_opening_brace() {
        // An unmatched opening brace without closing should not crash
        let diagnostics = validate(r#"{"model": "some {env:FOO text"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert!(oc_009.is_empty(), "Unmatched brace should be ignored");
    }

    #[test]
    fn test_oc_009_non_substitution_braces() {
        // Plain braces like JSON-in-string should not flag
        let diagnostics = validate(r#"{"model": "value with {json} content"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert!(
            oc_009.is_empty(),
            "{{json}} without colon should not be flagged"
        );
    }

    // ===== Fixture Integration =====

    #[test]
    fn test_valid_opencode_fixture_no_diagnostics() {
        let fixture = include_str!("../../../../tests/fixtures/opencode/opencode.json");
        let diagnostics = validate(fixture);
        assert!(
            diagnostics.is_empty(),
            "Valid opencode fixture should produce 0 diagnostics, got: {:?}",
            diagnostics
        );
    }

    // ===== find_key_line =====

    #[test]
    fn test_find_key_line() {
        let content = "{\n  \"share\": \"manual\",\n  \"instructions\": []\n}";
        assert_eq!(find_key_line(content, "share"), Some(2));
        assert_eq!(find_key_line(content, "instructions"), Some(3));
        assert_eq!(find_key_line(content, "nonexistent"), None);
    }

    #[test]
    fn test_find_key_line_ignores_value_match() {
        // "share" appears as a value, not as a key
        let content = r#"{"comment": "the share key is important", "share": "manual"}"#;
        // Should still find "share" as a key (second occurrence)
        assert_eq!(find_key_line(content, "share"), Some(1));
    }

    #[test]
    fn test_find_key_line_no_false_positive_on_value() {
        // "share" only appears as a value, never as a key
        let content = "{\n  \"comment\": \"share\"\n}";
        assert_eq!(find_key_line(content, "share"), None);
    }

    // ===== OC-CFG-001: Invalid Model Format =====
    #[test]
    fn test_oc_cfg_001_invalid_model() {
        let diagnostics = validate(r#"{"model": "gpt-4"}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-001"));
    }

    #[test]
    fn test_oc_cfg_001_valid_model() {
        let diagnostics = validate(r#"{"model": "openai/gpt-4"}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-001"));
    }

    #[test]
    fn test_oc_cfg_002_invalid_autoupdate() {
        let diagnostics = validate(r#"{"autoupdate": "yes"}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-002"));
    }

    #[test]
    fn test_oc_cfg_002_valid_autoupdate_notify() {
        let diagnostics = validate(r#"{"autoupdate": "notify"}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-002"));
    }

    #[test]
    fn test_oc_cfg_003_unknown_top_level_key() {
        let diagnostics = validate(r#"{"unknown_field": true}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-003"));
    }

    // ===== OC-CFG-004: Invalid Default Agent =====
    #[test]
    fn test_oc_cfg_004_invalid_agent() {
        let diagnostics = validate(r#"{"default_agent": "foo"}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-004"));
    }

    #[test]
    fn test_oc_cfg_004_valid_agent() {
        let diagnostics = validate(r#"{"default_agent": "build"}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-004"));
    }

    #[test]
    fn test_oc_cfg_004_non_string_agent() {
        let diagnostics = validate(r#"{"default_agent": 123}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-004"));
    }

    // ===== OC-CFG-005: Hardcoded API Key =====
    #[test]
    fn test_oc_cfg_005_hardcoded_key() {
        let diagnostics = validate(r#"{"provider": {"test": {"options": {"apiKey": "sk-123"}}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-005"));
    }

    #[test]
    fn test_oc_cfg_005_env_key() {
        let diagnostics =
            validate(r#"{"provider": {"test": {"options": {"apiKey": "{env:TEST}"}}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-005"));
    }

    // ===== OC-CFG-006 & OC-CFG-007: MCP Server =====
    #[test]
    fn test_oc_cfg_006_invalid_mcp_type() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"type": "foo"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-006"));
    }

    #[test]
    fn test_oc_cfg_007_missing_command() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"type": "local"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-007"));
    }

    #[test]
    fn test_oc_cfg_006_mcp_must_be_object() {
        let diagnostics = validate(r#"{"mcp": []}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-006"));
    }

    #[test]
    fn test_oc_cfg_007_local_command_type_check() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"type": "local", "command": "node"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-007"));
    }

    #[test]
    fn test_oc_cfg_007_remote_url_format_check() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"type": "remote", "url": "not-a-url"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-007"));
    }

    // ===== Agent tests =====
    #[test]
    fn test_oc_ag_001_invalid_mode() {
        let diagnostics = validate(r#"{"agent": {"a": {"mode": "foo"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-001"));
    }

    #[test]
    fn test_oc_ag_002_invalid_color() {
        let diagnostics = validate(r#"{"agent": {"a": {"color": "foo"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-002"));
    }

    #[test]
    fn test_oc_ag_002_invalid_hex_color() {
        let diagnostics = validate(r##"{"agent": {"a": {"color": "#12"}}}"##);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-002"));
    }

    #[test]
    fn test_oc_ag_003_invalid_temp() {
        let diagnostics = validate(r#"{"agent": {"a": {"temperature": 3.0}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-003"));
    }

    #[test]
    fn test_oc_ag_003_temperature_type_check() {
        let diagnostics = validate(r#"{"agent": {"a": {"temperature": "hot"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-003"));
    }

    #[test]
    fn test_oc_ag_004_invalid_steps() {
        let diagnostics = validate(r#"{"agent": {"a": {"steps": -1}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-004"));
    }

    #[test]
    fn test_oc_ag_004_steps_type_check() {
        let diagnostics = validate(r#"{"agent": {"a": {"steps": "many"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-004"));
    }

    #[test]
    fn test_oc_pm_001_invalid_action() {
        let diagnostics = validate(r#"{"permission": {"read": "yes"}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-PM-001"));
    }

    #[test]
    fn test_is_valid_hex_color_helper() {
        assert!(is_valid_hex_color("#fff"));
        assert!(is_valid_hex_color("#FF00AA"));
        assert!(!is_valid_hex_color("#12"));
        assert!(!is_valid_hex_color("red"));
    }

    #[test]
    fn test_oc_pm_002_invalid_perm() {
        let diagnostics = validate(r#"{"permission": {"foo": "allow"}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-PM-002"));
    }
}
