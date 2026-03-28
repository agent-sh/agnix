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
        DEPRECATED_KEYS, KNOWN_TUI_KEYS, VALID_DIFF_STYLES, VALID_LOG_LEVELS, VALID_NAMED_COLORS,
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
    "OC-CFG-008",
    "OC-CFG-009",
    "OC-CFG-010",
    "OC-CFG-011",
    "OC-CFG-012",
    "OC-AG-001",
    "OC-AG-002",
    "OC-AG-003",
    "OC-AG-004",
    "OC-AG-005",
    "OC-AG-006",
    "OC-AG-007",
    "OC-AG-008",
    "OC-DEP-001",
    "OC-DEP-002",
    "OC-DEP-003",
    "OC-DEP-004",
    "OC-LSP-001",
    "OC-LSP-002",
    "OC-TUI-001",
    "OC-TUI-002",
    "OC-TUI-003",
    "OC-PM-001",
    "OC-PM-002",
];

/// Builtin agent names recognized by OpenCode.
const BUILTIN_AGENTS: &[&str] = &[
    "build",
    "plan",
    "general",
    "explore",
    "compaction",
    "title",
    "summary",
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
                            let is_known = BUILTIN_AGENTS.contains(&agent_str)
                                || obj
                                    .get("agent")
                                    .and_then(|a| a.as_object())
                                    .is_some_and(|agents| agents.contains_key(agent_str));

                            if !is_known {
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
                                        if srv_type == "local" {
                                            if !srv.contains_key("command") {
                                                diagnostics.push(
                                                    Diagnostic::error(
                                                        path.to_path_buf(),
                                                        find_key_line(content, srv_name)
                                                            .unwrap_or(1),
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
                                            } else if let Some(command_val) = srv.get("command") {
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
                                            }
                                        } else if srv_type == "remote" {
                                            if !srv.contains_key("url") {
                                                diagnostics.push(
                                                    Diagnostic::error(
                                                        path.to_path_buf(),
                                                        find_key_line(content, srv_name)
                                                            .unwrap_or(1),
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
                                            } else if let Some(url_val) = srv.get("url") {
                                                let valid_url =
                                                    url_val.as_str().is_some_and(|url| {
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
                                        && !VALID_NAMED_COLORS.contains(&color_val)
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

                            // OC-AG-005: top_p out of range
                            if config.is_rule_enabled("OC-AG-005") {
                                if let Some(top_p_raw) = ag.get("top_p") {
                                    if let Some(top_p_val) = top_p_raw.as_f64() {
                                        if !(0.0..=1.0).contains(&top_p_val) {
                                            diagnostics.push(
                                                Diagnostic::error(
                                                    path.to_path_buf(),
                                                    find_key_line(content, "top_p").unwrap_or(1),
                                                    0,
                                                    "OC-AG-005",
                                                    format!(
                                                        "top_p must be between 0.0 and 1.0, got {}",
                                                        top_p_val
                                                    ),
                                                )
                                                .with_suggestion(
                                                    "Set top_p to a value between 0.0 and 1.0"
                                                        .to_string(),
                                                ),
                                            );
                                        }
                                    } else if !top_p_raw.is_null() {
                                        diagnostics.push(
                                            Diagnostic::error(
                                                path.to_path_buf(),
                                                find_key_line(content, "top_p").unwrap_or(1),
                                                0,
                                                "OC-AG-005",
                                                "top_p must be a number between 0.0 and 1.0"
                                                    .to_string(),
                                            )
                                            .with_suggestion(
                                                "Set top_p to a numeric value such as 0.9"
                                                    .to_string(),
                                            ),
                                        );
                                    }
                                }
                            }

                            // OC-AG-006: Invalid color (extended validation with named colors)
                            // Only fires for values that look like named colors (no '#' prefix)
                            // but aren't in VALID_NAMED_COLORS. Hex validation is handled by OC-AG-002.
                            if config.is_rule_enabled("OC-AG-006") {
                                if let Some(color_val) = ag.get("color").and_then(|c| c.as_str()) {
                                    if !color_val.starts_with('#')
                                        && !VALID_NAMED_COLORS.contains(&color_val)
                                    {
                                        let line = find_key_line(content, "color").unwrap_or(1);
                                        let mut diagnostic = Diagnostic::warning(
                                            path.to_path_buf(),
                                            line,
                                            0,
                                            "OC-AG-006",
                                            format!(
                                                "Invalid named color '{}'. Use a hex color or one of: {}",
                                                color_val,
                                                VALID_NAMED_COLORS.join(", ")
                                            ),
                                        )
                                        .with_suggestion(format!(
                                            "Use a hex color like '#FF5733' or one of: {}",
                                            VALID_NAMED_COLORS.join(", ")
                                        ));

                                        if let Some(suggested) =
                                            find_closest_value(color_val, VALID_NAMED_COLORS)
                                        {
                                            if let Some((start, end)) =
                                                find_unique_json_string_value_span(
                                                    content, "color", color_val,
                                                )
                                            {
                                                diagnostic = diagnostic.with_fix(Fix::replace(
                                                    start,
                                                    end,
                                                    suggested,
                                                    format!("Replace color with '{}'", suggested),
                                                    false,
                                                ));
                                            }
                                        }

                                        diagnostics.push(diagnostic);
                                    }
                                }
                            }

                            // OC-AG-007: Both steps and maxSteps present
                            if config.is_rule_enabled("OC-AG-007") {
                                if ag.contains_key("steps") && ag.contains_key("maxSteps") {
                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            find_key_line(content, "maxSteps").unwrap_or(1),
                                            0,
                                            "OC-AG-007",
                                            "Both 'steps' and 'maxSteps' are set. Use only 'steps'"
                                                .to_string(),
                                        )
                                        .with_suggestion(
                                            "Remove 'maxSteps' and use 'steps' only".to_string(),
                                        ),
                                    );
                                }
                            }

                            // OC-AG-008: hidden must be boolean
                            if config.is_rule_enabled("OC-AG-008") {
                                if let Some(hidden) = ag.get("hidden") {
                                    if !hidden.is_boolean() && !hidden.is_null() {
                                        diagnostics.push(
                                            Diagnostic::error(
                                                path.to_path_buf(),
                                                find_key_line(content, "hidden").unwrap_or(1),
                                                0,
                                                "OC-AG-008",
                                                "Agent 'hidden' field must be a boolean"
                                                    .to_string(),
                                            )
                                            .with_suggestion(
                                                "Set hidden to true or false".to_string(),
                                            ),
                                        );
                                    }
                                }
                            }

                            // OC-AG-009: disable must be boolean
                            if config.is_rule_enabled("OC-AG-009") {
                                if let Some(disable_val) = ag.get("disable") {
                                    if !disable_val.is_boolean() && !disable_val.is_null() {
                                        diagnostics.push(
                                            Diagnostic::error(
                                                path.to_path_buf(),
                                                find_key_line(content, "disable").unwrap_or(1),
                                                0,
                                                "OC-AG-009",
                                                format!(
                                                    "Agent '{}' has invalid 'disable' type. Must be a boolean",
                                                    ag_name
                                                ),
                                            )
                                            .with_suggestion(
                                                "Set disable to true or false".to_string(),
                                            ),
                                        );
                                    }
                                }
                            }

                            // OC-DEP-006: maxSteps deprecated in favor of steps
                            if config.is_rule_enabled("OC-DEP-006") {
                                if ag.contains_key("maxSteps") && !ag.contains_key("steps") {
                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            find_key_line(content, "maxSteps").unwrap_or(1),
                                            0,
                                            "OC-DEP-006",
                                            format!(
                                                "Agent '{}' uses deprecated 'maxSteps'. Use 'steps' instead",
                                                ag_name
                                            ),
                                        )
                                        .with_suggestion(
                                            "Rename 'maxSteps' to 'steps'".to_string(),
                                        ),
                                    );
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
                            "codesearch",
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

                // OC-DEP-001/002/003: Deprecated top-level keys
                for &(old_key, new_key) in DEPRECATED_KEYS {
                    let rule_id = match old_key {
                        "mode" => "OC-DEP-001",
                        "tools" => "OC-DEP-002",
                        "autoshare" => "OC-DEP-003",
                        _ => continue,
                    };
                    if config.is_rule_enabled(rule_id) && obj.contains_key(old_key) {
                        let line = find_key_line(content, old_key).unwrap_or(1);
                        let mut diagnostic = Diagnostic::warning(
                            path.to_path_buf(),
                            line,
                            0,
                            rule_id,
                            format!("Deprecated key '{}'. Use '{}' instead", old_key, new_key),
                        )
                        .with_suggestion(format!("Rename '{}' to '{}'", old_key, new_key));

                        if let Some((start, end)) = find_json_key_span(content, old_key) {
                            diagnostic = diagnostic.with_fix(Fix::replace(
                                start,
                                end,
                                new_key,
                                format!("Rename '{}' to '{}'", old_key, new_key),
                                true,
                            ));
                        }

                        diagnostics.push(diagnostic);
                    }
                }

                // OC-DEP-004: CONTEXT.md deprecated filename in instructions
                if config.is_rule_enabled("OC-DEP-004") {
                    if let Some(instructions) = obj.get("instructions").and_then(|v| v.as_array()) {
                        for instr in instructions {
                            if let Some(instr_str) = instr.as_str() {
                                let instr_path = Path::new(instr_str);
                                if instr_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .is_some_and(|n| n.to_lowercase() == "context.md")
                                {
                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            find_key_line(content, "instructions").unwrap_or(1),
                                            0,
                                            "OC-DEP-004",
                                            format!(
                                                "CONTEXT.md is deprecated. Rename '{}' to use AGENTS.md instead",
                                                instr_str
                                            ),
                                        )
                                        .with_suggestion(
                                            "Rename CONTEXT.md references to AGENTS.md".to_string(),
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }

                // OC-DEP-005: Deprecated TUI keys in opencode.json
                if config.is_rule_enabled("OC-DEP-005") {
                    let deprecated_tui_keys = ["theme", "keybinds", "tui"];
                    for &tui_key in &deprecated_tui_keys {
                        if obj.contains_key(tui_key) {
                            diagnostics.push(
                                Diagnostic::warning(
                                    path.to_path_buf(),
                                    find_key_line(content, tui_key).unwrap_or(1),
                                    0,
                                    "OC-DEP-005",
                                    format!("Deprecated top-level key '{}'. TUI settings should be in a separate tui.json file", tui_key),
                                )
                                .with_suggestion(
                                    format!("Move '{}' to tui.json", tui_key),
                                ),
                            );
                        }
                    }
                }

                // OC-CFG-008: Invalid logLevel
                if config.is_rule_enabled("OC-CFG-008") {
                    if let Some(log_val) = obj.get("logLevel") {
                        if let Some(log_str) = log_val.as_str() {
                            let lower = log_str.to_lowercase();
                            if !VALID_LOG_LEVELS.contains(&lower.as_str()) {
                                let line = find_key_line(content, "logLevel").unwrap_or(1);
                                let mut diagnostic = Diagnostic::error(
                                    path.to_path_buf(),
                                    line,
                                    0,
                                    "OC-CFG-008",
                                    format!(
                                        "Invalid logLevel '{}'. Must be one of: {}",
                                        log_str,
                                        VALID_LOG_LEVELS.join(", ")
                                    ),
                                )
                                .with_suggestion(format!(
                                    "Set logLevel to one of: {}",
                                    VALID_LOG_LEVELS.join(", ")
                                ));

                                if let Some(suggested) =
                                    find_closest_value(&lower, VALID_LOG_LEVELS)
                                {
                                    if let Some((start, end)) = find_unique_json_string_value_span(
                                        content, "logLevel", log_str,
                                    ) {
                                        diagnostic = diagnostic.with_fix(Fix::replace(
                                            start,
                                            end,
                                            suggested,
                                            format!("Replace logLevel with '{}'", suggested),
                                            false,
                                        ));
                                    }
                                }

                                diagnostics.push(diagnostic);
                            }
                        } else if !log_val.is_null() {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    find_key_line(content, "logLevel").unwrap_or(1),
                                    0,
                                    "OC-CFG-008",
                                    "logLevel must be a string".to_string(),
                                )
                                .with_suggestion(format!(
                                    "Set logLevel to one of: {}",
                                    VALID_LOG_LEVELS.join(", ")
                                )),
                            );
                        }
                    }
                }

                // OC-CFG-009: Invalid compaction.reserved
                if config.is_rule_enabled("OC-CFG-009") {
                    if let Some(compaction) = obj.get("compaction").and_then(|c| c.as_object()) {
                        if let Some(reserved) = compaction.get("reserved") {
                            if let Some(val) = reserved.as_i64() {
                                if val < 0 {
                                    diagnostics.push(
                                        Diagnostic::error(
                                            path.to_path_buf(),
                                            find_key_line(content, "reserved").unwrap_or(1),
                                            0,
                                            "OC-CFG-009",
                                            format!(
                                                "compaction.reserved must be >= 0, got {}",
                                                val
                                            ),
                                        )
                                        .with_suggestion(
                                            "Set reserved to a non-negative integer".to_string(),
                                        ),
                                    );
                                }
                            } else if !reserved.is_null() {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        find_key_line(content, "reserved").unwrap_or(1),
                                        0,
                                        "OC-CFG-009",
                                        "compaction.reserved must be an integer >= 0".to_string(),
                                    )
                                    .with_suggestion(
                                        "Set reserved to a non-negative integer".to_string(),
                                    ),
                                );
                            }
                        }
                    }
                }

                // OC-CFG-010: Invalid skills.urls
                if config.is_rule_enabled("OC-CFG-010") {
                    if let Some(skills) = obj.get("skills").and_then(|s| s.as_object()) {
                        if let Some(urls_val) = skills.get("urls") {
                            if let Some(urls) = urls_val.as_array() {
                                for url_val in urls {
                                    if let Some(url_str) = url_val.as_str() {
                                        if !url_str.starts_with("http://")
                                            && !url_str.starts_with("https://")
                                        {
                                            diagnostics.push(
                                                Diagnostic::error(
                                                    path.to_path_buf(),
                                                    find_key_line(content, "urls").unwrap_or(1),
                                                    0,
                                                    "OC-CFG-010",
                                                    format!(
                                                        "Invalid skills URL '{}'. Must start with http:// or https://",
                                                        truncate_for_display(url_str, 200)
                                                    ),
                                                )
                                                .with_suggestion(
                                                    "Use a full URL starting with http:// or https://"
                                                        .to_string(),
                                                ),
                                            );
                                        }
                                    } else {
                                        diagnostics.push(
                                            Diagnostic::error(
                                                path.to_path_buf(),
                                                find_key_line(content, "urls").unwrap_or(1),
                                                0,
                                                "OC-CFG-010",
                                                "skills.urls entries must be strings".to_string(),
                                            )
                                            .with_suggestion(
                                                "Each entry in skills.urls must be a URL string"
                                                    .to_string(),
                                            ),
                                        );
                                    }
                                }
                            } else {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        find_key_line(content, "urls").unwrap_or(1),
                                        0,
                                        "OC-CFG-010",
                                        "skills.urls must be an array".to_string(),
                                    )
                                    .with_suggestion(
                                        "Set skills.urls to an array of URL strings".to_string(),
                                    ),
                                );
                            }
                        }
                    }
                }

                // OC-CFG-011: MCP timeout must be positive integer
                if config.is_rule_enabled("OC-CFG-011") {
                    if let Some(mcp_obj) = obj.get("mcp").and_then(|m| m.as_object()) {
                        for (srv_name, srv_val) in mcp_obj {
                            if let Some(srv) = srv_val.as_object() {
                                if let Some(timeout) = srv.get("timeout") {
                                    if let Some(val) = timeout.as_i64() {
                                        if val <= 0 {
                                            diagnostics.push(
                                                Diagnostic::error(
                                                    path.to_path_buf(),
                                                    find_key_line(content, srv_name)
                                                        .unwrap_or(1),
                                                    0,
                                                    "OC-CFG-011",
                                                    format!(
                                                        "MCP server '{}' timeout must be a positive integer, got {}",
                                                        srv_name, val
                                                    ),
                                                )
                                                .with_suggestion(
                                                    "Set timeout to a positive integer (milliseconds)"
                                                        .to_string(),
                                                ),
                                            );
                                        }
                                    } else if !timeout.is_null() {
                                        diagnostics.push(
                                            Diagnostic::error(
                                                path.to_path_buf(),
                                                find_key_line(content, srv_name).unwrap_or(1),
                                                0,
                                                "OC-CFG-011",
                                                format!(
                                                    "MCP server '{}' timeout must be a positive integer",
                                                    srv_name
                                                ),
                                            )
                                            .with_suggestion(
                                                "Set timeout to a positive integer (milliseconds)"
                                                    .to_string(),
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                // OC-CFG-012: MCP OAuth validation
                if config.is_rule_enabled("OC-CFG-012") {
                    if let Some(mcp_obj) = obj.get("mcp").and_then(|m| m.as_object()) {
                        for (srv_name, srv_val) in mcp_obj {
                            if let Some(srv) = srv_val.as_object() {
                                if let Some(oauth) = srv.get("oauth") {
                                    if let Some(oauth_obj) = oauth.as_object() {
                                        let has_client_id = oauth_obj.contains_key("client_id");
                                        let has_auth_url =
                                            oauth_obj.contains_key("authorization_url");
                                        if !has_client_id || !has_auth_url {
                                            let missing: Vec<&str> = [
                                                (!has_client_id).then_some("client_id"),
                                                (!has_auth_url).then_some("authorization_url"),
                                            ]
                                            .into_iter()
                                            .flatten()
                                            .collect();
                                            diagnostics.push(
                                                Diagnostic::error(
                                                    path.to_path_buf(),
                                                    find_key_line(content, srv_name)
                                                        .unwrap_or(1),
                                                    0,
                                                    "OC-CFG-012",
                                                    format!(
                                                        "MCP server '{}' oauth missing required fields: {}",
                                                        truncate_for_display(srv_name, 200),
                                                        missing.join(", ")
                                                    ),
                                                )
                                                .with_suggestion(
                                                    "Add client_id and authorization_url to oauth config"
                                                        .to_string(),
                                                ),
                                            );
                                        }
                                    } else if !oauth.is_null() {
                                        diagnostics.push(
                                            Diagnostic::error(
                                                path.to_path_buf(),
                                                find_key_line(content, srv_name).unwrap_or(1),
                                                0,
                                                "OC-CFG-012",
                                                format!(
                                                    "MCP server '{}' oauth must be an object",
                                                    srv_name
                                                ),
                                            )
                                            .with_suggestion(
                                                "Set oauth to an object with client_id and authorization_url"
                                                    .to_string(),
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                // OC-CFG-013: Invalid server config
                if config.is_rule_enabled("OC-CFG-013") {
                    if let Some(server_val) = obj.get("server") {
                        if let Some(server_obj) = server_val.as_object() {
                            let server_line = find_key_line(content, "server").unwrap_or(1);
                            if let Some(port_val) = server_obj.get("port") {
                                if port_val.as_i64().is_none()
                                    && port_val.as_f64().is_none()
                                    && !port_val.is_null()
                                {
                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            find_key_line(content, "port").unwrap_or(server_line),
                                            0,
                                            "OC-CFG-013",
                                            "Invalid server.port type. Expected a number"
                                                .to_string(),
                                        )
                                        .with_suggestion(
                                            "Set port to a number such as 8080".to_string(),
                                        ),
                                    );
                                }
                            }
                            if let Some(hostname_val) = server_obj.get("hostname") {
                                if !hostname_val.is_string() && !hostname_val.is_null() {
                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            find_key_line(content, "hostname")
                                                .unwrap_or(server_line),
                                            0,
                                            "OC-CFG-013",
                                            "Invalid server.hostname type. Expected a string"
                                                .to_string(),
                                        )
                                        .with_suggestion(
                                            "Set hostname to a string such as \"localhost\""
                                                .to_string(),
                                        ),
                                    );
                                }
                            }
                            if let Some(mdns_val) = server_obj.get("mdns") {
                                if !mdns_val.is_boolean() && !mdns_val.is_null() {
                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            find_key_line(content, "mdns").unwrap_or(server_line),
                                            0,
                                            "OC-CFG-013",
                                            "Invalid server.mdns type. Expected a boolean"
                                                .to_string(),
                                        )
                                        .with_suggestion("Set mdns to true or false".to_string()),
                                    );
                                }
                            }
                            if let Some(cors_val) = server_obj.get("cors") {
                                if !cors_val.is_array() && !cors_val.is_null() {
                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            find_key_line(content, "cors").unwrap_or(server_line),
                                            0,
                                            "OC-CFG-013",
                                            "Invalid server.cors type. Expected an array"
                                                .to_string(),
                                        )
                                        .with_suggestion(
                                            "Set cors to an array of allowed origins".to_string(),
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }

                // OC-LSP-001: LSP entry with command but no extensions
                if config.is_rule_enabled("OC-LSP-001") {
                    if let Some(lsp_obj) = obj.get("lsp").and_then(|l| l.as_object()) {
                        for (lsp_name, lsp_val) in lsp_obj {
                            if let Some(lsp) = lsp_val.as_object() {
                                if lsp.contains_key("command") && !lsp.contains_key("extensions") {
                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            find_key_line(content, lsp_name).unwrap_or(1),
                                            0,
                                            "OC-LSP-001",
                                            format!(
                                                "LSP server '{}' has 'command' but no 'extensions'",
                                                lsp_name
                                            ),
                                        )
                                        .with_suggestion(
                                            "Add 'extensions' array to specify file extensions"
                                                .to_string(),
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }

                // OC-LSP-002: LSP entry with empty or invalid extensions
                if config.is_rule_enabled("OC-LSP-002") {
                    if let Some(lsp_obj) = obj.get("lsp").and_then(|l| l.as_object()) {
                        for (lsp_name, lsp_val) in lsp_obj {
                            if let Some(lsp) = lsp_val.as_object() {
                                if let Some(ext_val) = lsp.get("extensions") {
                                    let is_invalid = if let Some(arr) = ext_val.as_array() {
                                        arr.is_empty() || arr.iter().any(|v| !v.is_string())
                                    } else {
                                        true
                                    };
                                    if is_invalid {
                                        diagnostics.push(
                                            Diagnostic::error(
                                                path.to_path_buf(),
                                                find_key_line(content, lsp_name).unwrap_or(1),
                                                0,
                                                "OC-LSP-002",
                                                format!(
                                                    "LSP server '{}' extensions must be a non-empty array of strings",
                                                    lsp_name
                                                ),
                                            )
                                            .with_suggestion(
                                                "Set extensions to an array like [\".ts\", \".js\"]"
                                                    .to_string(),
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                // OC-TUI-001: Unknown TUI keys
                if config.is_rule_enabled("OC-TUI-001") {
                    if let Some(tui_obj) = obj.get("tui").and_then(|t| t.as_object()) {
                        for key in tui_obj.keys() {
                            if !KNOWN_TUI_KEYS.contains(&key.as_str()) {
                                diagnostics.push(
                                    Diagnostic::warning(
                                        path.to_path_buf(),
                                        find_key_line(content, key).unwrap_or(1),
                                        0,
                                        "OC-TUI-001",
                                        format!("Unknown TUI key '{}'", key),
                                    )
                                    .with_suggestion(format!(
                                        "Valid TUI keys: {}",
                                        KNOWN_TUI_KEYS.join(", ")
                                    )),
                                );
                            }
                        }
                    }
                }

                // OC-TUI-002: Invalid scroll_speed
                if config.is_rule_enabled("OC-TUI-002") {
                    if let Some(tui_obj) = obj.get("tui").and_then(|t| t.as_object()) {
                        if let Some(speed) = tui_obj.get("scroll_speed") {
                            if let Some(val) = speed.as_f64() {
                                if val < 0.001 {
                                    diagnostics.push(
                                        Diagnostic::error(
                                            path.to_path_buf(),
                                            find_key_line(content, "scroll_speed").unwrap_or(1),
                                            0,
                                            "OC-TUI-002",
                                            format!("scroll_speed must be >= 0.001, got {}", val),
                                        )
                                        .with_suggestion(
                                            "Set scroll_speed to a number >= 0.001".to_string(),
                                        ),
                                    );
                                }
                            } else if !speed.is_null() {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        find_key_line(content, "scroll_speed").unwrap_or(1),
                                        0,
                                        "OC-TUI-002",
                                        "scroll_speed must be a number >= 0.001".to_string(),
                                    )
                                    .with_suggestion(
                                        "Set scroll_speed to a number >= 0.001".to_string(),
                                    ),
                                );
                            }
                        }
                    }
                }

                // OC-TUI-003: Invalid diff_style
                if config.is_rule_enabled("OC-TUI-003") {
                    if let Some(tui_obj) = obj.get("tui").and_then(|t| t.as_object()) {
                        if let Some(style_val) = tui_obj.get("diff_style") {
                            if let Some(style_str) = style_val.as_str() {
                                if !VALID_DIFF_STYLES.contains(&style_str) {
                                    let line = find_key_line(content, "diff_style").unwrap_or(1);
                                    let mut diagnostic = Diagnostic::error(
                                        path.to_path_buf(),
                                        line,
                                        0,
                                        "OC-TUI-003",
                                        format!(
                                            "Invalid diff_style '{}'. Must be one of: {}",
                                            style_str,
                                            VALID_DIFF_STYLES.join(", ")
                                        ),
                                    )
                                    .with_suggestion(format!(
                                        "Set diff_style to one of: {}",
                                        VALID_DIFF_STYLES.join(", ")
                                    ));

                                    if let Some(suggested) =
                                        find_closest_value(style_str, VALID_DIFF_STYLES)
                                    {
                                        if let Some((start, end)) =
                                            find_unique_json_string_value_span(
                                                content,
                                                "diff_style",
                                                style_str,
                                            )
                                        {
                                            diagnostic = diagnostic.with_fix(Fix::replace(
                                                start,
                                                end,
                                                suggested,
                                                format!("Replace diff_style with '{}'", suggested),
                                                false,
                                            ));
                                        }
                                    }

                                    diagnostics.push(diagnostic);
                                }
                            } else if !style_val.is_null() {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        find_key_line(content, "diff_style").unwrap_or(1),
                                        0,
                                        "OC-TUI-003",
                                        "diff_style must be a string".to_string(),
                                    )
                                    .with_suggestion(format!(
                                        "Set diff_style to one of: {}",
                                        VALID_DIFF_STYLES.join(", ")
                                    )),
                                );
                            }
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

/// Find the byte span of a JSON key string (the text between quotes).
/// Returns (start, end) byte offsets of the key name (excluding quotes).
///
/// Uses a loop to skip false positives where the pattern appears inside
/// string values rather than as an object key. A match is only accepted
/// when the next non-whitespace character after the closing quote is `:`.
///
/// Limitation: when the same key appears in nested objects, this returns
/// the first occurrence.
fn find_json_key_span(content: &str, key: &str) -> Option<(usize, usize)> {
    let pattern = format!("\"{}\"", key);
    let mut search_from = 0;
    let mut found = None;
    let mut count = 0;
    while let Some(pos) = content[search_from..].find(&pattern) {
        let abs_pos = search_from + pos;
        let after_quote = abs_pos + pattern.len();
        if let Some(next_char) = content[after_quote..].trim_start().chars().next() {
            if next_char == ':' {
                count += 1;
                if found.is_none() {
                    found = Some((abs_pos + 1, abs_pos + 1 + key.len()));
                }
            }
        }
        search_from = abs_pos + pattern.len();
    }
    // Only return span if key appears exactly once as a key (safe to autofix)
    if count == 1 { found } else { None }
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

/// Truncate a string for display in diagnostic messages.
/// Appends "..." if the string exceeds `max` bytes.
fn truncate_for_display(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    // Find the last valid char boundary at or before max
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
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
    fn test_oc_ag_002_valid_named_color() {
        // VALID_NAMED_COLORS like "primary", "secondary" must not trigger OC-AG-002
        let diagnostics = validate(r#"{"agent": {"a": {"color": "primary"}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-002"));
        let diagnostics = validate(r#"{"agent": {"a": {"color": "secondary"}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-002"));
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

    // ===== OC-DEP-001: Deprecated mode key =====

    #[test]
    fn test_oc_dep_001_deprecated_mode() {
        let diagnostics = validate(r#"{"mode": "agent"}"#);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-001")
            .collect();
        assert_eq!(dep.len(), 1);
        assert_eq!(dep[0].level, DiagnosticLevel::Warning);
        assert!(dep[0].message.contains("mode"));
        assert!(dep[0].message.contains("agent"));
    }

    #[test]
    fn test_oc_dep_001_autofix() {
        let content = r#"{"mode": "agent"}"#;
        let diagnostics = validate(content);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-001")
            .collect();
        assert_eq!(dep.len(), 1);
        assert!(dep[0].has_fixes(), "OC-DEP-001 should have auto-fix");
        let fix = &dep[0].fixes[0];
        assert!(fix.safe, "OC-DEP-001 fix should be safe");
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "mode");
        assert_eq!(fix.replacement, "agent");
    }

    #[test]
    fn test_oc_dep_001_no_fire_when_absent() {
        let diagnostics = validate(r#"{"agent": {}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-DEP-001"));
    }

    // ===== OC-DEP-002: Deprecated tools key =====

    #[test]
    fn test_oc_dep_002_deprecated_tools() {
        let diagnostics = validate(r#"{"tools": {}}"#);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-002")
            .collect();
        assert_eq!(dep.len(), 1);
        assert!(dep[0].message.contains("tools"));
    }

    #[test]
    fn test_oc_dep_002_autofix() {
        let content = r#"{"tools": {}}"#;
        let diagnostics = validate(content);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-002")
            .collect();
        assert!(dep[0].has_fixes());
        let fix = &dep[0].fixes[0];
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "tools");
        assert_eq!(fix.replacement, "permission");
    }

    // ===== OC-DEP-003: Deprecated autoshare key =====

    #[test]
    fn test_oc_dep_003_deprecated_autoshare() {
        let diagnostics = validate(r#"{"autoshare": "manual"}"#);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-003")
            .collect();
        assert_eq!(dep.len(), 1);
        assert!(dep[0].message.contains("autoshare"));
    }

    #[test]
    fn test_oc_dep_003_autofix() {
        let content = r#"{"autoshare": "manual"}"#;
        let diagnostics = validate(content);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-003")
            .collect();
        assert!(dep[0].has_fixes());
        let fix = &dep[0].fixes[0];
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "autoshare");
        assert_eq!(fix.replacement, "share");
    }

    // ===== OC-DEP-004: CONTEXT.md deprecated =====

    #[test]
    fn test_oc_dep_004_context_md() {
        let diagnostics = validate(r#"{"instructions": ["CONTEXT.md"]}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-DEP-004"));
    }

    #[test]
    fn test_oc_dep_004_context_md_nested_path() {
        let diagnostics = validate(r#"{"instructions": ["docs/CONTEXT.md"]}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-DEP-004"));
    }

    #[test]
    fn test_oc_dep_004_not_context_md() {
        let diagnostics = validate(r#"{"instructions": ["AGENTS.md"]}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-DEP-004"));
    }

    #[test]
    fn test_oc_dep_004_no_instructions() {
        let diagnostics = validate(r#"{}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-DEP-004"));
    }

    // ===== OC-CFG-008: Invalid logLevel =====

    #[test]
    fn test_oc_cfg_008_invalid_loglevel() {
        let diagnostics = validate(r#"{"logLevel": "verbose"}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-008"));
    }

    #[test]
    fn test_oc_cfg_008_valid_loglevel() {
        let diagnostics = validate(r#"{"logLevel": "info"}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-008"));
    }

    #[test]
    fn test_oc_cfg_008_case_insensitive() {
        // "INFO" normalized to "info" should be valid
        let diagnostics = validate(r#"{"logLevel": "INFO"}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-008"));
    }

    #[test]
    fn test_oc_cfg_008_type_error() {
        let diagnostics = validate(r#"{"logLevel": 42}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-008"));
    }

    #[test]
    fn test_oc_cfg_008_autofix() {
        let content = r#"{"logLevel": "debu"}"#;
        let diagnostics = validate(content);
        let cfg: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-008")
            .collect();
        assert_eq!(cfg.len(), 1);
        assert!(
            cfg[0].has_fixes(),
            "OC-CFG-008 should have auto-fix for close match"
        );
        let fix = &cfg[0].fixes[0];
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "debu");
        assert_eq!(fix.replacement, "debug");
    }

    // ===== OC-CFG-009: Invalid compaction.reserved =====

    #[test]
    fn test_oc_cfg_009_negative_reserved() {
        let diagnostics = validate(r#"{"compaction": {"reserved": -1}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-009"));
    }

    #[test]
    fn test_oc_cfg_009_valid_reserved() {
        let diagnostics = validate(r#"{"compaction": {"reserved": 5}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-009"));
    }

    #[test]
    fn test_oc_cfg_009_zero_reserved() {
        let diagnostics = validate(r#"{"compaction": {"reserved": 0}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-009"));
    }

    #[test]
    fn test_oc_cfg_009_type_error() {
        let diagnostics = validate(r#"{"compaction": {"reserved": "five"}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-009"));
    }

    // ===== OC-CFG-010: Invalid skills.urls =====

    #[test]
    fn test_oc_cfg_010_invalid_url() {
        let diagnostics = validate(r#"{"skills": {"urls": ["not-a-url"]}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-010"));
    }

    #[test]
    fn test_oc_cfg_010_valid_url() {
        let diagnostics = validate(r#"{"skills": {"urls": ["https://example.com"]}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-010"));
    }

    // ===== OC-CFG-011: MCP timeout =====

    #[test]
    fn test_oc_cfg_011_negative_timeout() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"timeout": -5}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-011"));
    }

    #[test]
    fn test_oc_cfg_011_valid_timeout() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"timeout": 5000}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-011"));
    }

    #[test]
    fn test_oc_cfg_011_type_error() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"timeout": "slow"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-011"));
    }

    #[test]
    fn test_oc_cfg_011_zero_timeout() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"timeout": 0}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-011"));
    }

    // ===== OC-CFG-012: MCP OAuth =====

    #[test]
    fn test_oc_cfg_012_missing_fields() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"oauth": {"client_id": "abc"}}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-012"));
    }

    #[test]
    fn test_oc_cfg_012_valid_oauth() {
        let diagnostics = validate(
            r#"{"mcp": {"srv": {"oauth": {"client_id": "abc", "authorization_url": "https://auth.example.com"}}}}"#,
        );
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-012"));
    }

    #[test]
    fn test_oc_cfg_012_type_error() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"oauth": "invalid"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-012"));
    }

    // ===== OC-AG-005: top_p out of range =====

    #[test]
    fn test_oc_ag_005_invalid_top_p() {
        let diagnostics = validate(r#"{"agent": {"a": {"top_p": 1.5}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-005"));
    }

    #[test]
    fn test_oc_ag_005_valid_top_p() {
        let diagnostics = validate(r#"{"agent": {"a": {"top_p": 0.9}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-005"));
    }

    #[test]
    fn test_oc_ag_005_boundary() {
        let diagnostics = validate(r#"{"agent": {"a": {"top_p": 1.0}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-005"));
    }

    #[test]
    fn test_oc_ag_005_type_error() {
        let diagnostics = validate(r#"{"agent": {"a": {"top_p": "high"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-005"));
    }

    #[test]
    fn test_oc_ag_005_boundary_zero() {
        let diagnostics = validate(r#"{"agent": {"a": {"top_p": 0.0}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-005"));
    }

    #[test]
    fn test_oc_ag_005_negative() {
        let diagnostics = validate(r#"{"agent": {"a": {"top_p": -0.1}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-005"));
    }

    // ===== OC-AG-006: Invalid named color =====

    #[test]
    fn test_oc_ag_006_invalid_named_color() {
        let diagnostics = validate(r#"{"agent": {"a": {"color": "purple"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-006"));
    }

    #[test]
    fn test_oc_ag_006_valid_named_color() {
        let diagnostics = validate(r#"{"agent": {"a": {"color": "primary"}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-006"));
    }

    #[test]
    fn test_oc_ag_006_valid_hex_color() {
        let diagnostics = validate(r##"{"agent": {"a": {"color": "#FF5733"}}}"##);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-006"));
    }

    #[test]
    fn test_oc_ag_006_autofix() {
        let content = r#"{"agent": {"a": {"color": "erro"}}}"#;
        let diagnostics = validate(content);
        let ag: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-AG-006")
            .collect();
        assert_eq!(ag.len(), 1);
        assert!(
            ag[0].has_fixes(),
            "OC-AG-006 should have auto-fix for close match"
        );
        let fix = &ag[0].fixes[0];
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "erro");
        assert_eq!(fix.replacement, "error");
    }

    // ===== OC-AG-007: Both steps and maxSteps =====

    #[test]
    fn test_oc_ag_007_redundant_steps() {
        let diagnostics = validate(r#"{"agent": {"a": {"steps": 10, "maxSteps": 20}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-007"));
    }

    #[test]
    fn test_oc_ag_007_only_steps() {
        let diagnostics = validate(r#"{"agent": {"a": {"steps": 10}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-007"));
    }

    // ===== OC-AG-008: hidden must be boolean =====

    #[test]
    fn test_oc_ag_008_invalid_hidden() {
        let diagnostics = validate(r#"{"agent": {"a": {"hidden": "yes"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-008"));
    }

    #[test]
    fn test_oc_ag_008_valid_hidden() {
        let diagnostics = validate(r#"{"agent": {"a": {"hidden": true}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-008"));
    }

    // ===== OC-LSP-001: command without extensions =====

    #[test]
    fn test_oc_lsp_001_missing_extensions() {
        let diagnostics = validate(r#"{"lsp": {"ts": {"command": "typescript-language-server"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-LSP-001"));
    }

    #[test]
    fn test_oc_lsp_001_with_extensions() {
        let diagnostics = validate(
            r#"{"lsp": {"ts": {"command": "typescript-language-server", "extensions": [".ts"]}}}"#,
        );
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-LSP-001"));
    }

    // ===== OC-LSP-002: invalid extensions =====

    #[test]
    fn test_oc_lsp_002_empty_extensions() {
        let diagnostics = validate(r#"{"lsp": {"ts": {"extensions": []}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-LSP-002"));
    }

    #[test]
    fn test_oc_lsp_002_non_string_extensions() {
        let diagnostics = validate(r#"{"lsp": {"ts": {"extensions": [1, 2]}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-LSP-002"));
    }

    #[test]
    fn test_oc_lsp_002_not_array() {
        let diagnostics = validate(r#"{"lsp": {"ts": {"extensions": "ts"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-LSP-002"));
    }

    #[test]
    fn test_oc_lsp_002_null_extensions() {
        let diagnostics = validate(r#"{"lsp": {"ts": {"extensions": null}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-LSP-002"));
    }

    // ===== OC-TUI-001: Unknown TUI keys =====

    #[test]
    fn test_oc_tui_001_unknown_key() {
        let diagnostics = validate(r#"{"tui": {"unknown_opt": true}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-TUI-001"));
    }

    #[test]
    fn test_oc_tui_001_known_keys() {
        let diagnostics = validate(r#"{"tui": {"theme": "dark", "scroll_speed": 1.0}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-TUI-001"));
    }

    // ===== OC-TUI-002: Invalid scroll_speed =====

    #[test]
    fn test_oc_tui_002_too_small() {
        let diagnostics = validate(r#"{"tui": {"scroll_speed": 0.0001}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-TUI-002"));
    }

    #[test]
    fn test_oc_tui_002_valid() {
        let diagnostics = validate(r#"{"tui": {"scroll_speed": 1.0}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-TUI-002"));
    }

    #[test]
    fn test_oc_tui_002_type_error() {
        let diagnostics = validate(r#"{"tui": {"scroll_speed": "fast"}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-TUI-002"));
    }

    #[test]
    fn test_oc_tui_002_boundary() {
        let diagnostics = validate(r#"{"tui": {"scroll_speed": 0.001}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-TUI-002"));
    }

    // ===== OC-TUI-003: Invalid diff_style =====

    #[test]
    fn test_oc_tui_003_invalid() {
        let diagnostics = validate(r#"{"tui": {"diff_style": "unified"}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-TUI-003"));
    }

    #[test]
    fn test_oc_tui_003_valid() {
        let diagnostics = validate(r#"{"tui": {"diff_style": "auto"}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-TUI-003"));
    }

    #[test]
    fn test_oc_tui_003_autofix() {
        let content = r#"{"tui": {"diff_style": "stack"}}"#;
        let diagnostics = validate(content);
        let tui: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-TUI-003")
            .collect();
        assert_eq!(tui.len(), 1);
        assert!(
            tui[0].has_fixes(),
            "OC-TUI-003 should have auto-fix for close match"
        );
        let fix = &tui[0].fixes[0];
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "stack");
        assert_eq!(fix.replacement, "stacked");
    }

    // ===== find_json_key_span =====

    #[test]
    fn test_find_json_key_span() {
        let content = r#"{"mode": "agent"}"#;
        let span = find_json_key_span(content, "mode");
        assert!(span.is_some());
        let (start, end) = span.unwrap();
        assert_eq!(&content[start..end], "mode");
    }

    #[test]
    fn test_find_json_key_span_not_found() {
        let content = r#"{"share": "manual"}"#;
        assert!(find_json_key_span(content, "mode").is_none());
    }

    #[test]
    fn test_find_json_key_span_value_not_key() {
        // "mode" appears only as a value, not as a key
        let content = r#"{"comment": "mode"}"#;
        assert!(find_json_key_span(content, "mode").is_none());
    }

    // ===== Fix #3: OC-CFG-010 http:// URL should NOT fire =====

    #[test]
    fn test_oc_cfg_010_http_url_valid() {
        let diagnostics = validate(r#"{"skills": {"urls": ["http://example.com"]}}"#);
        assert!(
            !diagnostics.iter().any(|d| d.rule == "OC-CFG-010"),
            "http:// URLs should be valid for skills.urls"
        );
    }

    // ===== Fix #4: OC-CFG-012 empty oauth object =====

    #[test]
    fn test_oc_cfg_012_empty_oauth() {
        let diagnostics =
            validate(r#"{"mcp": {"srv": {"type": "remote", "url": "http://x", "oauth": {}}}}"#);
        let cfg_012: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-012")
            .collect();
        assert_eq!(
            cfg_012.len(),
            1,
            "Empty oauth object should fire OC-CFG-012 for missing required fields"
        );
    }

    // ===== Fix #5: OC-AG-007 only maxSteps (no steps) =====

    #[test]
    fn test_oc_ag_007_only_max_steps() {
        let diagnostics = validate(r#"{"agent": {"a": {"maxSteps": 20}}}"#);
        assert!(
            !diagnostics.iter().any(|d| d.rule == "OC-AG-007"),
            "Only maxSteps without steps should NOT fire OC-AG-007"
        );
    }

    // ===== Fix #6: DEP-002/DEP-003 negative tests =====

    #[test]
    fn test_oc_dep_002_no_fire_when_absent() {
        let diagnostics = validate(r#"{"permission": "allow"}"#);
        assert!(
            !diagnostics.iter().any(|d| d.rule == "OC-DEP-002"),
            "OC-DEP-002 should not fire when 'tools' key is absent"
        );
    }

    #[test]
    fn test_oc_dep_003_no_fire_when_absent() {
        let diagnostics = validate(r#"{"share": "manual"}"#);
        assert!(
            !diagnostics.iter().any(|d| d.rule == "OC-DEP-003"),
            "OC-DEP-003 should not fire when 'autoshare' key is absent"
        );
    }

    // ===== Fix #7: OC-TUI-003 type error =====

    #[test]
    fn test_oc_tui_003_type_error() {
        let diagnostics = validate(r#"{"tui": {"diff_style": 42}}"#);
        assert!(
            diagnostics.iter().any(|d| d.rule == "OC-TUI-003"),
            "Non-string diff_style should fire OC-TUI-003"
        );
    }

    // ===== Fix #8: find_json_key_span nested key =====

    #[test]
    fn test_find_json_key_span_nested_returns_none() {
        // When the same key appears at multiple nesting levels,
        // find_json_key_span returns None to prevent unsafe autofixes.
        let content = r#"{"name": "outer", "nested": {"name": "inner"}}"#;
        let span = find_json_key_span(content, "name");
        assert!(
            span.is_none(),
            "Should return None when key appears multiple times"
        );
    }

    // ===== Fix #11: truncate_for_display =====

    #[test]
    fn test_truncate_for_display_short() {
        assert_eq!(truncate_for_display("hello", 200), "hello");
    }

    #[test]
    fn test_truncate_for_display_long() {
        let long = "x".repeat(300);
        let result = truncate_for_display(&long, 200);
        assert_eq!(result.len(), 203); // 200 + "..."
        assert!(result.ends_with("..."));
    }

    // ===== Fix #12: DEP-004 case-insensitive =====

    #[test]
    fn test_oc_dep_004_case_insensitive() {
        let diagnostics = validate(r#"{"instructions": ["context.md"]}"#);
        assert!(
            diagnostics.iter().any(|d| d.rule == "OC-DEP-004"),
            "OC-DEP-004 should fire for case-insensitive match on context.md"
        );
    }

    // ===== OC-CFG-013: Invalid server config =====

    #[test]
    fn test_oc_cfg_013_invalid_port_type() {
        let diagnostics = validate(r#"{"server": {"port": "8080"}}"#);
        let cfg: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-013")
            .collect();
        assert_eq!(cfg.len(), 1);
        assert_eq!(cfg[0].level, DiagnosticLevel::Warning);
        assert!(cfg[0].message.contains("port"));
    }

    #[test]
    fn test_oc_cfg_013_valid_port() {
        let diagnostics = validate(r#"{"server": {"port": 8080}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-013"));
    }

    #[test]
    fn test_oc_cfg_013_invalid_hostname_type() {
        let diagnostics = validate(r#"{"server": {"hostname": 123}}"#);
        let cfg: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-013")
            .collect();
        assert_eq!(cfg.len(), 1);
        assert!(cfg[0].message.contains("hostname"));
    }

    #[test]
    fn test_oc_cfg_013_valid_hostname() {
        let diagnostics = validate(r#"{"server": {"hostname": "localhost"}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-013"));
    }

    #[test]
    fn test_oc_cfg_013_invalid_mdns_type() {
        let diagnostics = validate(r#"{"server": {"mdns": "yes"}}"#);
        let cfg: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-013")
            .collect();
        assert_eq!(cfg.len(), 1);
        assert!(cfg[0].message.contains("mdns"));
    }

    #[test]
    fn test_oc_cfg_013_valid_mdns() {
        let diagnostics = validate(r#"{"server": {"mdns": true}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-013"));
    }

    #[test]
    fn test_oc_cfg_013_invalid_cors_type() {
        let diagnostics = validate(r#"{"server": {"cors": "all"}}"#);
        let cfg: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-013")
            .collect();
        assert_eq!(cfg.len(), 1);
        assert!(cfg[0].message.contains("cors"));
    }

    #[test]
    fn test_oc_cfg_013_valid_cors() {
        let diagnostics = validate(r#"{"server": {"cors": ["http://localhost:3000"]}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-013"));
    }

    #[test]
    fn test_oc_cfg_013_multiple_invalid_fields() {
        let diagnostics = validate(r#"{"server": {"port": "bad", "mdns": 1}}"#);
        let cfg: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-013")
            .collect();
        assert_eq!(cfg.len(), 2);
    }

    // ===== OC-AG-009: Invalid agent disable type =====

    #[test]
    fn test_oc_ag_009_invalid_disable_type() {
        let diagnostics = validate(r#"{"agent": {"a": {"disable": "yes"}}}"#);
        let ag: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-AG-009")
            .collect();
        assert_eq!(ag.len(), 1);
        assert_eq!(ag[0].level, DiagnosticLevel::Error);
        assert!(ag[0].message.contains("disable"));
    }

    #[test]
    fn test_oc_ag_009_valid_disable_true() {
        let diagnostics = validate(r#"{"agent": {"a": {"disable": true}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-009"));
    }

    #[test]
    fn test_oc_ag_009_valid_disable_false() {
        let diagnostics = validate(r#"{"agent": {"a": {"disable": false}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-009"));
    }

    #[test]
    fn test_oc_ag_009_disable_number() {
        let diagnostics = validate(r#"{"agent": {"a": {"disable": 1}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-009"));
    }

    // ===== OC-DEP-005: Deprecated TUI keys in opencode.json =====

    #[test]
    fn test_oc_dep_005_theme_deprecated() {
        let diagnostics = validate(r#"{"theme": "dark"}"#);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-005")
            .collect();
        assert_eq!(dep.len(), 1);
        assert_eq!(dep[0].level, DiagnosticLevel::Warning);
        assert!(dep[0].message.contains("theme"));
    }

    #[test]
    fn test_oc_dep_005_keybinds_deprecated() {
        let diagnostics = validate(r#"{"keybinds": {}}"#);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-005")
            .collect();
        assert_eq!(dep.len(), 1);
        assert!(dep[0].message.contains("keybinds"));
    }

    #[test]
    fn test_oc_dep_005_tui_deprecated() {
        let diagnostics = validate(r#"{"tui": {"scroll_speed": 1.0}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-DEP-005"));
    }

    #[test]
    fn test_oc_dep_005_no_fire_when_absent() {
        let diagnostics = validate(r#"{"share": "manual"}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-DEP-005"));
    }

    // ===== OC-DEP-006: Deprecated maxSteps =====

    #[test]
    fn test_oc_dep_006_max_steps_deprecated() {
        let diagnostics = validate(r#"{"agent": {"a": {"maxSteps": 20}}}"#);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-006")
            .collect();
        assert_eq!(dep.len(), 1);
        assert_eq!(dep[0].level, DiagnosticLevel::Warning);
        assert!(dep[0].message.contains("maxSteps"));
        assert!(dep[0].message.contains("steps"));
    }

    #[test]
    fn test_oc_dep_006_no_fire_with_steps() {
        // When both steps and maxSteps are present, OC-AG-007 fires instead
        let diagnostics = validate(r#"{"agent": {"a": {"steps": 10, "maxSteps": 20}}}"#);
        assert!(
            !diagnostics.iter().any(|d| d.rule == "OC-DEP-006"),
            "OC-DEP-006 should not fire when 'steps' is also present"
        );
    }

    #[test]
    fn test_oc_dep_006_no_fire_only_steps() {
        let diagnostics = validate(r#"{"agent": {"a": {"steps": 10}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-DEP-006"));
    }
}
