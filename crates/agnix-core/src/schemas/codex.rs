//! Codex CLI configuration file schema helpers
//!
//! Provides parsing and validation for `.codex/config.toml` configuration files.
//!
//! Validates:
//! - `approvalMode` field values (suggest, auto-edit, full-auto)
//! - `fullAutoErrorMode` field values (ask-user, ignore-and-continue)
//! - Unknown config keys (CDX-004)
//! - `project_doc_max_bytes` limits (CDX-005)
//! - `project_doc_fallback_filenames` shape/content (CDX-006)

use serde::{Deserialize, Serialize};

/// Valid values for the `approvalMode` field
pub const VALID_APPROVAL_MODES: &[&str] = &["suggest", "auto-edit", "full-auto"];

/// Valid values for the `fullAutoErrorMode` field
pub const VALID_FULL_AUTO_ERROR_MODES: &[&str] = &["ask-user", "ignore-and-continue"];

/// Valid values for `sandbox_workspace_write.mode`
pub const VALID_SANDBOX_WORKSPACE_WRITE_MODES: &[&str] = &["allowlist", "denylist", "all"];

/// Valid values for `model_reasoning_summary`
pub const VALID_MODEL_REASONING_SUMMARIES: &[&str] =
    &["auto", "always", "none", "concise", "detailed"];

/// Valid values for `mcp_oauth_credentials_store`
pub const VALID_MCP_OAUTH_STORES: &[&str] = &["file", "keyring", "auto", "ephemeral"];

/// Maximum allowed size (bytes) for AGENTS.md in Codex projects
pub const AGENTS_MD_MAX_SIZE: usize = 100_000;

/// Known valid top-level keys for .codex/config.toml
///
/// Sourced from the upstream JSON schema at
/// `codex-rs/core/config.schema.json`; current baseline is `rust-v0.142.0`
/// (see `.github/tool-release-baselines.json`). Prose overview:
/// <https://developers.openai.com/codex/>.
///
/// When catching up to a new Codex release, regenerate this list by diffing
/// `codex-rs/core/config.schema.json` against these constants.
///
/// The list is intentionally a lenient superset, with two classes of entries
/// that are *not* in the current schema and are kept deliberately:
/// 1. Older-version tolerance - keys a real version once shipped but a newer
///    schema dropped (e.g. `commit_attribution`,
///    `experimental_use_freeform_apply_patch`, `windows_wsl_setup_acknowledged`,
///    `experimental_thread_store_endpoint`, and `zsh_path` were in earlier
///    releases and later removed by `rust-v0.133.0` / `rust-v0.136.0`).
/// 2. Legacy camelCase keys (`approvalMode`, `fullAutoErrorMode`) - never in the
///    (snake_case) schema, accepted from early Codex configs.
///
/// Leniency only weakens typo detection, never false-positives. Keys that no
/// audited schema ever contained and have no such backwards-compat reason are
/// removed instead (see #969).
pub const KNOWN_TOP_LEVEL_KEYS: &[&str] = &[
    // Core model settings (alphabetized within block)
    "log_dir",
    "model",
    "model_auto_compact_token_limit",
    "model_catalog_json",
    "model_context_window",
    "model_provider",
    "model_reasoning_effort",
    "model_reasoning_summary",
    "model_supports_reasoning_summaries",
    "model_verbosity",
    "oss_provider",
    "personality",
    "plan_mode_reasoning_effort",
    "review_model",
    "tool_output_token_limit",
    // Instructions
    "compact_prompt",
    "developer_instructions",
    "experimental_compact_prompt_file",
    "instructions",
    "model_instructions_file",
    // Notifications
    "notify",
    // Approval & sandbox
    "approval_policy",
    "default_permissions",
    "sandbox_mode",
    // Authentication
    "chatgpt_base_url",
    "cli_auth_credentials_store",
    "forced_chatgpt_workspace_id",
    "forced_login_method",
    "mcp_oauth_callback_port",
    "mcp_oauth_callback_url",
    "mcp_oauth_credentials_store",
    "openai_base_url",
    // Project docs
    "project_doc_fallback_filenames",
    "project_doc_max_bytes",
    "project_root_markers",
    // UI
    "check_for_update_on_startup",
    "disable_paste_burst",
    "file_opener",
    "hide_agent_reasoning",
    "show_raw_agent_reasoning",
    "windows_wsl_setup_acknowledged",
    // Web search
    "web_search",
    // Profiles
    "profile",
    // Shell / system (alphabetized within block; added in rust-v0.128.0 catch-up)
    "allow_login_shell",
    "background_terminal_max_timeout",
    "sqlite_home",
    "zsh_path",
    // Telemetry / attribution (added in rust-v0.128.0 catch-up)
    "commit_attribution",
    "service_tier",
    "suppress_unstable_features_warning",
    // Experimental (alphabetized)
    "experimental_realtime_start_instructions",
    "experimental_realtime_webrtc_call_base_url",
    "experimental_realtime_ws_backend_prompt",
    "experimental_realtime_ws_base_url",
    "experimental_realtime_ws_model",
    "experimental_realtime_ws_startup_context",
    "experimental_thread_config_endpoint",
    "experimental_thread_store_endpoint",
    "experimental_use_freeform_apply_patch",
    "experimental_use_unified_exec_tool",
    // Instruction-section toggles (added in Codex rust-v0.122.0 catch-up)
    "include_apps_instructions",
    "include_environment_context",
    "include_permissions_instructions",
    // Plugin marketplaces (added in Codex rust-v0.122.0 catch-up; can also be
    // present as a TOML table)
    "marketplaces",
    // Realtime config (added in Codex rust-v0.122.0 catch-up; can also be
    // present as a TOML table)
    "realtime",
    // Tool suggestions toggle (added in Codex rust-v0.122.0 catch-up)
    "tool_suggest",
    // Added in Codex rust-v0.133.0 catch-up (sourced from upstream
    // codex-rs/core/config.schema.json). Scalar top-level keys:
    // `apps_mcp_product_sku` (product SKU forwarded on host-owned Apps MCP
    // requests), `include_collaboration_mode_instructions` (inject the
    // <collaboration_mode> developer block), and
    // `model_auto_compact_token_limit_scope` (enum total|body_after_prefix).
    "apps_mcp_product_sku",
    "include_collaboration_mode_instructions",
    "model_auto_compact_token_limit_scope",
    // Legacy camelCase keys: never in `config.schema.json` (which is snake_case)
    // but accepted from very early Codex configs for backwards compatibility.
    "approvalMode",
    "fullAutoErrorMode",
];

/// Known valid TOML table names (sections like `[sandbox_workspace_write]`)
pub const KNOWN_TABLE_KEYS: &[&str] = &[
    "sandbox_workspace_write",
    "shell_environment_policy",
    "history",
    "tui",
    "features",
    "mcp_servers",
    "model_providers",
    "profiles",
    "projects",
    "otel",
    "skills",
    "feedback",
    "notice",
    // Added in Codex rust-v0.122.0 catch-up - both can appear as TOML tables
    // ([realtime] / [[marketplaces]]) in addition to inline values.
    "realtime",
    "marketplaces",
    // Added in Codex rust-v0.128.0 catch-up (sourced from upstream
    // codex-rs/core/config.schema.json). Alphabetized within block.
    "agents",
    "analytics",
    "approvals_reviewer",
    "apps",
    "audio",
    "auto_review",
    "experimental_thread_store",
    "ghost_snapshot",
    "hooks",
    "memories",
    // Added in Codex rust-v0.142.0 - orchestrator-owned feature toggles for
    // skills and MCP exposure.
    "orchestrator",
    "permissions",
    "plugins",
    "tools",
    "windows",
    // Added in Codex rust-v0.129.0 catch-up - `[debug]` table for config
    // lockfile debugging (nested `[debug.config_lockfile]` carries
    // `allow_codex_version_mismatch`, `export_dir`, `load_path`, and
    // `save_fields_resolved_from_model_catalog`).
    "debug",
    // Added in Codex rust-v0.133.0 catch-up - `[desktop]` table for opaque
    // desktop settings stored alongside the rest of config.toml
    // (`additionalProperties: true` upstream, so nested keys are not
    // enumerated).
    "desktop",
];

/// Single source of truth for whether a Codex config top-level key is known.
///
/// Both backends consult this: the TOML path (`detect_unknown_keys` here) and
/// the JSON/YAML path (`rules::codex::collect_unknown_codex_keys`). Keeping one
/// predicate prevents the two from drifting (a key valid as TOML but flagged as
/// JSON, or vice versa). A top-level key may be either a scalar
/// (`KNOWN_TOP_LEVEL_KEYS`) or a `[section]` table (`KNOWN_TABLE_KEYS`).
#[must_use]
pub fn is_known_top_level_key(key: &str) -> bool {
    KNOWN_TOP_LEVEL_KEYS.contains(&key) || KNOWN_TABLE_KEYS.contains(&key)
}

/// An unknown key found in config
#[derive(Debug, Clone)]
pub struct UnknownKey {
    pub key: String,
    pub line: usize,
    pub column: usize,
}

/// Partial schema for .codex/config.toml (only fields we validate)
///
/// Note: The actual TOML keys use camelCase (`approvalMode`, `fullAutoErrorMode`).
/// We use manual `toml::Value` parsing in `parse_codex_toml` rather than serde
/// deserialization so that type mismatches (e.g. `approvalMode = true`) are
/// reported as CDX-001/CDX-002 diagnostics instead of generic parse errors.
/// The `#[serde(rename)]` attributes are kept for documentation and in case
/// the struct is ever deserialized directly.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodexConfigSchema {
    /// Approval mode for Codex CLI (TOML key: `approvalMode`)
    #[serde(default, rename = "approvalMode")]
    pub approval_mode: Option<String>,

    /// Error handling mode for full-auto mode (TOML key: `fullAutoErrorMode`)
    #[serde(default, rename = "fullAutoErrorMode")]
    pub full_auto_error_mode: Option<String>,

    /// Maximum size for project documentation files in bytes
    #[serde(default)]
    pub project_doc_max_bytes: Option<i64>,

    /// Fallback filenames used when AGENTS.md is not found
    #[serde(default)]
    pub project_doc_fallback_filenames: Option<Vec<String>>,
}

/// Result of parsing .codex/config.toml
#[derive(Debug, Clone)]
pub struct ParsedCodexConfig {
    /// The parsed schema (if valid TOML)
    pub schema: Option<CodexConfigSchema>,
    /// Parse error if TOML is invalid
    pub parse_error: Option<ParseError>,
    /// Whether `approvalMode` key exists but has wrong type (not a string)
    pub approval_mode_wrong_type: bool,
    /// Whether `fullAutoErrorMode` key exists but has wrong type (not a string)
    pub full_auto_error_mode_wrong_type: bool,
    /// Whether `project_doc_max_bytes` key exists but has wrong type (not an integer)
    pub project_doc_max_bytes_wrong_type: bool,
    /// Whether `project_doc_fallback_filenames` key exists but has wrong type (not an array)
    pub project_doc_fallback_filenames_wrong_type: bool,
    /// Zero-based indexes of non-string entries in `project_doc_fallback_filenames`
    pub project_doc_fallback_filename_non_string_indices: Vec<usize>,
    /// Zero-based indexes of empty/whitespace-only entries in `project_doc_fallback_filenames`
    pub project_doc_fallback_filename_empty_indices: Vec<usize>,
    /// Unknown top-level keys found in config
    pub unknown_keys: Vec<UnknownKey>,
}

/// A TOML parse error with location information
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

/// Parse .codex/config.toml content
///
/// Uses a two-pass approach: first validates TOML syntax with `toml::Value`,
/// then extracts the typed schema. This ensures that type mismatches (e.g.,
/// `approvalMode = true`) are reported as CDX-001/CDX-002 issues rather than
/// generic parse errors.
///
/// # Input size
///
/// Callers are expected to enforce file size limits before calling this function.
/// In production, `file_utils::safe_read_file` enforces a 1 MiB limit upstream,
/// so content passed here is already bounded.
pub fn parse_codex_toml(content: &str) -> ParsedCodexConfig {
    // First pass: parse the TOML document as a top-level table.
    //
    // `toml::Value` parsing accepts full documents on older toml releases, but
    // newer releases treat `Value` parsing as a single-value parser. Using
    // `toml::Table` keeps behavior stable across versions.
    let parsed_table: toml::Table = match toml::from_str::<toml::Table>(content) {
        Ok(v) => v,
        Err(e) => {
            // toml crate provides span info; extract line/column
            let (line, column) = e
                .span()
                .map(|span| {
                    let mut l = 1usize;
                    let mut c = 1usize;
                    for (i, ch) in content.char_indices() {
                        if i >= span.start {
                            break;
                        }
                        if ch == '\n' {
                            l += 1;
                            c = 1;
                        } else {
                            c += 1;
                        }
                    }
                    (l, c)
                })
                .unwrap_or((1, 0));

            return ParsedCodexConfig {
                schema: None,
                parse_error: Some(ParseError {
                    message: e.message().to_string(),
                    line,
                    column,
                }),
                approval_mode_wrong_type: false,
                full_auto_error_mode_wrong_type: false,
                project_doc_max_bytes_wrong_type: false,
                project_doc_fallback_filenames_wrong_type: false,
                project_doc_fallback_filename_non_string_indices: Vec::new(),
                project_doc_fallback_filename_empty_indices: Vec::new(),
                unknown_keys: Vec::new(),
            };
        }
    };

    // Second pass: extract typed fields permissively, tracking type mismatches
    // TOML keys use camelCase: approvalMode, fullAutoErrorMode
    let table = Some(&parsed_table);

    let approval_mode_value = table.and_then(|t| t.get("approvalMode"));
    let approval_mode_wrong_type = approval_mode_value.is_some_and(|v| !v.is_str());
    let approval_mode = approval_mode_value
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let full_auto_error_mode_value = table.and_then(|t| t.get("fullAutoErrorMode"));
    let full_auto_error_mode_wrong_type = full_auto_error_mode_value.is_some_and(|v| !v.is_str());
    let full_auto_error_mode = full_auto_error_mode_value
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Extract project_doc_max_bytes (CDX-005)
    let project_doc_max_bytes_value = table.and_then(|t| t.get("project_doc_max_bytes"));
    let project_doc_max_bytes_wrong_type =
        project_doc_max_bytes_value.is_some_and(|v| !v.is_integer());
    let project_doc_max_bytes = project_doc_max_bytes_value.and_then(|v| v.as_integer());

    // Extract project_doc_fallback_filenames (CDX-006)
    let project_doc_fallback_filenames_value =
        table.and_then(|t| t.get("project_doc_fallback_filenames"));
    let project_doc_fallback_filenames_wrong_type =
        project_doc_fallback_filenames_value.is_some_and(|v| !v.is_array());
    let (
        project_doc_fallback_filenames,
        project_doc_fallback_filename_non_string_indices,
        project_doc_fallback_filename_empty_indices,
    ) = if let Some(values) = project_doc_fallback_filenames_value.and_then(|v| v.as_array()) {
        let mut filenames = Vec::new();
        let mut non_string_indices = Vec::new();
        let mut empty_indices = Vec::new();

        for (idx, value) in values.iter().enumerate() {
            if let Some(filename) = value.as_str() {
                if filename.trim().is_empty() {
                    empty_indices.push(idx);
                }
                filenames.push(filename.to_string());
            } else {
                non_string_indices.push(idx);
            }
        }

        (Some(filenames), non_string_indices, empty_indices)
    } else {
        (None, Vec::new(), Vec::new())
    };

    // Detect unknown top-level keys (CDX-004)
    let unknown_keys = detect_unknown_keys(table, content);

    ParsedCodexConfig {
        schema: Some(CodexConfigSchema {
            approval_mode,
            full_auto_error_mode,
            project_doc_max_bytes,
            project_doc_fallback_filenames,
        }),
        parse_error: None,
        approval_mode_wrong_type,
        full_auto_error_mode_wrong_type,
        project_doc_max_bytes_wrong_type,
        project_doc_fallback_filenames_wrong_type,
        project_doc_fallback_filename_non_string_indices,
        project_doc_fallback_filename_empty_indices,
        unknown_keys,
    }
}

/// Detect unknown top-level keys by comparing TOML table keys against the known sets.
fn detect_unknown_keys(
    table: Option<&toml::map::Map<String, toml::Value>>,
    content: &str,
) -> Vec<UnknownKey> {
    let Some(table) = table else {
        return Vec::new();
    };

    // Use .contains() on the slices directly - they are small (~90 entries)
    // and this avoids HashSet allocation on every call.
    let mut unknown = Vec::new();
    for key in table.keys() {
        if !is_known_top_level_key(key.as_str()) {
            unknown.push(UnknownKey {
                key: key.clone(),
                line: find_toml_key_line(content, key).unwrap_or(1),
                column: 0,
            });
        }
    }
    unknown
}

/// Find the 1-indexed line number of a TOML key in the content.
///
/// Searches for a bare key or quoted key followed by `=` to prevent partial
/// matches and value-position false positives.
fn find_toml_key_line(content: &str, key: &str) -> Option<usize> {
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        // Skip table headers like [section]
        if trimmed.starts_with('[') {
            continue;
        }
        // Try bare key match
        if let Some(after) = trimmed.strip_prefix(key) {
            if after.trim_start().starts_with('=') {
                return Some(i + 1);
            }
        }
        // Try quoted key match
        let quoted = format!("\"{}\"", key);
        if let Some(after) = trimmed.strip_prefix(&quoted) {
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

    #[test]
    fn test_parse_valid_config() {
        let content = r#"
model = "o4-mini"
approvalMode = "suggest"
fullAutoErrorMode = "ask-user"
notify = true
"#;
        let result = parse_codex_toml(content);
        assert!(result.schema.is_some());
        assert!(result.parse_error.is_none());
        let schema = result.schema.unwrap();
        assert_eq!(schema.approval_mode, Some("suggest".to_string()));
        assert_eq!(schema.full_auto_error_mode, Some("ask-user".to_string()));
    }

    #[test]
    fn test_parse_minimal_config() {
        let content = "";
        let result = parse_codex_toml(content);
        assert!(result.schema.is_some());
        assert!(result.parse_error.is_none());
        let schema = result.schema.unwrap();
        assert!(schema.approval_mode.is_none());
        assert!(schema.full_auto_error_mode.is_none());
        assert!(schema.project_doc_fallback_filenames.is_none());
    }

    #[test]
    fn test_parse_invalid_toml() {
        let content = "invalid = [unclosed";
        let result = parse_codex_toml(content);
        assert!(result.schema.is_none());
        assert!(result.parse_error.is_some());
    }

    #[test]
    fn test_valid_approval_modes() {
        for mode in VALID_APPROVAL_MODES {
            let content = format!("approvalMode = \"{}\"", mode);
            let result = parse_codex_toml(&content);
            assert!(result.schema.is_some());
            assert_eq!(result.schema.unwrap().approval_mode, Some(mode.to_string()));
        }
    }

    #[test]
    fn test_valid_full_auto_error_modes() {
        for mode in VALID_FULL_AUTO_ERROR_MODES {
            let content = format!("fullAutoErrorMode = \"{}\"", mode);
            let result = parse_codex_toml(&content);
            assert!(result.schema.is_some());
            assert_eq!(
                result.schema.unwrap().full_auto_error_mode,
                Some(mode.to_string())
            );
        }
    }

    #[test]
    fn test_parse_extra_fields_ignored() {
        let content = r#"
model = "o4-mini"
approvalMode = "suggest"
fullAutoErrorMode = "ask-user"
notify = true
provider = "openai"
"#;
        let result = parse_codex_toml(content);
        assert!(result.schema.is_some());
        assert!(result.parse_error.is_none());
    }

    #[test]
    fn test_approval_mode_wrong_type() {
        let content = "approvalMode = true";
        let result = parse_codex_toml(content);
        assert!(result.approval_mode_wrong_type);
        assert!(!result.full_auto_error_mode_wrong_type);
        assert!(result.schema.is_some());
        assert!(result.schema.unwrap().approval_mode.is_none());
    }

    #[test]
    fn test_full_auto_error_mode_wrong_type() {
        let content = "fullAutoErrorMode = 123";
        let result = parse_codex_toml(content);
        assert!(!result.approval_mode_wrong_type);
        assert!(result.full_auto_error_mode_wrong_type);
        assert!(result.schema.is_some());
        assert!(result.schema.unwrap().full_auto_error_mode.is_none());
    }

    #[test]
    fn test_parse_error_location() {
        let content = "approvalMode = [unclosed";
        let result = parse_codex_toml(content);
        assert!(result.parse_error.is_some());
        let err = result.parse_error.unwrap();
        assert!(err.line > 0);
    }

    #[test]
    fn test_parse_error_fallback_line() {
        // When span() returns None the code falls back to (line=1, column=0).
        // In practice the toml crate always provides spans for parse errors,
        // so we verify the fallback indirectly: any parse error must have
        // line >= 1 (the minimum from the fallback path).
        let content = "= value_without_key";
        let result = parse_codex_toml(content);
        assert!(result.parse_error.is_some());
        let err = result.parse_error.unwrap();
        assert!(
            err.line >= 1,
            "Parse error line should be at least 1 (fallback or span-derived)"
        );
    }

    // ===== Unknown Keys Detection =====

    #[test]
    fn test_unknown_keys_detected() {
        let content = "completely_unknown_key = true\nmodel = \"o4-mini\"";
        let result = parse_codex_toml(content);
        assert!(result.parse_error.is_none());
        assert_eq!(result.unknown_keys.len(), 1);
        assert_eq!(result.unknown_keys[0].key, "completely_unknown_key");
        assert_eq!(result.unknown_keys[0].line, 1);
    }

    #[test]
    fn test_known_keys_not_flagged() {
        let content = r#"
model = "o4-mini"
approvalMode = "suggest"
fullAutoErrorMode = "ask-user"
notify = true
project_doc_max_bytes = 32768
project_doc_fallback_filenames = ["AGENTS.md", "README.md"]
"#;
        let result = parse_codex_toml(content);
        assert!(result.unknown_keys.is_empty(), "All keys are known");
    }

    #[test]
    fn test_known_table_keys_not_flagged() {
        let content = r#"
model = "o4-mini"

[mcp_servers]
name = "test"
"#;
        let result = parse_codex_toml(content);
        assert!(
            result.unknown_keys.is_empty(),
            "Known table keys should not be flagged"
        );
    }

    #[test]
    fn test_codex_0_128_0_new_scalar_keys_not_flagged() {
        // Scalar keys introduced (or surfaced) in Codex rust-v0.128.0's upstream
        // JSON schema. Regression guard: if any of these ever get removed from
        // KNOWN_TOP_LEVEL_KEYS by mistake, CDX-004 starts false-positive-ing on
        // valid 0.128 configs.
        let content = r#"
plan_mode_reasoning_effort = "low"
model_catalog_json = "/tmp/catalog.json"
default_permissions = "strict"
mcp_oauth_callback_url = "http://localhost:7711/callback"
openai_base_url = "https://api.openai.com/v1"
allow_login_shell = true
zsh_path = "/usr/local/bin/zsh"
sqlite_home = "/var/lib/codex"
background_terminal_max_timeout = 60
commit_attribution = "codex"
service_tier = "fast"
suppress_unstable_features_warning = false
experimental_realtime_ws_backend_prompt = "hello"
experimental_realtime_ws_base_url = "wss://example.com/rt"
experimental_realtime_ws_model = "gpt-realtime"
experimental_thread_config_endpoint = "https://example.com/threads"
"#;
        let result = parse_codex_toml(content);
        assert!(result.parse_error.is_none());
        assert!(
            result.unknown_keys.is_empty(),
            "0.128 top-level keys should not be flagged as unknown, got: {:?}",
            result
                .unknown_keys
                .iter()
                .map(|u| u.key.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_codex_0_128_0_new_table_keys_not_flagged() {
        // Table sections introduced (or surfaced) in Codex rust-v0.128.0's upstream
        // JSON schema. Each one is a `[section]` header.
        let content = r#"
[agents]
max_threads = 4

[analytics]

[apps]

[audio]

[auto_review]

[approvals_reviewer]

[experimental_thread_store]

[ghost_snapshot]

[hooks]

[memories]

[permissions]

[plugins]

[tools]

[windows]
"#;
        let result = parse_codex_toml(content);
        assert!(result.parse_error.is_none());
        assert!(
            result.unknown_keys.is_empty(),
            "0.128 table sections should not be flagged as unknown, got: {:?}",
            result
                .unknown_keys
                .iter()
                .map(|u| u.key.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_codex_0_129_0_new_table_keys_not_flagged() {
        // Table section introduced in Codex rust-v0.129.0 - `[debug]` and
        // its nested `[debug.config_lockfile]` sub-table. Regression guard:
        // if `debug` ever gets removed from KNOWN_TABLE_KEYS by mistake,
        // CDX-004 starts false-positive-ing on valid 0.129 configs.
        let content = r#"
[debug]

[debug.config_lockfile]
allow_codex_version_mismatch = true
export_dir = "/tmp/codex-locks"
load_path = "/tmp/codex-locks/session.lock"
save_fields_resolved_from_model_catalog = true
"#;
        let result = parse_codex_toml(content);
        assert!(result.parse_error.is_none());
        assert!(
            result.unknown_keys.is_empty(),
            "0.129 `debug` table should not be flagged as unknown, got: {:?}",
            result
                .unknown_keys
                .iter()
                .map(|u| u.key.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_codex_0_133_0_new_top_level_keys_not_flagged() {
        // Top-level keys/table introduced in Codex rust-v0.133.0. Regression
        // guard: if any of these get dropped from KNOWN_TOP_LEVEL_KEYS /
        // KNOWN_TABLE_KEYS, CDX-004 starts false-positive-ing on valid 0.133
        // configs. `[desktop]` is an opaque table (additionalProperties:true
        // upstream) so its nested keys must not be flagged either.
        let content = r#"
apps_mcp_product_sku = "codex-pro"
include_collaboration_mode_instructions = true
model_auto_compact_token_limit_scope = "body_after_prefix"

[desktop]
some_opaque_setting = "value"
nested_number = 42
"#;
        let result = parse_codex_toml(content);
        assert!(result.parse_error.is_none());
        assert!(
            result.unknown_keys.is_empty(),
            "0.133 top-level keys/`[desktop]` table should not be flagged, got: {:?}",
            result
                .unknown_keys
                .iter()
                .map(|u| u.key.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_codex_0_140_0_new_top_level_key_not_flagged() {
        // Top-level key introduced in Codex rust-v0.140.0. Regression guard
        // against CDX-004 false positives on valid realtime WebRTC configs.
        let content = r#"
experimental_realtime_webrtc_call_base_url = "https://realtime.example.com/call"
"#;
        let result = parse_codex_toml(content);
        assert!(result.parse_error.is_none());
        assert!(
            result.unknown_keys.is_empty(),
            "0.140 realtime WebRTC key should not be flagged, got: {:?}",
            result
                .unknown_keys
                .iter()
                .map(|u| u.key.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_codex_stale_js_repl_keys_flagged() {
        // Audit (issue #969): `js_repl_node_path` / `js_repl_node_module_dirs`
        // were never present in any audited Codex schema (rust-v0.129.0 through
        // rust-v0.134.0-alpha.3) and are not `[features]` sub-keys - they were a
        // stale agnix allow-list entry. They were dropped from
        // KNOWN_TOP_LEVEL_KEYS, so both backends now flag them. (The TOML and
        // JSON/YAML paths still agree, which was the #966 invariant - they just
        // agree on "unknown" now.)
        let content =
            "js_repl_node_path = \"/usr/bin/node\"\njs_repl_node_module_dirs = [\"/x\"]\n";
        let result = parse_codex_toml(content);
        assert!(result.parse_error.is_none());
        let flagged: Vec<&str> = result.unknown_keys.iter().map(|u| u.key.as_str()).collect();
        assert!(
            flagged.contains(&"js_repl_node_path") && flagged.contains(&"js_repl_node_module_dirs"),
            "stale js_repl_* keys should now be flagged on the TOML path, got: {flagged:?}"
        );
    }

    #[test]
    fn test_unknown_keys_empty_on_parse_error() {
        let content = "invalid = [unclosed";
        let result = parse_codex_toml(content);
        assert!(result.parse_error.is_some());
        assert!(result.unknown_keys.is_empty());
    }

    // ===== project_doc_max_bytes Parsing =====

    #[test]
    fn test_project_doc_max_bytes_parsed() {
        let content = "project_doc_max_bytes = 32768";
        let result = parse_codex_toml(content);
        assert!(result.schema.is_some());
        assert_eq!(result.schema.unwrap().project_doc_max_bytes, Some(32768));
        assert!(!result.project_doc_max_bytes_wrong_type);
    }

    #[test]
    fn test_project_doc_max_bytes_wrong_type() {
        let content = "project_doc_max_bytes = \"not a number\"";
        let result = parse_codex_toml(content);
        assert!(result.project_doc_max_bytes_wrong_type);
    }

    #[test]
    fn test_project_doc_max_bytes_absent() {
        let content = "model = \"o4-mini\"";
        let result = parse_codex_toml(content);
        assert!(result.schema.is_some());
        assert!(result.schema.unwrap().project_doc_max_bytes.is_none());
        assert!(!result.project_doc_max_bytes_wrong_type);
    }

    // ===== project_doc_fallback_filenames Parsing =====

    #[test]
    fn test_project_doc_fallback_filenames_parsed() {
        let content = "project_doc_fallback_filenames = [\"AGENTS.md\", \"README.md\"]";
        let result = parse_codex_toml(content);
        assert!(result.schema.is_some());
        assert_eq!(
            result.schema.unwrap().project_doc_fallback_filenames,
            Some(vec!["AGENTS.md".to_string(), "README.md".to_string()])
        );
        assert!(!result.project_doc_fallback_filenames_wrong_type);
        assert!(
            result
                .project_doc_fallback_filename_non_string_indices
                .is_empty()
        );
        assert!(
            result
                .project_doc_fallback_filename_empty_indices
                .is_empty()
        );
    }

    #[test]
    fn test_project_doc_fallback_filenames_wrong_type() {
        let content = "project_doc_fallback_filenames = \"AGENTS.md\"";
        let result = parse_codex_toml(content);
        assert!(result.project_doc_fallback_filenames_wrong_type);
        assert!(
            result
                .project_doc_fallback_filename_non_string_indices
                .is_empty()
        );
        assert!(
            result
                .project_doc_fallback_filename_empty_indices
                .is_empty()
        );
    }

    #[test]
    fn test_project_doc_fallback_filenames_non_string_items() {
        let content = "project_doc_fallback_filenames = [\"AGENTS.md\", 42, true]";
        let result = parse_codex_toml(content);
        assert!(!result.project_doc_fallback_filenames_wrong_type);
        assert_eq!(
            result.project_doc_fallback_filename_non_string_indices,
            vec![1, 2]
        );
    }

    #[test]
    fn test_project_doc_fallback_filenames_empty_items() {
        let content = "project_doc_fallback_filenames = [\"\", \"   \", \"AGENTS.md\"]";
        let result = parse_codex_toml(content);
        assert!(!result.project_doc_fallback_filenames_wrong_type);
        assert_eq!(
            result.project_doc_fallback_filename_empty_indices,
            vec![0, 1]
        );
    }

    // ===== find_toml_key_line =====

    #[test]
    fn test_find_toml_key_line_basic() {
        let content = "model = \"o4-mini\"\nunknown_key = true";
        assert_eq!(find_toml_key_line(content, "model"), Some(1));
        assert_eq!(find_toml_key_line(content, "unknown_key"), Some(2));
        assert_eq!(find_toml_key_line(content, "nonexistent"), None);
    }

    #[test]
    fn test_find_toml_key_line_skips_table_headers() {
        let content = "[mcp_servers]\nname = \"test\"";
        // Should not match "mcp_servers" in a table header
        assert_eq!(find_toml_key_line(content, "name"), Some(2));
    }
}
