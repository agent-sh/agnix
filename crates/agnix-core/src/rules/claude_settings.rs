//! Claude Code `.claude/settings.json` top-level field validation (CC-SET-*).
//!
//! This validator complements the hooks-focused rules in `hooks/mod.rs` by
//! checking top-level settings fields documented at
//! <https://code.claude.com/docs/en/settings>.
//!
//! Rules:
//! - `prUrlTemplate` (CC-SET-001, added in Claude Code v2.1.119).
//! - `channelsEnabled` boolean check (CC-SET-002, added in Claude Code v2.1.128).
//! - `worktree.baseRef` enum check (CC-SET-003, added in Claude Code v2.1.133).
//! - `sandbox.bwrapPath`/`sandbox.socatPath` type check (CC-SET-004, added in Claude Code v2.1.133).
//! - `parentSettingsBehavior` enum check (CC-SET-005, added in Claude Code v2.1.133).
//! - `disableBundledSkills` boolean check (CC-SET-006, added in Claude Code v2.1.169).
//! - `enforceAvailableModels` boolean check (CC-SET-007, added in Claude Code v2.1.175).
//! - `sandbox.allowAppleEvents` boolean check (CC-SET-008, added in Claude Code v2.1.181).
//! - `attribution.sessionUrl` boolean check (CC-SET-009, added in Claude Code v2.1.183).
//! - `teammateMode` enum check (CC-SET-010, `iterm2` added in Claude Code v2.1.186).
//! - `respondToBashCommands` boolean check (CC-SET-011, added in Claude Code v2.1.186).
//! - `sandbox.credentials` shape check (CC-SET-012, added in Claude Code v2.1.187).
//! - `autoMode.classifyAllShell` boolean check (CC-SET-013, added in Claude Code v2.1.193).
//! - autoMode ignored in settings.local.json (CC-SET-014, changed in Claude Code v2.1.207).
//! - pluginConfigs dead in project-level settings (CC-SET-015, changed in Claude Code v2.1.207).
//! - deprecated Write/NotebookEdit/Glob permission-rule forms (CC-SET-016, added in Claude Code v2.1.210).
//! - `sandbox.filesystem.disabled` boolean check (CC-SET-017, added in Claude Code v2.1.216).
//! - `emojiCompletionEnabled` boolean check (CC-SET-018, added in Claude Code v2.1.217).
//! - `sandbox.network.strictAllowlist` boolean check (CC-SET-019, added in Claude Code v2.1.219).
//! - `workflowSizeGuideline` enum check (CC-SET-020, added in Claude Code v2.1.219).
//! - ineffective project-level `remoteControlAtStartup: true` (CC-SET-021, changed in Claude Code v2.1.222).
//! - `crossSessionInbound` enum check (CC-SET-022, added in Claude Code v2.1.224).
//! - `dialogExpiry` enum check (CC-SET-023, added in Claude Code v2.1.224).
//! - ineffective project-level `sandbox.ripgrep` (CC-SET-024, changed in Claude Code v2.1.232).
//! - `modelPicker` scope and shape (CC-SET-025, added in Claude Code v2.1.242).
//! - `promptCacheTtl` enum (CC-SET-026, added in Claude Code v2.1.242).
//! - `subagentPromptCacheTtl` enum (CC-SET-027, added in Claude Code v2.1.242).
//! - `feedbackDrafts` enum (CC-SET-028, added in Claude Code v2.1.247).
//! - `spinnerTipsOverride` shape (CC-SET-029, tip objects/`tipsFile`/`label`
//!   added in Claude Code v2.1.247).
//!
//! Runs on FileType::Hooks (which covers `.claude/settings.json` -
//! see `file_types/detection.rs`). Skips non-Claude Code settings paths
//! (e.g. `.amp/settings.json`) via a parent-directory check.

use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    file_types::{is_claude_managed_settings_path, is_claude_system_managed_settings_path},
    rules::{Validator, ValidatorMetadata},
};
use rust_i18n::t;
use std::{collections::HashMap, path::Path};

const RULE_IDS: &[&str] = &[
    "CC-SET-001",
    "CC-SET-002",
    "CC-SET-003",
    "CC-SET-004",
    "CC-SET-005",
    "CC-SET-006",
    "CC-SET-007",
    "CC-SET-008",
    "CC-SET-009",
    "CC-SET-010",
    "CC-SET-011",
    "CC-SET-012",
    "CC-SET-013",
    "CC-SET-014",
    "CC-SET-015",
    "CC-SET-016",
    "CC-SET-017",
    "CC-SET-018",
    "CC-SET-019",
    "CC-SET-020",
    "CC-SET-021",
    "CC-SET-022",
    "CC-SET-023",
    "CC-SET-024",
    "CC-SET-025",
    "CC-SET-026",
    "CC-SET-027",
    "CC-SET-028",
    "CC-SET-029",
];

/// Allowed values for `worktree.baseRef` per Claude Code v2.1.133 release notes.
const WORKTREE_BASE_REF_ALLOWED: &[&str] = &["fresh", "head"];

/// Allowed values for `parentSettingsBehavior` per Claude Code v2.1.133 release notes.
const PARENT_SETTINGS_BEHAVIOR_ALLOWED: &[&str] = &["first-wins", "merge"];

/// Documented `sandbox.*` string-valued path settings added in v2.1.133.
const SANDBOX_PATH_FIELDS: &[&str] = &["bwrapPath", "socatPath"];

/// Allowed values for `teammateMode`. Claude Code v2.1.186 added `iterm2`.
const TEAMMATE_MODE_ALLOWED: &[&str] = &["in-process", "auto", "tmux", "iterm2"];

/// Allowed values for `workflowSizeGuideline` per Claude Code v2.1.219.
const WORKFLOW_SIZE_GUIDELINE_ALLOWED: &[&str] = &["unrestricted", "small", "medium", "large"];

/// Allowed values for cross-session inbound message handling per Claude Code v2.1.224.
const CROSS_SESSION_INBOUND_ALLOWED: &[&str] = &["accept", "hold", "refuse"];

/// Allowed values for remote dialog expiry per Claude Code v2.1.224.
const DIALOG_EXPIRY_ALLOWED: &[&str] = &["60s", "5m", "10m", "never"];

/// Allowed prompt-cache lifetimes documented for the main and subagent settings.
const PROMPT_CACHE_TTL_ALLOWED: &[&str] = &["5m", "1h"];

/// Allowed values for `feedbackDrafts` per the settings reference. `"off"`
/// removes the SendFeedback tool entirely.
const FEEDBACK_DRAFTS_ALLOWED: &[&str] = &["notify", "quiet", "off"];

/// Documented `spinnerTipsOverride` fields, all optional.
const SPINNER_TIPS_OVERRIDE_FIELDS: &[&str] = &["tips", "tipsFile", "label", "excludeDefault"];

/// Documented limits for a `spinnerTipsOverride` tip object. Claude Code drops
/// an invalid entry with a debug warning rather than rejecting the file, so a
/// bad tip is silently never shown.
const SPINNER_TIP_ID_MAX_LEN: usize = 64;
const SPINNER_TIP_TEXT_MAX_LEN: usize = 500;
const SPINNER_TIP_LABEL_MAX_LEN: usize = 40;
const SPINNER_TIP_COOLDOWN_RANGE: (i64, i64) = (0, 1000);
const SPINNER_TIP_PRIORITY_RANGE: (i64, i64) = (-10, 10);

/// Placeholders documented for `prUrlTemplate` at
/// <https://code.claude.com/docs/en/settings>.
const PR_URL_TEMPLATE_PLACEHOLDERS: &[&str] = &["{host}", "{owner}", "{repo}", "{number}", "{url}"];

pub struct ClaudeSettingsValidator;

impl Validator for ClaudeSettingsValidator {
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

        if !is_claude_settings_path(path) {
            return diagnostics;
        }

        // Parse JSON once; bail silently on parse errors. The hooks
        // validator already surfaces malformed settings.json through its
        // own parse-error path, so we don't duplicate that diagnostic here.
        let Ok(value) = serde_json::from_str::<serde_json::Value>(content) else {
            return diagnostics;
        };

        if config.is_rule_enabled("CC-SET-001") {
            validate_pr_url_template(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-002") {
            validate_channels_enabled(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-003") {
            validate_worktree_base_ref(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-004") {
            validate_sandbox_paths(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-005") {
            validate_parent_settings_behavior(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-006") {
            validate_disable_bundled_skills(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-007") {
            validate_enforce_available_models(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-008") {
            validate_sandbox_allow_apple_events(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-009") {
            validate_attribution_session_url(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-010") {
            validate_teammate_mode(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-011") {
            validate_respond_to_bash_commands(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-012") {
            validate_sandbox_credentials(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-013") {
            validate_auto_mode_classify_all_shell(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-014") {
            validate_auto_mode_in_settings_local(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-015") {
            validate_plugin_configs_scope(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-016") {
            validate_deprecated_permission_tool_forms(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-017") {
            validate_sandbox_filesystem_disabled(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-018") {
            validate_emoji_completion_enabled(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-019") {
            validate_sandbox_network_strict_allowlist(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-020") {
            validate_workflow_size_guideline(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-021") {
            validate_remote_control_at_startup_scope(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-022") {
            validate_cross_session_inbound(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-023") {
            validate_dialog_expiry(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-024") {
            validate_sandbox_ripgrep_scope(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-025") {
            validate_model_picker(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-026") {
            validate_prompt_cache_ttl(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-027") {
            validate_subagent_prompt_cache_ttl(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-028") {
            validate_feedback_drafts(path, content, &value, &mut diagnostics);
        }

        if config.is_rule_enabled("CC-SET-029") {
            validate_spinner_tips_override(path, content, &value, &mut diagnostics);
        }

        diagnostics
    }
}

/// Only validate `.claude/settings.json`, `.claude/settings.local.json`, and
/// Claude Code managed-settings paths. Skip `.amp/`, `.kiro/`, etc. settings
/// files - they are classified as FileType::Hooks too but have entirely
/// different field sets, and a false positive here would be disruptive.
fn is_claude_settings_path(path: &Path) -> bool {
    if is_claude_managed_settings_path(path) {
        return true;
    }

    let parent_is_claude = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(|n| n.eq_ignore_ascii_case(".claude"))
        .unwrap_or(false);
    if !parent_is_claude {
        return false;
    }
    let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    matches!(filename, "settings.json" | "settings.local.json")
}

/// CC-SET-001: Validate prUrlTemplate. Three failure modes:
///
/// 1. Not a string (number, bool, array, object, null)
/// 2. Empty string
/// 3. String with no placeholders at all (probably a hardcoded URL that
///    won't substitute per-PR fields - worth flagging as a probable bug)
///
/// Unknown placeholders in the string (e.g. `{branch}`) are NOT flagged:
/// Claude Code substitutes the documented ones and leaves others as-is,
/// so a user extending their template with a literal `{branch}` isn't
/// technically wrong - just a no-op.
fn validate_pr_url_template(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value.get("prUrlTemplate") else {
        return;
    };

    let line = find_key_line(content, "prUrlTemplate").unwrap_or(1);

    let Some(template) = field_value.as_str() else {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                0,
                "CC-SET-001",
                t!("rules.cc_set_001.type_error"),
            )
            .with_suggestion(t!("rules.cc_set_001.suggestion")),
        );
        return;
    };

    if template.is_empty() {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                0,
                "CC-SET-001",
                t!("rules.cc_set_001.empty"),
            )
            .with_suggestion(t!("rules.cc_set_001.suggestion")),
        );
        return;
    }

    let has_placeholder = PR_URL_TEMPLATE_PLACEHOLDERS
        .iter()
        .any(|ph| template.contains(ph));
    if !has_placeholder {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                0,
                "CC-SET-001",
                t!("rules.cc_set_001.no_placeholder"),
            )
            .with_suggestion(t!("rules.cc_set_001.suggestion")),
        );
    }
}

/// CC-SET-002: Validate `channelsEnabled`. Claude Code 2.1.128 made
/// `--channels` work with console (API-key) authentication, but console
/// organisations with managed settings must opt in explicitly by setting
/// `channelsEnabled: true`. When present, the field MUST be a boolean -
/// a quoted `"true"` or a truthy number leaves Channels silently disabled
/// (same shape as the MCP-025 `alwaysLoad` footgun).
///
/// Missing field is fine: Channels just stays disabled (the pre-v2.1.128
/// default for console orgs). We only flag explicitly-set non-boolean
/// values. A literal `false` is also fine - an explicit opt-out matches
/// the documented behavior.
///
/// While the release note calls out "console orgs with managed settings",
/// we intentionally validate the key across all three Claude Code
/// settings paths (`settings.json`, `settings.local.json`,
/// `managed-settings.json`). If a user puts it in the wrong file, the
/// mis-typed *value* is still wrong; flagging consistently is more
/// useful than trying to infer org-type here.
fn validate_channels_enabled(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value.get("channelsEnabled") else {
        return;
    };

    // Boolean is the one legal shape. `null` is treated as "field-absent"
    // by JSON convention - we don't flag it to avoid doubling up with any
    // future "no explicit value" rule.
    if field_value.is_boolean() || field_value.is_null() {
        return;
    }

    let actual = describe_json_type(field_value);
    let line = find_key_line(content, "channelsEnabled").unwrap_or(1);

    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-002",
            format!(
                "channelsEnabled must be a boolean when present (got {}); Claude Code leaves Channels disabled when the value is not strictly true/false",
                actual
            ),
        )
        .with_suggestion(
            "Set channelsEnabled to an unquoted true or false. Quoted strings and numbers are not treated as opt-in by Claude Code 2.1.128+.",
        ),
    );
}

/// CC-SET-003: Validate `worktree.baseRef`. Claude Code 2.1.133 added
/// the `worktree` nested object with a `baseRef` enum - allowed values
/// are `"fresh"` (branch from `origin/<default>`, the v2.1.133 default)
/// and `"head"` (branch from local `HEAD`, the pre-2.1.133 default for
/// `EnterWorktree`). Any other value leaves the user in an ambiguous
/// state - the release note is explicit that only these two strings
/// are recognized.
///
/// Missing field, missing `worktree` key, or `worktree.baseRef == null`
/// are all fine - Claude Code falls through to the `"fresh"` default.
/// Non-object `worktree` (array/string/number/bool) is intentionally
/// ignored here - a future CC-SET rule may cover that class of bug,
/// and we don't want this rule to false-positive on schema extensions.
fn validate_worktree_base_ref(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(worktree) = value.get("worktree") else {
        return;
    };
    // Null worktree = field absent. Non-object worktree is a separate
    // class of bug we don't want to false-positive on (a CC-SET-006-style
    // rule might cover it later); leave it alone for now.
    if worktree.is_null() {
        return;
    }
    let Some(worktree_obj) = worktree.as_object() else {
        return;
    };
    let Some(base_ref) = worktree_obj.get("baseRef") else {
        return;
    };
    if base_ref.is_null() {
        return;
    }

    let line = find_key_line(content, "baseRef")
        .unwrap_or_else(|| find_key_line(content, "worktree").unwrap_or(1));

    match base_ref.as_str() {
        Some(s) if WORKTREE_BASE_REF_ALLOWED.contains(&s) => {}
        Some(other) => {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-003",
                    format!(
                        "worktree.baseRef must be \"fresh\" or \"head\" (got \"{}\"); Claude Code 2.1.133+ only recognizes these two values",
                        other
                    ),
                )
                .with_suggestion(
                    "Set worktree.baseRef to \"fresh\" (branch from origin/<default>) or \"head\" (branch from local HEAD).",
                ),
            );
        }
        None => {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-003",
                    format!(
                        "worktree.baseRef must be a string (got {})",
                        describe_json_type(base_ref)
                    ),
                )
                .with_suggestion("Set worktree.baseRef to \"fresh\" or \"head\"."),
            );
        }
    }
}

/// CC-SET-004: Validate `sandbox.bwrapPath` and `sandbox.socatPath`.
/// Claude Code 2.1.133 added these two Linux/WSL managed-settings fields
/// so admins can point the sandbox at a non-system bubblewrap/socat
/// binary. Values must be strings and must not be empty. We do not
/// stat the path (agnix validates files, not filesystem state) and we
/// do not check platform - the setting being Linux-only means a macOS/
/// Windows user writing it has a typo or a misconfiguration, but the
/// shape check is what we can enforce.
///
/// Null values are treated as "field absent" by JSON convention.
fn validate_sandbox_paths(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(sandbox) = value.get("sandbox") else {
        return;
    };
    if sandbox.is_null() {
        return;
    }
    let Some(sandbox_obj) = sandbox.as_object() else {
        return;
    };

    for field in SANDBOX_PATH_FIELDS {
        let Some(field_value) = sandbox_obj.get(*field) else {
            continue;
        };
        if field_value.is_null() {
            continue;
        }

        let line = find_key_line(content, field)
            .or_else(|| find_key_line(content, "sandbox"))
            .unwrap_or(1);

        match field_value.as_str() {
            Some(s) if !s.is_empty() => {}
            Some(_) => {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        0,
                        "CC-SET-004",
                        format!(
                            "sandbox.{} must not be an empty string; Claude Code uses this path to locate the sandbox helper binary",
                            field
                        ),
                    )
                    .with_suggestion(format!(
                        "Remove sandbox.{} to fall back to Claude Code's default lookup, or set it to an absolute path to a {} binary.",
                        field,
                        if *field == "bwrapPath" { "bwrap" } else { "socat" }
                    )),
                );
            }
            None => {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        0,
                        "CC-SET-004",
                        format!(
                            "sandbox.{} must be a string (got {})",
                            field,
                            describe_json_type(field_value)
                        ),
                    )
                    .with_suggestion(format!(
                        "Set sandbox.{} to an absolute path string, or remove it to use Claude Code's default lookup.",
                        field
                    )),
                );
            }
        }
    }
}

/// CC-SET-005: Validate `parentSettingsBehavior`. Claude Code 2.1.133
/// added this admin-tier top-level key to let admins opt SDK
/// `managedSettings` (the parent tier) into the policy merge. Allowed
/// values are `"first-wins"` (preserve existing behavior) and `"merge"`
/// (include parent-tier managedSettings in the merge).
///
/// Missing / null -> field absent (default behavior). Any other
/// non-enum string, or a non-string type, is a silent misconfiguration
/// because Claude Code falls back to the default with no warning.
fn validate_parent_settings_behavior(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value.get("parentSettingsBehavior") else {
        return;
    };
    if field_value.is_null() {
        return;
    }

    let line = find_key_line(content, "parentSettingsBehavior").unwrap_or(1);

    match field_value.as_str() {
        Some(s) if PARENT_SETTINGS_BEHAVIOR_ALLOWED.contains(&s) => {}
        Some(other) => {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-005",
                    format!(
                        "parentSettingsBehavior must be \"first-wins\" or \"merge\" (got \"{}\"); Claude Code 2.1.133+ only recognizes these two values",
                        other
                    ),
                )
                .with_suggestion(
                    "Set parentSettingsBehavior to \"first-wins\" (preserve existing behavior) or \"merge\" (include SDK managedSettings in the policy merge).",
                ),
            );
        }
        None => {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-005",
                    format!(
                        "parentSettingsBehavior must be a string (got {})",
                        describe_json_type(field_value)
                    ),
                )
                .with_suggestion("Set parentSettingsBehavior to \"first-wins\" or \"merge\"."),
            );
        }
    }
}

/// CC-SET-006: Validate `disableBundledSkills`. Claude Code 2.1.169
/// added this boolean setting (and the `CLAUDE_CODE_DISABLE_BUNDLED_SKILLS`
/// environment variable) to hide bundled skills, workflows, and built-in
/// slash commands from the model. The settings docs document only strict
/// `true`/`false` for the key - a quoted `"true"` or a truthy number is
/// not a documented opt-in (same shape as the CC-SET-002 `channelsEnabled`
/// footgun).
///
/// Missing field is fine: bundled skills just stay enabled (the default).
/// A literal `false` is also fine - an explicit opt-out matches the
/// documented default. `null` is treated as "field-absent" by JSON
/// convention, consistent with the other CC-SET rules.
fn validate_disable_bundled_skills(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value.get("disableBundledSkills") else {
        return;
    };

    if field_value.is_boolean() || field_value.is_null() {
        return;
    }

    let actual = describe_json_type(field_value);
    let line = find_key_line(content, "disableBundledSkills").unwrap_or(1);

    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-006",
            t!("rules.cc_set_006.message", actual = actual),
        )
        .with_suggestion(t!("rules.cc_set_006.suggestion")),
    );
}

/// CC-SET-007: Validate `enforceAvailableModels`. Claude Code 2.1.175
/// added this managed setting so an admin-provided `availableModels`
/// allowlist also constrains the Default model and cannot be widened by
/// user/project settings. The release note documents enablement as a
/// strict boolean - quoted strings, numbers, arrays, or objects are not
/// a documented opt-in.
///
/// Missing field is fine: Claude Code keeps the existing allowlist
/// behavior. `null` is treated as "field absent" by JSON convention,
/// consistent with the other CC-SET boolean rules.
fn validate_enforce_available_models(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value.get("enforceAvailableModels") else {
        return;
    };

    if field_value.is_boolean() || field_value.is_null() {
        return;
    }

    let actual = describe_json_type(field_value);
    let line = find_key_line(content, "enforceAvailableModels").unwrap_or(1);

    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-007",
            t!("rules.cc_set_007.message", actual = actual),
        )
        .with_suggestion(t!("rules.cc_set_007.suggestion")),
    );
}

/// CC-SET-008: Validate `sandbox.allowAppleEvents`. Claude Code 2.1.181
/// added this macOS sandbox opt-in so sandboxed commands can send Apple
/// Events. The setting is a strict boolean toggle - quoted strings,
/// numbers, arrays, or objects are not a documented opt-in.
///
/// Missing field is fine: Apple Events remain blocked by default. `null`
/// is treated as "field absent" by JSON convention, consistent with the
/// other CC-SET boolean rules.
fn validate_sandbox_allow_apple_events(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(sandbox) = value.get("sandbox") else {
        return;
    };
    if sandbox.is_null() {
        return;
    }
    let Some(sandbox_obj) = sandbox.as_object() else {
        return;
    };
    let Some(field_value) = sandbox_obj.get("allowAppleEvents") else {
        return;
    };

    if field_value.is_boolean() || field_value.is_null() {
        return;
    }

    let actual = describe_json_type(field_value);
    let line = find_key_line(content, "allowAppleEvents")
        .or_else(|| find_key_line(content, "sandbox"))
        .unwrap_or(1);

    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-008",
            t!("rules.cc_set_008.message", actual = actual),
        )
        .with_suggestion(t!("rules.cc_set_008.suggestion")),
    );
}

/// CC-SET-009: Validate `attribution.sessionUrl`. Claude Code v2.1.183
/// added this nested attribution toggle so web and Remote Control sessions
/// can omit the claude.ai session link from commits and PR descriptions. The
/// setting is a strict boolean: `true` keeps the default link, `false` omits it.
///
/// Missing field is fine: session URLs remain enabled by default. `null` is
/// treated as "field absent" by JSON convention, consistent with the other
/// CC-SET boolean rules. Non-object `attribution` values are ignored by this
/// rule to avoid conflating container-shape errors with the specific boolean
/// opt-in.
fn validate_attribution_session_url(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value.pointer("/attribution/sessionUrl") else {
        return;
    };

    if field_value.is_boolean() || field_value.is_null() {
        return;
    }

    let actual = describe_json_type(field_value);
    let line = find_key_line(content, "sessionUrl")
        .or_else(|| find_key_line(content, "attribution"))
        .unwrap_or(1);

    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-009",
            t!("rules.cc_set_009.message", actual = actual),
        )
        .with_suggestion(t!("rules.cc_set_009.suggestion")),
    );
}

/// CC-SET-010: Validate `teammateMode`. Claude Code documents four display
/// modes for agent-team teammates: `in-process`, `auto`, `tmux`, and `iterm2`
/// (`iterm2` was added in v2.1.186). Invalid strings silently fall back to
/// default behavior in practice, so catching typos keeps teams from launching
/// in an unexpected display mode.
fn validate_teammate_mode(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value.get("teammateMode") else {
        return;
    };
    if field_value.is_null() {
        return;
    }

    let line = find_key_line(content, "teammateMode").unwrap_or(1);

    match field_value.as_str() {
        Some(mode) if TEAMMATE_MODE_ALLOWED.contains(&mode) => {}
        Some(mode) => diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                0,
                "CC-SET-010",
                format!(
                    "teammateMode must be one of: {} (got \"{}\")",
                    TEAMMATE_MODE_ALLOWED.join(", "),
                    mode
                ),
            )
            .with_suggestion(
                "Set teammateMode to \"in-process\", \"auto\", \"tmux\", or \"iterm2\".",
            ),
        ),
        None => diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                0,
                "CC-SET-010",
                format!(
                    "teammateMode must be a string (got {})",
                    describe_json_type(field_value)
                ),
            )
            .with_suggestion(
                "Set teammateMode to \"in-process\", \"auto\", \"tmux\", or \"iterm2\".",
            ),
        ),
    }
}

/// CC-SET-011: Validate `respondToBashCommands`. Claude Code v2.1.186 changed
/// `!` bash command behavior so Claude automatically responds to the output by
/// default. The documented opt-out is a strict boolean
/// `"respondToBashCommands": false`; quoted strings or numbers won't express
/// that intent reliably.
fn validate_respond_to_bash_commands(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value.get("respondToBashCommands") else {
        return;
    };

    if field_value.is_boolean() || field_value.is_null() {
        return;
    }

    let actual = describe_json_type(field_value);
    let line = find_key_line(content, "respondToBashCommands").unwrap_or(1);

    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-011",
            format!(
                "respondToBashCommands must be a boolean when present (got {actual}); Claude Code v2.1.186+ documents this as a strict true/false setting"
            ),
        )
        .with_suggestion(
            "Set respondToBashCommands to an unquoted true or false. Use false to keep ! bash command output context-only.",
        ),
    );
}

/// CC-SET-012: Validate `sandbox.credentials`. Claude Code v2.1.187 added a
/// dedicated credential-deny block for sandboxed Bash commands. The documented
/// shape is:
///
/// ```json
/// {
///   "sandbox": {
///     "credentials": {
///       "files": [{"path": "~/.aws/credentials", "mode": "deny"}],
///       "envVars": [{"name": "GITHUB_TOKEN", "mode": "deny"}]
///     }
///   }
/// }
/// ```
///
/// Both arrays are optional, but when present they must be arrays of objects.
/// Each entry must name the credential target and carry a supported `mode`.
/// Claude Code v2.1.199 added environment-variable masking, and v2.1.221 added
/// file masking plus controls for narrowing injection and extraction. Claude
/// Code v2.1.224 added AWS credential re-signing, which requires
/// `sandbox.network.tlsTerminate`.
fn validate_sandbox_credentials(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(credentials) = value.pointer("/sandbox/credentials") else {
        return;
    };

    let credentials_line = find_key_line(content, "credentials")
        .or_else(|| find_key_line(content, "sandbox"))
        .unwrap_or(1);

    let Some(credentials_obj) = credentials.as_object() else {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                credentials_line,
                0,
                "CC-SET-012",
                t!(
                    "rules.cc_set_012.credentials_object",
                    actual = describe_json_type(credentials)
                ),
            )
            .with_suggestion(t!("rules.cc_set_012.credentials_object_suggestion")),
        );
        return;
    };

    if let Some(value) = credentials_obj.get("allowPlaintextInject")
        && !value.is_boolean()
    {
        let line = find_key_line(content, "allowPlaintextInject")
            .or_else(|| find_key_line(content, "credentials"))
            .unwrap_or(1);
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                0,
                "CC-SET-012",
                t!(
                    "rules.cc_set_012.allow_plaintext_type",
                    actual = describe_json_type(value)
                ),
            )
            .with_suggestion(t!("rules.cc_set_012.allow_plaintext_suggestion")),
        );
    }

    validate_sandbox_credential_entries(
        path,
        content,
        credentials_obj.get("files"),
        SandboxCredentialKind::File,
        diagnostics,
    );
    validate_sandbox_credential_entries(
        path,
        content,
        credentials_obj.get("envVars"),
        SandboxCredentialKind::EnvironmentVariable,
        diagnostics,
    );
    validate_sandbox_credential_aws_pairs(
        path,
        content,
        credentials_obj.get("awsPairs"),
        diagnostics,
    );
    validate_sandbox_credential_sigv4(path, content, credentials_obj.get("sigv4"), diagnostics);
    validate_sandbox_credential_aws_tls_termination(
        path,
        content,
        value,
        credentials_obj,
        diagnostics,
    );
    validate_sandbox_credential_aws_scope(path, content, credentials_obj, diagnostics);
}

#[derive(Clone, Copy)]
enum SandboxCredentialKind {
    File,
    EnvironmentVariable,
}

impl SandboxCredentialKind {
    fn array_key(self) -> &'static str {
        match self {
            Self::File => "files",
            Self::EnvironmentVariable => "envVars",
        }
    }

    fn required_key(self) -> &'static str {
        match self {
            Self::File => "path",
            Self::EnvironmentVariable => "name",
        }
    }
}

fn validate_sandbox_credential_entries(
    path: &Path,
    content: &str,
    value: Option<&serde_json::Value>,
    kind: SandboxCredentialKind,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(value) = value else {
        return;
    };
    let array_key = kind.array_key();
    let required_key = kind.required_key();

    let line = find_key_line(content, array_key)
        .or_else(|| find_key_line(content, "credentials"))
        .or_else(|| find_key_line(content, "sandbox"))
        .unwrap_or(1);

    let Some(entries) = value.as_array() else {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                0,
                "CC-SET-012",
                t!(
                    "rules.cc_set_012.entries_array",
                    array_key = array_key,
                    actual = describe_json_type(value)
                ),
            )
            .with_suggestion(t!(
                "rules.cc_set_012.entries_array_suggestion",
                array_key = array_key,
                required_key = required_key
            )),
        );
        return;
    };

    for (index, entry) in entries.iter().enumerate() {
        let Some(entry_obj) = entry.as_object() else {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-012",
                    t!(
                        "rules.cc_set_012.entry_object",
                        array_key = array_key,
                        index = index,
                        actual = describe_json_type(entry)
                    ),
                )
                .with_suggestion(t!(
                    "rules.cc_set_012.entry_object_suggestion",
                    array_key = array_key,
                    required_key = required_key
                )),
            );
            continue;
        };

        match entry_obj.get(required_key).and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => {}
            Some(_) => diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-012",
                    t!(
                        "rules.cc_set_012.required_empty",
                        array_key = array_key,
                        index = index,
                        required_key = required_key
                    ),
                )
                .with_suggestion(t!(
                    "rules.cc_set_012.required_suggestion",
                    array_key = array_key,
                    index = index,
                    required_key = required_key
                )),
            ),
            None => {
                let actual = entry_obj
                    .get(required_key)
                    .map(describe_json_type)
                    .unwrap_or("missing");
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        0,
                        "CC-SET-012",
                        t!(
                            "rules.cc_set_012.required_type",
                            array_key = array_key,
                            index = index,
                            required_key = required_key,
                            actual = actual
                        ),
                    )
                    .with_suggestion(t!(
                        "rules.cc_set_012.required_suggestion",
                        array_key = array_key,
                        index = index,
                        required_key = required_key
                    )),
                );
            }
        }

        if matches!(kind, SandboxCredentialKind::EnvironmentVariable)
            && let Some(name) = entry_obj.get(required_key).and_then(|v| v.as_str())
            && !name.is_empty()
            && !is_valid_environment_variable_name(name)
        {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-012",
                    t!(
                        "rules.cc_set_012.env_name",
                        array_key = array_key,
                        index = index,
                        name = name
                    ),
                )
                .with_suggestion(t!(
                    "rules.cc_set_012.env_name_suggestion",
                    array_key = array_key,
                    index = index
                )),
            );
        }

        let mode = entry_obj.get("mode").and_then(|v| v.as_str());
        match mode {
            Some("deny" | "mask") => {}
            Some(mode) => diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-012",
                    t!(
                        "rules.cc_set_012.mode_value",
                        array_key = array_key,
                        index = index,
                        mode = mode
                    ),
                )
                .with_suggestion(t!(
                    "rules.cc_set_012.mode_suggestion",
                    array_key = array_key,
                    index = index
                )),
            ),
            None => {
                let actual = entry_obj
                    .get("mode")
                    .map(describe_json_type)
                    .unwrap_or("missing");
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        0,
                        "CC-SET-012",
                        t!(
                            "rules.cc_set_012.mode_type",
                            array_key = array_key,
                            index = index,
                            actual = actual
                        ),
                    )
                    .with_suggestion(t!(
                        "rules.cc_set_012.mode_suggestion",
                        array_key = array_key,
                        index = index
                    )),
                );
            }
        }

        // Claude Code accepts these fields on deny entries but removes and
        // ignores them before schema validation. Only mask entries use them.
        if mode != Some("deny") {
            validate_sandbox_credential_inject_hosts(
                path,
                line,
                array_key,
                index,
                entry_obj.get("injectHosts"),
                diagnostics,
            );

            validate_sandbox_credential_mask_options(
                path,
                line,
                array_key,
                index,
                entry_obj,
                kind,
                diagnostics,
            );
        }
    }
}

fn is_valid_environment_variable_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn validate_sandbox_credential_inject_hosts(
    path: &Path,
    line: usize,
    array_key: &str,
    index: usize,
    value: Option<&serde_json::Value>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(hosts) = value.as_array() else {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                0,
                "CC-SET-012",
                t!(
                    "rules.cc_set_012.inject_hosts_array",
                    array_key = array_key,
                    index = index,
                    actual = describe_json_type(value)
                ),
            )
            .with_suggestion(t!(
                "rules.cc_set_012.inject_hosts_suggestion",
                array_key = array_key,
                index = index
            )),
        );
        return;
    };

    for (host_index, host) in hosts.iter().enumerate() {
        if !host.is_string() {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-012",
                    t!(
                        "rules.cc_set_012.inject_host_type",
                        array_key = array_key,
                        index = index,
                        host_index = host_index,
                        actual = describe_json_type(host)
                    ),
                )
                .with_suggestion(t!(
                    "rules.cc_set_012.inject_host_suggestion",
                    array_key = array_key,
                    index = index,
                    host_index = host_index
                )),
            );
        }
    }
}

fn validate_sandbox_credential_mask_options(
    path: &Path,
    line: usize,
    array_key: &str,
    index: usize,
    entry: &serde_json::Map<String, serde_json::Value>,
    kind: SandboxCredentialKind,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mode = entry.get("mode").and_then(serde_json::Value::as_str);

    if matches!(kind, SandboxCredentialKind::File)
        && mode == Some("mask")
        && entry
            .get("path")
            .and_then(|value| value.as_str())
            .is_some_and(|value| value.ends_with('/'))
    {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                0,
                "CC-SET-012",
                t!(
                    "rules.cc_set_012.mask_file",
                    array_key = array_key,
                    index = index
                ),
            )
            .with_suggestion(t!("rules.cc_set_012.mask_file_suggestion")),
        );
    }

    if let Some(value) = entry.get("extract") {
        match value.as_str() {
            Some(pattern) if let Err(error) = regress::Regex::new(pattern) => {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        0,
                        "CC-SET-012",
                        t!(
                            "rules.cc_set_012.extract_regex",
                            array_key = array_key,
                            index = index,
                            error = error
                        ),
                    )
                    .with_suggestion(t!(
                        "rules.cc_set_012.extract_regex_suggestion",
                        array_key = array_key,
                        index = index
                    )),
                );
            }
            Some(pattern) if mode == Some("mask") && !has_javascript_capturing_group(pattern) => {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        0,
                        "CC-SET-012",
                        t!(
                            "rules.cc_set_012.extract_capture",
                            array_key = array_key,
                            index = index
                        ),
                    )
                    .with_suggestion(t!(
                        "rules.cc_set_012.extract_capture_suggestion",
                        array_key = array_key,
                        index = index
                    )),
                );
            }
            Some(_) => {}
            None => diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-012",
                    t!(
                        "rules.cc_set_012.extract_type",
                        array_key = array_key,
                        index = index,
                        actual = describe_json_type(value)
                    ),
                )
                .with_suggestion(t!(
                    "rules.cc_set_012.extract_type_suggestion",
                    array_key = array_key,
                    index = index
                )),
            ),
        }
    }

    let decode = entry.get("decode");
    if let Some(value) = decode {
        match value.as_str() {
            Some("jwt") => {}
            Some(actual) => diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-012",
                    t!(
                        "rules.cc_set_012.decode_value",
                        array_key = array_key,
                        index = index,
                        actual = actual
                    ),
                )
                .with_suggestion(t!(
                    "rules.cc_set_012.decode_suggestion",
                    array_key = array_key,
                    index = index
                )),
            ),
            None => diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-012",
                    t!(
                        "rules.cc_set_012.decode_type",
                        array_key = array_key,
                        index = index,
                        actual = describe_json_type(value)
                    ),
                )
                .with_suggestion(t!(
                    "rules.cc_set_012.decode_suggestion",
                    array_key = array_key,
                    index = index
                )),
            ),
        }
    }

    if matches!(kind, SandboxCredentialKind::EnvironmentVariable)
        && entry.contains_key("extract")
        && entry.contains_key("decode")
    {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                0,
                "CC-SET-012",
                t!(
                    "rules.cc_set_012.extract_decode_conflict",
                    array_key = array_key,
                    index = index
                ),
            )
            .with_suggestion(t!(
                "rules.cc_set_012.extract_decode_conflict_suggestion",
                array_key = array_key,
                index = index
            )),
        );
    }

    if let Some(value) = entry.get("maskClaims") {
        let claims_are_valid = value.as_array().is_some_and(|claims| {
            !claims.is_empty()
                && claims
                    .iter()
                    .all(|claim| claim.as_str().is_some_and(|claim| !claim.is_empty()))
        });
        if !claims_are_valid {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-012",
                    t!(
                        "rules.cc_set_012.mask_claims_shape",
                        array_key = array_key,
                        index = index,
                        actual = describe_json_type(value)
                    ),
                )
                .with_suggestion(t!(
                    "rules.cc_set_012.mask_claims_suggestion",
                    array_key = array_key,
                    index = index
                )),
            );
        } else if decode.and_then(serde_json::Value::as_str) != Some("jwt") {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-012",
                    t!(
                        "rules.cc_set_012.mask_claims_requires_decode",
                        array_key = array_key,
                        index = index
                    ),
                )
                .with_suggestion(t!(
                    "rules.cc_set_012.mask_claims_suggestion",
                    array_key = array_key,
                    index = index
                )),
            );
        }
    }

    if let Some(value) = entry.get("onExtractNoMatch") {
        match value.as_str() {
            Some("warn") => {}
            Some("deny" | "error")
                if !matches!(kind, SandboxCredentialKind::EnvironmentVariable)
                    || decode.and_then(serde_json::Value::as_str) != Some("jwt") => {}
            Some(actual)
                if matches!(kind, SandboxCredentialKind::EnvironmentVariable)
                    && decode.and_then(serde_json::Value::as_str) == Some("jwt") =>
            {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        0,
                        "CC-SET-012",
                        t!(
                            "rules.cc_set_012.extract_no_match_jwt_value",
                            array_key = array_key,
                            index = index,
                            actual = actual
                        ),
                    )
                    .with_suggestion(t!(
                        "rules.cc_set_012.extract_no_match_jwt_suggestion",
                        array_key = array_key,
                        index = index
                    )),
                );
            }
            Some(actual) => diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-012",
                    t!(
                        "rules.cc_set_012.extract_no_match_value",
                        array_key = array_key,
                        index = index,
                        actual = actual
                    ),
                )
                .with_suggestion(t!(
                    "rules.cc_set_012.extract_no_match_suggestion",
                    array_key = array_key,
                    index = index
                )),
            ),
            None => diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-012",
                    t!(
                        "rules.cc_set_012.extract_no_match_type",
                        array_key = array_key,
                        index = index,
                        actual = describe_json_type(value)
                    ),
                )
                .with_suggestion(t!(
                    "rules.cc_set_012.extract_no_match_suggestion",
                    array_key = array_key,
                    index = index
                )),
            ),
        }
    }

    if matches!(kind, SandboxCredentialKind::File)
        && let Some(value) = entry.get("maskDuplicates")
        && !value.is_boolean()
    {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                0,
                "CC-SET-012",
                t!(
                    "rules.cc_set_012.mask_duplicates_type",
                    array_key = array_key,
                    index = index,
                    actual = describe_json_type(value)
                ),
            )
            .with_suggestion(t!(
                "rules.cc_set_012.mask_duplicates_suggestion",
                array_key = array_key,
                index = index
            )),
        );
    }
}

fn validate_sandbox_credential_aws_pairs(
    path: &Path,
    content: &str,
    value: Option<&serde_json::Value>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(value) = value else {
        return;
    };
    let line = find_key_line(content, "awsPairs")
        .or_else(|| find_key_line(content, "credentials"))
        .unwrap_or(1);
    let Some(pairs) = value.as_array() else {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                0,
                "CC-SET-012",
                t!(
                    "rules.cc_set_012.aws_pairs_array",
                    actual = describe_json_type(value)
                ),
            )
            .with_suggestion(t!("rules.cc_set_012.aws_pairs_suggestion")),
        );
        return;
    };
    let mut seen_variables = HashMap::new();

    for (index, pair) in pairs.iter().enumerate() {
        let Some(pair) = pair.as_object() else {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-012",
                    t!(
                        "rules.cc_set_012.aws_pair_object",
                        index = index,
                        actual = describe_json_type(pair)
                    ),
                )
                .with_suggestion(t!("rules.cc_set_012.aws_pairs_suggestion")),
            );
            continue;
        };

        for field in ["accessKeyIdVar", "secretAccessKeyVar", "sessionTokenVar"] {
            let Some(field_value) = pair.get(field) else {
                if field != "sessionTokenVar" {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            line,
                            0,
                            "CC-SET-012",
                            t!(
                                "rules.cc_set_012.aws_pair_field",
                                index = index,
                                field = field
                            ),
                        )
                        .with_suggestion(t!("rules.cc_set_012.aws_pairs_suggestion")),
                    );
                }
                continue;
            };
            let Some(variable) = field_value.as_str().filter(|variable| !variable.is_empty())
            else {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        0,
                        "CC-SET-012",
                        t!(
                            "rules.cc_set_012.aws_pair_field",
                            index = index,
                            field = field
                        ),
                    )
                    .with_suggestion(t!("rules.cc_set_012.aws_pairs_suggestion")),
                );
                continue;
            };

            if let Some((first_index, first_field)) = seen_variables.get(variable) {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        0,
                        "CC-SET-012",
                        t!(
                            "rules.cc_set_012.aws_pair_duplicate",
                            index = index,
                            field = field,
                            variable = variable,
                            first_index = *first_index,
                            first_field = *first_field
                        ),
                    )
                    .with_suggestion(t!("rules.cc_set_012.aws_pair_duplicate_suggestion")),
                );
            } else {
                seen_variables.insert(variable, (index, field));
            }
        }
    }
}

fn validate_sandbox_credential_sigv4(
    path: &Path,
    content: &str,
    value: Option<&serde_json::Value>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(value) = value else {
        return;
    };
    let line = find_key_line(content, "sigv4")
        .or_else(|| find_key_line(content, "credentials"))
        .unwrap_or(1);
    let Some(policies) = value.as_object() else {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                0,
                "CC-SET-012",
                t!(
                    "rules.cc_set_012.sigv4_object",
                    actual = describe_json_type(value)
                ),
            )
            .with_suggestion(t!("rules.cc_set_012.sigv4_suggestion")),
        );
        return;
    };

    for (field, policy) in policies {
        let valid_field = matches!(field.as_str(), "streaming" | "presigned" | "sigv4a");
        let valid_value = matches!(policy.as_str(), Some("deny" | "passthrough"));
        if !valid_field || !valid_value {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-012",
                    t!(
                        "rules.cc_set_012.sigv4_entry",
                        field = field,
                        actual = policy
                            .as_str()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| describe_json_type(policy).to_string())
                    ),
                )
                .with_suggestion(t!("rules.cc_set_012.sigv4_suggestion")),
            );
        }
    }
}

fn validate_sandbox_credential_aws_tls_termination(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    credentials: &serde_json::Map<String, serde_json::Value>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(feature) = sandbox_credential_aws_feature_label(credentials) else {
        return;
    };

    let tls_terminate = value.pointer("/sandbox/network/tlsTerminate");
    if tls_terminate.is_some_and(serde_json::Value::is_object) {
        return;
    }

    // System policy files are merged with ordered managed-settings.d
    // fragments. Without the merged context, a missing value in this one file
    // does not prove that the effective policy lacks tlsTerminate.
    if tls_terminate.is_none() && is_claude_system_managed_settings_path(path) {
        return;
    }

    let line = find_key_line(content, "awsPairs")
        .or_else(|| find_key_line(content, "sigv4"))
        .or_else(|| find_key_line(content, "credentials"))
        .unwrap_or(1);
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-012",
            t!("rules.cc_set_012.tls_terminate_required", feature = feature),
        )
        .with_suggestion(t!("rules.cc_set_012.tls_terminate_suggestion")),
    );
}

fn validate_sandbox_credential_aws_scope(
    path: &Path,
    content: &str,
    credentials: &serde_json::Map<String, serde_json::Value>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(feature) = sandbox_credential_aws_feature_label(credentials) else {
        return;
    };

    if is_claude_managed_settings_path(path) || is_user_settings_path(path) {
        return;
    }

    let line = find_key_line(content, "awsPairs")
        .or_else(|| find_key_line(content, "sigv4"))
        .or_else(|| find_key_line(content, "credentials"))
        .unwrap_or(1);
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-012",
            t!("rules.cc_set_012.aws_scope", feature = feature),
        )
        .with_suggestion(t!("rules.cc_set_012.aws_scope_suggestion")),
    );
}

fn sandbox_credential_aws_feature_label(
    credentials: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    match (
        credentials.contains_key("awsPairs"),
        credentials.contains_key("sigv4"),
    ) {
        (true, true) => Some(t!("rules.cc_set_012.aws_features_both").to_string()),
        (true, false) => Some(t!("rules.cc_set_012.aws_feature_aws_pairs").to_string()),
        (false, true) => Some(t!("rules.cc_set_012.aws_feature_sigv4").to_string()),
        (false, false) => None,
    }
}

fn is_user_settings_path(path: &Path) -> bool {
    if path.file_name().and_then(|name| name.to_str()) != Some("settings.json") {
        return false;
    }

    let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else {
        return false;
    };
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        let Ok(current_dir) = std::env::current_dir() else {
            return false;
        };
        current_dir.join(path)
    };

    absolute_path == Path::new(&home).join(".claude").join("settings.json")
}

fn has_javascript_capturing_group(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let mut escaped = false;
    let mut in_character_class = false;
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        if byte == b'\\' {
            escaped = true;
            index += 1;
            continue;
        }
        if byte == b'[' {
            in_character_class = true;
            index += 1;
            continue;
        }
        if byte == b']' && in_character_class {
            in_character_class = false;
            index += 1;
            continue;
        }
        if byte != b'(' || in_character_class {
            index += 1;
            continue;
        }

        if bytes.get(index + 1) != Some(&b'?') {
            return true;
        }
        if bytes.get(index + 2) == Some(&b'<')
            && !matches!(bytes.get(index + 3), Some(b'=') | Some(b'!'))
        {
            return true;
        }
        index += 1;
    }

    false
}

/// CC-SET-013: Validate `autoMode.classifyAllShell`. Claude Code v2.1.193
/// added this nested auto-mode setting so all Bash/PowerShell commands route
/// through the auto-mode classifier. The release note describes it as an
/// enabled/disabled setting, so agnix treats it like the other CC-SET boolean
/// toggles: only strict JSON booleans are valid when present.
fn validate_auto_mode_classify_all_shell(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value.pointer("/autoMode/classifyAllShell") else {
        return;
    };

    if field_value.is_boolean() || field_value.is_null() {
        return;
    }

    let actual = describe_json_type(field_value);
    let line = find_key_line(content, "classifyAllShell")
        .or_else(|| find_key_line(content, "autoMode"))
        .unwrap_or(1);

    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-013",
            t!("rules.cc_set_013.message", actual = actual),
        )
        .with_suggestion(t!("rules.cc_set_013.suggestion")),
    );
}

/// CC-SET-014: Warn when `autoMode` appears in `.claude/settings.local.json`.
///
/// Claude Code v2.1.207 stopped reading `autoMode` from the repo-resident
/// `.claude/settings.local.json`; the key is silently ignored there. Auto
/// mode configuration must live in `~/.claude/settings.json` instead.
///
/// Fires on any non-null value — the problem is presence in the wrong file,
/// not value shape (CC-SET-013 covers the classifyAllShell shape and can
/// co-fire). `null` is treated as field-absent, consistent with the rest of
/// the CC-SET family. Does NOT fire on `settings.json` or
/// `managed-settings.json` — `autoMode` is still read from those.
fn validate_auto_mode_in_settings_local(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let is_local = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n == "settings.local.json")
        .unwrap_or(false);
    if !is_local {
        return;
    }

    let Some(field_value) = value.get("autoMode") else {
        return;
    };
    if field_value.is_null() {
        return;
    }

    let line = find_key_line(content, "autoMode").unwrap_or(1);

    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-014",
            t!("rules.cc_set_014.message"),
        )
        .with_suggestion(t!("rules.cc_set_014.suggestion")),
    );
}

/// CC-SET-015: Warn on `pluginConfigs` in project-level settings files.
///
/// Claude Code v2.1.207 changed where plugin option values are read: only
/// user-level settings (`~/.claude/settings.json`), `--settings` files, and
/// managed settings are honored. Repo-resident `.claude/settings.json` and
/// `.claude/settings.local.json` no longer supply `pluginConfigs`; the key
/// is silently ignored there. Managed settings files are honored, so they
/// are skipped.
///
/// Fires on any non-null value; `null` is treated as field-absent,
/// consistent with the CC-SET family. Note agnix lints repo checkouts —
/// a user linting their home directory would hit a false positive on
/// `~/.claude/settings.json`, the same accepted ambiguity discussed in the
/// CC-SET-002 doc comment.
fn validate_plugin_configs_scope(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if is_claude_managed_settings_path(path) {
        return;
    }

    let Some(field_value) = value.get("pluginConfigs") else {
        return;
    };
    if field_value.is_null() {
        return;
    }

    let line = find_key_line(content, "pluginConfigs").unwrap_or(1);

    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-015",
            t!("rules.cc_set_015.message"),
        )
        .with_suggestion(t!("rules.cc_set_015.suggestion")),
    );
}

/// CC-SET-016: Warn on deprecated path-scoped permission rule forms.
///
/// Claude Code v2.1.210 added a startup warning for permission entries of the
/// form `Write(path)`, `NotebookEdit(path)`, and `Glob(path)`. These forms are
/// deprecated; users should use `Edit(path)` or `Read(path)` instead.
///
/// Fires on `permissions.allow`, `permissions.deny`, and `permissions.ask`
/// arrays in `.claude/settings.json`, `.claude/settings.local.json`, and
/// `.claude/managed-settings.json`. For each deprecated entry:
/// - `Write(...)` and `NotebookEdit(...)` → suggest `Edit(...)` with the same
///   argument.
/// - `Glob(...)` → suggest `Read(...)` with the same argument.
///
/// Only path-scoped forms (those with a `(`) are flagged; bare tool names like
/// `"Write"` alone are NOT flagged — the deprecation is specifically for the
/// `Write(path)` forms per the v2.1.210 release notes.
///
/// Non-array `permissions.allow/deny/ask`, non-string entries, and absent
/// `permissions` key are silently skipped (other rules' concern).
fn validate_deprecated_permission_tool_forms(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(permissions) = value.get("permissions") else {
        return;
    };
    let Some(permissions_obj) = permissions.as_object() else {
        return;
    };

    for array_key in &["allow", "deny", "ask"] {
        let Some(arr) = permissions_obj.get(*array_key) else {
            continue;
        };
        let Some(arr) = arr.as_array() else {
            continue;
        };

        for entry in arr {
            let Some(s) = entry.as_str() else {
                continue;
            };

            // Only flag path-scoped forms: must contain '('
            let replacement = s
                .strip_prefix("Write(")
                .or_else(|| s.strip_prefix("NotebookEdit("))
                .map(|inner| format!("Edit({inner}"))
                .or_else(|| s.strip_prefix("Glob(").map(|inner| format!("Read({inner}")));

            let Some(replacement) = replacement else {
                continue;
            };

            // Array string values aren't found by find_key_line (no trailing
            // ':'), so fall back to the containing array key line.
            let line = find_key_line(content, array_key)
                .or_else(|| find_key_line(content, "permissions"))
                .unwrap_or(1);

            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-016",
                    t!(
                        "rules.cc_set_016.message",
                        rule = s,
                        replacement = replacement.as_str()
                    ),
                )
                .with_suggestion(t!(
                    "rules.cc_set_016.suggestion",
                    rule = s,
                    replacement = replacement.as_str()
                )),
            );
        }
    }
}

/// CC-SET-017: Validate `sandbox.filesystem.disabled`. Claude Code 2.1.216
/// added this setting to disable filesystem isolation while leaving network
/// isolation active. It is a strict boolean toggle; quoted strings and other
/// JSON types are rejected by the settings schema.
///
/// Missing and null values are treated as absent. A non-object `sandbox` or
/// `filesystem` container is left to a broader shape validator.
fn validate_sandbox_filesystem_disabled(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value
        .get("sandbox")
        .and_then(serde_json::Value::as_object)
        .and_then(|sandbox| sandbox.get("filesystem"))
        .and_then(serde_json::Value::as_object)
        .and_then(|filesystem| filesystem.get("disabled"))
    else {
        return;
    };

    if field_value.is_boolean() || field_value.is_null() {
        return;
    }

    let actual = describe_json_type(field_value);
    let line = find_key_line(content, "disabled")
        .or_else(|| find_key_line(content, "filesystem"))
        .unwrap_or(1);
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-017",
            t!("rules.cc_set_017.message", actual = actual),
        )
        .with_suggestion(t!("rules.cc_set_017.suggestion")),
    );
}

/// CC-SET-018: Validate `emojiCompletionEnabled`. Claude Code 2.1.217 added
/// this setting to control `:emoji_name:` completion. Only a strict boolean is
/// accepted by the settings schema.
fn validate_emoji_completion_enabled(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value.get("emojiCompletionEnabled") else {
        return;
    };

    if field_value.is_boolean() || field_value.is_null() {
        return;
    }

    let actual = describe_json_type(field_value);
    let line = find_key_line(content, "emojiCompletionEnabled").unwrap_or(1);
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-018",
            t!("rules.cc_set_018.message", actual = actual),
        )
        .with_suggestion(t!("rules.cc_set_018.suggestion")),
    );
}

/// CC-SET-019: Validate `sandbox.network.strictAllowlist`. Claude Code 2.1.219
/// added this strict boolean setting for managed network allowlists.
fn validate_sandbox_network_strict_allowlist(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value
        .get("sandbox")
        .and_then(serde_json::Value::as_object)
        .and_then(|sandbox| sandbox.get("network"))
        .and_then(serde_json::Value::as_object)
        .and_then(|network| network.get("strictAllowlist"))
    else {
        return;
    };

    if field_value.is_boolean() || field_value.is_null() {
        return;
    }

    let actual = describe_json_type(field_value);
    let line = find_key_line(content, "strictAllowlist")
        .or_else(|| find_key_line(content, "network"))
        .unwrap_or(1);
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-019",
            t!("rules.cc_set_019.message", actual = actual),
        )
        .with_suggestion(t!("rules.cc_set_019.suggestion")),
    );
}

/// CC-SET-020: Validate the workflow size enum added in Claude Code 2.1.219.
fn validate_workflow_size_guideline(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value.get("workflowSizeGuideline") else {
        return;
    };
    if field_value.is_null() {
        return;
    }

    if field_value
        .as_str()
        .is_some_and(|value| WORKFLOW_SIZE_GUIDELINE_ALLOWED.contains(&value))
    {
        return;
    }

    let actual = field_value
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_else(|| describe_json_type(field_value).to_string());
    let line = find_key_line(content, "workflowSizeGuideline").unwrap_or(1);
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-020",
            t!("rules.cc_set_020.message", actual = actual),
        )
        .with_suggestion(t!("rules.cc_set_020.suggestion")),
    );
}

/// CC-SET-021: Warn when project settings try to enable Remote Control.
///
/// Claude Code 2.1.222 stopped honoring `remoteControlAtStartup: true` in
/// repo-local settings. A project may still set it to `false` to disable
/// Remote Control, while enabling it must happen at user scope via `/config`.
/// Managed settings remain an honored policy tier and are excluded.
fn validate_remote_control_at_startup_scope(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if is_claude_managed_settings_path(path)
        || value.get("remoteControlAtStartup") != Some(&serde_json::Value::Bool(true))
    {
        return;
    }

    let line = find_key_line(content, "remoteControlAtStartup").unwrap_or(1);
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-021",
            t!("rules.cc_set_021.message"),
        )
        .with_suggestion(t!("rules.cc_set_021.suggestion")),
    );
}

/// CC-SET-022: Validate the cross-session inbound-message policy enum.
fn validate_cross_session_inbound(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value.get("crossSessionInbound") else {
        return;
    };
    if field_value.is_null()
        || field_value
            .as_str()
            .is_some_and(|value| CROSS_SESSION_INBOUND_ALLOWED.contains(&value))
    {
        return;
    }

    let actual = field_value
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_else(|| describe_json_type(field_value).to_string());
    let line = find_key_line(content, "crossSessionInbound").unwrap_or(1);
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-022",
            t!("rules.cc_set_022.message", actual = actual),
        )
        .with_suggestion(t!("rules.cc_set_022.suggestion")),
    );
}

/// CC-SET-023: Validate the remote-dialog expiry enum.
fn validate_dialog_expiry(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value.get("dialogExpiry") else {
        return;
    };
    if field_value.is_null()
        || field_value
            .as_str()
            .is_some_and(|value| DIALOG_EXPIRY_ALLOWED.contains(&value))
    {
        return;
    }

    let actual = field_value
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_else(|| describe_json_type(field_value).to_string());
    let line = find_key_line(content, "dialogExpiry").unwrap_or(1);
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-023",
            t!("rules.cc_set_023.message", actual = actual),
        )
        .with_suggestion(t!("rules.cc_set_023.suggestion")),
    );
}

/// CC-SET-024: Warn when project settings override the sandbox ripgrep binary.
///
/// Claude Code 2.1.232 restricted `sandbox.ripgrep` to user settings, managed
/// settings, and the `--settings` flag, so a checked-out repository can no
/// longer point the sandbox at its own ripgrep. The same scope rule already
/// covers `sandbox.credentials` masking (CC-SET-012), so user and managed
/// settings are excluded here for the same reason.
///
/// Null is treated as "field absent" by JSON convention.
fn validate_sandbox_ripgrep_scope(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(sandbox) = value.get("sandbox").and_then(|sandbox| sandbox.as_object()) else {
        return;
    };
    match sandbox.get("ripgrep") {
        None => return,
        Some(field_value) if field_value.is_null() => return,
        Some(_) => {}
    }

    if is_claude_managed_settings_path(path) || is_user_settings_path(path) {
        return;
    }

    let line = find_key_line(content, "ripgrep")
        .or_else(|| find_key_line(content, "sandbox"))
        .unwrap_or(1);
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CC-SET-024",
            t!("rules.cc_set_024.message"),
        )
        .with_suggestion(t!("rules.cc_set_024.suggestion")),
    );
}

/// CC-SET-025: Validate the user-or-managed `modelPicker` setting introduced
/// in Claude Code 2.1.242. Repository-local values are ignored. At an honored
/// scope the setting is an object with a required `options` array, optional
/// boolean `replaceBuiltInOptions`, and rows containing a required non-empty
/// `model` plus optional string `label` and `description` fields.
fn validate_model_picker(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(model_picker) = value.get("modelPicker") else {
        return;
    };
    if model_picker.is_null() {
        return;
    }

    let line = find_key_line(content, "modelPicker").unwrap_or(1);
    if !is_claude_managed_settings_path(path) && !is_user_settings_path(path) {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                0,
                "CC-SET-025",
                t!("rules.cc_set_025.scope_message"),
            )
            .with_suggestion(t!("rules.cc_set_025.scope_suggestion")),
        );
    }

    let Some(object) = model_picker.as_object() else {
        diagnostics.push(model_picker_shape_diagnostic(
            path,
            line,
            t!("rules.cc_set_025.reasons.not_object").to_string(),
        ));
        return;
    };

    match object.get("options") {
        Some(options) if options.is_array() => {
            for (index, row) in options
                .as_array()
                .expect("checked array")
                .iter()
                .enumerate()
            {
                let Some(row) = row.as_object() else {
                    diagnostics.push(model_picker_shape_diagnostic(
                        path,
                        line,
                        t!("rules.cc_set_025.reasons.row_not_object", index = index).to_string(),
                    ));
                    continue;
                };
                if !row
                    .get("model")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|model| !model.trim().is_empty())
                {
                    diagnostics.push(model_picker_shape_diagnostic(
                        path,
                        line,
                        t!("rules.cc_set_025.reasons.model_not_string", index = index).to_string(),
                    ));
                }
                for field in ["label", "description"] {
                    if row.get(field).is_some_and(|field_value| {
                        !field_value.is_null() && !field_value.is_string()
                    }) {
                        diagnostics.push(model_picker_shape_diagnostic(
                            path,
                            line,
                            t!(
                                "rules.cc_set_025.reasons.field_not_string",
                                index = index,
                                field = field
                            )
                            .to_string(),
                        ));
                    }
                }
            }
        }
        _ => diagnostics.push(model_picker_shape_diagnostic(
            path,
            line,
            t!("rules.cc_set_025.reasons.options_not_array").to_string(),
        )),
    }

    if object
        .get("replaceBuiltInOptions")
        .is_some_and(|replace| !replace.is_null() && !replace.is_boolean())
    {
        diagnostics.push(model_picker_shape_diagnostic(
            path,
            line,
            t!("rules.cc_set_025.reasons.replace_not_boolean").to_string(),
        ));
    }
}

fn model_picker_shape_diagnostic(path: &Path, line: usize, reason: String) -> Diagnostic {
    Diagnostic::warning(
        path.to_path_buf(),
        line,
        0,
        "CC-SET-025",
        t!("rules.cc_set_025.shape_message", reason = reason),
    )
    .with_suggestion(t!("rules.cc_set_025.shape_suggestion"))
}

/// CC-SET-028: Validate `feedbackDrafts`.
///
/// A string enum, not a boolean: `"off"` removes the SendFeedback tool, while
/// `"notify"` and `"quiet"` differ only in whether a card appears above the
/// prompt. A boolean `false` is the natural wrong guess and silently leaves the
/// default `"notify"` in place.
fn validate_feedback_drafts(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value.get("feedbackDrafts") else {
        return;
    };
    if field_value.is_null()
        || field_value
            .as_str()
            .is_some_and(|mode| FEEDBACK_DRAFTS_ALLOWED.contains(&mode))
    {
        return;
    }

    let actual = field_value
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_else(|| describe_json_type(field_value).to_string());
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            find_key_line(content, "feedbackDrafts").unwrap_or(1),
            0,
            "CC-SET-028",
            t!("rules.cc_set_028.message", actual = actual),
        )
        .with_suggestion(t!("rules.cc_set_028.suggestion")),
    );
}

/// CC-SET-029: Validate the `spinnerTipsOverride` object and its tip entries.
///
/// Claude Code drops an invalid tip entry with a debug warning instead of
/// rejecting the settings file, so a malformed tip is simply never shown - the
/// silent-failure class agnix exists to catch.
fn validate_spinner_tips_override(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value.get("spinnerTipsOverride") else {
        return;
    };
    if field_value.is_null() {
        return;
    }

    let line = find_key_line(content, "spinnerTipsOverride").unwrap_or(1);
    let mut push = |message: String| {
        diagnostics.push(
            Diagnostic::warning(path.to_path_buf(), line, 0, "CC-SET-029", message)
                .with_suggestion(t!("rules.cc_set_029.suggestion")),
        );
    };

    let Some(object) = field_value.as_object() else {
        push(
            t!(
                "rules.cc_set_029.not_object",
                actual = describe_json_type(field_value)
            )
            .to_string(),
        );
        return;
    };

    for key in object.keys() {
        if !SPINNER_TIPS_OVERRIDE_FIELDS.contains(&key.as_str()) {
            push(t!("rules.cc_set_029.unknown_field", field = key.as_str()).to_string());
        }
    }

    if let Some(label) = object.get("label") {
        match label.as_str() {
            Some(text) if text.chars().count() <= SPINNER_TIP_LABEL_MAX_LEN => {}
            Some(text) => push(
                t!(
                    "rules.cc_set_029.label_too_long",
                    len = text.chars().count(),
                    max = SPINNER_TIP_LABEL_MAX_LEN
                )
                .to_string(),
            ),
            None => push(
                t!(
                    "rules.cc_set_029.label_not_string",
                    actual = describe_json_type(label)
                )
                .to_string(),
            ),
        }
    }

    if let Some(exclude) = object.get("excludeDefault")
        && !exclude.is_boolean()
    {
        push(
            t!(
                "rules.cc_set_029.exclude_default_not_bool",
                actual = describe_json_type(exclude)
            )
            .to_string(),
        );
    }

    if let Some(tips_file) = object.get("tipsFile") {
        match tips_file.as_str() {
            // Documented as "an absolute or `~/` path": a relative path is
            // resolved against nothing predictable, so the file never loads.
            Some(text) if text.starts_with('/') || text.starts_with("~/") => {}
            Some(text) if is_windows_absolute_path(text) => {}
            Some(text) => push(t!("rules.cc_set_029.tips_file_relative", path = text).to_string()),
            None => push(
                t!(
                    "rules.cc_set_029.tips_file_not_string",
                    actual = describe_json_type(tips_file)
                )
                .to_string(),
            ),
        }
    }

    let Some(tips) = object.get("tips") else {
        return;
    };
    let Some(entries) = tips.as_array() else {
        push(
            t!(
                "rules.cc_set_029.tips_not_array",
                actual = describe_json_type(tips)
            )
            .to_string(),
        );
        return;
    };

    for (index, entry) in entries.iter().enumerate() {
        let position = index + 1;
        // A plain string is a valid tip and takes the documented defaults.
        if entry.is_string() {
            continue;
        }
        let Some(tip) = entry.as_object() else {
            push(
                t!(
                    "rules.cc_set_029.tip_not_object",
                    index = position,
                    actual = describe_json_type(entry)
                )
                .to_string(),
            );
            continue;
        };

        match tip.get("id").and_then(serde_json::Value::as_str) {
            Some(id)
                if !id.is_empty()
                    && id.chars().count() <= SPINNER_TIP_ID_MAX_LEN
                    && id
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')) => {}
            _ => push(t!("rules.cc_set_029.tip_id", index = position).to_string()),
        }

        match tip.get("text").and_then(serde_json::Value::as_str) {
            Some(text)
                if !text.trim().is_empty() && text.chars().count() <= SPINNER_TIP_TEXT_MAX_LEN => {}
            _ => push(
                t!(
                    "rules.cc_set_029.tip_text",
                    index = position,
                    max = SPINNER_TIP_TEXT_MAX_LEN
                )
                .to_string(),
            ),
        }

        for (field, (min, max)) in [
            ("cooldownSessions", SPINNER_TIP_COOLDOWN_RANGE),
            ("priority", SPINNER_TIP_PRIORITY_RANGE),
        ] {
            if let Some(number) = tip.get(field) {
                let in_range = number
                    .as_i64()
                    .is_some_and(|value| (min..=max).contains(&value));
                if !in_range {
                    push(
                        t!(
                            "rules.cc_set_029.tip_range",
                            index = position,
                            field = field,
                            min = min,
                            max = max
                        )
                        .to_string(),
                    );
                }
            }
        }

        for key in tip.keys() {
            if !["id", "text", "cooldownSessions", "priority"].contains(&key.as_str()) {
                push(
                    t!(
                        "rules.cc_set_029.tip_unknown_field",
                        index = position,
                        field = key.as_str()
                    )
                    .to_string(),
                );
            }
        }
    }
}

/// `C:\path` or `C:/path`, which `tipsFile` accepts on Windows.
fn is_windows_absolute_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\')
}

/// CC-SET-026: Validate the main-conversation prompt-cache TTL.
fn validate_prompt_cache_ttl(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_prompt_cache_ttl_field(
        path,
        content,
        value,
        "promptCacheTtl",
        "CC-SET-026",
        diagnostics,
    );
}

/// CC-SET-027: Validate the subagent/background-request prompt-cache TTL.
fn validate_subagent_prompt_cache_ttl(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_prompt_cache_ttl_field(
        path,
        content,
        value,
        "subagentPromptCacheTtl",
        "CC-SET-027",
        diagnostics,
    );
}

#[allow(clippy::too_many_arguments)]
fn validate_prompt_cache_ttl_field(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    field: &str,
    rule: &'static str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field_value) = value.get(field) else {
        return;
    };
    if field_value.is_null()
        || field_value
            .as_str()
            .is_some_and(|ttl| PROMPT_CACHE_TTL_ALLOWED.contains(&ttl))
    {
        return;
    }

    let actual = field_value
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_else(|| describe_json_type(field_value).to_string());
    let (message, suggestion) = match rule {
        "CC-SET-026" => (
            t!("rules.cc_set_026.message", actual = actual).to_string(),
            t!("rules.cc_set_026.suggestion").to_string(),
        ),
        "CC-SET-027" => (
            t!("rules.cc_set_027.message", actual = actual).to_string(),
            t!("rules.cc_set_027.suggestion").to_string(),
        ),
        _ => unreachable!("prompt-cache validator called with unknown rule"),
    };
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            find_key_line(content, field).unwrap_or(1),
            0,
            rule,
            message,
        )
        .with_suggestion(suggestion),
    );
}

/// Renders a `serde_json::Value` variant as a short human-readable type
/// name for diagnostic messages. The CC-SET-002 caller filters `Bool` and
/// `Null` before reaching this, but every variant is covered so the
/// helper stays useful for future reuse.
fn describe_json_type(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::String(_) => "string",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Null => "null",
    }
}

/// 1-indexed line of the first occurrence of `"<key>":` in a JSON document,
/// skipping matches inside string literals. Returns None if the key isn't
/// found. We look for the quoted key followed by JSON whitespace and `:`
/// so `"prUrlTemplateX"` doesn't accidentally match when searching for
/// `prUrlTemplate`, and so that a key-looking fragment inside a prose
/// value (like `"note": "prUrlTemplate in prose"`) is ignored.
///
/// Byte-slice comparison against the needle keeps the scanner safe across
/// UTF-8 content: `bytes[i..j] == needle_bytes` cannot panic mid-codepoint
/// the way `content[i..j] == needle` can when the tail lands inside a
/// multi-byte char.
///
/// Only ASCII keys are supported (the needle contains a bare `"` prefix/
/// suffix with no JSON-string escaping). That's fine for the documented
/// Claude Code settings keys, which are all ASCII identifiers. If future
/// rules need keys with escapes, build the needle with proper escaping.
fn find_key_line(content: &str, key: &str) -> Option<usize> {
    debug_assert!(
        key.is_ascii() && !key.contains('"') && !key.contains('\\'),
        "find_key_line expects ASCII key without quotes or backslashes"
    );
    let needle = format!("\"{key}\"");
    let needle_bytes = needle.as_bytes();
    let needle_len = needle_bytes.len();
    let bytes = content.as_bytes();
    let mut in_string = false;
    let mut escape = false;
    let mut line = 1usize;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\n' {
            line += 1;
            i += 1;
            continue;
        }
        if escape {
            escape = false;
            i += 1;
            continue;
        }
        if b == b'\\' && in_string {
            escape = true;
            i += 1;
            continue;
        }
        if b == b'"' {
            // Tentatively match "<key>": at this position.
            if !in_string
                && i + needle_len <= bytes.len()
                && &bytes[i..i + needle_len] == needle_bytes
            {
                // Skip JSON whitespace (space, tab, CR, LF) between the key
                // and `:`. JSON RFC 8259 section 2 defines these four.
                // The returned line is the line where the key opens, so we
                // don't need to track newlines past i - we just need to
                // know the colon is reachable.
                let mut j = i + needle_len;
                while j < bytes.len() && matches!(bytes[j], b' ' | b'\t' | b'\n' | b'\r') {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b':' {
                    return Some(line);
                }
            }
            in_string = !in_string;
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::LintConfig, pipeline::validate_content, registry::ValidatorRegistry};
    use std::path::PathBuf;

    fn validate(content: &str) -> Vec<Diagnostic> {
        validate_at(".claude/settings.json", content)
    }

    fn validate_at(path_str: &str, content: &str) -> Vec<Diagnostic> {
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(path_str);
        validator.validate(&path, content, &LintConfig::default())
    }

    // ===== Scope guard: only runs on Claude Code settings =====

    #[test]
    fn test_ignores_non_claude_settings_path() {
        let content = r#"{"prUrlTemplate": 123}"#;
        let diagnostics = validate_at(".amp/settings.json", content);
        assert!(
            diagnostics.is_empty(),
            ".amp/settings.json must not be validated by CC-SET"
        );
    }

    #[test]
    fn test_ignores_random_json() {
        let content = r#"{"prUrlTemplate": 123}"#;
        let diagnostics = validate_at("some/other/file.json", content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_runs_on_settings_local() {
        let content = r#"{"prUrlTemplate": 123}"#;
        let diagnostics = validate_at(".claude/settings.local.json", content);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_runs_on_managed_settings() {
        let content = r#"{"prUrlTemplate": 123}"#;
        let diagnostics = validate_at(".claude/managed-settings.json", content);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_system_managed_settings_run_aws_checks_through_pipeline() {
        let content = r#"{
          "crossSessionInbound": "prompt",
          "dialogExpiry": "30s",
          "pluginConfigs": {"my-plugin": {}},
          "remoteControlAtStartup": true,
          "sandbox": {
            "network": {"tlsTerminate": {}},
            "credentials": {
              "awsPairs": [{
                "accessKeyIdVar": "AWS_KEY",
                "secretAccessKeyVar": "AWS_KEY"
              }]
            }
          }
        }"#;
        let config = LintConfig::default();
        let registry = ValidatorRegistry::with_defaults();

        for path in [
            "/Library/Application Support/ClaudeCode/managed-settings.json",
            "/etc/claude-code/managed-settings.json",
            "C:/Program Files/ClaudeCode/managed-settings.json",
            "/Library/Application Support/ClaudeCode/managed-settings.d/10-security.json",
            "/etc/claude-code/managed-settings.d/sandbox.json",
            "C:/Program Files/ClaudeCode/managed-settings.d/90-credentials.json",
        ] {
            let diagnostics = validate_content(Path::new(path), content, &config, &registry);
            for rule in ["CC-SET-012", "CC-SET-022", "CC-SET-023"] {
                assert_eq!(
                    diagnostics
                        .iter()
                        .filter(|diagnostic| diagnostic.rule == rule)
                        .count(),
                    1,
                    "expected {rule} through production dispatch for {path}"
                );
            }
            assert!(
                diagnostics
                    .iter()
                    .find(|diagnostic| diagnostic.rule == "CC-SET-012")
                    .is_some_and(|diagnostic| diagnostic.message.contains("reuses"))
            );
            for rule in ["CC-SET-015", "CC-SET-021"] {
                assert!(
                    diagnostics.iter().all(|diagnostic| diagnostic.rule != rule),
                    "unexpected project-scope {rule} for managed settings path {path}"
                );
            }
        }
    }

    #[test]
    fn test_split_system_managed_settings_do_not_false_positive_aws_tls() {
        let credentials = r#"{"sandbox":{"credentials":{"awsPairs":[]}}}"#;
        let network = r#"{"sandbox":{"network":{"tlsTerminate":{}}}}"#;
        let config = LintConfig::default();
        let registry = ValidatorRegistry::with_defaults();

        for (credentials_path, network_path) in [
            (
                "/Library/Application Support/ClaudeCode/managed-settings.d/10-credentials.json",
                "/Library/Application Support/ClaudeCode/managed-settings.d/20-network.json",
            ),
            (
                "/etc/claude-code/managed-settings.d/10-credentials.json",
                "/etc/claude-code/managed-settings.d/20-network.json",
            ),
            (
                "C:/Program Files/ClaudeCode/managed-settings.d/10-credentials.json",
                "C:/Program Files/ClaudeCode/managed-settings.d/20-network.json",
            ),
        ] {
            for (path, content) in [(credentials_path, credentials), (network_path, network)] {
                let diagnostics = validate_content(Path::new(path), content, &config, &registry);
                assert!(
                    diagnostics.iter().all(|diagnostic| {
                        diagnostic.rule != "CC-SET-012"
                            || !diagnostic.message.contains("tlsTerminate")
                    }),
                    "unexpected per-file tlsTerminate diagnostic for merged source {path}"
                );
            }
        }
    }

    // ===== CC-SET-001 positive =====

    #[test]
    fn test_absent_field_is_fine() {
        let content = r#"{"model": "claude-sonnet-4"}"#;
        let diagnostics = validate(content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_valid_template_with_owner_repo_number() {
        let content =
            r#"{"prUrlTemplate": "https://reviews.example.com/{owner}/{repo}/pull/{number}"}"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.is_empty(),
            "documented example template must not flag, got {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_valid_template_with_just_url_placeholder() {
        // The docs list {url} as one of the substitutable placeholders -
        // using only {url} is valid.
        let content = r#"{"prUrlTemplate": "https://shortlinks.example.com/pr?u={url}"}"#;
        let diagnostics = validate(content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_valid_template_with_host() {
        let content = r#"{"prUrlTemplate": "https://{host}/{owner}/{repo}/pull/{number}"}"#;
        let diagnostics = validate(content);
        assert!(diagnostics.is_empty());
    }

    // ===== CC-SET-001 negative =====

    #[test]
    fn test_type_error_number() {
        let content = r#"{"prUrlTemplate": 123}"#;
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-001")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(
            hits[0].message.contains("string"),
            "type-error message should mention string, got: {}",
            hits[0].message
        );
    }

    #[test]
    fn test_type_error_array() {
        let content =
            r#"{"prUrlTemplate": ["https://reviews.example.com/{owner}/{repo}/pull/{number}"]}"#;
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-001")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn test_type_error_null() {
        let content = r#"{"prUrlTemplate": null}"#;
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-001")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn test_empty_string_flags() {
        let content = r#"{"prUrlTemplate": ""}"#;
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-001")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.to_lowercase().contains("empty"));
    }

    #[test]
    fn test_template_without_any_placeholder_warns() {
        // Likely misconfiguration: the URL won't substitute per-PR fields.
        let content = r#"{"prUrlTemplate": "https://reviews.example.com/"}"#;
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-001")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits[0].level,
            crate::diagnostics::DiagnosticLevel::Warning,
            "missing-placeholder is a WARNING, not ERROR"
        );
        assert!(hits[0].message.to_lowercase().contains("placeholder"));
    }

    #[test]
    fn test_unknown_placeholder_is_not_flagged() {
        // {branch} is not in the documented list but Claude Code leaves
        // unknown placeholders literal. Having at least one documented
        // placeholder ({number}) is enough.
        let content =
            r#"{"prUrlTemplate": "https://reviews.example.com/{owner}/{repo}/{branch}/{number}"}"#;
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-001")
            .collect();
        assert!(
            hits.is_empty(),
            "unknown placeholders are not flagged when at least one documented placeholder is present"
        );
    }

    // ===== Line reporting =====

    #[test]
    fn test_line_points_at_prurltemplate_line() {
        let content = "{\n  \"model\": \"claude-sonnet-4\",\n  \"prUrlTemplate\": 123\n}";
        let diagnostics = validate(content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-SET-001")
            .expect("CC-SET-001 diagnostic");
        assert_eq!(hit.line, 3, "line must point at the prUrlTemplate line");
    }

    #[test]
    fn test_key_in_string_literal_does_not_confuse_line_scanner() {
        let content =
            "{\n  \"note\": \"prUrlTemplate mentioned in prose\",\n  \"prUrlTemplate\": 123\n}";
        let diagnostics = validate(content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-SET-001")
            .expect("CC-SET-001 diagnostic");
        assert_eq!(
            hit.line, 3,
            "scanner must ignore \"prUrlTemplate\" mentions inside string values"
        );
    }

    #[test]
    fn test_does_not_panic_on_non_ascii_json_content() {
        // Regression: byte-slice comparison keeps find_key_line safe when
        // string values contain multi-byte UTF-8. A &str slice over an
        // arbitrary byte window could panic mid-codepoint.
        let content = "{\n  \"note\": \"\u{1F525} prUrlTemplate mentioned in UTF-8 value \u{4e2d}\u{6587}\",\n  \"prUrlTemplate\": 123\n}";
        let diagnostics = validate(content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-SET-001")
            .expect("CC-SET-001 diagnostic");
        assert_eq!(hit.line, 3);
    }

    #[test]
    fn test_accepts_newline_between_key_and_colon() {
        // JSON permits any whitespace (space, tab, CR, LF) between a key
        // and its colon. The scanner must handle newlines in that gap or
        // it'll fall back to line 1 on pretty-printed configs.
        let content = "{\n  \"prUrlTemplate\"\n    : 123\n}";
        let diagnostics = validate(content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-SET-001")
            .expect("CC-SET-001 diagnostic");
        // The diagnostic line is where the key opens (line 2), which is
        // the most useful target for editor squigglies.
        assert_eq!(hit.line, 2);
    }

    #[test]
    fn test_prefix_typo_does_not_match() {
        // "prUrlTemplateX" must not match when searching for prUrlTemplate.
        let content = "{\n  \"prUrlTemplateX\": \"ignored\",\n  \"prUrlTemplate\": 123\n}";
        let diagnostics = validate(content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-SET-001")
            .expect("CC-SET-001 diagnostic");
        assert_eq!(hit.line, 3);
    }

    // ===== Rule disable =====

    #[test]
    fn test_can_be_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CC-SET-001".to_string()];
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.json");
        let diagnostics = validator.validate(&path, r#"{"prUrlTemplate": 123}"#, &config);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_malformed_json_is_silent() {
        // Hooks validator emits a diagnostic for malformed JSON; we don't
        // duplicate that here - just bail silently so we don't double-report.
        let content = r#"{"prUrlTemplate": not valid json"#;
        let diagnostics = validate(content);
        assert!(diagnostics.is_empty());
    }

    // ===== CC-SET-002 =====

    #[test]
    fn test_channels_enabled_absent_is_fine() {
        let content = r#"{"model": "claude-sonnet-4"}"#;
        let diagnostics = validate(content);
        assert!(
            !diagnostics.iter().any(|d| d.rule == "CC-SET-002"),
            "CC-SET-002 must not fire when channelsEnabled is absent"
        );
    }

    #[test]
    fn test_channels_enabled_true_is_fine() {
        let content = r#"{"channelsEnabled": true}"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-SET-002"));
    }

    #[test]
    fn test_channels_enabled_false_is_fine() {
        // Explicit opt-out is a valid configuration.
        let content = r#"{"channelsEnabled": false}"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-SET-002"));
    }

    #[test]
    fn test_channels_enabled_null_is_not_flagged() {
        // null is conventionally "field-absent" - don't double up with any
        // future no-explicit-value rule.
        let content = r#"{"channelsEnabled": null}"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-SET-002"));
    }

    #[test]
    fn test_channels_enabled_quoted_string_flags() {
        // The documented footgun: "true" as a string leaves Channels
        // silently disabled.
        let content = r#"{"channelsEnabled": "true"}"#;
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-002")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits[0].level,
            crate::diagnostics::DiagnosticLevel::Warning,
            "CC-SET-002 is a warning, not a hard error"
        );
        assert!(
            hits[0].message.to_lowercase().contains("boolean"),
            "message should mention the required type"
        );
        assert!(
            hits[0].message.to_lowercase().contains("string"),
            "message should name the actual type seen"
        );
    }

    #[test]
    fn test_channels_enabled_number_flags() {
        let content = r#"{"channelsEnabled": 1}"#;
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-002")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.to_lowercase().contains("number"));
    }

    #[test]
    fn test_channels_enabled_array_flags() {
        let content = r#"{"channelsEnabled": [true]}"#;
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-002")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn test_channels_enabled_runs_on_managed_settings() {
        // This is the documented target (console orgs with managed
        // settings), but we validate the key on every Claude Code
        // settings path - mis-typed values are equally wrong anywhere.
        let content = r#"{"channelsEnabled": "true"}"#;
        let diagnostics = validate_at(".claude/managed-settings.json", content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-002")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn test_channels_enabled_line_points_at_key() {
        let content = "{\n  \"model\": \"claude-sonnet-4\",\n  \"channelsEnabled\": \"true\"\n}";
        let diagnostics = validate(content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-SET-002")
            .expect("CC-SET-002 diagnostic");
        assert_eq!(hit.line, 3);
    }

    #[test]
    fn test_channels_enabled_can_be_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CC-SET-002".to_string()];
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.json");
        let diagnostics = validator.validate(&path, r#"{"channelsEnabled": "true"}"#, &config);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_channels_enabled_and_pr_url_template_coexist() {
        // Both rules fire independently when both fields are wrong; each
        // diagnostic must carry its own rule id so suppressing one does
        // not silence the other.
        let content = r#"{"channelsEnabled": "true", "prUrlTemplate": 123}"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-001"));
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-002"));
    }

    // ===== CC-SET-003: worktree.baseRef =====

    #[test]
    fn test_worktree_base_ref_fresh_is_fine() {
        let content = r#"{"worktree": {"baseRef": "fresh"}}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-003"));
    }

    #[test]
    fn test_worktree_base_ref_head_is_fine() {
        let content = r#"{"worktree": {"baseRef": "head"}}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-003"));
    }

    #[test]
    fn test_worktree_base_ref_absent_is_fine() {
        let content = r#"{"worktree": {}}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-003"));
    }

    #[test]
    fn test_worktree_base_ref_null_is_fine() {
        let content = r#"{"worktree": {"baseRef": null}}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-003"));
    }

    #[test]
    fn test_worktree_base_ref_invalid_enum_flags() {
        let content = r#"{"worktree": {"baseRef": "main"}}"#;
        let diagnostics = validate(content);
        let cc003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-003")
            .collect();
        assert_eq!(cc003.len(), 1);
        assert!(cc003[0].message.contains("fresh"));
        assert!(cc003[0].message.contains("head"));
    }

    #[test]
    fn test_worktree_base_ref_is_case_sensitive() {
        let content = r#"{"worktree": {"baseRef": "FRESH"}}"#;
        let diagnostics = validate(content);
        assert_eq!(
            diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SET-003")
                .count(),
            1,
            "enum check must be case-sensitive - Claude Code parses the value exactly"
        );
    }

    #[test]
    fn test_worktree_base_ref_non_string_flags() {
        let content = r#"{"worktree": {"baseRef": true}}"#;
        let diagnostics = validate(content);
        let cc003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-003")
            .collect();
        assert_eq!(cc003.len(), 1);
        assert!(cc003[0].message.contains("string"));
    }

    #[test]
    fn test_worktree_base_ref_line_points_at_key() {
        let content = "{\n  \"worktree\": {\n    \"baseRef\": \"main\"\n  }\n}";
        let diagnostics = validate(content);
        let d = diagnostics.iter().find(|d| d.rule == "CC-SET-003").unwrap();
        assert_eq!(d.line, 3, "should point at the baseRef line, not worktree");
    }

    #[test]
    fn test_worktree_base_ref_can_be_disabled() {
        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-003");
        let config = builder.build().unwrap();
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.json");
        let content = r#"{"worktree": {"baseRef": "main"}}"#;
        let diagnostics = validator.validate(&path, content, &config);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-003"));
    }

    // ===== CC-SET-004: sandbox.bwrapPath / sandbox.socatPath =====

    #[test]
    fn test_sandbox_paths_absent_is_fine() {
        let content = r#"{}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-004"));
    }

    #[test]
    fn test_sandbox_paths_valid_strings_are_fine() {
        let content =
            r#"{"sandbox": {"bwrapPath": "/usr/local/bin/bwrap", "socatPath": "/usr/bin/socat"}}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-004"));
    }

    #[test]
    fn test_sandbox_paths_null_is_fine() {
        let content = r#"{"sandbox": {"bwrapPath": null, "socatPath": null}}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-004"));
    }

    #[test]
    fn test_sandbox_bwrap_path_empty_string_flags() {
        let content = r#"{"sandbox": {"bwrapPath": ""}}"#;
        let diagnostics = validate(content);
        let cc004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-004")
            .collect();
        assert_eq!(cc004.len(), 1);
        assert!(cc004[0].message.contains("bwrapPath"));
        assert!(cc004[0].message.contains("empty"));
    }

    #[test]
    fn test_sandbox_socat_path_non_string_flags() {
        let content = r#"{"sandbox": {"socatPath": 42}}"#;
        let diagnostics = validate(content);
        let cc004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-004")
            .collect();
        assert_eq!(cc004.len(), 1);
        assert!(cc004[0].message.contains("socatPath"));
        assert!(cc004[0].message.contains("string"));
    }

    #[test]
    fn test_sandbox_paths_both_flag_independently() {
        // Two bad fields should produce two diagnostics, not one.
        let content = r#"{"sandbox": {"bwrapPath": "", "socatPath": 0}}"#;
        let diagnostics = validate(content);
        assert_eq!(
            diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SET-004")
                .count(),
            2
        );
    }

    #[test]
    fn test_sandbox_non_object_does_not_crash() {
        // Array-typed sandbox is a distinct class of bug; CC-SET-004
        // intentionally does not false-positive on it (might be covered
        // by a future CC-SET rule).
        let content = r#"{"sandbox": []}"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-004"));
    }

    #[test]
    fn test_sandbox_paths_runs_on_managed_settings() {
        // This is the primary deployment target per the release note
        // ("managed settings (Linux/WSL)").
        let content = r#"{"sandbox": {"bwrapPath": 0}}"#;
        let diagnostics = validate_at(".claude/managed-settings.json", content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-004"));
    }

    #[test]
    fn test_sandbox_paths_can_be_disabled() {
        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-004");
        let config = builder.build().unwrap();
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.json");
        let content = r#"{"sandbox": {"bwrapPath": ""}}"#;
        let diagnostics = validator.validate(&path, content, &config);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-004"));
    }

    // ===== CC-SET-005: parentSettingsBehavior =====

    #[test]
    fn test_parent_settings_behavior_first_wins_is_fine() {
        let content = r#"{"parentSettingsBehavior": "first-wins"}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-005"));
    }

    #[test]
    fn test_parent_settings_behavior_merge_is_fine() {
        let content = r#"{"parentSettingsBehavior": "merge"}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-005"));
    }

    #[test]
    fn test_parent_settings_behavior_absent_is_fine() {
        let content = r#"{}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-005"));
    }

    #[test]
    fn test_parent_settings_behavior_null_is_fine() {
        let content = r#"{"parentSettingsBehavior": null}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-005"));
    }

    #[test]
    fn test_parent_settings_behavior_invalid_enum_flags() {
        let content = r#"{"parentSettingsBehavior": "override"}"#;
        let diagnostics = validate(content);
        let cc005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-005")
            .collect();
        assert_eq!(cc005.len(), 1);
        assert!(cc005[0].message.contains("first-wins"));
        assert!(cc005[0].message.contains("merge"));
    }

    #[test]
    fn test_parent_settings_behavior_is_case_sensitive() {
        let content = r#"{"parentSettingsBehavior": "Merge"}"#;
        let diagnostics = validate(content);
        assert_eq!(
            diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SET-005")
                .count(),
            1,
            "enum check must be case-sensitive"
        );
    }

    #[test]
    fn test_parent_settings_behavior_non_string_flags() {
        let content = r#"{"parentSettingsBehavior": true}"#;
        let diagnostics = validate(content);
        let cc005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-005")
            .collect();
        assert_eq!(cc005.len(), 1);
        assert!(cc005[0].message.contains("string"));
    }

    #[test]
    fn test_parent_settings_behavior_runs_on_managed_settings() {
        // Release note scopes this to admin tier but we validate across
        // all three files; a mis-typed value is still wrong wherever it
        // lands.
        let content = r#"{"parentSettingsBehavior": "override"}"#;
        let diagnostics = validate_at(".claude/managed-settings.json", content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-005"));
    }

    #[test]
    fn test_parent_settings_behavior_can_be_disabled() {
        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-005");
        let config = builder.build().unwrap();
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.json");
        let content = r#"{"parentSettingsBehavior": "override"}"#;
        let diagnostics = validator.validate(&path, content, &config);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-005"));
    }

    #[test]
    fn test_v2_1_133_rules_coexist() {
        // All three new v2.1.133 rules fire independently when all
        // three fields are wrong; suppressing one must not silence
        // the others.
        let content = r#"{
            "worktree": {"baseRef": "main"},
            "sandbox": {"bwrapPath": ""},
            "parentSettingsBehavior": "override"
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-003"));
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-004"));
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-005"));
    }

    // ===== CC-SET-006: disableBundledSkills =====

    #[test]
    fn test_disable_bundled_skills_absent_is_fine() {
        let content = r#"{"model": "claude-sonnet-4"}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-006"));
    }

    #[test]
    fn test_disable_bundled_skills_true_is_fine() {
        let content = r#"{"disableBundledSkills": true}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-006"));
    }

    #[test]
    fn test_disable_bundled_skills_false_is_fine() {
        // Explicit opt-out matches the documented default.
        let content = r#"{"disableBundledSkills": false}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-006"));
    }

    #[test]
    fn test_disable_bundled_skills_null_is_not_flagged() {
        // null is conventionally "field-absent" across the CC-SET family.
        let content = r#"{"disableBundledSkills": null}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-006"));
    }

    #[test]
    fn test_disable_bundled_skills_quoted_string_flags() {
        // The footgun the rule exists for: "true" as a string is not a
        // documented opt-in.
        let content = r#"{"disableBundledSkills": "true"}"#;
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-006")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits[0].level,
            crate::diagnostics::DiagnosticLevel::Warning,
            "CC-SET-006 is a warning, not a hard error"
        );
        assert!(
            hits[0].message.to_lowercase().contains("boolean"),
            "message should mention the required type"
        );
        assert!(
            hits[0].message.to_lowercase().contains("string"),
            "message should name the actual type seen"
        );
    }

    #[test]
    fn test_disable_bundled_skills_number_flags() {
        let content = r#"{"disableBundledSkills": 1}"#;
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-006")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.to_lowercase().contains("number"));
    }

    #[test]
    fn test_disable_bundled_skills_array_flags() {
        let content = r#"{"disableBundledSkills": [true]}"#;
        let diagnostics = validate(content);
        assert_eq!(
            diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SET-006")
                .count(),
            1
        );
    }

    #[test]
    fn test_disable_bundled_skills_runs_on_managed_settings() {
        // Admins hiding bundled skills org-wide is the likely deployment
        // target; the key is validated across all three settings paths.
        let content = r#"{"disableBundledSkills": "true"}"#;
        let diagnostics = validate_at(".claude/managed-settings.json", content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-006"));
    }

    #[test]
    fn test_disable_bundled_skills_line_points_at_key() {
        let content =
            "{\n  \"model\": \"claude-sonnet-4\",\n  \"disableBundledSkills\": \"true\"\n}";
        let diagnostics = validate(content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-SET-006")
            .expect("CC-SET-006 diagnostic");
        assert_eq!(hit.line, 3);
    }

    #[test]
    fn test_disable_bundled_skills_can_be_disabled() {
        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-006");
        let config = builder.build().unwrap();
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.json");
        let content = r#"{"disableBundledSkills": "true"}"#;
        let diagnostics = validator.validate(&path, content, &config);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-006"));
    }

    #[test]
    fn test_disable_bundled_skills_coexists_with_other_cc_set_rules() {
        // CC-SET-006 fires independently alongside the older rules; each
        // diagnostic carries its own rule id so suppressing one does not
        // silence the others.
        let content = r#"{
            "channelsEnabled": "true",
            "disableBundledSkills": "true"
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-002"));
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-006"));
    }

    // ===== CC-SET-007: enforceAvailableModels =====

    #[test]
    fn test_enforce_available_models_absent_is_fine() {
        let content = r#"{"availableModels": ["claude-sonnet-4-5"]}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-007"));
    }

    #[test]
    fn test_enforce_available_models_boolean_values_are_fine() {
        assert!(
            validate(r#"{"enforceAvailableModels": true}"#)
                .iter()
                .all(|d| d.rule != "CC-SET-007")
        );
        assert!(
            validate(r#"{"enforceAvailableModels": false}"#)
                .iter()
                .all(|d| d.rule != "CC-SET-007")
        );
    }

    #[test]
    fn test_enforce_available_models_null_is_not_flagged() {
        let content = r#"{"enforceAvailableModels": null}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-007"));
    }

    #[test]
    fn test_enforce_available_models_quoted_string_flags() {
        let content = r#"{"enforceAvailableModels": "true"}"#;
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-007")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].level, crate::diagnostics::DiagnosticLevel::Warning);
        assert!(hits[0].message.to_lowercase().contains("boolean"));
        assert!(hits[0].message.to_lowercase().contains("string"));
    }

    #[test]
    fn test_enforce_available_models_number_flags() {
        let content = r#"{"enforceAvailableModels": 1}"#;
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-007")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.to_lowercase().contains("number"));
    }

    #[test]
    fn test_enforce_available_models_object_flags() {
        let content = r#"{"enforceAvailableModels": {"enabled": true}}"#;
        let diagnostics = validate(content);
        assert_eq!(
            diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SET-007")
                .count(),
            1
        );
    }

    #[test]
    fn test_enforce_available_models_runs_on_managed_settings() {
        let content = r#"{"enforceAvailableModels": "true"}"#;
        let diagnostics = validate_at(".claude/managed-settings.json", content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-007"));
    }

    #[test]
    fn test_enforce_available_models_line_points_at_key() {
        let content = "{\n  \"availableModels\": [\"claude-sonnet-4-5\"],\n  \"enforceAvailableModels\": \"true\"\n}";
        let diagnostics = validate(content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-SET-007")
            .expect("CC-SET-007 diagnostic");
        assert_eq!(hit.line, 3);
    }

    #[test]
    fn test_enforce_available_models_can_be_disabled() {
        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-007");
        let config = builder.build().unwrap();
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.json");
        let content = r#"{"enforceAvailableModels": "true"}"#;
        let diagnostics = validator.validate(&path, content, &config);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-007"));
    }

    // ===== CC-SET-008: sandbox.allowAppleEvents =====

    #[test]
    fn test_sandbox_allow_apple_events_absent_is_fine() {
        let content = r#"{"sandbox": {"bwrapPath": "/usr/bin/bwrap"}}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-008"));
    }

    #[test]
    fn test_sandbox_allow_apple_events_boolean_values_are_fine() {
        assert!(
            validate(r#"{"sandbox": {"allowAppleEvents": true}}"#)
                .iter()
                .all(|d| d.rule != "CC-SET-008")
        );
        assert!(
            validate(r#"{"sandbox": {"allowAppleEvents": false}}"#)
                .iter()
                .all(|d| d.rule != "CC-SET-008")
        );
    }

    #[test]
    fn test_sandbox_allow_apple_events_null_is_not_flagged() {
        let content = r#"{"sandbox": {"allowAppleEvents": null}}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-008"));
    }

    #[test]
    fn test_sandbox_allow_apple_events_quoted_string_flags() {
        let content = r#"{"sandbox": {"allowAppleEvents": "true"}}"#;
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-008")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].level, crate::diagnostics::DiagnosticLevel::Warning);
        assert!(hits[0].message.to_lowercase().contains("boolean"));
        assert!(hits[0].message.to_lowercase().contains("string"));
    }

    #[test]
    fn test_sandbox_allow_apple_events_number_flags() {
        let content = r#"{"sandbox": {"allowAppleEvents": 1}}"#;
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-008")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.to_lowercase().contains("number"));
    }

    #[test]
    fn test_sandbox_allow_apple_events_runs_on_managed_settings() {
        let content = r#"{"sandbox": {"allowAppleEvents": "true"}}"#;
        let diagnostics = validate_at(".claude/managed-settings.json", content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-008"));
    }

    #[test]
    fn test_sandbox_allow_apple_events_line_points_at_key() {
        let content = "{\n  \"sandbox\": {\n    \"allowAppleEvents\": \"true\"\n  }\n}";
        let diagnostics = validate(content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-SET-008")
            .expect("CC-SET-008 diagnostic");
        assert_eq!(hit.line, 3);
    }

    #[test]
    fn test_sandbox_allow_apple_events_can_be_disabled() {
        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-008");
        let config = builder.build().unwrap();
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.json");
        let content = r#"{"sandbox": {"allowAppleEvents": "true"}}"#;
        let diagnostics = validator.validate(&path, content, &config);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-008"));
    }

    #[test]
    fn test_sandbox_allow_apple_events_coexists_with_sandbox_paths() {
        let content = r#"{"sandbox": {"bwrapPath": "", "allowAppleEvents": "true"}}"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-004"));
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-008"));
    }

    // ===== CC-SET-009: attribution.sessionUrl =====

    #[test]
    fn test_attribution_session_url_absent_is_fine() {
        let content =
            r#"{"attribution": {"commit": "Co-authored-by: Claude <noreply@anthropic.com>"}}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-009"));
    }

    #[test]
    fn test_attribution_session_url_boolean_values_are_fine() {
        assert!(
            validate(r#"{"attribution": {"sessionUrl": true}}"#)
                .iter()
                .all(|d| d.rule != "CC-SET-009")
        );
        assert!(
            validate(r#"{"attribution": {"sessionUrl": false}}"#)
                .iter()
                .all(|d| d.rule != "CC-SET-009")
        );
    }

    #[test]
    fn test_attribution_session_url_null_is_not_flagged() {
        let content = r#"{"attribution": {"sessionUrl": null}}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-009"));
    }

    #[test]
    fn test_attribution_session_url_quoted_string_flags() {
        let content = r#"{"attribution": {"sessionUrl": "false"}}"#;
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-009")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].level, crate::diagnostics::DiagnosticLevel::Warning);
        assert!(hits[0].message.to_lowercase().contains("boolean"));
        assert!(hits[0].message.to_lowercase().contains("string"));
    }

    #[test]
    fn test_attribution_session_url_number_flags() {
        let content = r#"{"attribution": {"sessionUrl": 0}}"#;
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-009")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.to_lowercase().contains("number"));
    }

    #[test]
    fn test_attribution_session_url_runs_on_managed_settings() {
        let content = r#"{"attribution": {"sessionUrl": "false"}}"#;
        let diagnostics = validate_at(".claude/managed-settings.json", content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-009"));
    }

    #[test]
    fn test_attribution_session_url_line_points_at_key() {
        let content = "{\n  \"attribution\": {\n    \"sessionUrl\": \"false\"\n  }\n}";
        let diagnostics = validate(content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-SET-009")
            .expect("CC-SET-009 diagnostic");
        assert_eq!(hit.line, 3);
    }

    #[test]
    fn test_attribution_session_url_can_be_disabled() {
        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-009");
        let config = builder.build().unwrap();
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.json");
        let content = r#"{"attribution": {"sessionUrl": "false"}}"#;
        let diagnostics = validator.validate(&path, content, &config);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-009"));
    }

    #[test]
    fn test_attribution_session_url_coexists_with_other_cc_set_rules() {
        let content = r#"{
            "sandbox": {"allowAppleEvents": "true"},
            "attribution": {"sessionUrl": "false"}
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-008"));
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-009"));
    }

    // ===== CC-SET-010: teammateMode =====

    #[test]
    fn test_teammate_mode_documented_values_are_fine() {
        for mode in ["in-process", "auto", "tmux", "iterm2"] {
            let content = format!(r#"{{"teammateMode": "{mode}"}}"#);
            assert!(
                validate(&content).iter().all(|d| d.rule != "CC-SET-010"),
                "{mode} must be accepted"
            );
        }
    }

    #[test]
    fn test_teammate_mode_invalid_string_flags() {
        let diagnostics = validate(r#"{"teammateMode": "screen"}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-010")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("iterm2"));
    }

    #[test]
    fn test_teammate_mode_non_string_flags() {
        let diagnostics = validate(r#"{"teammateMode": true}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-010")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.to_lowercase().contains("string"));
    }

    #[test]
    fn test_teammate_mode_can_be_disabled() {
        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-010");
        let config = builder.build().unwrap();
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.json");
        let diagnostics = validator.validate(&path, r#"{"teammateMode": "screen"}"#, &config);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-010"));
    }

    // ===== CC-SET-011: respondToBashCommands =====

    #[test]
    fn test_respond_to_bash_commands_boolean_values_are_fine() {
        assert!(
            validate(r#"{"respondToBashCommands": true}"#)
                .iter()
                .all(|d| d.rule != "CC-SET-011")
        );
        assert!(
            validate(r#"{"respondToBashCommands": false}"#)
                .iter()
                .all(|d| d.rule != "CC-SET-011")
        );
    }

    #[test]
    fn test_respond_to_bash_commands_quoted_string_flags() {
        let diagnostics = validate(r#"{"respondToBashCommands": "false"}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-011")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.to_lowercase().contains("boolean"));
        assert!(hits[0].message.to_lowercase().contains("string"));
    }

    #[test]
    fn test_respond_to_bash_commands_line_points_at_key() {
        let content =
            "{\n  \"model\": \"claude-sonnet-4\",\n  \"respondToBashCommands\": \"false\"\n}";
        let diagnostics = validate(content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-SET-011")
            .expect("CC-SET-011 diagnostic");
        assert_eq!(hit.line, 3);
    }

    #[test]
    fn test_respond_to_bash_commands_can_be_disabled() {
        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-011");
        let config = builder.build().unwrap();
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.json");
        let diagnostics =
            validator.validate(&path, r#"{"respondToBashCommands": "false"}"#, &config);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-011"));
    }

    // ===== CC-SET-012: sandbox.credentials =====

    #[test]
    fn test_sandbox_credentials_documented_example_is_fine() {
        let content = r#"{
          "sandbox": {
            "enabled": true,
            "credentials": {
              "files": [
                { "path": "~/.aws/credentials", "mode": "deny" },
                {
                  "path": "~/.netrc",
                  "mode": "mask",
                  "extract": "password:\\s*(\\S+)",
                  "onExtractNoMatch": "deny",
                  "maskDuplicates": true,
                  "injectHosts": ["api.example.com"]
                }
              ],
              "envVars": [
                { "name": "GITHUB_TOKEN", "mode": "deny" },
                {
                  "name": "NPM_TOKEN",
                  "mode": "mask",
                  "injectHosts": ["registry.npmjs.org"]
                }
              ],
              "allowPlaintextInject": false
            }
          }
        }"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-012"));
    }

    #[test]
    fn test_sandbox_credentials_absent_is_fine() {
        let content = r#"{"sandbox": {"enabled": true}}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-012"));
    }

    #[test]
    fn test_sandbox_credentials_empty_object_is_fine() {
        let content = r#"{"sandbox": {"credentials": {}}}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-012"));
    }

    #[test]
    fn test_sandbox_credentials_non_object_flags() {
        let diagnostics = validate(r#"{"sandbox": {"credentials": []}}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-012")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("object"));
    }

    #[test]
    fn test_sandbox_credentials_files_must_be_array() {
        let diagnostics = validate(r#"{"sandbox": {"credentials": {"files": true}}}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-012")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("files"));
        assert!(hits[0].message.contains("array"));
    }

    #[test]
    fn test_sandbox_credentials_env_vars_must_be_array() {
        let diagnostics = validate(r#"{"sandbox": {"credentials": {"envVars": "GITHUB_TOKEN"}}}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-012")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("envVars"));
        assert!(hits[0].message.contains("array"));
    }

    #[test]
    fn test_sandbox_credentials_entries_must_be_objects() {
        let diagnostics = validate(r#"{"sandbox": {"credentials": {"files": ["~/.ssh"]}}}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-012")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("files[0]"));
        assert!(hits[0].message.contains("object"));
    }

    #[test]
    fn test_sandbox_credentials_file_entry_requires_path() {
        let diagnostics =
            validate(r#"{"sandbox": {"credentials": {"files": [{"mode": "deny"}]}}}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-012")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains(".path"));
        assert!(hits[0].message.contains("missing"));
    }

    #[test]
    fn test_sandbox_credentials_env_var_entry_requires_name() {
        let diagnostics =
            validate(r#"{"sandbox": {"credentials": {"envVars": [{"mode": "deny"}]}}}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-012")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains(".name"));
        assert!(hits[0].message.contains("missing"));
    }

    #[test]
    fn test_sandbox_credentials_empty_path_flags() {
        let diagnostics =
            validate(r#"{"sandbox": {"credentials": {"files": [{"path": "", "mode": "deny"}]}}}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-012")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("must not be empty"));
    }

    #[test]
    fn test_sandbox_credentials_mode_must_be_supported() {
        let diagnostics = validate(
            r#"{"sandbox": {"credentials": {"files": [{"path": "~/.aws/credentials", "mode": "allow"}]}}}"#,
        );
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-012")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("mode"));
        assert!(hits[0].message.contains("deny"));
        assert!(hits[0].message.contains("mask"));
    }

    #[test]
    fn test_sandbox_credentials_mask_mode_is_valid_for_files_and_env_vars() {
        let content = r#"{
          "sandbox": {
            "credentials": {
              "files": [{"path": "~/.netrc", "mode": "mask"}],
              "envVars": [{"name": "GITHUB_TOKEN", "mode": "mask"}]
            }
          }
        }"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-012"));
    }

    #[test]
    fn test_sandbox_credentials_mask_directory_flags() {
        let diagnostics = validate(
            r#"{"sandbox": {"credentials": {"files": [{"path": "~/.aws/", "mode": "mask"}]}}}"#,
        );
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-012")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("single file"));
    }

    #[test]
    fn test_sandbox_credentials_extract_requires_capture_for_mask() {
        let diagnostics = validate(
            r#"{"sandbox": {"credentials": {"files": [{"path": "~/.netrc", "mode": "mask", "extract": "password=\\S+"}]}}}"#,
        );
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-012")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("capturing group"));

        let valid = validate(
            r#"{"sandbox": {"credentials": {"files": [{"path": "~/.netrc", "mode": "mask", "extract": "password=(?<secret>\\S+)"}]}}}"#,
        );
        assert!(valid.iter().all(|d| d.rule != "CC-SET-012"));
    }

    #[test]
    fn test_sandbox_credentials_extract_rejects_invalid_ecmascript_regex() {
        let diagnostics = validate(
            r#"{"sandbox": {"credentials": {"files": [{"path": "~/.netrc", "mode": "mask", "extract": "password=([A-Z]+"}]}}}"#,
        );
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-012")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("valid ECMAScript regex"));
    }

    #[test]
    fn test_sandbox_credentials_extract_accepts_ecmascript_lookaround_and_backreference() {
        let diagnostics = validate(
            r#"{"sandbox": {"credentials": {"files": [{"path": "~/.netrc", "mode": "mask", "extract": "(?<=token=)([A-Z]+)\\1"}]}}}"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-012"));
    }

    #[test]
    fn test_sandbox_credentials_v2_1_224_options_are_valid() {
        let content = r#"{
          "sandbox": {
            "network": {
              "tlsTerminate": {}
            },
            "credentials": {
              "files": [{
                "path": "~/.config/service/token.json",
                "mode": "mask",
                "decode": "jwt",
                "maskClaims": ["api_key"],
                "onExtractNoMatch": "error",
                "maskDuplicates": true
              }],
              "envVars": [
                {
                  "name": "DATABASE_URL",
                  "mode": "mask",
                  "extract": "://[^:]+:([^@]+)@",
                  "onExtractNoMatch": "deny"
                },
                {
                  "name": "JWT_TOKEN",
                  "mode": "mask",
                  "decode": "jwt",
                  "maskClaims": ["api_key"],
                  "onExtractNoMatch": "warn"
                }
              ],
              "awsPairs": [{
                "accessKeyIdVar": "MY_KEY_ID",
                "secretAccessKeyVar": "MY_SECRET_KEY",
                "sessionTokenVar": "MY_SESSION_TOKEN"
              }],
              "sigv4": {
                "streaming": "passthrough",
                "presigned": "deny",
                "sigv4a": "deny"
              }
            }
          }
        }"#;
        assert!(
            validate_at(".claude/managed-settings.json", content)
                .iter()
                .all(|d| d.rule != "CC-SET-012")
        );
    }

    #[test]
    fn test_sandbox_credentials_v2_1_224_invalid_options_flag() {
        let content = r#"{
          "sandbox": {
            "network": {
              "tlsTerminate": {}
            },
            "credentials": {
              "files": [{
                "path": "~/.config/service/token.json",
                "mode": "mask",
                "decode": "base64",
                "maskClaims": []
              }],
              "envVars": [{
                "name": "JWT_TOKEN",
                "mode": "mask",
                "extract": "(token)",
                "decode": "jwt",
                "onExtractNoMatch": "deny"
              }],
              "awsPairs": [{
                "accessKeyIdVar": "",
                "secretAccessKeyVar": 42
              }],
              "sigv4": {
                "streaming": "allow",
                "unknown": "deny"
              }
            }
          }
        }"#;
        let hits: Vec<_> = validate_at(".claude/managed-settings.json", content)
            .into_iter()
            .filter(|diagnostic| diagnostic.rule == "CC-SET-012")
            .collect();
        assert!(hits.iter().any(|hit| hit.message.contains(".decode")));
        assert!(hits.iter().any(|hit| hit.message.contains("maskClaims")));
        assert!(
            hits.iter()
                .any(|hit| hit.message.contains("cannot combine extract and decode"))
        );
        assert!(
            hits.iter()
                .any(|hit| hit.message.contains("onExtractNoMatch"))
        );
        assert!(hits.iter().any(|hit| hit.message.contains("awsPairs")));
        assert!(hits.iter().any(|hit| hit.message.contains("sigv4")));
    }

    #[test]
    fn test_sandbox_credentials_aws_pairs_and_sigv4_containers_are_checked() {
        let diagnostics = validate_at(
            ".claude/managed-settings.json",
            r#"{"sandbox":{"network":{"tlsTerminate":{}},"credentials":{"awsPairs":true,"sigv4":[]}}}"#,
        );
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.rule == "CC-SET-012")
            .collect();
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().any(|hit| hit.message.contains("awsPairs")));
        assert!(hits.iter().any(|hit| hit.message.contains("sigv4")));
    }

    #[test]
    fn test_sandbox_credentials_aws_pair_variables_must_be_unique_within_pair() {
        for content in [
            r#"{
              "sandbox": {
                "network": {"tlsTerminate": {}},
                "credentials": {
                  "awsPairs": [{
                    "accessKeyIdVar": "AWS_KEY",
                    "secretAccessKeyVar": "AWS_KEY"
                  }]
                }
              }
            }"#,
            r#"{
              "sandbox": {
                "network": {"tlsTerminate": {}},
                "credentials": {
                  "awsPairs": [{
                    "accessKeyIdVar": "AWS_KEY",
                    "secretAccessKeyVar": "AWS_SECRET",
                    "sessionTokenVar": "AWS_SECRET"
                  }]
                }
              }
            }"#,
        ] {
            let hits: Vec<_> = validate_at(".claude/managed-settings.json", content)
                .into_iter()
                .filter(|diagnostic| diagnostic.rule == "CC-SET-012")
                .collect();
            assert_eq!(hits.len(), 1);
            assert!(hits[0].message.contains("reuses"));
        }
    }

    #[test]
    fn test_sandbox_credentials_aws_pair_variables_must_be_unique_across_pairs() {
        let content = r#"{
          "sandbox": {
            "network": {"tlsTerminate": {}},
            "credentials": {
              "awsPairs": [
                {
                  "accessKeyIdVar": "AWS_KEY",
                  "secretAccessKeyVar": "AWS_SECRET_1"
                },
                {
                  "accessKeyIdVar": "AWS_KEY",
                  "secretAccessKeyVar": "AWS_SECRET_2"
                }
              ]
            }
          }
        }"#;
        let hits: Vec<_> = validate_at(".claude/managed-settings.json", content)
            .into_iter()
            .filter(|diagnostic| diagnostic.rule == "CC-SET-012")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("awsPairs[1].accessKeyIdVar"));
        assert!(hits[0].message.contains("awsPairs[0].accessKeyIdVar"));
    }

    #[test]
    fn test_sandbox_credentials_aws_resigning_requires_tls_termination() {
        for content in [
            r#"{"sandbox":{"credentials":{"awsPairs":[],"sigv4":{}}}}"#,
            r#"{"sandbox":{"network":{"tlsTerminate":true},"credentials":{"awsPairs":[]}}}"#,
        ] {
            let hits: Vec<_> = validate_at(".claude/managed-settings.json", content)
                .into_iter()
                .filter(|diagnostic| diagnostic.rule == "CC-SET-012")
                .collect();
            assert_eq!(hits.len(), 1);
            assert!(hits[0].message.contains("tlsTerminate"));
        }
    }

    #[test]
    fn test_sandbox_credentials_aws_tls_missing_is_suppressed_for_system_sources() {
        let content = r#"{"sandbox":{"credentials":{"awsPairs":[]}}}"#;

        for path in [
            "/Library/Application Support/ClaudeCode/managed-settings.json",
            "/etc/claude-code/managed-settings.json",
            "C:/Program Files/ClaudeCode/managed-settings.json",
            "/Library/Application Support/ClaudeCode/managed-settings.d/10-credentials.json",
            "/etc/claude-code/managed-settings.d/10-credentials.json",
            "C:/Program Files/ClaudeCode/managed-settings.d/10-credentials.json",
        ] {
            assert!(
                validate_at(path, content).iter().all(|diagnostic| {
                    diagnostic.rule != "CC-SET-012" || !diagnostic.message.contains("tlsTerminate")
                }),
                "unexpected per-file tlsTerminate diagnostic for merged source {path}"
            );
        }
    }

    #[test]
    fn test_sandbox_credentials_aws_tls_invalid_value_still_warns_in_fragment() {
        let content =
            r#"{"sandbox":{"network":{"tlsTerminate":true},"credentials":{"awsPairs":[]}}}"#;
        let hits: Vec<_> = validate_at(
            "/etc/claude-code/managed-settings.d/10-credentials.json",
            content,
        )
        .into_iter()
        .filter(|diagnostic| diagnostic.rule == "CC-SET-012")
        .collect();

        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("tlsTerminate"));
    }

    #[test]
    fn test_sandbox_credentials_aws_resigning_flags_project_scopes() {
        let content = r#"{
          "sandbox": {
            "network": {"tlsTerminate": {}},
            "credentials": {"awsPairs": [], "sigv4": {}}
          }
        }"#;
        for path in [".claude/settings.json", ".claude/settings.local.json"] {
            let hits: Vec<_> = validate_at(path, content)
                .into_iter()
                .filter(|diagnostic| diagnostic.rule == "CC-SET-012")
                .collect();
            assert_eq!(hits.len(), 1, "expected one scope diagnostic for {path}");
            assert!(hits[0].message.contains("project-level"));
        }
    }

    #[test]
    fn test_sandbox_credentials_aws_resigning_allows_honored_scopes() {
        let content = r#"{
          "sandbox": {
            "network": {"tlsTerminate": {}},
            "credentials": {"awsPairs": [], "sigv4": {}}
          }
        }"#;
        assert!(
            validate_at(".claude/managed-settings.json", content)
                .iter()
                .all(|diagnostic| diagnostic.rule != "CC-SET-012")
        );

        let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))
        else {
            return;
        };
        let user_settings = PathBuf::from(home).join(".claude").join("settings.json");
        let diagnostics =
            ClaudeSettingsValidator.validate(&user_settings, content, &LintConfig::default());
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.rule != "CC-SET-012")
        );
    }

    #[test]
    fn test_sandbox_credentials_combined_aws_feature_label_is_localized() {
        for (locale, conjunction) in [("es", "awsPairs y sigv4"), ("zh-CN", "awsPairs 和 sigv4")] {
            let feature = rust_i18n::t!("rules.cc_set_012.aws_features_both", locale = locale);
            let tls_message = rust_i18n::t!(
                "rules.cc_set_012.tls_terminate_required",
                locale = locale,
                feature = feature.clone()
            );
            let scope_message = rust_i18n::t!(
                "rules.cc_set_012.aws_scope",
                locale = locale,
                feature = feature
            );

            assert!(tls_message.contains(conjunction));
            assert!(scope_message.contains(conjunction));
            assert!(!tls_message.contains("awsPairs and sigv4"));
            assert!(!scope_message.contains("awsPairs and sigv4"));
        }
    }

    #[test]
    fn test_sandbox_credentials_spanish_translations_preserve_diacritics() {
        let cases = [
            (
                rust_i18n::t!(
                    "rules.cc_set_012.extract_no_match_jwt_suggestion",
                    locale = "es",
                    array_key = "envVars",
                    index = 0
                ),
                "elimínalo",
            ),
            (
                rust_i18n::t!(
                    "rules.cc_set_012.mask_claims_shape",
                    locale = "es",
                    array_key = "envVars",
                    index = 0,
                    actual = "[]"
                ),
                "vacío",
            ),
            (
                rust_i18n::t!(
                    "rules.cc_set_012.aws_pair_field",
                    locale = "es",
                    index = 0,
                    field = "accessKeyIdVar"
                ),
                "vacía",
            ),
            (
                rust_i18n::t!("rules.cc_set_012.sigv4_suggestion", locale = "es"),
                "políticas",
            ),
            (
                rust_i18n::t!(
                    "rules.cc_set_012.aws_scope",
                    locale = "es",
                    feature = "awsPairs"
                ),
                "configuración",
            ),
        ];

        for (message, expected) in cases {
            assert!(
                message.contains(expected),
                "expected Spanish translation to contain {expected:?}, got {message:?}"
            );
        }
    }

    #[test]
    fn test_sandbox_credentials_optional_field_types_are_checked() {
        let diagnostics = validate(
            r#"{
              "sandbox": {
                "credentials": {
                  "files": [{
                    "path": "~/.netrc",
                    "mode": "mask",
                    "extract": 42,
                    "onExtractNoMatch": "ignore",
                    "maskDuplicates": "true",
                    "injectHosts": ["api.example.com", 42]
                  }],
                  "envVars": [{
                    "name": "BAD-NAME",
                    "mode": "mask",
                    "injectHosts": "api.example.com"
                  }],
                  "allowPlaintextInject": "false"
                }
              }
            }"#,
        );
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-012")
            .collect();
        assert_eq!(hits.len(), 7);
        assert!(
            hits.iter()
                .any(|d| d.message.contains("allowPlaintextInject"))
        );
        assert!(hits.iter().any(|d| d.message.contains("BAD-NAME")));
        assert!(hits.iter().any(|d| d.message.contains("extract")));
        assert!(hits.iter().any(|d| d.message.contains("onExtractNoMatch")));
        assert!(hits.iter().any(|d| d.message.contains("maskDuplicates")));
        assert_eq!(
            hits.iter()
                .filter(|d| d.message.contains("injectHosts"))
                .count(),
            2
        );
    }

    #[test]
    fn test_sandbox_credentials_deny_mode_ignores_mask_only_fields() {
        let content = r#"{
          "sandbox": {
            "credentials": {
              "files": [{
                "path": "~/.netrc",
                "mode": "deny",
                "extract": 42,
                "decode": 42,
                "maskClaims": 42,
                "onExtractNoMatch": 42,
                "maskDuplicates": "true",
                "injectHosts": 42
              }],
              "envVars": [{
                "name": "GITHUB_TOKEN",
                "mode": "deny",
                "extract": 42,
                "decode": 42,
                "maskClaims": 42,
                "onExtractNoMatch": 42,
                "injectHosts": 42
              }]
            }
          }
        }"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-012"));
    }

    #[test]
    fn test_sandbox_credentials_mode_is_required() {
        let diagnostics =
            validate(r#"{"sandbox": {"credentials": {"envVars": [{"name": "GITHUB_TOKEN"}]}}}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-012")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("mode"));
        assert!(hits[0].message.contains("missing"));
    }

    #[test]
    fn test_sandbox_credentials_multiple_bad_entries_flag_independently() {
        let diagnostics = validate(
            r#"{
              "sandbox": {
                "credentials": {
                  "files": [
                    {"path": "~/.ssh", "mode": "allow"},
                    {"path": 42, "mode": "deny"}
                  ],
                  "envVars": [
                    {"name": "", "mode": "deny"}
                  ]
                }
              }
            }"#,
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SET-012")
                .count(),
            3
        );
    }

    #[test]
    fn test_sandbox_credentials_runs_on_managed_settings() {
        let content = r#"{"sandbox": {"credentials": {"files": [{"path": "~/.ssh"}]}}}"#;
        let diagnostics = validate_at(".claude/managed-settings.json", content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-012"));
    }

    #[test]
    fn test_sandbox_credentials_line_points_at_array_key() {
        let content =
            "{\n  \"sandbox\": {\n    \"credentials\": {\n      \"files\": true\n    }\n  }\n}";
        let diagnostics = validate(content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-SET-012")
            .expect("CC-SET-012 diagnostic");
        assert_eq!(hit.line, 4);
    }

    #[test]
    fn test_sandbox_credentials_can_be_disabled() {
        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-012");
        let config = builder.build().unwrap();
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.json");
        let diagnostics = validator.validate(
            &path,
            r#"{"sandbox": {"credentials": {"files": [{"path": "~/.ssh"}]}}}"#,
            &config,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-012"));
    }

    #[test]
    fn test_sandbox_credentials_coexists_with_other_sandbox_rules() {
        let content = r#"{
          "sandbox": {
            "bwrapPath": "",
            "allowAppleEvents": "true",
            "credentials": {
              "files": [{"path": "~/.ssh", "mode": "allow"}]
            }
          }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-004"));
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-008"));
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-012"));
    }

    // ===== CC-SET-013: autoMode.classifyAllShell =====

    #[test]
    fn test_auto_mode_classify_all_shell_boolean_values_are_fine() {
        assert!(
            validate(r#"{"autoMode": {"classifyAllShell": true}}"#)
                .iter()
                .all(|d| d.rule != "CC-SET-013")
        );
        assert!(
            validate(r#"{"autoMode": {"classifyAllShell": false}}"#)
                .iter()
                .all(|d| d.rule != "CC-SET-013")
        );
    }

    #[test]
    fn test_auto_mode_classify_all_shell_null_is_not_flagged() {
        let content = r#"{"autoMode": {"classifyAllShell": null}}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-013"));
    }

    #[test]
    fn test_auto_mode_classify_all_shell_quoted_string_flags() {
        let diagnostics = validate(r#"{"autoMode": {"classifyAllShell": "true"}}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-013")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].level, crate::diagnostics::DiagnosticLevel::Warning);
        assert!(hits[0].message.to_lowercase().contains("boolean"));
        assert!(hits[0].message.to_lowercase().contains("string"));
    }

    #[test]
    fn test_auto_mode_classify_all_shell_number_flags() {
        let diagnostics = validate(r#"{"autoMode": {"classifyAllShell": 1}}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-013")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.to_lowercase().contains("number"));
    }

    #[test]
    fn test_auto_mode_classify_all_shell_ignores_non_object_auto_mode() {
        let content = r#"{"autoMode": "enabled"}"#;
        assert!(validate(content).iter().all(|d| d.rule != "CC-SET-013"));
    }

    #[test]
    fn test_auto_mode_classify_all_shell_runs_on_managed_settings() {
        let content = r#"{"autoMode": {"classifyAllShell": "true"}}"#;
        let diagnostics = validate_at(".claude/managed-settings.json", content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-013"));
    }

    #[test]
    fn test_auto_mode_classify_all_shell_line_points_at_key() {
        let content = "{\n  \"autoMode\": {\n    \"classifyAllShell\": \"true\"\n  }\n}";
        let diagnostics = validate(content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-SET-013")
            .expect("CC-SET-013 diagnostic");
        assert_eq!(hit.line, 3);
    }

    #[test]
    fn test_auto_mode_classify_all_shell_can_be_disabled() {
        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-013");
        let config = builder.build().unwrap();
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.json");
        let diagnostics = validator.validate(
            &path,
            r#"{"autoMode": {"classifyAllShell": "true"}}"#,
            &config,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-013"));
    }

    // ===== CC-SET-014: autoMode in settings.local.json =====

    #[test]
    fn test_auto_mode_in_settings_local_flags() {
        let content = r#"{"autoMode": {"classifyAllShell": true}}"#;
        let diagnostics = validate_at(".claude/settings.local.json", content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-014")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].level, crate::diagnostics::DiagnosticLevel::Warning);
    }

    #[test]
    fn test_auto_mode_in_settings_json_does_not_flag() {
        // autoMode is still read from settings.json — must not fire CC-SET-014.
        let content = r#"{"autoMode": {"classifyAllShell": true}}"#;
        let diagnostics = validate_at(".claude/settings.json", content);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-014"));
    }

    #[test]
    fn test_auto_mode_in_managed_settings_does_not_flag() {
        // managed-settings.json is honored — must not fire CC-SET-014.
        let content = r#"{"autoMode": {"classifyAllShell": true}}"#;
        let diagnostics = validate_at(".claude/managed-settings.json", content);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-014"));
    }

    #[test]
    fn test_auto_mode_absent_in_settings_local_is_fine() {
        let content = r#"{"model": "claude-sonnet-4"}"#;
        let diagnostics = validate_at(".claude/settings.local.json", content);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-014"));
    }

    #[test]
    fn test_auto_mode_null_in_settings_local_is_fine() {
        let content = r#"{"autoMode": null}"#;
        let diagnostics = validate_at(".claude/settings.local.json", content);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-014"));
    }

    #[test]
    fn test_auto_mode_in_settings_local_fires_for_any_non_null_type() {
        // object, string, bool, number, array — all must produce exactly 1 CC-SET-014
        for content in &[
            r#"{"autoMode": {"classifyAllShell": true}}"#,
            r#"{"autoMode": "enabled"}"#,
            r#"{"autoMode": true}"#,
            r#"{"autoMode": 1}"#,
            r#"{"autoMode": []}"#,
        ] {
            let diagnostics = validate_at(".claude/settings.local.json", content);
            let count = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SET-014")
                .count();
            assert_eq!(count, 1, "expected 1 CC-SET-014 for content: {}", content);
        }
    }

    #[test]
    fn test_auto_mode_in_settings_local_line_points_at_key() {
        let content =
            "{\n  \"model\": \"claude-sonnet-4\",\n  \"autoMode\": {\"classifyAllShell\": true}\n}";
        let diagnostics = validate_at(".claude/settings.local.json", content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-SET-014")
            .expect("CC-SET-014 diagnostic");
        assert_eq!(hit.line, 3, "line must point at the autoMode key");
    }

    #[test]
    fn test_auto_mode_in_settings_local_can_be_disabled() {
        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-014");
        let config = builder.build().unwrap();
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.local.json");
        let diagnostics = validator.validate(
            &path,
            r#"{"autoMode": {"classifyAllShell": true}}"#,
            &config,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-014"));
    }

    #[test]
    fn test_auto_mode_in_settings_local_coexists_with_cc_set_013() {
        // classifyAllShell with a bad type in settings.local.json should fire
        // both CC-SET-013 (wrong type) and CC-SET-014 (wrong file).
        let content = r#"{"autoMode": {"classifyAllShell": "true"}}"#;
        let diagnostics = validate_at(".claude/settings.local.json", content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-013"));
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-014"));
    }

    // ===== CC-SET-015: pluginConfigs in project-level settings =====

    #[test]
    fn test_plugin_configs_in_settings_json_flags() {
        let content = r#"{"pluginConfigs": {"my-plugin": {"apiKey": "abc"}}}"#;
        let diagnostics = validate_at(".claude/settings.json", content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-015")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].level, crate::diagnostics::DiagnosticLevel::Warning);
    }

    #[test]
    fn test_plugin_configs_in_settings_local_flags() {
        let content = r#"{"pluginConfigs": {"my-plugin": {"apiKey": "abc"}}}"#;
        let diagnostics = validate_at(".claude/settings.local.json", content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-015")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn test_plugin_configs_in_managed_settings_does_not_flag() {
        // managed-settings is an honored tier — must not fire CC-SET-015.
        let content = r#"{"pluginConfigs": {"my-plugin": {"apiKey": "abc"}}}"#;
        let diagnostics = validate_at(".claude/managed-settings.json", content);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-015"));
    }

    #[test]
    fn test_plugin_configs_absent_is_fine() {
        let content = r#"{"model": "claude-sonnet-4"}"#;
        let diagnostics = validate_at(".claude/settings.json", content);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-015"));
    }

    #[test]
    fn test_plugin_configs_null_is_fine() {
        let content = r#"{"pluginConfigs": null}"#;
        let diagnostics = validate_at(".claude/settings.json", content);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-015"));
    }

    #[test]
    fn test_plugin_configs_empty_object_flags() {
        // An empty object is still a non-null value — the presence of the key
        // is the problem regardless of whether nested config is populated.
        let content = r#"{"pluginConfigs": {}}"#;
        let diagnostics = validate_at(".claude/settings.json", content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-015")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn test_plugin_configs_line_points_at_key() {
        let content = "{\n  \"model\": \"claude-sonnet-4\",\n  \"pluginConfigs\": {\"p\": {}}\n}";
        let diagnostics = validate_at(".claude/settings.json", content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-SET-015")
            .expect("CC-SET-015 diagnostic");
        assert_eq!(hit.line, 3, "line must point at the pluginConfigs key");
    }

    #[test]
    fn test_plugin_configs_can_be_disabled() {
        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-015");
        let config = builder.build().unwrap();
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.json");
        let diagnostics = validator.validate(
            &path,
            r#"{"pluginConfigs": {"my-plugin": {"apiKey": "abc"}}}"#,
            &config,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-015"));
    }

    #[test]
    fn test_plugin_configs_coexists_with_other_cc_set_rules() {
        // Both CC-SET-002 (channelsEnabled) and CC-SET-015 fire independently.
        let content = r#"{"channelsEnabled": "true", "pluginConfigs": {"p": {}}}"#;
        let diagnostics = validate_at(".claude/settings.json", content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-002"));
        assert!(diagnostics.iter().any(|d| d.rule == "CC-SET-015"));
    }

    // ===== CC-SET-016: deprecated Write/NotebookEdit/Glob permission-rule forms =====

    #[test]
    fn test_write_in_allow_flags_with_edit_suggestion() {
        let content = r#"{"permissions": {"allow": ["Write(src/**)"]}}"#;
        let diagnostics = validate_at(".claude/settings.json", content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-016")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].level, crate::diagnostics::DiagnosticLevel::Warning);
        let msg = &hits[0].message;
        assert!(
            msg.contains("Edit(src/**)"),
            "message should mention Edit replacement, got: {msg}"
        );
    }

    #[test]
    fn test_notebook_edit_in_deny_flags_with_edit_suggestion() {
        let content = r#"{"permissions": {"deny": ["NotebookEdit(notebooks/**)"]}}"#;
        let diagnostics = validate_at(".claude/settings.json", content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-016")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].level, crate::diagnostics::DiagnosticLevel::Warning);
        let msg = &hits[0].message;
        assert!(
            msg.contains("Edit(notebooks/**)"),
            "message should mention Edit replacement, got: {msg}"
        );
    }

    #[test]
    fn test_glob_in_ask_flags_with_read_suggestion() {
        let content = r#"{"permissions": {"ask": ["Glob(docs/**)"]}}"#;
        let diagnostics = validate_at(".claude/settings.json", content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-016")
            .collect();
        assert_eq!(hits.len(), 1);
        let msg = &hits[0].message;
        assert!(
            msg.contains("Read(docs/**)"),
            "message should mention Read replacement, got: {msg}"
        );
    }

    #[test]
    fn test_edit_and_read_entries_are_clean() {
        let content =
            r#"{"permissions": {"allow": ["Edit(src/**)", "Read(docs/**)", "Bash(npm:*)")]}"#;
        let diagnostics = validate_at(".claude/settings.json", content);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-016"));
    }

    #[test]
    fn test_bare_write_without_paren_is_clean() {
        // "Write" alone (no parenthesized path) must not fire CC-SET-016.
        let content = r#"{"permissions": {"allow": ["Write", "NotebookEdit", "Glob"]}}"#;
        let diagnostics = validate_at(".claude/settings.json", content);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-016"));
    }

    #[test]
    fn test_multiple_bad_entries_produce_one_diagnostic_each() {
        let content =
            r#"{"permissions": {"allow": ["Write(a/**)", "NotebookEdit(b/**)", "Glob(c/**)"]}}"#;
        let diagnostics = validate_at(".claude/settings.json", content);
        let count = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SET-016")
            .count();
        assert_eq!(count, 3, "expected one diagnostic per deprecated entry");
    }

    #[test]
    fn test_deprecated_forms_in_all_three_settings_filenames() {
        let content = r#"{"permissions": {"allow": ["Write(src/**)"]}}"#;
        for path_str in &[
            ".claude/settings.json",
            ".claude/settings.local.json",
            ".claude/managed-settings.json",
        ] {
            let diagnostics = validate_at(path_str, content);
            let count = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SET-016")
                .count();
            assert_eq!(
                count, 1,
                "expected CC-SET-016 in {path_str} but got {count}"
            );
        }
    }

    #[test]
    fn test_permissions_absent_is_clean() {
        let content = r#"{"model": "claude-sonnet-4"}"#;
        let diagnostics = validate_at(".claude/settings.json", content);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-016"));
    }

    #[test]
    fn test_allow_not_an_array_is_clean() {
        let content = r#"{"permissions": {"allow": "Write(src/**)"}}"#;
        let diagnostics = validate_at(".claude/settings.json", content);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-016"));
    }

    #[test]
    fn test_non_string_entry_in_allow_is_skipped() {
        let content = r#"{"permissions": {"allow": [42, true, null]}}"#;
        let diagnostics = validate_at(".claude/settings.json", content);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-016"));
    }

    #[test]
    fn test_rule_disabled_via_config() {
        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-016");
        let config = builder.build().unwrap();
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.json");
        let diagnostics = validator.validate(
            &path,
            r#"{"permissions": {"allow": ["Write(src/**)"]}}"#,
            &config,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-016"));
    }

    #[test]
    fn test_other_tool_forms_like_bash_are_clean() {
        let content = r#"{"permissions": {"allow": ["Bash(npm:*)", "Bash(git:*)", "Agent(*)"]}}"#;
        let diagnostics = validate_at(".claude/settings.json", content);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-016"));
    }

    // ===== CC-SET-017: sandbox.filesystem.disabled boolean =====

    #[test]
    fn test_sandbox_filesystem_disabled_non_boolean_flags() {
        for invalid in [r#""true""#, "1", "[]", "{}"] {
            let content = format!(r#"{{"sandbox": {{"filesystem": {{"disabled": {invalid}}}}}}}"#);
            let diagnostics = validate_at(".claude/settings.json", &content);
            let hits: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SET-017")
                .collect();
            assert_eq!(hits.len(), 1, "expected one hit for {invalid}");
            assert_eq!(hits[0].level, crate::diagnostics::DiagnosticLevel::Warning);
        }
    }

    #[test]
    fn test_sandbox_filesystem_disabled_boolean_or_null_is_clean() {
        for valid in ["true", "false", "null"] {
            let content = format!(r#"{{"sandbox": {{"filesystem": {{"disabled": {valid}}}}}}}"#);
            let diagnostics = validate_at(".claude/settings.json", &content);
            assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-017"));
        }
    }

    #[test]
    fn test_sandbox_filesystem_disabled_line_and_disable_config() {
        let content = "{\n  \"sandbox\": {\n    \"filesystem\": {\n      \"disabled\": \"false\"\n    }\n  }\n}";
        let diagnostics = validate_at(".claude/settings.json", content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-SET-017")
            .expect("CC-SET-017 diagnostic");
        assert_eq!(hit.line, 4);

        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-017");
        let config = builder.build().unwrap();
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.json");
        let diagnostics = validator.validate(&path, content, &config);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-017"));
    }

    // ===== CC-SET-018: emojiCompletionEnabled boolean =====

    #[test]
    fn test_emoji_completion_enabled_non_boolean_flags() {
        for invalid in [r#""false""#, "0", "[]", "{}"] {
            let content = format!(r#"{{"emojiCompletionEnabled": {invalid}}}"#);
            let diagnostics = validate_at(".claude/settings.json", &content);
            let hits: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SET-018")
                .collect();
            assert_eq!(hits.len(), 1, "expected one hit for {invalid}");
            assert_eq!(hits[0].level, crate::diagnostics::DiagnosticLevel::Warning);
        }
    }

    #[test]
    fn test_emoji_completion_enabled_boolean_or_null_is_clean() {
        for valid in ["true", "false", "null"] {
            let content = format!(r#"{{"emojiCompletionEnabled": {valid}}}"#);
            let diagnostics = validate_at(".claude/settings.json", &content);
            assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-018"));
        }
    }

    #[test]
    fn test_emoji_completion_enabled_line_and_disable_config() {
        let content =
            "{\n  \"model\": \"claude-sonnet-4\",\n  \"emojiCompletionEnabled\": \"true\"\n}";
        let diagnostics = validate_at(".claude/settings.json", content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-SET-018")
            .expect("CC-SET-018 diagnostic");
        assert_eq!(hit.line, 3);

        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-018");
        let config = builder.build().unwrap();
        let validator = ClaudeSettingsValidator;
        let path = PathBuf::from(".claude/settings.json");
        let diagnostics = validator.validate(&path, content, &config);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-SET-018"));
    }

    // ===== CC-SET-019: sandbox.network.strictAllowlist boolean =====

    #[test]
    fn test_sandbox_network_strict_allowlist_type() {
        for invalid in [r#""true""#, "1", "[]", "{}"] {
            let content =
                r#"{"sandbox":{"network":{"strictAllowlist":VALUE}}}"#.replace("VALUE", invalid);
            let diagnostics = validate(&content);
            assert_eq!(
                diagnostics
                    .iter()
                    .filter(|d| d.rule == "CC-SET-019")
                    .count(),
                1,
                "expected one hit for {invalid}"
            );
        }
        for valid in ["true", "false", "null"] {
            let content =
                r#"{"sandbox":{"network":{"strictAllowlist":VALUE}}}"#.replace("VALUE", valid);
            assert!(validate(&content).iter().all(|d| d.rule != "CC-SET-019"));
        }
    }

    // ===== CC-SET-020: workflowSizeGuideline enum =====

    #[test]
    fn test_workflow_size_guideline_enum() {
        for valid in ["unrestricted", "small", "medium", "large"] {
            let content = format!(r#"{{"workflowSizeGuideline": "{valid}"}}"#);
            assert!(validate(&content).iter().all(|d| d.rule != "CC-SET-020"));
        }

        for invalid in [r#""tiny""#, "1", "true", "[]"] {
            let content = format!(r#"{{"workflowSizeGuideline": {invalid}}}"#);
            assert_eq!(
                validate(&content)
                    .iter()
                    .filter(|d| d.rule == "CC-SET-020")
                    .count(),
                1,
                "expected one hit for {invalid}"
            );
        }
    }

    // ===== CC-SET-021: project-level Remote Control auto-start =====

    #[test]
    fn test_remote_control_at_startup_true_flags_project_settings() {
        let content = r#"{"remoteControlAtStartup": true}"#;
        for path in [".claude/settings.json", ".claude/settings.local.json"] {
            let diagnostics = validate_at(path, content);
            assert_eq!(
                diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.rule == "CC-SET-021")
                    .count(),
                1,
                "expected one hit in {path}"
            );
        }
    }

    #[test]
    fn test_remote_control_at_startup_false_null_and_absent_are_valid() {
        for content in [
            r#"{"remoteControlAtStartup": false}"#,
            r#"{"remoteControlAtStartup": null}"#,
            r#"{"model": "sonnet"}"#,
        ] {
            assert!(
                validate(content)
                    .iter()
                    .all(|diagnostic| diagnostic.rule != "CC-SET-021"),
                "unexpected CC-SET-021 for {content}"
            );
        }
    }

    #[test]
    fn test_remote_control_at_startup_true_is_allowed_in_managed_settings() {
        let diagnostics = validate_at(
            ".claude/managed-settings.json",
            r#"{"remoteControlAtStartup": true}"#,
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.rule != "CC-SET-021")
        );
    }

    #[test]
    fn test_remote_control_at_startup_diagnostic_points_to_key_and_can_be_disabled() {
        let content = "{\n  \"model\": \"sonnet\",\n  \"remoteControlAtStartup\": true\n}";
        let diagnostics = validate(content);
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.rule == "CC-SET-021")
            .expect("CC-SET-021 diagnostic");
        assert_eq!(diagnostic.line, 3);

        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-021");
        let config = builder.build().expect("valid config");
        let path = PathBuf::from(".claude/settings.json");
        let per_file = config.for_path(&path);
        assert!(
            ClaudeSettingsValidator
                .validate(&path, content, &per_file)
                .iter()
                .all(|diagnostic| diagnostic.rule != "CC-SET-021")
        );
    }

    // ===== CC-SET-022: crossSessionInbound enum =====

    #[test]
    fn test_cross_session_inbound_documented_values_are_valid() {
        for value in ["accept", "hold", "refuse"] {
            let content = format!(r#"{{"crossSessionInbound":"{value}"}}"#);
            assert!(
                validate(&content)
                    .iter()
                    .all(|diagnostic| diagnostic.rule != "CC-SET-022")
            );
        }
    }

    #[test]
    fn test_cross_session_inbound_invalid_values_flag() {
        for value in [r#""prompt""#, "true", "[]"] {
            let content = format!(r#"{{"crossSessionInbound":{value}}}"#);
            assert_eq!(
                validate(&content)
                    .iter()
                    .filter(|diagnostic| diagnostic.rule == "CC-SET-022")
                    .count(),
                1
            );
        }
    }

    // ===== CC-SET-023: dialogExpiry enum =====

    #[test]
    fn test_dialog_expiry_documented_values_are_valid() {
        for value in ["60s", "5m", "10m", "never"] {
            let content = format!(r#"{{"dialogExpiry":"{value}"}}"#);
            assert!(
                validate(&content)
                    .iter()
                    .all(|diagnostic| diagnostic.rule != "CC-SET-023")
            );
        }
    }

    #[test]
    fn test_dialog_expiry_invalid_values_flag() {
        for value in [r#""30s""#, "300", "false"] {
            let content = format!(r#"{{"dialogExpiry":{value}}}"#);
            assert_eq!(
                validate(&content)
                    .iter()
                    .filter(|diagnostic| diagnostic.rule == "CC-SET-023")
                    .count(),
                1
            );
        }
    }

    // ===== CC-SET-024: project-level sandbox.ripgrep =====

    #[test]
    fn test_sandbox_ripgrep_flags_project_settings() {
        let content = r#"{"sandbox": {"ripgrep": "/opt/rg/bin/rg"}}"#;
        for path in [".claude/settings.json", ".claude/settings.local.json"] {
            assert_eq!(
                validate_at(path, content)
                    .iter()
                    .filter(|diagnostic| diagnostic.rule == "CC-SET-024")
                    .count(),
                1,
                "expected one hit in {path}"
            );
        }
    }

    #[test]
    fn test_sandbox_ripgrep_absent_null_and_other_sandbox_keys_are_valid() {
        for content in [
            r#"{"sandbox": {"enabled": true}}"#,
            r#"{"sandbox": {"ripgrep": null}}"#,
            r#"{"sandbox": null}"#,
            r#"{"model": "sonnet"}"#,
        ] {
            assert!(
                validate(content)
                    .iter()
                    .all(|diagnostic| diagnostic.rule != "CC-SET-024"),
                "unexpected CC-SET-024 for {content}"
            );
        }
    }

    #[test]
    fn test_sandbox_ripgrep_is_allowed_in_managed_and_user_settings() {
        let content = r#"{"sandbox": {"ripgrep": "/opt/rg/bin/rg"}}"#;
        assert!(
            validate_at(".claude/managed-settings.json", content)
                .iter()
                .all(|diagnostic| diagnostic.rule != "CC-SET-024")
        );

        let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))
        else {
            return;
        };
        let user_settings = PathBuf::from(home).join(".claude").join("settings.json");
        assert!(
            ClaudeSettingsValidator
                .validate(&user_settings, content, &LintConfig::default())
                .iter()
                .all(|diagnostic| diagnostic.rule != "CC-SET-024")
        );
    }

    #[test]
    fn test_sandbox_ripgrep_diagnostic_points_to_key_and_can_be_disabled() {
        let content =
            "{\n  \"model\": \"sonnet\",\n  \"sandbox\": {\n    \"ripgrep\": \"rg\"\n  }\n}";
        let diagnostic = validate(content)
            .into_iter()
            .find(|diagnostic| diagnostic.rule == "CC-SET-024")
            .expect("CC-SET-024 diagnostic");
        assert_eq!(diagnostic.line, 4);

        let mut builder = LintConfig::builder();
        builder.disable_rule("CC-SET-024");
        let config = builder.build().expect("valid config");
        let path = PathBuf::from(".claude/settings.json");
        let per_file = config.for_path(&path);
        assert!(
            ClaudeSettingsValidator
                .validate(&path, content, &per_file)
                .iter()
                .all(|diagnostic| diagnostic.rule != "CC-SET-024")
        );
    }

    // ===== CC-SET-025: modelPicker scope and shape =====

    #[test]
    fn test_model_picker_valid_shape_is_allowed_in_managed_settings() {
        let content = r#"{
            "modelPicker": {
                "options": [
                    {"model": "opus"},
                    {"model": "us.anthropic.claude-sonnet-4-6", "label": "Sonnet", "description": "Daily"}
                ],
                "replaceBuiltInOptions": true
            }
        }"#;
        assert!(
            validate_at(".claude/managed-settings.json", content)
                .iter()
                .all(|diagnostic| diagnostic.rule != "CC-SET-025")
        );
    }

    #[test]
    fn test_model_picker_optional_null_fields_are_treated_as_absent() {
        let content = r#"{
            "modelPicker": {
                "options": [{
                    "model": "opus",
                    "label": null,
                    "description": null
                }],
                "replaceBuiltInOptions": null
            }
        }"#;

        let diags = validate_at("/etc/claude-code/managed-settings.json", content);
        assert!(
            !diags
                .iter()
                .any(|diagnostic| diagnostic.rule == "CC-SET-025")
        );
    }

    #[test]
    fn test_model_picker_flags_project_scope() {
        let content = r#"{"modelPicker":{"options":[{"model":"opus"}]}}"#;
        for path in [".claude/settings.json", ".claude/settings.local.json"] {
            assert_eq!(
                validate_at(path, content)
                    .iter()
                    .filter(|diagnostic| diagnostic.rule == "CC-SET-025")
                    .count(),
                1,
                "expected one scope diagnostic for {path}"
            );
        }
    }

    #[test]
    fn test_model_picker_flags_invalid_shapes() {
        let cases = [
            r#"{"modelPicker":[]}"#,
            r#"{"modelPicker":{}}"#,
            r#"{"modelPicker":{"options":"opus"}}"#,
            r#"{"modelPicker":{"options":["opus"]}}"#,
            r#"{"modelPicker":{"options":[{}]}}"#,
            r#"{"modelPicker":{"options":[{"model":""}]}}"#,
            r#"{"modelPicker":{"options":[{"model":"opus","label":1}]}}"#,
            r#"{"modelPicker":{"options":[],"replaceBuiltInOptions":"true"}}"#,
        ];
        for content in cases {
            assert!(
                validate_at(".claude/managed-settings.json", content)
                    .iter()
                    .any(|diagnostic| diagnostic.rule == "CC-SET-025"),
                "expected CC-SET-025 for {content}"
            );
        }
    }

    #[test]
    fn test_model_picker_shape_reasons_are_fully_localized() {
        let spanish = t!(
            "rules.cc_set_025.reasons.model_not_string",
            locale = "es",
            index = 0
        );
        assert!(spanish.contains("debe ser una cadena no vacía"));
        assert!(!spanish.contains("must be"));

        let chinese = t!(
            "rules.cc_set_025.reasons.field_not_string",
            locale = "zh-CN",
            index = 1,
            field = "label"
        );
        assert!(chinese.contains("必须是字符串"));
        assert!(!chinese.contains("must be"));
    }

    // ===== CC-SET-028: feedbackDrafts enum =====

    #[test]
    fn test_feedback_drafts_documented_values_are_valid() {
        for mode in ["notify", "quiet", "off"] {
            let content = format!(r#"{{"feedbackDrafts":"{mode}"}}"#);
            assert!(
                validate(&content)
                    .iter()
                    .all(|diagnostic| diagnostic.rule != "CC-SET-028"),
                "{mode} should be accepted"
            );
        }
    }

    #[test]
    fn test_feedback_drafts_boolean_is_flagged() {
        // The natural wrong guess: it reads like an on/off switch but is a
        // string enum, and a boolean silently leaves the default in place.
        for invalid in ["false", "true", "0", r#""disabled""#, "[]"] {
            let content = format!(r#"{{"feedbackDrafts":{invalid}}}"#);
            assert_eq!(
                validate(&content)
                    .iter()
                    .filter(|diagnostic| diagnostic.rule == "CC-SET-028")
                    .count(),
                1,
                "expected CC-SET-028 for {content}"
            );
        }
    }

    #[test]
    fn test_feedback_drafts_null_and_absent_are_valid() {
        for content in [r#"{"feedbackDrafts":null}"#, r#"{"model":"sonnet"}"#] {
            assert!(
                validate(content)
                    .iter()
                    .all(|diagnostic| diagnostic.rule != "CC-SET-028")
            );
        }
    }

    // ===== CC-SET-029: spinnerTipsOverride shape =====

    #[test]
    fn test_spinner_tips_override_documented_example_is_valid() {
        // The settings-reference example, verbatim in shape.
        let content = r#"{
          "spinnerTipsOverride": {
            "label": "Acme tip",
            "excludeDefault": false,
            "tips": [
              "Run /review before opening a PR",
              {
                "id": "gateway-errors",
                "text": "Seeing 5xx errors? Check the gateway status page first",
                "cooldownSessions": 5,
                "priority": 2
              }
            ]
          }
        }"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.rule != "CC-SET-029"),
            "documented example should be accepted, got {diagnostics:?}"
        );
    }

    #[test]
    fn test_spinner_tips_override_tip_object_requires_id_and_text() {
        for invalid_tip in [
            r#"{"text":"Missing an id"}"#,
            r#"{"id":"no-text"}"#,
            r#"{"id":"","text":"Empty id"}"#,
            r#"{"id":"bad id","text":"Space in id"}"#,
            r#"{"id":"ok","text":"   "}"#,
        ] {
            let content = format!(r#"{{"spinnerTipsOverride":{{"tips":[{invalid_tip}]}}}}"#);
            assert!(
                validate(&content)
                    .iter()
                    .any(|diagnostic| diagnostic.rule == "CC-SET-029"),
                "expected CC-SET-029 for {content}"
            );
        }
    }

    #[test]
    fn test_spinner_tips_override_numeric_ranges_are_enforced() {
        for invalid in [
            r#"{"id":"a","text":"t","cooldownSessions":1001}"#,
            r#"{"id":"a","text":"t","cooldownSessions":-1}"#,
            r#"{"id":"a","text":"t","priority":11}"#,
            r#"{"id":"a","text":"t","priority":-11}"#,
        ] {
            let content = format!(r#"{{"spinnerTipsOverride":{{"tips":[{invalid}]}}}}"#);
            assert!(
                validate(&content)
                    .iter()
                    .any(|diagnostic| diagnostic.rule == "CC-SET-029"),
                "expected CC-SET-029 for {content}"
            );
        }
    }

    #[test]
    fn test_spinner_tips_override_boundary_values_are_valid() {
        let content = r#"{"spinnerTipsOverride":{"tips":[{"id":"a.b_c-d","text":"t","cooldownSessions":0,"priority":-10},{"id":"e","text":"t","cooldownSessions":1000,"priority":10}]}}"#;
        assert!(
            validate(content)
                .iter()
                .all(|diagnostic| diagnostic.rule != "CC-SET-029")
        );
    }

    #[test]
    fn test_spinner_tips_override_field_and_path_checks() {
        let cases = [
            (r#"{"spinnerTipsOverride":[]}"#, "non-object value"),
            (
                r#"{"spinnerTipsOverride":{"tips":{}}}"#,
                "tips not an array",
            ),
            (
                r#"{"spinnerTipsOverride":{"tipsFile":"tips.json"}}"#,
                "relative tipsFile",
            ),
            (
                r#"{"spinnerTipsOverride":{"excludeDefault":"yes"}}"#,
                "non-boolean excludeDefault",
            ),
            (
                r#"{"spinnerTipsOverride":{"label":42}}"#,
                "non-string label",
            ),
            (
                r#"{"spinnerTipsOverride":{"tipss":[]}}"#,
                "misspelled field",
            ),
        ];
        for (content, why) in cases {
            assert!(
                validate(content)
                    .iter()
                    .any(|diagnostic| diagnostic.rule == "CC-SET-029"),
                "expected CC-SET-029 for {why}: {content}"
            );
        }
    }

    #[test]
    fn test_spinner_tips_override_accepts_absolute_and_home_tips_files() {
        for tips_file in ["/etc/agnix/tips.json", "~/tips.json", "C:/tips.json"] {
            let content = format!(r#"{{"spinnerTipsOverride":{{"tipsFile":"{tips_file}"}}}}"#);
            assert!(
                validate(&content)
                    .iter()
                    .all(|diagnostic| diagnostic.rule != "CC-SET-029"),
                "{tips_file} should be accepted"
            );
        }
    }

    // ===== CC-SET-026/027: prompt cache TTL enums =====

    #[test]
    fn test_prompt_cache_ttl_documented_values_are_valid() {
        for field in ["promptCacheTtl", "subagentPromptCacheTtl"] {
            for ttl in ["5m", "1h"] {
                let content = format!(r#"{{"{field}":"{ttl}"}}"#);
                assert!(validate(&content).iter().all(|diagnostic| {
                    diagnostic.rule != "CC-SET-026" && diagnostic.rule != "CC-SET-027"
                }));
            }
        }
    }

    #[test]
    fn test_prompt_cache_ttl_invalid_values_flag_the_matching_rule() {
        for (field, rule) in [
            ("promptCacheTtl", "CC-SET-026"),
            ("subagentPromptCacheTtl", "CC-SET-027"),
        ] {
            for invalid in [r#""30m""#, "300", "false", "[]"] {
                let content = format!(r#"{{"{field}":{invalid}}}"#);
                assert_eq!(
                    validate(&content)
                        .iter()
                        .filter(|diagnostic| diagnostic.rule == rule)
                        .count(),
                    1,
                    "expected {rule} for {content}"
                );
            }
        }
    }

    #[test]
    fn test_prompt_cache_ttl_null_and_absent_are_valid() {
        for content in [
            r#"{"promptCacheTtl":null}"#,
            r#"{"subagentPromptCacheTtl":null}"#,
            r#"{"model":"sonnet"}"#,
        ] {
            assert!(validate(content).iter().all(|diagnostic| {
                diagnostic.rule != "CC-SET-026" && diagnostic.rule != "CC-SET-027"
            }));
        }
    }
}
