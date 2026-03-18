//! Codex CLI validation rules (CDX-*, CDX-CFG-*, CDX-AG-*, CDX-APP-*)
//!
//! Validates:
//! - CDX-000: TOML Parse Error (HIGH) - invalid TOML syntax in config.toml
//! - CDX-001: Invalid approvalMode (HIGH) - must be "suggest", "auto-edit", or "full-auto"
//! - CDX-002: Invalid fullAutoErrorMode (HIGH) - must be "ask-user" or "ignore-and-continue"
//! - CDX-003: AGENTS.override.md in version control (MEDIUM) - should be in .gitignore
//! - CDX-004: Unknown config key (MEDIUM) - unrecognized key in .codex/config.toml
//! - CDX-005: project_doc_max_bytes exceeds limit (HIGH) - must be <= 65536
//! - CDX-006: Invalid project_doc_fallback_filenames (HIGH) - must be unique non-empty filenames
//! - CDX-CFG-001..022: Codex config schema/value checks (approval, sandbox, enums, unknown keys, etc.)
//! - CDX-AG-001..007: AGENTS.md quality and secret-safety checks for Codex
//! - CDX-APP-001..003: App and plugin configuration validation

use crate::{
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::{Validator, ValidatorMetadata},
    schemas::claude_md::find_generic_instructions,
    schemas::codex::{
        AGENTS_MD_MAX_SIZE, VALID_APPROVAL_MODES, VALID_FULL_AUTO_ERROR_MODES,
        VALID_MCP_OAUTH_STORES, VALID_MODEL_REASONING_SUMMARIES,
        VALID_SANDBOX_WORKSPACE_WRITE_MODES, parse_codex_toml,
    },
};
use rust_i18n::t;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::Path;

use crate::rules::find_closest_value;

/// Find the byte span of a TOML string value for the given key.
/// Returns byte positions of the inner string (without quotes).
/// Returns None if the key is not found or appears more than once (uniqueness guard).
fn find_toml_string_value_span(
    content: &str,
    key: &str,
    current_value: &str,
) -> Option<(usize, usize)> {
    crate::span_utils::find_unique_toml_string_value(content, key, current_value)
}

const CODEX_MARKDOWN_RULE_IDS: &[&str] = &[
    "CDX-003",
    "CDX-AG-001",
    "CDX-AG-002",
    "CDX-AG-003",
    "CDX-AG-004",
    "CDX-AG-005",
    "CDX-AG-006",
    "CDX-AG-007",
];

const CODEX_CONFIG_RULE_IDS: &[&str] = &[
    "CDX-000",
    "CDX-001",
    "CDX-002",
    "CDX-004",
    "CDX-005",
    "CDX-006",
    "CDX-CFG-001",
    "CDX-CFG-002",
    "CDX-CFG-003",
    "CDX-CFG-004",
    "CDX-CFG-005",
    "CDX-CFG-006",
    "CDX-CFG-007",
    "CDX-CFG-008",
    "CDX-CFG-009",
    "CDX-CFG-010",
    "CDX-CFG-011",
    "CDX-CFG-012",
    "CDX-CFG-013",
    "CDX-CFG-014",
    "CDX-CFG-015",
    "CDX-CFG-016",
    "CDX-CFG-017",
    "CDX-CFG-018",
    "CDX-CFG-019",
    "CDX-CFG-020",
    "CDX-CFG-021",
    "CDX-CFG-022",
    "CDX-APP-001",
    "CDX-APP-002",
    "CDX-APP-003",
];

pub struct CodexValidator;
pub struct CodexConfigValidator;

const VALID_APPROVAL_POLICIES: &[&str] = &["untrusted", "on-request", "never", "on-failure"];
const VALID_SANDBOX_MODES: &[&str] = &["read-only", "workspace-write", "danger-full-access"];
const VALID_MODEL_REASONING_EFFORTS: &[&str] =
    &["none", "minimal", "low", "medium", "high", "xhigh"];
const VALID_MODEL_VERBOSITY: &[&str] = &["low", "medium", "high"];
const VALID_PERSONALITIES: &[&str] = &["none", "friendly", "pragmatic"];
const VALID_SHELL_ENVIRONMENT_INHERIT: &[&str] = &["core", "all", "none"];
const VALID_CLI_AUTH_STORES: &[&str] = &["file", "keyring", "auto", "ephemeral"];
const VALID_DEFAULT_TOOLS_APPROVAL_MODES: &[&str] = &["auto", "prompt", "approve"];

const KNOWN_CONFIG_TOP_LEVEL_KEYS: &[&str] = &[
    "agents",
    "allow_login_shell",
    "analytics",
    "approval_policy",
    "apps",
    "audio",
    "background_terminal_max_timeout",
    "chatgpt_base_url",
    "check_for_update_on_startup",
    "cli_auth_credentials_store",
    "commit_attribution",
    "compact_prompt",
    "developer_instructions",
    "disable_paste_burst",
    "experimental_compact_prompt_file",
    "experimental_realtime_ws_backend_prompt",
    "experimental_realtime_ws_base_url",
    "experimental_realtime_ws_model",
    "experimental_use_freeform_apply_patch",
    "experimental_use_unified_exec_tool",
    "features",
    "feedback",
    "file_opener",
    "forced_chatgpt_workspace_id",
    "forced_login_method",
    "ghost_snapshot",
    "hide_agent_reasoning",
    "history",
    "instructions",
    "js_repl_node_module_dirs",
    "js_repl_node_path",
    "log_dir",
    "mcp_oauth_callback_port",
    "mcp_oauth_callback_url",
    "mcp_oauth_credentials_store",
    "mcp_servers",
    "memories",
    "model",
    "model_auto_compact_token_limit",
    "model_catalog_json",
    "model_context_window",
    "model_instructions_file",
    "model_provider",
    "model_providers",
    "model_reasoning_effort",
    "model_reasoning_summary",
    "model_supports_reasoning_summaries",
    "model_verbosity",
    "notice",
    "notify",
    "oss_provider",
    "otel",
    "permissions",
    "personality",
    "plan_mode_reasoning_effort",
    "plugins",
    "profile",
    "profiles",
    "project_doc_fallback_filenames",
    "project_doc_max_bytes",
    "project_root_markers",
    "projects",
    "review_model",
    "sandbox_mode",
    "sandbox_workspace_write",
    "shell_environment_policy",
    "show_raw_agent_reasoning",
    "skills",
    "sqlite_home",
    "suppress_unstable_features_warning",
    "tool_output_token_limit",
    "tools",
    "tui",
    "web_search",
    "windows",
    "windows_wsl_setup_acknowledged",
    "zsh_path",
    // Legacy compatibility in existing fixtures/tests.
    "approvalMode",
    "fullAutoErrorMode",
];

const KNOWN_FEATURE_KEYS: &[&str] = &[
    "apply_patch_freeform",
    "apps",
    "apps_mcp_gateway",
    "child_agents_md",
    "codex_git_commit",
    "collab",
    "collaboration_modes",
    "connectors",
    "default_mode_request_user_input",
    "elevated_windows_sandbox",
    "enable_experimental_windows_sandbox",
    "enable_request_compression",
    "experimental_use_freeform_apply_patch",
    "experimental_use_unified_exec_tool",
    "experimental_windows_sandbox",
    "include_apply_patch_tool",
    "js_repl",
    "js_repl_tools_only",
    "memories",
    "memory_tool",
    "multi_agent",
    "personality",
    "plugins",
    "powershell_utf8",
    "prevent_idle_sleep",
    "realtime_conversation",
    "remote_models",
    "request_permissions",
    "request_rule",
    "responses_websockets",
    "responses_websockets_v2",
    "runtime_metrics",
    "search_tool",
    "shell_snapshot",
    "shell_tool",
    "shell_zsh_fork",
    "skill_env_var_dependency_prompt",
    "skill_mcp_dependency_install",
    "sqlite",
    "steer",
    "undo",
    "unified_exec",
    "use_linux_sandbox_bwrap",
    "voice_transcription",
    "web_search",
    "web_search_cached",
    "web_search_request",
];

const KNOWN_TUI_KEYS: &[&str] = &[
    "alternate_screen",
    "animations",
    "model_availability_nux",
    "notification_method",
    "notifications",
    "show_tooltips",
    "status_line",
    "theme",
];

const KNOWN_SHELL_ENVIRONMENT_POLICY_KEYS: &[&str] = &[
    "exclude",
    "experimental_use_profile",
    "ignore_default_excludes",
    "include_only",
    "inherit",
    "set",
];

const KNOWN_MCP_SERVER_KEYS: &[&str] = &[
    "args",
    "bearer_token",
    "bearer_token_env_var",
    "command",
    "cwd",
    "disabled_tools",
    "enabled",
    "enabled_tools",
    "env",
    "env_http_headers",
    "env_vars",
    "http_headers",
    "oauth_resource",
    "required",
    "scopes",
    "startup_timeout_ms",
    "startup_timeout_sec",
    "tool_timeout_sec",
    "url",
];

const KNOWN_APPS_DEFAULT_KEYS: &[&str] = &["enabled", "destructive_enabled", "open_world_enabled"];
const KNOWN_APP_CONFIG_KEYS: &[&str] = &[
    "enabled",
    "destructive_enabled",
    "open_world_enabled",
    "default_tools_approval_mode",
    "default_tools_enabled",
    "tools",
];
const KNOWN_APP_TOOL_CONFIG_KEYS: &[&str] = &["enabled", "approval_mode"];

impl Validator for CodexValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: CODEX_MARKDOWN_RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Determine whether this is a .md file (ClaudeMd) or a .toml file (CodexConfig)
        // using a direct filename check instead of the full detect_file_type() call.
        // This runs on every ClaudeMd file but the cost is negligible: a single
        // OsStr comparison before early return.
        let is_markdown = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|name| name.ends_with(".md"));

        if is_markdown {
            diagnostics.extend(validate_codex_markdown_rules(path, content, config));
            return diagnostics;
        }

        let ext = path.extension().and_then(OsStr::to_str).unwrap_or_default();
        let is_toml = ext.eq_ignore_ascii_case("toml");
        let is_config_file = is_toml
            || ext.eq_ignore_ascii_case("json")
            || ext.eq_ignore_ascii_case("yaml")
            || ext.eq_ignore_ascii_case("yml");

        if is_config_file {
            // Build key-to-line mappings once for all config-based rules (CDX-000..006 + CDX-CFG-*)
            let key_lines = build_key_line_map(content);

            // For Codex TOML config files, run legacy CDX-000..006 checks.
            // Skip TOML parsing entirely when all TOML-dependent rules are disabled.
            if is_toml {
                let cdx_001_enabled = config.is_rule_enabled("CDX-001");
                let cdx_002_enabled = config.is_rule_enabled("CDX-002");
                let cdx_004_enabled = config.is_rule_enabled("CDX-004");
                let cdx_005_enabled = config.is_rule_enabled("CDX-005");
                let cdx_006_enabled = config.is_rule_enabled("CDX-006");
                let legacy_enabled = config.is_rule_enabled("CDX-000")
                    || cdx_001_enabled
                    || cdx_002_enabled
                    || cdx_004_enabled
                    || cdx_005_enabled
                    || cdx_006_enabled;
                if legacy_enabled {
                    let parsed = parse_codex_toml(content);

                    // If TOML is broken, emit a diagnostic so users can fix invalid syntax
                    if config.is_rule_enabled("CDX-000")
                        && let Some(parse_error) = &parsed.parse_error
                    {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                parse_error.line,
                                parse_error.column,
                                "CDX-000",
                                t!("rules.cdx_000.message", error = parse_error.message),
                            )
                            .with_suggestion(t!("rules.cdx_000.suggestion")),
                        );
                        return diagnostics;
                    }

                    // CDX-004: Unknown config keys (WARNING)
                    // Runs on unknown_keys which are populated whenever TOML parses successfully,
                    // even when schema extraction fails.
                    if cdx_004_enabled {
                        for unknown in &parsed.unknown_keys {
                            let mut diagnostic = Diagnostic::warning(
                                path.to_path_buf(),
                                unknown.line,
                                unknown.column,
                                "CDX-004",
                                t!("rules.cdx_004.message", key = unknown.key.as_str()),
                            )
                            .with_suggestion(t!("rules.cdx_004.suggestion"));

                            if let Some((start, end)) =
                                crate::rules::line_byte_range(content, unknown.line)
                            {
                                diagnostic = diagnostic.with_fix(Fix::delete(
                                    start,
                                    end,
                                    format!("Remove unknown config key '{}'", unknown.key),
                                    false,
                                ));
                            }

                            diagnostics.push(diagnostic);
                        }
                    }

                    let schema = match parsed.schema {
                        Some(s) => s,
                        None => return diagnostics,
                    };

                    // CDX-001: Invalid approvalMode (ERROR)
                    if cdx_001_enabled {
                        if parsed.approval_mode_wrong_type {
                            let line = key_lines.get("approvalMode").copied().unwrap_or(1);
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    line,
                                    0,
                                    "CDX-001",
                                    t!("rules.cdx_001.type_error"),
                                )
                                .with_suggestion(t!("rules.cdx_001.suggestion")),
                            );
                        } else if let Some(ref approval_value) = schema.approval_mode {
                            if !VALID_APPROVAL_MODES.contains(&approval_value.as_str()) {
                                let line = key_lines.get("approvalMode").copied().unwrap_or(1);
                                let mut diagnostic = Diagnostic::error(
                                    path.to_path_buf(),
                                    line,
                                    0,
                                    "CDX-001",
                                    t!("rules.cdx_001.message", value = approval_value.as_str()),
                                )
                                .with_suggestion(t!("rules.cdx_001.suggestion"));

                                // Unsafe auto-fix: replace with closest valid approval mode.
                                if let Some(suggested) =
                                    find_closest_value(approval_value, VALID_APPROVAL_MODES)
                                {
                                    if let Some((start, end)) = find_toml_string_value_span(
                                        content,
                                        "approvalMode",
                                        approval_value,
                                    ) {
                                        diagnostic = diagnostic.with_fix(Fix::replace(
                                            start,
                                            end,
                                            suggested,
                                            t!("rules.cdx_001.fix", fixed = suggested),
                                            false,
                                        ));
                                    }
                                }

                                diagnostics.push(diagnostic);
                            }
                        }
                    }

                    // CDX-002: Invalid fullAutoErrorMode (ERROR)
                    if cdx_002_enabled {
                        if parsed.full_auto_error_mode_wrong_type {
                            let line = key_lines.get("fullAutoErrorMode").copied().unwrap_or(1);
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    line,
                                    0,
                                    "CDX-002",
                                    t!("rules.cdx_002.type_error"),
                                )
                                .with_suggestion(t!("rules.cdx_002.suggestion")),
                            );
                        } else if let Some(ref error_mode_value) = schema.full_auto_error_mode {
                            if !VALID_FULL_AUTO_ERROR_MODES.contains(&error_mode_value.as_str()) {
                                let line = key_lines.get("fullAutoErrorMode").copied().unwrap_or(1);
                                let mut diagnostic = Diagnostic::error(
                                    path.to_path_buf(),
                                    line,
                                    0,
                                    "CDX-002",
                                    t!("rules.cdx_002.message", value = error_mode_value.as_str()),
                                )
                                .with_suggestion(t!("rules.cdx_002.suggestion"));

                                // Unsafe auto-fix: replace with closest valid error mode.
                                if let Some(suggested) = find_closest_value(
                                    error_mode_value,
                                    VALID_FULL_AUTO_ERROR_MODES,
                                ) {
                                    if let Some((start, end)) = find_toml_string_value_span(
                                        content,
                                        "fullAutoErrorMode",
                                        error_mode_value,
                                    ) {
                                        diagnostic = diagnostic.with_fix(Fix::replace(
                                            start,
                                            end,
                                            suggested,
                                            t!("rules.cdx_002.fix", fixed = suggested),
                                            false,
                                        ));
                                    }
                                }

                                diagnostics.push(diagnostic);
                            }
                        }
                    }

                    // CDX-005: project_doc_max_bytes exceeds limit (ERROR)
                    if cdx_005_enabled {
                        if parsed.project_doc_max_bytes_wrong_type {
                            let line = key_lines.get("project_doc_max_bytes").copied().unwrap_or(1);
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    line,
                                    0,
                                    "CDX-005",
                                    t!("rules.cdx_005.type_error"),
                                )
                                .with_suggestion(t!("rules.cdx_005.suggestion")),
                            );
                        } else if let Some(value) = schema.project_doc_max_bytes {
                            let line = key_lines.get("project_doc_max_bytes").copied().unwrap_or(1);
                            if value <= 0 {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        line,
                                        0,
                                        "CDX-005",
                                        t!("rules.cdx_005.type_error"),
                                    )
                                    .with_suggestion(t!("rules.cdx_005.suggestion")),
                                );
                            } else if value > 65536 {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        line,
                                        0,
                                        "CDX-005",
                                        t!("rules.cdx_005.message", value = &value.to_string()),
                                    )
                                    .with_suggestion(t!("rules.cdx_005.suggestion")),
                                );
                            }
                        }
                    }

                    // CDX-006: project_doc_fallback_filenames validation
                    if cdx_006_enabled {
                        let line = key_lines
                            .get("project_doc_fallback_filenames")
                            .copied()
                            .unwrap_or(1);

                        if parsed.project_doc_fallback_filenames_wrong_type {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    line,
                                    0,
                                    "CDX-006",
                                    t!("rules.cdx_006.type_error"),
                                )
                                .with_suggestion(t!("rules.cdx_006.suggestion")),
                            );
                        } else {
                            for idx in &parsed.project_doc_fallback_filename_non_string_indices {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        line,
                                        0,
                                        "CDX-006",
                                        t!(
                                            "rules.cdx_006.non_string",
                                            index = &(idx + 1).to_string()
                                        ),
                                    )
                                    .with_suggestion(t!("rules.cdx_006.suggestion")),
                                );
                            }

                            for idx in &parsed.project_doc_fallback_filename_empty_indices {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        line,
                                        0,
                                        "CDX-006",
                                        t!("rules.cdx_006.empty", index = &(idx + 1).to_string()),
                                    )
                                    .with_suggestion(t!("rules.cdx_006.suggestion")),
                                );
                            }

                            if let Some(filenames) = &schema.project_doc_fallback_filenames {
                                // Single pass: detect duplicates and suspicious filenames together
                                let mut seen: HashSet<String> = HashSet::new();
                                let mut duplicates: HashSet<String> = HashSet::new();
                                let mut suspicious: HashSet<String> = HashSet::new();
                                for filename in filenames {
                                    let normalized = filename.trim();
                                    if normalized.is_empty() {
                                        continue;
                                    }
                                    if !seen.insert(normalized.to_string()) {
                                        duplicates.insert(normalized.to_string());
                                    }
                                    if is_suspicious_fallback_filename(normalized) {
                                        suspicious.insert(normalized.to_string());
                                    }
                                }

                                let mut dup_sorted: Vec<_> = duplicates.into_iter().collect();
                                dup_sorted.sort();
                                for filename in dup_sorted {
                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            line,
                                            0,
                                            "CDX-006",
                                            t!(
                                                "rules.cdx_006.duplicate",
                                                value = filename.as_str()
                                            ),
                                        )
                                        .with_suggestion(t!("rules.cdx_006.suggestion")),
                                    );
                                }

                                let mut sus_sorted: Vec<_> = suspicious.into_iter().collect();
                                sus_sorted.sort();
                                for filename in sus_sorted {
                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            line,
                                            0,
                                            "CDX-006",
                                            t!(
                                                "rules.cdx_006.suspicious",
                                                value = filename.as_str()
                                            ),
                                        )
                                        .with_suggestion(t!("rules.cdx_006.suggestion")),
                                    );
                                }
                            }
                        }
                    }
                }
            } // is_toml (legacy CDX-000..006)

            diagnostics.extend(validate_codex_config_rules(
                path, content, config, &key_lines,
            ));
        }

        diagnostics
    }
}

impl Validator for CodexConfigValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: CODEX_CONFIG_RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = CodexValidator;
        validator.validate(path, content, config)
    }
}

fn validate_codex_markdown_rules(
    path: &Path,
    content: &str,
    config: &LintConfig,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();

    if config.is_rule_enabled("CDX-003") && filename == "AGENTS.override.md" {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                1,
                0,
                "CDX-003",
                t!("rules.cdx_003.message"),
            )
            .with_suggestion(t!("rules.cdx_003.suggestion")),
        );
    }

    if !matches!(
        filename,
        "AGENTS.md" | "AGENTS.local.md" | "AGENTS.override.md"
    ) {
        return diagnostics;
    }

    if config.is_rule_enabled("CDX-AG-001") && content.trim().is_empty() {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CDX-AG-001",
                t!("rules.cdx_ag_001.message"),
            )
            .with_suggestion(t!("rules.cdx_ag_001.suggestion")),
        );
    }

    if config.is_rule_enabled("CDX-AG-002") {
        for (line_no, line) in content.lines().enumerate() {
            // Quick pre-check: only lowercase if the line might contain a marker.
            let might_have_marker = line.bytes().any(|b| {
                matches!(
                    b.to_ascii_lowercase(),
                    b'a' | b's' | b't' | b'p' | b'b' | b'g' | b'x'
                )
            });
            if !might_have_marker {
                continue;
            }
            let lower = line.to_ascii_lowercase();
            // Sensitive keywords only count when they appear in an assignment
            // context (e.g., "token=abc", "secret: xyz"), not as prose words
            // like "Token efficiency" or "save tokens". The keyword must be
            // directly followed by optional whitespace then = or :.
            let has_sensitive_key = ["api_key", "apikey", "secret", "token", "password", "bearer"]
                .iter()
                .any(|needle| has_keyword_assignment(&lower, needle));
            let contains_key_prefix = has_sk_token_prefix(line)
                || has_token_prefix(line, "ghp_")
                || has_token_prefix(line, "gho_")
                || has_token_prefix(line, "AKIA")
                || has_token_prefix(line, "xoxb-")
                || has_token_prefix(line, "xoxp-")
                || has_token_prefix(line, "glpat-")
                || has_token_prefix(line, "sk-ant-")
                || has_token_prefix(line, "sk-proj-");
            let has_interpolation = line.contains("${") || line.contains("$(");
            if (has_sensitive_key || contains_key_prefix) && !has_interpolation {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line_no + 1,
                        0,
                        "CDX-AG-002",
                        t!("rules.cdx_ag_002.message"),
                    )
                    .with_suggestion(t!("rules.cdx_ag_002.suggestion")),
                );
                break;
            }
        }
    }

    if config.is_rule_enabled("CDX-AG-003") {
        let generic_count = find_generic_instructions(content).len();
        let trimmed = content.trim();
        let too_short = trimmed.len() < 120;
        let low_specificity =
            !trimmed.contains('`') && !trimmed.contains('/') && !trimmed.contains("--");
        if generic_count > 0 && too_short && low_specificity {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "CDX-AG-003",
                    t!("rules.cdx_ag_003.message"),
                )
                .with_suggestion(t!("rules.cdx_ag_003.suggestion")),
            );
        }
    }

    // CDX-AG-004: AGENTS.md exceeds size limit
    if config.is_rule_enabled("CDX-AG-004") && content.len() > AGENTS_MD_MAX_SIZE {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                1,
                0,
                "CDX-AG-004",
                t!(
                    "rules.cdx_ag_004.message",
                    size = &content.len().to_string(),
                    limit = &AGENTS_MD_MAX_SIZE.to_string()
                ),
            )
            .with_suggestion(t!("rules.cdx_ag_004.suggestion")),
        );
    }

    // CDX-AG-005: AGENTS.md references missing file (simplified: check for broken markdown links)
    if config.is_rule_enabled("CDX-AG-005") {
        let fs = config.fs();
        for (line_no, line) in content.lines().enumerate() {
            // Match markdown-style file references like `path/to/file.ext`
            for cap in find_backtick_file_refs(line) {
                let reference = cap.trim();
                if reference.is_empty()
                    || reference.starts_with("http")
                    || reference.starts_with('$')
                    || reference.starts_with('<')
                    || !reference.contains('.')
                {
                    continue;
                }
                // Resolve relative to project root (parent of AGENTS.md's parent)
                let resolved = path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(reference);
                if !fs.exists(&resolved) {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            line_no + 1,
                            0,
                            "CDX-AG-005",
                            t!("rules.cdx_ag_005.message", reference = reference),
                        )
                        .with_suggestion(t!("rules.cdx_ag_005.suggestion")),
                    );
                    break; // One diagnostic per file is enough
                }
            }
        }
    }

    // CDX-AG-006: AGENTS.md missing project context
    if config.is_rule_enabled("CDX-AG-006") {
        let trimmed = content.trim();
        let has_heading = trimmed.contains("# ");
        let has_command = trimmed.contains('`');
        let has_path = trimmed.contains('/') || trimmed.contains('\\');
        let long_enough = trimmed.len() >= 200;
        if !trimmed.is_empty() && !has_heading && !has_command && !has_path && !long_enough {
            diagnostics.push(
                Diagnostic::info(
                    path.to_path_buf(),
                    1,
                    0,
                    "CDX-AG-006",
                    t!("rules.cdx_ag_006.message"),
                )
                .with_suggestion(t!("rules.cdx_ag_006.suggestion")),
            );
        }
    }

    // CDX-AG-007: AGENTS.md contradicts config.toml (placeholder - requires cross-file analysis)
    // This rule is registered but validation requires project-level context;
    // actual cross-file checking is deferred to project-level validators.

    diagnostics
}

/// Extract file-like references inside backticks from a line.
fn find_backtick_file_refs(line: &str) -> Vec<&str> {
    let mut refs = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find('`') {
        let after = &rest[start + 1..];
        if let Some(end) = after.find('`') {
            let inner = &after[..end];
            if !inner.is_empty() && !inner.contains(' ') {
                refs.push(inner);
            }
            rest = &after[end + 1..];
        } else {
            break;
        }
    }
    refs
}

/// Check if `haystack` contains `word` at a word boundary followed (with optional whitespace)
/// by `=` or `:`. Used by CDX-AG-002 to distinguish secret assignments like `token=abc`
/// from prose like "Token efficiency".
fn has_keyword_assignment(haystack: &str, word: &str) -> bool {
    let bytes = haystack.as_bytes();
    let word_bytes = word.as_bytes();
    if word_bytes.len() > bytes.len() {
        return false;
    }
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(word) {
        let abs = start + pos;
        let before_ok = abs == 0 || !bytes[abs - 1].is_ascii_alphanumeric();
        let after_idx = abs + word_bytes.len();
        let after_ok = after_idx >= bytes.len() || !bytes[after_idx].is_ascii_alphanumeric();
        if before_ok && after_ok {
            // Check if keyword is followed by optional whitespace then = or :
            let rest = &haystack[after_idx..];
            let trimmed = rest.trim_start();
            if trimmed.starts_with('=') || trimmed.starts_with(':') {
                return true;
            }
        }
        start = abs + 1;
    }
    false
}

/// Check if a line contains a known secret token prefix at a word boundary.
fn has_token_prefix(line: &str, prefix: &str) -> bool {
    line.match_indices(prefix).any(|(idx, _)| {
        let prev_is_alnum = idx > 0
            && line[..idx]
                .chars()
                .next_back()
                .is_some_and(|ch| ch.is_ascii_alphanumeric());
        let next_is_alnum = line[idx + prefix.len()..]
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphanumeric());
        !prev_is_alnum && next_is_alnum
    })
}

fn has_sk_token_prefix(line: &str) -> bool {
    line.match_indices("sk-").any(|(idx, _)| {
        let prev_is_alnum = idx > 0
            && line[..idx]
                .chars()
                .next_back()
                .is_some_and(|ch| ch.is_ascii_alphanumeric());
        let next_is_alnum = line[idx + 3..]
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphanumeric());
        !prev_is_alnum && next_is_alnum
    })
}

fn validate_codex_config_rules(
    path: &Path,
    content: &str,
    config: &LintConfig,
    key_lines: &HashMap<String, usize>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let root = match parse_codex_config_value(path, content) {
        Ok(root) => root,
        Err(parse_error) => {
            if config.is_rule_enabled("CDX-000") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        parse_error.line,
                        parse_error.column,
                        "CDX-000",
                        t!("rules.cdx_000.message", error = parse_error.message),
                    )
                    .with_suggestion(t!("rules.cdx_000.suggestion")),
                );
            }
            return diagnostics;
        }
    };

    let line_for = |key: &str| key_lines.get(key).copied().unwrap_or(1);

    if config.is_rule_enabled("CDX-CFG-001")
        && let Some(value) = value_at_path(&root, &["approval_policy"])
    {
        if let Some(policy) = value.as_str() {
            if !VALID_APPROVAL_POLICIES.contains(&policy) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line_for("approval_policy"),
                        0,
                        "CDX-CFG-001",
                        t!("rules.cdx_cfg_001.message", value = policy),
                    )
                    .with_suggestion(t!("rules.cdx_cfg_001.suggestion")),
                );
            }
        } else if !value.is_null() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line_for("approval_policy"),
                    0,
                    "CDX-CFG-001",
                    t!("rules.cdx_cfg_001.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_001.suggestion")),
            );
        }
    }

    if config.is_rule_enabled("CDX-CFG-002")
        && let Some(value) = value_at_path(&root, &["sandbox_mode"])
    {
        if let Some(mode) = value.as_str() {
            if !VALID_SANDBOX_MODES.contains(&mode) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line_for("sandbox_mode"),
                        0,
                        "CDX-CFG-002",
                        t!("rules.cdx_cfg_002.message", value = mode),
                    )
                    .with_suggestion(t!("rules.cdx_cfg_002.suggestion")),
                );
            }
        } else if !value.is_null() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line_for("sandbox_mode"),
                    0,
                    "CDX-CFG-002",
                    t!("rules.cdx_cfg_002.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_002.suggestion")),
            );
        }
    }

    if config.is_rule_enabled("CDX-CFG-003")
        && let Some(value) = value_at_path(&root, &["model_reasoning_effort"])
    {
        if let Some(effort) = value.as_str() {
            if !VALID_MODEL_REASONING_EFFORTS.contains(&effort) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line_for("model_reasoning_effort"),
                        0,
                        "CDX-CFG-003",
                        t!("rules.cdx_cfg_003.message", value = effort),
                    )
                    .with_suggestion(t!("rules.cdx_cfg_003.suggestion")),
                );
            }
        } else if !value.is_null() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line_for("model_reasoning_effort"),
                    0,
                    "CDX-CFG-003",
                    t!("rules.cdx_cfg_003.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_003.suggestion")),
            );
        }
    }

    if config.is_rule_enabled("CDX-CFG-004")
        && let Some(value) = value_at_path(&root, &["model_verbosity"])
    {
        if let Some(verbosity) = value.as_str() {
            if !VALID_MODEL_VERBOSITY.contains(&verbosity) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line_for("model_verbosity"),
                        0,
                        "CDX-CFG-004",
                        t!("rules.cdx_cfg_004.message", value = verbosity),
                    )
                    .with_suggestion(t!("rules.cdx_cfg_004.suggestion")),
                );
            }
        } else if !value.is_null() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line_for("model_verbosity"),
                    0,
                    "CDX-CFG-004",
                    t!("rules.cdx_cfg_004.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_004.suggestion")),
            );
        }
    }

    if config.is_rule_enabled("CDX-CFG-005")
        && let Some(value) = value_at_path(&root, &["personality"])
    {
        if let Some(personality) = value.as_str() {
            if !VALID_PERSONALITIES.contains(&personality) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line_for("personality"),
                        0,
                        "CDX-CFG-005",
                        t!("rules.cdx_cfg_005.message", value = personality),
                    )
                    .with_suggestion(t!("rules.cdx_cfg_005.suggestion")),
                );
            }
        } else if !value.is_null() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line_for("personality"),
                    0,
                    "CDX-CFG-005",
                    t!("rules.cdx_cfg_005.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_005.suggestion")),
            );
        }
    }

    if config.is_rule_enabled("CDX-CFG-006") {
        let is_toml = path
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"));
        let skip_top_level = is_toml && config.is_rule_enabled("CDX-004");
        for path_key in collect_unknown_codex_keys(&root) {
            if skip_top_level && !path_key.contains('.') {
                continue;
            }
            let line = line_for(path_key.rsplit('.').next().unwrap_or(path_key.as_str()));
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CDX-CFG-006",
                    t!("rules.cdx_cfg_006.message", key = path_key.as_str()),
                )
                .with_suggestion(t!("rules.cdx_cfg_006.suggestion")),
            );
        }
    }

    if config.is_rule_enabled("CDX-CFG-007")
        && let Some(value) = value_at_path(&root, &["sandbox_mode"])
        && value.as_str() == Some("danger-full-access")
    {
        let acknowledged = bool_at_path(&root, &["notice", "hide_full_access_warning"]);
        if acknowledged != Some(true) {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line_for("sandbox_mode"),
                    0,
                    "CDX-CFG-007",
                    t!("rules.cdx_cfg_007.message"),
                )
                .with_suggestion(t!("rules.cdx_cfg_007.suggestion")),
            );
        }
    }

    if config.is_rule_enabled("CDX-CFG-008") {
        let shell_inherit = value_at_path(&root, &["shell_environment_policy", "inherit"])
            .or_else(|| value_at_path(&root, &["shell_environment", "inherit"]));
        if let Some(value) = shell_inherit {
            if let Some(inherit) = value.as_str() {
                if !VALID_SHELL_ENVIRONMENT_INHERIT.contains(&inherit) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line_for("inherit"),
                            0,
                            "CDX-CFG-008",
                            t!("rules.cdx_cfg_008.message", value = inherit),
                        )
                        .with_suggestion(t!("rules.cdx_cfg_008.suggestion")),
                    );
                }
            } else if !value.is_null() {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line_for("inherit"),
                        0,
                        "CDX-CFG-008",
                        t!("rules.cdx_cfg_008.type_error"),
                    )
                    .with_suggestion(t!("rules.cdx_cfg_008.suggestion")),
                );
            }
        }
    }

    if config.is_rule_enabled("CDX-CFG-009")
        && let Some(mcp_servers) = value_at_path(&root, &["mcp_servers"])
    {
        if let Some(servers) = mcp_servers.as_object() {
            for (server_name, server) in servers {
                let Some(server_obj) = server.as_object() else {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line_for("mcp_servers"),
                            0,
                            "CDX-CFG-009",
                            t!(
                                "rules.cdx_cfg_009.invalid_server",
                                server = server_name.as_str()
                            ),
                        )
                        .with_suggestion(t!("rules.cdx_cfg_009.suggestion")),
                    );
                    continue;
                };

                let has_command = server_obj
                    .get("command")
                    .and_then(Value::as_str)
                    .is_some_and(|v| !v.trim().is_empty());
                let has_url = server_obj
                    .get("url")
                    .and_then(Value::as_str)
                    .is_some_and(|v| !v.trim().is_empty());
                if !has_command && !has_url {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line_for("mcp_servers"),
                            0,
                            "CDX-CFG-009",
                            t!(
                                "rules.cdx_cfg_009.missing_transport",
                                server = server_name.as_str()
                            ),
                        )
                        .with_suggestion(t!("rules.cdx_cfg_009.suggestion")),
                    );
                }
            }
        } else if !mcp_servers.is_null() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line_for("mcp_servers"),
                    0,
                    "CDX-CFG-009",
                    t!("rules.cdx_cfg_009.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_009.suggestion")),
            );
        }
    }

    if config.is_rule_enabled("CDX-CFG-010") {
        let mut secret_paths = Vec::new();
        collect_hardcoded_secret_paths(&root, "", &mut secret_paths);
        secret_paths.sort();
        secret_paths.dedup();
        for secret_path in secret_paths {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line_for(secret_path.rsplit('.').next().unwrap_or("config")),
                    0,
                    "CDX-CFG-010",
                    t!("rules.cdx_cfg_010.message", key = secret_path.as_str()),
                )
                .with_suggestion(t!("rules.cdx_cfg_010.suggestion")),
            );
        }
    }

    if config.is_rule_enabled("CDX-CFG-011")
        && let Some(features) = value_at_path(&root, &["features"])
    {
        if let Some(features_obj) = features.as_object() {
            for feature_name in features_obj.keys() {
                if !KNOWN_FEATURE_KEYS.contains(&feature_name.as_str()) {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            line_for(feature_name.as_str()),
                            0,
                            "CDX-CFG-011",
                            t!("rules.cdx_cfg_011.message", key = feature_name.as_str()),
                        )
                        .with_suggestion(t!("rules.cdx_cfg_011.suggestion")),
                    );
                }
            }
        } else if !features.is_null() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line_for("features"),
                    0,
                    "CDX-CFG-011",
                    t!("rules.cdx_cfg_011.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_011.suggestion")),
            );
        }
    }

    if config.is_rule_enabled("CDX-CFG-012")
        && let Some(value) = value_at_path(&root, &["cli_auth_credentials_store"])
    {
        if let Some(store) = value.as_str() {
            if !VALID_CLI_AUTH_STORES.contains(&store) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line_for("cli_auth_credentials_store"),
                        0,
                        "CDX-CFG-012",
                        t!("rules.cdx_cfg_012.message", value = store),
                    )
                    .with_suggestion(t!("rules.cdx_cfg_012.suggestion")),
                );
            }
        } else if !value.is_null() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line_for("cli_auth_credentials_store"),
                    0,
                    "CDX-CFG-012",
                    t!("rules.cdx_cfg_012.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_012.suggestion")),
            );
        }
    }

    // CDX-CFG-013: Invalid sandbox_workspace_write mode
    if config.is_rule_enabled("CDX-CFG-013")
        && let Some(value) = value_at_path(&root, &["sandbox_workspace_write", "mode"])
    {
        if let Some(mode) = value.as_str() {
            if !VALID_SANDBOX_WORKSPACE_WRITE_MODES.contains(&mode) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line_for("mode"),
                        0,
                        "CDX-CFG-013",
                        t!("rules.cdx_cfg_013.message", value = mode),
                    )
                    .with_suggestion(t!("rules.cdx_cfg_013.suggestion")),
                );
            }
        } else if !value.is_null() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line_for("mode"),
                    0,
                    "CDX-CFG-013",
                    t!("rules.cdx_cfg_013.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_013.suggestion")),
            );
        }
    }

    // CDX-CFG-014: Invalid model value (must be a string)
    if config.is_rule_enabled("CDX-CFG-014")
        && let Some(value) = value_at_path(&root, &["model"])
    {
        if !value.is_string() && !value.is_null() {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line_for("model"),
                    0,
                    "CDX-CFG-014",
                    t!("rules.cdx_cfg_014.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_014.suggestion")),
            );
        }
    }

    // CDX-CFG-015: Invalid model_provider value (must be a string)
    if config.is_rule_enabled("CDX-CFG-015")
        && let Some(value) = value_at_path(&root, &["model_provider"])
    {
        if !value.is_string() && !value.is_null() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line_for("model_provider"),
                    0,
                    "CDX-CFG-015",
                    t!("rules.cdx_cfg_015.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_015.suggestion")),
            );
        }
    }

    // CDX-CFG-016: Invalid model_reasoning_summary value
    if config.is_rule_enabled("CDX-CFG-016")
        && let Some(value) = value_at_path(&root, &["model_reasoning_summary"])
    {
        if let Some(summary) = value.as_str() {
            if !VALID_MODEL_REASONING_SUMMARIES.contains(&summary) {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line_for("model_reasoning_summary"),
                        0,
                        "CDX-CFG-016",
                        t!("rules.cdx_cfg_016.message", value = summary),
                    )
                    .with_suggestion(t!("rules.cdx_cfg_016.suggestion")),
                );
            }
        } else if !value.is_null() {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line_for("model_reasoning_summary"),
                    0,
                    "CDX-CFG-016",
                    t!("rules.cdx_cfg_016.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_016.suggestion")),
            );
        }
    }

    // CDX-CFG-017: Invalid history configuration (must be an object)
    if config.is_rule_enabled("CDX-CFG-017")
        && let Some(value) = value_at_path(&root, &["history"])
    {
        if !value.is_object() && !value.is_null() {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line_for("history"),
                    0,
                    "CDX-CFG-017",
                    t!("rules.cdx_cfg_017.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_017.suggestion")),
            );
        }
    }

    // CDX-CFG-018: Invalid tui configuration (must be an object)
    if config.is_rule_enabled("CDX-CFG-018")
        && let Some(value) = value_at_path(&root, &["tui"])
    {
        if !value.is_object() && !value.is_null() {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line_for("tui"),
                    0,
                    "CDX-CFG-018",
                    t!("rules.cdx_cfg_018.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_018.suggestion")),
            );
        }
    }

    // CDX-CFG-019: Invalid file_opener value (must be a string)
    if config.is_rule_enabled("CDX-CFG-019")
        && let Some(value) = value_at_path(&root, &["file_opener"])
    {
        if !value.is_string() && !value.is_null() {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line_for("file_opener"),
                    0,
                    "CDX-CFG-019",
                    t!("rules.cdx_cfg_019.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_019.suggestion")),
            );
        }
    }

    // CDX-CFG-020: Invalid MCP OAuth config (mcp_oauth_credentials_store must be valid)
    if config.is_rule_enabled("CDX-CFG-020")
        && let Some(value) = value_at_path(&root, &["mcp_oauth_credentials_store"])
    {
        if let Some(store) = value.as_str() {
            if !VALID_MCP_OAUTH_STORES.contains(&store) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line_for("mcp_oauth_credentials_store"),
                        0,
                        "CDX-CFG-020",
                        t!("rules.cdx_cfg_020.message", value = store),
                    )
                    .with_suggestion(t!("rules.cdx_cfg_020.suggestion")),
                );
            }
        } else if !value.is_null() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line_for("mcp_oauth_credentials_store"),
                    0,
                    "CDX-CFG-020",
                    t!("rules.cdx_cfg_020.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_020.suggestion")),
            );
        }
    }

    // CDX-CFG-021: Invalid model_context_window (must be a positive integer)
    if config.is_rule_enabled("CDX-CFG-021")
        && let Some(value) = value_at_path(&root, &["model_context_window"])
    {
        if let Some(num) = value.as_i64() {
            if num <= 0 {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line_for("model_context_window"),
                        0,
                        "CDX-CFG-021",
                        t!("rules.cdx_cfg_021.message", value = &num.to_string()),
                    )
                    .with_suggestion(t!("rules.cdx_cfg_021.suggestion")),
                );
            }
        } else if !value.is_null() && !value.is_u64() {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line_for("model_context_window"),
                    0,
                    "CDX-CFG-021",
                    t!("rules.cdx_cfg_021.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_021.suggestion")),
            );
        }
    }

    // CDX-CFG-022: Invalid model_auto_compact_token_limit (must be a positive integer)
    if config.is_rule_enabled("CDX-CFG-022")
        && let Some(value) = value_at_path(&root, &["model_auto_compact_token_limit"])
    {
        if let Some(num) = value.as_i64() {
            if num <= 0 {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line_for("model_auto_compact_token_limit"),
                        0,
                        "CDX-CFG-022",
                        t!("rules.cdx_cfg_022.message", value = &num.to_string()),
                    )
                    .with_suggestion(t!("rules.cdx_cfg_022.suggestion")),
                );
            }
        } else if !value.is_null() && !value.is_u64() {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line_for("model_auto_compact_token_limit"),
                    0,
                    "CDX-CFG-022",
                    t!("rules.cdx_cfg_022.type_error"),
                )
                .with_suggestion(t!("rules.cdx_cfg_022.suggestion")),
            );
        }
    }

    if config.is_rule_enabled("CDX-APP-001")
        && let Some(apps) = value_at_path(&root, &["apps"])
    {
        if let Some(apps_obj) = apps.as_object() {
            for (app_name, app_value) in apps_obj {
                if app_name == "_default" {
                    continue;
                }
                let Some(app_obj) = app_value.as_object() else {
                    continue;
                };
                if let Some(mode) = app_obj.get("default_tools_approval_mode") {
                    if let Some(mode_str) = mode.as_str() {
                        if !VALID_DEFAULT_TOOLS_APPROVAL_MODES.contains(&mode_str) {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    line_for("default_tools_approval_mode"),
                                    0,
                                    "CDX-APP-001",
                                    t!(
                                        "rules.cdx_app_001.message",
                                        app = app_name.as_str(),
                                        value = mode_str
                                    ),
                                )
                                .with_suggestion(t!("rules.cdx_app_001.suggestion")),
                            );
                        }
                    } else if !mode.is_null() {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                line_for("default_tools_approval_mode"),
                                0,
                                "CDX-APP-001",
                                t!("rules.cdx_app_001.type_error", app = app_name.as_str()),
                            )
                            .with_suggestion(t!("rules.cdx_app_001.suggestion")),
                        );
                    }
                }
            }
        }
    }

    // CDX-APP-002: Invalid skills configuration (must be an object)
    if config.is_rule_enabled("CDX-APP-002")
        && let Some(value) = value_at_path(&root, &["skills"])
    {
        if !value.is_object() && !value.is_null() {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line_for("skills"),
                    0,
                    "CDX-APP-002",
                    t!("rules.cdx_app_002.type_error"),
                )
                .with_suggestion(t!("rules.cdx_app_002.suggestion")),
            );
        }
    }

    // CDX-APP-003: Invalid profile configuration (must be a string)
    if config.is_rule_enabled("CDX-APP-003")
        && let Some(value) = value_at_path(&root, &["profile"])
    {
        if !value.is_string() && !value.is_null() {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line_for("profile"),
                    0,
                    "CDX-APP-003",
                    t!("rules.cdx_app_003.type_error"),
                )
                .with_suggestion(t!("rules.cdx_app_003.suggestion")),
            );
        }
    }

    diagnostics
}

struct CodexConfigParseError {
    line: usize,
    column: usize,
    message: String,
}

fn parse_codex_config_value(path: &Path, content: &str) -> Result<Value, CodexConfigParseError> {
    let extension = path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "toml" => {
            let parsed = parse_codex_toml(content);
            if let Some(parse_error) = parsed.parse_error {
                return Err(CodexConfigParseError {
                    line: parse_error.line.max(1),
                    column: parse_error.column,
                    message: parse_error.message,
                });
            }

            let table: toml::Table =
                toml::from_str(content).map_err(|error| CodexConfigParseError {
                    line: 1,
                    column: 0,
                    message: error.to_string(),
                })?;
            serde_json::to_value(table).map_err(|error| CodexConfigParseError {
                line: 1,
                column: 0,
                message: error.to_string(),
            })
        }
        "json" => serde_json::from_str::<Value>(content).map_err(|error| CodexConfigParseError {
            line: error.line().max(1),
            column: error.column(),
            message: error.to_string(),
        }),
        "yaml" | "yml" => {
            let yaml: serde_yaml::Value =
                serde_yaml::from_str(content).map_err(|error| CodexConfigParseError {
                    line: error.location().map_or(1, |loc| loc.line().max(1)),
                    column: error.location().map_or(0, |loc| loc.column()),
                    message: error.to_string(),
                })?;
            serde_json::to_value(yaml).map_err(|error| CodexConfigParseError {
                line: 1,
                column: 0,
                message: error.to_string(),
            })
        }
        _ => Err(CodexConfigParseError {
            line: 1,
            column: 0,
            message: "unsupported Codex config file extension".to_string(),
        }),
    }
}

fn value_at_path<'a>(root: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = root;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}

fn bool_at_path(root: &Value, path: &[&str]) -> Option<bool> {
    value_at_path(root, path).and_then(Value::as_bool)
}

fn collect_unknown_codex_keys(root: &Value) -> Vec<String> {
    let mut unknown = Vec::new();
    let Some(root_obj) = root.as_object() else {
        return unknown;
    };

    for key in root_obj.keys() {
        if !KNOWN_CONFIG_TOP_LEVEL_KEYS.contains(&key.as_str()) {
            unknown.push(key.clone());
        }
    }

    if let Some(features) = root_obj.get("features").and_then(Value::as_object) {
        for key in features.keys() {
            if !KNOWN_FEATURE_KEYS.contains(&key.as_str()) {
                unknown.push(format!("features.{key}"));
            }
        }
    }

    if let Some(tui) = root_obj.get("tui").and_then(Value::as_object) {
        for key in tui.keys() {
            if !KNOWN_TUI_KEYS.contains(&key.as_str()) {
                unknown.push(format!("tui.{key}"));
            }
        }
    }

    if let Some(shell) = root_obj
        .get("shell_environment_policy")
        .and_then(Value::as_object)
    {
        for key in shell.keys() {
            if !KNOWN_SHELL_ENVIRONMENT_POLICY_KEYS.contains(&key.as_str()) {
                unknown.push(format!("shell_environment_policy.{key}"));
            }
        }
    }

    if let Some(mcp_servers) = root_obj.get("mcp_servers").and_then(Value::as_object) {
        for (server_name, server_cfg) in mcp_servers {
            if let Some(server_obj) = server_cfg.as_object() {
                for key in server_obj.keys() {
                    if !KNOWN_MCP_SERVER_KEYS.contains(&key.as_str()) {
                        unknown.push(format!("mcp_servers.{server_name}.{key}"));
                    }
                }
            }
        }
    }

    if let Some(apps) = root_obj.get("apps").and_then(Value::as_object) {
        for (app_name, app_cfg) in apps {
            if app_name == "_default" {
                if let Some(default_obj) = app_cfg.as_object() {
                    for key in default_obj.keys() {
                        if !KNOWN_APPS_DEFAULT_KEYS.contains(&key.as_str()) {
                            unknown.push(format!("apps._default.{key}"));
                        }
                    }
                }
                continue;
            }

            if let Some(app_obj) = app_cfg.as_object() {
                for key in app_obj.keys() {
                    if !KNOWN_APP_CONFIG_KEYS.contains(&key.as_str()) {
                        unknown.push(format!("apps.{app_name}.{key}"));
                    }
                }

                if let Some(tools_obj) = app_obj.get("tools").and_then(Value::as_object) {
                    for (tool_name, tool_cfg) in tools_obj {
                        if let Some(tool_obj) = tool_cfg.as_object() {
                            for key in tool_obj.keys() {
                                if !KNOWN_APP_TOOL_CONFIG_KEYS.contains(&key.as_str()) {
                                    unknown
                                        .push(format!("apps.{app_name}.tools.{tool_name}.{key}"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    unknown.sort();
    unknown.dedup();
    unknown
}

fn collect_hardcoded_secret_paths(value: &Value, path: &str, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, nested_value) in map {
                let next = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };

                if let Some(str_value) = nested_value.as_str()
                    && seems_hardcoded_secret(key, str_value)
                {
                    out.push(next.clone());
                }
                collect_hardcoded_secret_paths(nested_value, &next, out);
            }
        }
        Value::Array(values) => {
            for nested_value in values {
                collect_hardcoded_secret_paths(nested_value, path, out);
            }
        }
        _ => {}
    }
}

fn seems_hardcoded_secret(key: &str, value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.starts_with('$')
        || trimmed.starts_with("${")
        || trimmed.starts_with("env:")
        || trimmed.contains("${")
    {
        return false;
    }

    if trimmed.contains("sk-") && trimmed.len() >= 20 {
        return true;
    }

    let sensitive_key = [
        "api_key",
        "apikey",
        "secret",
        "token",
        "password",
        "bearer",
        "credential",
    ]
    .iter()
    .any(|needle| key.to_ascii_lowercase().contains(needle));
    if !sensitive_key {
        return false;
    }

    let has_letter = trimmed.chars().any(|c| c.is_ascii_alphabetic());
    let has_digit = trimmed.chars().any(|c| c.is_ascii_digit());
    trimmed.len() >= 12 && has_letter && has_digit
}

fn is_suspicious_fallback_filename(filename: &str) -> bool {
    filename.contains('/') || filename.contains('\\') || is_windows_absolute_path(filename)
}

fn is_windows_absolute_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

/// Build a map of TOML key names to their 1-indexed line numbers in a single pass.
///
/// Scans each line for a key followed by `=` (the TOML key-value separator).
/// Extracts keys by finding '=' positions; indexing is safe because find() returns
/// char-boundary positions in valid UTF-8. Handles both bare keys and simple quoted keys
/// (e.g., `"approvalMode"`), stripping quotes to normalize lookups. Prevents partial
/// matches by extracting only up to `=` (e.g., `approvalMode` will not match
/// `approvalModeExtra`).
///
/// Returns only the first occurrence of each key, which matches TOML semantics.
fn build_key_line_map(content: &str) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        // Extract the key portion: everything up to `=` or whitespace before `=`
        if let Some(eq_pos) = trimmed.find('=') {
            let key_part = trimmed[..eq_pos].trim_end();

            // Handle both bare keys and simple quoted keys.
            let key = if key_part.starts_with('"') && key_part.ends_with('"') && key_part.len() >= 2
            {
                key_part[1..key_part.len() - 1].to_string()
            } else {
                key_part.to_string()
            };

            // Only record the first occurrence (TOML spec: duplicate keys are errors)
            if !key.is_empty() && !map.contains_key(&key) {
                map.insert(key, i + 1);
            }
        }
    }
    map
}

/// Find the 1-indexed line number of a TOML key in the content.
///
/// Uses `strip_prefix` for UTF-8 safety and verifies the next non-whitespace
/// character is `=` to prevent partial key matches (e.g., `approvalMode`
/// does not match `approvalModeExtra`).
///
/// Production code uses `build_key_line_map` for single-pass efficiency;
/// this function is retained for targeted lookups in tests.
#[cfg(test)]
fn find_key_line(content: &str, key: &str) -> Option<usize> {
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(after) = trimmed.strip_prefix(key) {
            if after.trim_start().starts_with('=') {
                return Some(i + 1);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate_config(content: &str) -> Vec<Diagnostic> {
        let validator = CodexValidator;
        validator.validate(
            Path::new(".codex/config.toml"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_config_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = CodexValidator;
        validator.validate(Path::new(".codex/config.toml"), content, config)
    }

    fn validate_config_at_path(path: &str, content: &str) -> Vec<Diagnostic> {
        let validator = CodexValidator;
        validator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn validate_claude_md(path: &str, content: &str) -> Vec<Diagnostic> {
        let validator = CodexValidator;
        validator.validate(Path::new(path), content, &LintConfig::default())
    }

    // ===== CDX-001: Invalid approvalMode =====

    #[test]
    fn test_cdx_001_invalid_approval_mode() {
        let diagnostics = validate_config("approvalMode = \"yolo\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        assert_eq!(cdx_001[0].level, DiagnosticLevel::Error);
        assert!(cdx_001[0].message.contains("yolo"));
    }

    #[test]
    fn test_cdx_001_valid_suggest() {
        let diagnostics = validate_config("approvalMode = \"suggest\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert!(cdx_001.is_empty());
    }

    #[test]
    fn test_cdx_001_valid_auto_edit() {
        let diagnostics = validate_config("approvalMode = \"auto-edit\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert!(cdx_001.is_empty());
    }

    #[test]
    fn test_cdx_001_valid_full_auto() {
        let diagnostics = validate_config("approvalMode = \"full-auto\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert!(cdx_001.is_empty());
    }

    #[test]
    fn test_cdx_001_all_valid_modes() {
        for mode in VALID_APPROVAL_MODES {
            let content = format!("approvalMode = \"{}\"", mode);
            let diagnostics = validate_config(&content);
            let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
            assert!(cdx_001.is_empty(), "Mode '{}' should be valid", mode);
        }
    }

    #[test]
    fn test_cdx_001_absent_approval_mode() {
        let diagnostics = validate_config("model = \"o4-mini\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert!(cdx_001.is_empty());
    }

    #[test]
    fn test_cdx_001_empty_string() {
        let diagnostics = validate_config("approvalMode = \"\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
    }

    #[test]
    fn test_cdx_001_case_sensitive() {
        let diagnostics = validate_config("approvalMode = \"Suggest\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1, "approvalMode should be case-sensitive");
    }

    #[test]
    fn test_cdx_001_autofix_case_insensitive() {
        // "Suggest" is a case-insensitive match to "suggest"
        let diagnostics = validate_config("approvalMode = \"Suggest\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        assert!(
            cdx_001[0].has_fixes(),
            "CDX-001 should have auto-fix for case mismatch"
        );
        let fix = &cdx_001[0].fixes[0];
        assert!(!fix.safe, "CDX-001 fix should be unsafe");
        assert_eq!(fix.replacement, "suggest", "Fix should suggest 'suggest'");
    }

    #[test]
    fn test_cdx_001_no_autofix_when_duplicate() {
        // The regex pattern `approvalMode\s*=\s*"Suggest"` appears twice in this
        // valid TOML because a [section] table also uses the same key name.
        // The uniqueness guard should prevent autofix when there are multiple matches.
        let content = "approvalMode = \"Suggest\"\n\n[overrides]\napprovalMode = \"Suggest\"";
        let diagnostics = validate_config(content);
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        assert!(
            !cdx_001[0].has_fixes(),
            "CDX-001 should not have auto-fix when value pattern appears multiple times"
        );
    }

    #[test]
    fn test_cdx_001_no_autofix_nonsense() {
        let diagnostics = validate_config("approvalMode = \"yolo\"");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        // "yolo" has no close match - should NOT get a fix
        assert!(
            !cdx_001[0].has_fixes(),
            "CDX-001 should not auto-fix nonsense values"
        );
    }

    #[test]
    fn test_cdx_001_autofix_targets_correct_bytes() {
        let content = "approvalMode = \"Suggest\"";
        let diagnostics = validate_config(content);
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        assert!(cdx_001[0].has_fixes());
        let fix = &cdx_001[0].fixes[0];
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "Suggest", "Fix should target the inner value");
    }

    #[test]
    fn test_cdx_001_type_mismatch() {
        let diagnostics = validate_config("approvalMode = true");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        assert!(cdx_001[0].message.contains("string"));
    }

    #[test]
    fn test_cdx_001_type_mismatch_number() {
        let diagnostics = validate_config("approvalMode = 42");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
    }

    #[test]
    fn test_cdx_001_type_mismatch_float() {
        let diagnostics = validate_config("approvalMode = 1.5");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        assert!(
            cdx_001[0].message.contains("string"),
            "Expected type error message for float value"
        );
    }

    #[test]
    fn test_cdx_001_type_mismatch_array() {
        let diagnostics = validate_config("approvalMode = [\"suggest\"]");
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        assert!(
            cdx_001[0].message.contains("string"),
            "Expected type error message for array value"
        );
    }

    #[test]
    fn test_cdx_001_line_number() {
        let content = "model = \"o4-mini\"\napprovalMode = \"invalid\"";
        let diagnostics = validate_config(content);
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert_eq!(cdx_001.len(), 1);
        assert_eq!(cdx_001[0].line, 2);
    }

    // ===== CDX-002: Invalid fullAutoErrorMode =====

    #[test]
    fn test_cdx_002_invalid_error_mode() {
        let diagnostics = validate_config("fullAutoErrorMode = \"crash\"");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(cdx_002.len(), 1);
        assert_eq!(cdx_002[0].level, DiagnosticLevel::Error);
        assert!(cdx_002[0].message.contains("crash"));
    }

    #[test]
    fn test_cdx_002_valid_ask_user() {
        let diagnostics = validate_config("fullAutoErrorMode = \"ask-user\"");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert!(cdx_002.is_empty());
    }

    #[test]
    fn test_cdx_002_valid_ignore_and_continue() {
        let diagnostics = validate_config("fullAutoErrorMode = \"ignore-and-continue\"");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert!(cdx_002.is_empty());
    }

    #[test]
    fn test_cdx_002_all_valid_modes() {
        for mode in VALID_FULL_AUTO_ERROR_MODES {
            let content = format!("fullAutoErrorMode = \"{}\"", mode);
            let diagnostics = validate_config(&content);
            let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
            assert!(cdx_002.is_empty(), "Mode '{}' should be valid", mode);
        }
    }

    #[test]
    fn test_cdx_002_absent_error_mode() {
        let diagnostics = validate_config("model = \"o4-mini\"");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert!(cdx_002.is_empty());
    }

    #[test]
    fn test_cdx_002_empty_string() {
        let diagnostics = validate_config("fullAutoErrorMode = \"\"");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(cdx_002.len(), 1);
    }

    #[test]
    fn test_cdx_002_autofix_case_insensitive() {
        // "Ask-User" is a case-insensitive match to "ask-user"
        let diagnostics = validate_config("fullAutoErrorMode = \"Ask-User\"");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(cdx_002.len(), 1);
        assert!(
            cdx_002[0].has_fixes(),
            "CDX-002 should have auto-fix for case mismatch"
        );
        let fix = &cdx_002[0].fixes[0];
        assert!(!fix.safe, "CDX-002 fix should be unsafe");
        assert_eq!(fix.replacement, "ask-user", "Fix should suggest 'ask-user'");
    }

    #[test]
    fn test_cdx_002_no_autofix_nonsense() {
        let diagnostics = validate_config("fullAutoErrorMode = \"crash\"");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(cdx_002.len(), 1);
        // "crash" has no close match - should NOT get a fix
        assert!(
            !cdx_002[0].has_fixes(),
            "CDX-002 should not auto-fix nonsense values"
        );
    }

    #[test]
    fn test_cdx_002_autofix_targets_correct_bytes() {
        let content = "fullAutoErrorMode = \"Ask-User\"";
        let diagnostics = validate_config(content);
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(cdx_002.len(), 1);
        assert!(cdx_002[0].has_fixes());
        let fix = &cdx_002[0].fixes[0];
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "Ask-User", "Fix should target the inner value");
    }

    #[test]
    fn test_cdx_002_type_mismatch() {
        let diagnostics = validate_config("fullAutoErrorMode = false");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(cdx_002.len(), 1);
        assert!(cdx_002[0].message.contains("string"));
    }

    #[test]
    fn test_cdx_002_case_sensitive() {
        let diagnostics = validate_config("fullAutoErrorMode = \"Ask-User\"");
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(
            cdx_002.len(),
            1,
            "fullAutoErrorMode should be case-sensitive"
        );
    }

    #[test]
    fn test_cdx_002_line_number() {
        let content = "model = \"o4-mini\"\nfullAutoErrorMode = \"crash\"";
        let diagnostics = validate_config(content);
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(cdx_002.len(), 1);
        assert_eq!(cdx_002[0].line, 2);
    }

    // ===== CDX-003: AGENTS.override.md in version control =====

    #[test]
    fn test_cdx_003_agents_override_md() {
        let diagnostics = validate_claude_md("AGENTS.override.md", "# Override");
        let cdx_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-003").collect();
        assert_eq!(cdx_003.len(), 1);
        assert_eq!(cdx_003[0].level, DiagnosticLevel::Warning);
        assert!(cdx_003[0].message.contains("AGENTS.override.md"));
    }

    #[test]
    fn test_cdx_003_not_triggered_on_claude_md() {
        let diagnostics = validate_claude_md("CLAUDE.md", "# My project");
        let cdx_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-003").collect();
        assert!(cdx_003.is_empty());
    }

    #[test]
    fn test_cdx_003_not_triggered_on_agents_md() {
        let diagnostics = validate_claude_md("AGENTS.md", "# My project");
        let cdx_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-003").collect();
        assert!(cdx_003.is_empty());
    }

    #[test]
    fn test_cdx_003_case_sensitive_extension() {
        // AGENTS.override.MD (wrong extension case) should NOT trigger CDX-003
        let diagnostics = validate_claude_md("AGENTS.override.MD", "# test");
        let cdx_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-003").collect();
        assert!(
            cdx_003.is_empty(),
            "CDX-003 should not fire for AGENTS.override.MD"
        );
    }

    #[test]
    fn test_cdx_003_not_triggered_on_config() {
        let diagnostics = validate_config("approvalMode = \"suggest\"");
        let cdx_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-003").collect();
        assert!(cdx_003.is_empty());
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_codex_category() {
        let mut config = LintConfig::default();
        config.rules_mut().codex = false;

        let diagnostics = validate_config_with_config("approvalMode = \"invalid\"", &config);
        let cdx_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CDX-"))
            .collect();
        assert!(cdx_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CDX-001".to_string()];

        let diagnostics = validate_config_with_config("approvalMode = \"invalid\"", &config);
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        assert!(cdx_001.is_empty());
    }

    #[test]
    fn test_all_cdx_rules_can_be_disabled() {
        let rules = [
            "CDX-001", "CDX-002", "CDX-003", "CDX-004", "CDX-005", "CDX-006",
        ];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];

            let (content, path): (&str, &str) = match rule {
                "CDX-001" => ("approvalMode = \"invalid\"", ".codex/config.toml"),
                "CDX-002" => ("fullAutoErrorMode = \"crash\"", ".codex/config.toml"),
                "CDX-003" => ("# Override", "AGENTS.override.md"),
                "CDX-004" => ("totally_unknown_key = true", ".codex/config.toml"),
                "CDX-005" => ("project_doc_max_bytes = 999999", ".codex/config.toml"),
                "CDX-006" => (
                    "project_doc_fallback_filenames = [\"AGENTS.md\", \"AGENTS.md\"]",
                    ".codex/config.toml",
                ),
                _ => unreachable!(),
            };

            let validator = CodexValidator;
            let diagnostics = validator.validate(Path::new(path), content, &config);

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
        let content = r#"
model = "o4-mini"
approvalMode = "suggest"
fullAutoErrorMode = "ask-user"
notify = true
"#;
        let diagnostics = validate_config(content);
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_empty_config_no_issues() {
        let diagnostics = validate_config("");
        assert!(diagnostics.is_empty());
    }

    // ===== Multiple Issues =====

    #[test]
    fn test_multiple_issues() {
        let content = "approvalMode = \"yolo\"\nfullAutoErrorMode = \"crash\"";
        let diagnostics = validate_config(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-001"));
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-002"));
    }

    #[test]
    fn test_cdx_002_empty_with_cdx_001_invalid() {
        // Both CDX-001 (invalid value) and CDX-002 (empty string) should fire together
        let content = "approvalMode = \"invalid\"\nfullAutoErrorMode = \"\"";
        let diagnostics = validate_config(content);
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(
            cdx_001.len(),
            1,
            "CDX-001 should fire for invalid approvalMode"
        );
        assert_eq!(
            cdx_002.len(),
            1,
            "CDX-002 should fire for empty fullAutoErrorMode"
        );
    }

    #[test]
    fn test_both_fields_wrong_type() {
        let content = "approvalMode = true\nfullAutoErrorMode = 123";
        let diagnostics = validate_config(content);
        let cdx_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-001").collect();
        let cdx_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-002").collect();
        assert_eq!(
            cdx_001.len(),
            1,
            "CDX-001 should fire for wrong-type approvalMode"
        );
        assert_eq!(
            cdx_002.len(),
            1,
            "CDX-002 should fire for wrong-type fullAutoErrorMode"
        );
        assert!(cdx_001[0].message.contains("string"));
        assert!(cdx_002[0].message.contains("string"));
    }

    // ===== CDX-004: Unknown config keys =====

    #[test]
    fn test_cdx_004_unknown_key() {
        let diagnostics = validate_config("totally_unknown_key = true");
        let cdx_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-004").collect();
        assert_eq!(cdx_004.len(), 1);
        assert_eq!(cdx_004[0].level, DiagnosticLevel::Warning);
        assert!(cdx_004[0].message.contains("totally_unknown_key"));
    }

    #[test]
    fn test_cdx_004_known_keys_no_warning() {
        let content = r#"
model = "o4-mini"
approvalMode = "suggest"
fullAutoErrorMode = "ask-user"
notify = true
project_doc_max_bytes = 32768
project_doc_fallback_filenames = ["AGENTS.md", "README.md"]
"#;
        let diagnostics = validate_config(content);
        let cdx_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-004").collect();
        assert!(cdx_004.is_empty(), "Known keys should not trigger CDX-004");
    }

    #[test]
    fn test_cdx_004_multiple_unknown_keys() {
        let content = "unknown_a = true\nunknown_b = false\nmodel = \"o4-mini\"";
        let diagnostics = validate_config(content);
        let cdx_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-004").collect();
        assert_eq!(cdx_004.len(), 2);
    }

    #[test]
    fn test_cdx_004_line_number() {
        let content = "model = \"o4-mini\"\nmy_custom_key = true";
        let diagnostics = validate_config(content);
        let cdx_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-004").collect();
        assert_eq!(cdx_004.len(), 1);
        assert_eq!(cdx_004[0].line, 2);
    }

    #[test]
    fn test_cdx_004_table_keys_not_flagged() {
        let content = r#"
model = "o4-mini"

[mcp_servers]
name = "test"
"#;
        let diagnostics = validate_config(content);
        let cdx_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-004").collect();
        assert!(
            cdx_004.is_empty(),
            "Known table keys should not trigger CDX-004"
        );
    }

    #[test]
    fn test_cdx_004_has_suggestion() {
        let diagnostics = validate_config("bogus_setting = 42");
        let cdx_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-004").collect();
        assert_eq!(cdx_004.len(), 1);
        assert!(
            cdx_004[0].suggestion.is_some(),
            "CDX-004 should have a suggestion"
        );
    }

    // ===== CDX-005: project_doc_max_bytes exceeds limit =====

    #[test]
    fn test_cdx_005_exceeds_limit() {
        let diagnostics = validate_config("project_doc_max_bytes = 100000");
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert_eq!(cdx_005.len(), 1);
        assert_eq!(cdx_005[0].level, DiagnosticLevel::Error);
        assert!(cdx_005[0].message.contains("100000"));
    }

    #[test]
    fn test_cdx_005_at_limit() {
        let diagnostics = validate_config("project_doc_max_bytes = 65536");
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert!(cdx_005.is_empty(), "65536 is at the limit, should be valid");
    }

    #[test]
    fn test_cdx_005_below_limit() {
        let diagnostics = validate_config("project_doc_max_bytes = 32768");
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert!(cdx_005.is_empty(), "32768 is below limit, should be valid");
    }

    #[test]
    fn test_cdx_005_just_over_limit() {
        let diagnostics = validate_config("project_doc_max_bytes = 65537");
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert_eq!(cdx_005.len(), 1, "65537 exceeds 65536 limit");
    }

    #[test]
    fn test_cdx_005_wrong_type() {
        let diagnostics = validate_config("project_doc_max_bytes = \"not a number\"");
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert_eq!(cdx_005.len(), 1);
        assert!(cdx_005[0].message.contains("integer"));
    }

    #[test]
    fn test_cdx_005_absent() {
        let diagnostics = validate_config("model = \"o4-mini\"");
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert!(
            cdx_005.is_empty(),
            "Absent field should not trigger CDX-005"
        );
    }

    #[test]
    fn test_cdx_005_line_number() {
        let content = "model = \"o4-mini\"\nproject_doc_max_bytes = 999999";
        let diagnostics = validate_config(content);
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert_eq!(cdx_005.len(), 1);
        assert_eq!(cdx_005[0].line, 2);
    }

    #[test]
    fn test_cdx_005_has_suggestion() {
        let diagnostics = validate_config("project_doc_max_bytes = 100000");
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert_eq!(cdx_005.len(), 1);
        assert!(
            cdx_005[0].suggestion.is_some(),
            "CDX-005 should have a suggestion"
        );
    }

    #[test]
    fn test_cdx_005_negative_value() {
        // Negative values are invalid because project_doc_max_bytes must be a positive integer
        let diagnostics = validate_config("project_doc_max_bytes = -1");
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert_eq!(cdx_005.len(), 1, "Negative values should trigger CDX-005");
        assert!(
            cdx_005[0].message.contains("positive integer"),
            "Error message should indicate positive integer requirement"
        );
    }

    #[test]
    fn test_cdx_005_zero_value() {
        // Zero is invalid because project_doc_max_bytes must be a positive integer
        let diagnostics = validate_config("project_doc_max_bytes = 0");
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert_eq!(cdx_005.len(), 1, "Zero should trigger CDX-005");
        assert!(
            cdx_005[0].message.contains("positive integer"),
            "Error message should indicate positive integer requirement"
        );
    }

    #[test]
    fn test_cdx_004_and_cdx_005_combined() {
        // Both an unknown key and exceeding limit in the same file
        let content = "unknown_key = true\nproject_doc_max_bytes = 999999";
        let diagnostics = validate_config(content);
        let cdx_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-004").collect();
        let cdx_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-005").collect();
        assert_eq!(cdx_004.len(), 1, "CDX-004 should fire for unknown_key");
        assert_eq!(cdx_005.len(), 1, "CDX-005 should fire for exceeding limit");
    }

    // ===== CDX-006: project_doc_fallback_filenames validation =====

    #[test]
    fn test_cdx_006_valid_fallback_filenames() {
        let diagnostics =
            validate_config("project_doc_fallback_filenames = [\"AGENTS.md\", \"README.md\"]");
        let cdx_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-006").collect();
        assert!(
            cdx_006.is_empty(),
            "Valid fallback filenames should not trigger CDX-006"
        );
    }

    #[test]
    fn test_cdx_006_wrong_type() {
        let diagnostics = validate_config("project_doc_fallback_filenames = \"AGENTS.md\"");
        let cdx_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-006").collect();
        assert_eq!(cdx_006.len(), 1);
        assert_eq!(cdx_006[0].level, DiagnosticLevel::Error);
        assert!(cdx_006[0].message.contains("must be an array"));
    }

    #[test]
    fn test_cdx_006_non_string_entries() {
        let diagnostics = validate_config("project_doc_fallback_filenames = [\"AGENTS.md\", 42]");
        let cdx_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-006").collect();
        assert_eq!(cdx_006.len(), 1);
        assert_eq!(cdx_006[0].level, DiagnosticLevel::Error);
        assert!(cdx_006[0].message.contains("index 2"));
    }

    #[test]
    fn test_cdx_006_empty_entry() {
        let diagnostics = validate_config("project_doc_fallback_filenames = [\"\", \"AGENTS.md\"]");
        let cdx_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-006").collect();
        assert_eq!(cdx_006.len(), 1);
        assert_eq!(cdx_006[0].level, DiagnosticLevel::Error);
        assert!(cdx_006[0].message.contains("index 1"));
    }

    #[test]
    fn test_cdx_006_duplicate_entry_warns() {
        let diagnostics = validate_config(
            "project_doc_fallback_filenames = [\"AGENTS.md\", \"README.md\", \"AGENTS.md\"]",
        );
        let cdx_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-006").collect();
        assert_eq!(cdx_006.len(), 1);
        assert_eq!(cdx_006[0].level, DiagnosticLevel::Warning);
        assert!(cdx_006[0].message.contains("duplicate"));
    }

    #[test]
    fn test_cdx_006_suspicious_path_warns() {
        let diagnostics = validate_config(
            "project_doc_fallback_filenames = [\"AGENTS.md\", \"docs/AGENTS.md\", \"C:/tmp/a.md\"]",
        );
        let cdx_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-006").collect();
        assert_eq!(cdx_006.len(), 2);
        assert!(cdx_006.iter().all(|d| d.level == DiagnosticLevel::Warning));
        assert!(cdx_006.iter().any(|d| d.message.contains("docs/AGENTS.md")));
        assert!(cdx_006.iter().any(|d| d.message.contains("C:/tmp/a.md")));
    }

    #[test]
    fn test_cdx_006_line_number() {
        let content = "model = \"o4-mini\"\nproject_doc_fallback_filenames = [\"\"]";
        let diagnostics = validate_config(content);
        let cdx_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-006").collect();
        assert_eq!(cdx_006.len(), 1);
        assert_eq!(cdx_006[0].line, 2);
    }

    #[test]
    fn test_cdx_006_has_suggestion() {
        let diagnostics = validate_config("project_doc_fallback_filenames = [\"\"]");
        let cdx_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-006").collect();
        assert_eq!(cdx_006.len(), 1);
        assert!(
            cdx_006[0].suggestion.is_some(),
            "CDX-006 should have a suggestion"
        );
    }

    // ===== Fixture Integration =====

    #[test]
    fn test_valid_codex_fixture_no_diagnostics() {
        let fixture = include_str!("../../../../tests/fixtures/codex/.codex/config.toml");
        let diagnostics = validate_config(fixture);
        assert!(
            diagnostics.is_empty(),
            "Valid codex fixture should produce 0 diagnostics, got: {:?}",
            diagnostics
        );
    }

    // ===== find_key_line =====

    #[test]
    fn test_find_key_line() {
        let content =
            "model = \"o4-mini\"\napprovalMode = \"suggest\"\nfullAutoErrorMode = \"ask-user\"";
        assert_eq!(find_key_line(content, "model"), Some(1));
        assert_eq!(find_key_line(content, "approvalMode"), Some(2));
        assert_eq!(find_key_line(content, "fullAutoErrorMode"), Some(3));
        assert_eq!(find_key_line(content, "nonexistent"), None);
    }

    #[test]
    fn test_find_key_line_ignores_value_match() {
        // "approvalMode" appears as part of a string value, not as a key
        let content = "comment = \"the approvalMode field\"\napprovalMode = \"suggest\"";
        assert_eq!(find_key_line(content, "approvalMode"), Some(2));
    }

    #[test]
    fn test_find_key_line_at_start_of_content() {
        // Key on the very first line with no preceding content
        let content = "approvalMode = \"suggest\"";
        assert_eq!(find_key_line(content, "approvalMode"), Some(1));
    }

    #[test]
    fn test_find_key_line_with_leading_whitespace() {
        // Key with leading whitespace (indented)
        let content = "  approvalMode = \"suggest\"";
        assert_eq!(find_key_line(content, "approvalMode"), Some(1));
    }

    #[test]
    fn test_find_key_line_no_partial_match() {
        // "approvalMode" must not match "approvalModeExtra"
        let content = "approvalModeExtra = \"value\"\napprovalMode = \"suggest\"";
        assert_eq!(find_key_line(content, "approvalMode"), Some(2));
    }

    // ===== build_key_line_map =====

    #[test]
    fn test_build_key_line_map() {
        let content =
            "model = \"o4-mini\"\napprovalMode = \"suggest\"\nfullAutoErrorMode = \"ask-user\"";
        let map = build_key_line_map(content);
        assert_eq!(map.get("model"), Some(&1));
        assert_eq!(map.get("approvalMode"), Some(&2));
        assert_eq!(map.get("fullAutoErrorMode"), Some(&3));
        assert_eq!(map.get("nonexistent"), None);
    }

    #[test]
    fn test_build_key_line_map_no_partial_match() {
        let content = "approvalModeExtra = \"value\"\napprovalMode = \"suggest\"";
        let map = build_key_line_map(content);
        assert_eq!(map.get("approvalModeExtra"), Some(&1));
        assert_eq!(map.get("approvalMode"), Some(&2));
    }

    // ===== CDX-000 suggestion test =====

    #[test]
    fn test_cdx_000_has_suggestion() {
        let content = "this is not valid toml {{{}}}";
        let diagnostics = validate_config(content);

        let cdx_000: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-000").collect();
        assert_eq!(cdx_000.len(), 1);
        assert!(
            cdx_000[0].suggestion.is_some(),
            "CDX-000 should have a suggestion"
        );
        assert!(
            cdx_000[0]
                .suggestion
                .as_ref()
                .unwrap()
                .contains("TOML syntax"),
            "CDX-000 suggestion should mention TOML syntax"
        );
    }

    #[test]
    fn test_cdx_000_uses_localized_message() {
        let content = "this is not valid toml {{{}}}";
        let diagnostics = validate_config(content);

        let cdx_000: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-000").collect();
        assert_eq!(cdx_000.len(), 1);
        assert!(
            cdx_000[0]
                .message
                .contains("Failed to parse .codex/config.toml as TOML"),
            "CDX-000 message should use localized text, got: {}",
            cdx_000[0].message
        );
    }

    // ===== Autofix Tests =====

    #[test]
    fn test_cdx_004_has_fix() {
        let diagnostics = validate_config("totally_unknown_key = true");
        let cdx_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-004").collect();
        assert_eq!(cdx_004.len(), 1);
        assert!(cdx_004[0].has_fixes(), "CDX-004 should have fix");
        assert!(!cdx_004[0].fixes[0].safe, "CDX-004 fix should be unsafe");
        assert!(cdx_004[0].fixes[0].is_deletion());
    }

    #[test]
    fn test_cdx_004_fix_application() {
        let content = "model = \"o4-mini\"\ntotally_unknown_key = true\nnotify = true";
        let diagnostics = validate_config(content);
        let cdx_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CDX-004").collect();
        assert_eq!(cdx_004.len(), 1);
        assert!(cdx_004[0].has_fixes());
        let fix = &cdx_004[0].fixes[0];
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        assert!(!fixed.contains("totally_unknown_key"));
        assert!(fixed.contains("model"));
        assert!(fixed.contains("notify"));
    }

    // ===== CDX-CFG / CDX-AG / CDX-APP =====

    #[test]
    fn test_cdx_cfg_001_invalid_approval_policy() {
        let diagnostics = validate_config("approval_policy = \"always\"");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-001"));
    }

    #[test]
    fn test_cdx_cfg_002_invalid_sandbox_mode() {
        let diagnostics = validate_config("sandbox_mode = \"open\"");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-002"));
    }

    #[test]
    fn test_cdx_cfg_003_invalid_reasoning_effort() {
        let diagnostics = validate_config("model_reasoning_effort = \"turbo\"");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-003"));
    }

    #[test]
    fn test_cdx_cfg_004_invalid_model_verbosity() {
        let diagnostics = validate_config("model_verbosity = \"verbose\"");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-004"));
    }

    #[test]
    fn test_cdx_cfg_005_invalid_personality() {
        let diagnostics = validate_config("personality = \"assistant\"");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-005"));
    }

    #[test]
    fn test_cdx_cfg_006_unknown_nested_key() {
        let diagnostics = validate_config("[features]\nunknown_flag = true");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-006"));
    }

    #[test]
    fn test_cdx_cfg_007_danger_full_access_without_ack() {
        let diagnostics = validate_config("sandbox_mode = \"danger-full-access\"");
        let cdx_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CDX-CFG-007")
            .collect();
        assert_eq!(cdx_007.len(), 1);
        assert_eq!(cdx_007[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_cdx_cfg_008_invalid_shell_environment_inherit() {
        let diagnostics = validate_config("[shell_environment_policy]\ninherit = \"system\"");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-008"));
    }

    #[test]
    fn test_cdx_cfg_009_invalid_mcp_server_shape() {
        let diagnostics = validate_config("[mcp_servers.local]\nenabled = true");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-009"));
    }

    #[test]
    fn test_cdx_cfg_010_detects_hardcoded_secret() {
        let diagnostics = validate_config("api_key = \"sk-test-secret-value-123456\"");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-010"));
    }

    #[test]
    fn test_cdx_cfg_011_invalid_feature_flag_name() {
        let diagnostics = validate_config("[features]\nthis_flag_does_not_exist = true");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-011"));
    }

    #[test]
    fn test_cdx_cfg_012_invalid_cli_auth_store() {
        let diagnostics = validate_config("cli_auth_credentials_store = \"vault\"");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-012"));
    }

    #[test]
    fn test_cdx_ag_rules_on_agents_md() {
        let empty = validate_claude_md("AGENTS.md", "");
        assert!(empty.iter().any(|d| d.rule == "CDX-AG-001"));

        let secret = validate_claude_md("AGENTS.md", "api_key = sk-secret-value-123456");
        assert!(secret.iter().any(|d| d.rule == "CDX-AG-002"));

        let no_false_positive = validate_claude_md(
            "AGENTS.md",
            "Use task-runner and ask-for-help in local workflows.",
        );
        assert!(!no_false_positive.iter().any(|d| d.rule == "CDX-AG-002"));

        // Issue #659: prose containing "token" or "secret" should not trigger
        let prose_token = validate_claude_md(
            "AGENTS.md",
            "8. **Token efficiency** - Save tokens over decorations.",
        );
        assert!(
            !prose_token.iter().any(|d| d.rule == "CDX-AG-002"),
            "Prose word 'Token' should not trigger secret detection"
        );

        // But keyword + assignment should still trigger
        let real_secret = validate_claude_md("AGENTS.md", "token = sk-real-secret-123");
        assert!(
            real_secret.iter().any(|d| d.rule == "CDX-AG-002"),
            "Keyword in assignment context should still trigger"
        );

        let generic = validate_claude_md("AGENTS.md", "Be helpful and accurate.");
        assert!(generic.iter().any(|d| d.rule == "CDX-AG-003"));
    }

    #[test]
    fn test_cdx_app_001_invalid_default_tools_approval_mode() {
        let diagnostics = validate_config(
            "[apps.my_app]\nenabled = true\ndefault_tools_approval_mode = \"manual\"",
        );
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-APP-001"));
    }

    #[test]
    fn test_cdx_cfg_013_invalid_sandbox_workspace_write_mode() {
        let diagnostics = validate_config("[sandbox_workspace_write]\nmode = \"everything\"");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-013"));
    }

    #[test]
    fn test_cdx_cfg_014_invalid_model_type() {
        let diagnostics = validate_config("model = 123");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-014"));
    }

    #[test]
    fn test_cdx_cfg_014_valid_model_string_no_diagnostic() {
        let diagnostics = validate_config("model = \"o4-mini\"");
        assert!(diagnostics.iter().all(|d| d.rule != "CDX-CFG-014"));
    }

    #[test]
    fn test_cdx_cfg_015_invalid_model_provider_type() {
        let diagnostics = validate_config("model_provider = 42");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-015"));
    }

    #[test]
    fn test_cdx_cfg_016_invalid_model_reasoning_summary() {
        let diagnostics = validate_config("model_reasoning_summary = \"verbose\"");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-016"));
    }

    #[test]
    fn test_cdx_cfg_017_invalid_history_type() {
        let diagnostics = validate_config("history = \"not an object\"");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-017"));
    }

    #[test]
    fn test_cdx_cfg_018_invalid_tui_type() {
        let diagnostics = validate_config("tui = \"not an object\"");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-018"));
    }

    #[test]
    fn test_cdx_cfg_019_invalid_file_opener_type() {
        let diagnostics = validate_config("file_opener = 123");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-019"));
    }

    #[test]
    fn test_cdx_cfg_020_invalid_mcp_oauth_store() {
        let diagnostics = validate_config("mcp_oauth_credentials_store = \"vault\"");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-020"));
    }

    #[test]
    fn test_cdx_cfg_021_invalid_model_context_window() {
        let diagnostics = validate_config("model_context_window = -1");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-021"));
    }

    #[test]
    fn test_cdx_cfg_021_valid_model_context_window_no_diagnostic() {
        let diagnostics = validate_config("model_context_window = 128000");
        assert!(diagnostics.iter().all(|d| d.rule != "CDX-CFG-021"));
    }

    #[test]
    fn test_cdx_cfg_022_invalid_model_auto_compact_token_limit() {
        let diagnostics = validate_config("model_auto_compact_token_limit = -100");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-CFG-022"));
    }

    #[test]
    fn test_cdx_cfg_022_valid_limit_no_diagnostic() {
        let diagnostics = validate_config("model_auto_compact_token_limit = 4096");
        assert!(diagnostics.iter().all(|d| d.rule != "CDX-CFG-022"));
    }

    #[test]
    fn test_cdx_ag_004_agents_md_exceeds_size_limit() {
        let content = "# Project\n".to_string() + &"x".repeat(200_000);
        let diagnostics = validate_claude_md("AGENTS.md", &content);
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-AG-004"));
    }

    #[test]
    fn test_cdx_ag_005_missing_file_reference() {
        let diagnostics =
            validate_claude_md("AGENTS.md", "Check `nonexistent-file.ts` for details.");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-AG-005"));
    }

    #[test]
    fn test_cdx_ag_006_missing_project_context() {
        let diagnostics = validate_claude_md("AGENTS.md", "Be helpful and write good code.");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-AG-006"));
    }

    #[test]
    fn test_cdx_ag_006_has_context_no_diagnostic() {
        let diagnostics = validate_claude_md(
            "AGENTS.md",
            "# Project\nRun `npm test` to verify. Check src/ for source code.",
        );
        assert!(diagnostics.iter().all(|d| d.rule != "CDX-AG-006"));
    }

    #[test]
    fn test_cdx_ag_007_placeholder_no_diagnostics() {
        // CDX-AG-007 is a placeholder for cross-file analysis; it should not produce diagnostics
        let diagnostics = validate_claude_md(
            "AGENTS.md",
            "# Project\nRun `npm test` to verify. Check src/ for source code.",
        );
        assert!(diagnostics.iter().all(|d| d.rule != "CDX-AG-007"));
    }

    #[test]
    fn test_cdx_app_002_invalid_skills_type() {
        let diagnostics = validate_config("skills = \"not an object\"");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-APP-002"));
    }

    #[test]
    fn test_cdx_app_002_valid_skills_no_diagnostic() {
        let diagnostics = validate_config("[skills]");
        assert!(diagnostics.iter().all(|d| d.rule != "CDX-APP-002"));
    }

    #[test]
    fn test_cdx_app_003_invalid_profile_type() {
        let diagnostics = validate_config("profile = 123");
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-APP-003"));
    }

    #[test]
    fn test_cdx_app_003_valid_profile_no_diagnostic() {
        let diagnostics = validate_config("profile = \"default\"");
        assert!(diagnostics.iter().all(|d| d.rule != "CDX-APP-003"));
    }

    // M10: Comprehensive negative test - valid config values don't trigger CDX-CFG rules
    #[test]
    fn test_cdx_cfg_negative_all_valid_values() {
        let content = r#"
model = "o4-mini"
approval_policy = "on-request"
sandbox_mode = "workspace-write"
model_reasoning_effort = "medium"
model_verbosity = "medium"
personality = "pragmatic"
model_reasoning_summary = "auto"
cli_auth_credentials_store = "auto"
mcp_oauth_credentials_store = "auto"
model_context_window = 128000
model_auto_compact_token_limit = 50000
profile = "default"
file_opener = "code"

[sandbox_workspace_write]
mode = "allowlist"

[shell_environment_policy]
inherit = "core"

[history]
max_entries = 100

[tui]
theme = "dark"

[features]
web_search = true

[skills]
enabled = true

[mcp_servers.local]
command = "node"
args = ["server.js"]
url = "https://example.com"
"#;
        let diagnostics = validate_config(content);
        let cdx_cfg: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CDX-CFG-"))
            .collect();
        assert!(
            cdx_cfg.is_empty(),
            "No CDX-CFG rules should fire for valid config, got: {:?}",
            cdx_cfg.iter().map(|d| &d.rule).collect::<Vec<_>>()
        );
    }

    // M11: CDX-AG-002 interpolation exclusion
    #[test]
    fn test_cdx_ag_002_interpolation_not_flagged() {
        let diagnostics = validate_claude_md("AGENTS.md", "api_key=${MY_SECRET}");
        assert!(diagnostics.iter().all(|d| d.rule != "CDX-AG-002"));
    }

    // M12: CDX-CFG-007 suppression with hide_full_access_warning
    #[test]
    fn test_cdx_cfg_007_suppressed_with_hide_warning() {
        let content = r#"
sandbox_mode = "danger-full-access"

[notice]
hide_full_access_warning = true
"#;
        let diagnostics = validate_config(content);
        assert!(diagnostics.iter().all(|d| d.rule != "CDX-CFG-007"));
    }

    // M13: CDX-CFG-010 env-var exclusion
    #[test]
    fn test_cdx_cfg_010_env_var_not_flagged() {
        let content = r#"api_key = "${MY_SECRET}""#;
        let diagnostics = validate_config(content);
        assert!(diagnostics.iter().all(|d| d.rule != "CDX-CFG-010"));
    }

    #[test]
    fn test_cdx_cfg_rules_parse_json_and_yaml_configs() {
        let json = r#"{"approval_policy":"always"}"#;
        let json_diags = validate_config_at_path(".codex/config.json", json);
        assert!(json_diags.iter().any(|d| d.rule == "CDX-CFG-001"));

        let yaml = "approval_policy: always\n";
        let yaml_diags = validate_config_at_path(".codex/config.yaml", yaml);
        assert!(yaml_diags.iter().any(|d| d.rule == "CDX-CFG-001"));
    }

    #[test]
    fn test_cdx_000_reports_json_yaml_parse_errors() {
        let invalid_json = r#"{"approval_policy":"always""#;
        let json_diags = validate_config_at_path(".codex/config.json", invalid_json);
        assert!(json_diags.iter().any(|d| d.rule == "CDX-000"));

        let invalid_yaml = "approval_policy: [always\n";
        let yaml_diags = validate_config_at_path(".codex/config.yaml", invalid_yaml);
        assert!(yaml_diags.iter().any(|d| d.rule == "CDX-000"));
    }
}
