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
//!
//! Runs on FileType::Hooks (which covers `.claude/settings.json` -
//! see `file_types/detection.rs`). Skips non-Claude Code settings paths
//! (e.g. `.amp/settings.json`) via a parent-directory check.

use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
};
use rust_i18n::t;
use std::path::Path;

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
];

/// Allowed values for `worktree.baseRef` per Claude Code v2.1.133 release notes.
const WORKTREE_BASE_REF_ALLOWED: &[&str] = &["fresh", "head"];

/// Allowed values for `parentSettingsBehavior` per Claude Code v2.1.133 release notes.
const PARENT_SETTINGS_BEHAVIOR_ALLOWED: &[&str] = &["first-wins", "merge"];

/// Documented `sandbox.*` string-valued path settings added in v2.1.133.
const SANDBOX_PATH_FIELDS: &[&str] = &["bwrapPath", "socatPath"];

/// Allowed values for `teammateMode`. Claude Code v2.1.186 added `iterm2`.
const TEAMMATE_MODE_ALLOWED: &[&str] = &["in-process", "auto", "tmux", "iterm2"];

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

        diagnostics
    }
}

/// Only validate `.claude/settings.json`, `.claude/settings.local.json`, and
/// managed/project variants. Skip `.amp/`, `.kiro/`, etc. settings files -
/// they are classified as FileType::Hooks too but have entirely different
/// field sets, and a false positive here would be disruptive.
fn is_claude_settings_path(path: &Path) -> bool {
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
    matches!(
        filename,
        "settings.json" | "settings.local.json" | "managed-settings.json"
    )
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
/// Each entry must name the credential target and carry `"mode": "deny"`; no
/// other mode is currently supported.
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
                format!(
                    "sandbox.credentials must be an object with optional files and envVars arrays (got {})",
                    describe_json_type(credentials)
                ),
            )
            .with_suggestion(
                "Set sandbox.credentials to an object like {\"files\":[{\"path\":\"~/.aws/credentials\",\"mode\":\"deny\"}],\"envVars\":[{\"name\":\"GITHUB_TOKEN\",\"mode\":\"deny\"}]} or remove it.",
            ),
        );
        return;
    };

    validate_sandbox_credential_entries(
        path,
        content,
        credentials_obj.get("files"),
        "files",
        "path",
        "credential file path",
        diagnostics,
    );
    validate_sandbox_credential_entries(
        path,
        content,
        credentials_obj.get("envVars"),
        "envVars",
        "name",
        "credential environment variable name",
        diagnostics,
    );
}

fn validate_sandbox_credential_entries(
    path: &Path,
    content: &str,
    value: Option<&serde_json::Value>,
    array_key: &str,
    required_key: &str,
    required_label: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(value) = value else {
        return;
    };

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
                format!(
                    "sandbox.credentials.{array_key} must be an array (got {})",
                    describe_json_type(value)
                ),
            )
            .with_suggestion(format!(
                "Set sandbox.credentials.{array_key} to an array of objects with \"{required_key}\" and \"mode\": \"deny\".",
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
                    format!(
                        "sandbox.credentials.{array_key}[{index}] must be an object (got {})",
                        describe_json_type(entry)
                    ),
                )
                .with_suggestion(format!(
                    "Use {{\"{required_key}\": \"...\", \"mode\": \"deny\"}} for each sandbox.credentials.{array_key} entry.",
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
                    format!(
                        "sandbox.credentials.{array_key}[{index}].{required_key} must not be empty"
                    ),
                )
                .with_suggestion(format!(
                    "Set sandbox.credentials.{array_key}[{index}].{required_key} to a non-empty {required_label}.",
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
                        format!(
                            "sandbox.credentials.{array_key}[{index}].{required_key} must be a string (got {actual})"
                        ),
                    )
                    .with_suggestion(format!(
                        "Set sandbox.credentials.{array_key}[{index}].{required_key} to a non-empty {required_label}.",
                    )),
                );
            }
        }

        match entry_obj.get("mode").and_then(|v| v.as_str()) {
            Some("deny") => {}
            Some(mode) => diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "CC-SET-012",
                    format!(
                        "sandbox.credentials.{array_key}[{index}].mode must be \"deny\" (got \"{mode}\")"
                    ),
                )
                .with_suggestion(format!(
                    "Set sandbox.credentials.{array_key}[{index}].mode to \"deny\"; no other mode is documented.",
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
                        format!(
                            "sandbox.credentials.{array_key}[{index}].mode must be \"deny\" (got {actual})"
                        ),
                    )
                    .with_suggestion(format!(
                        "Set sandbox.credentials.{array_key}[{index}].mode to \"deny\".",
                    )),
                );
            }
        }
    }
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
    use crate::config::LintConfig;
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
                { "path": "~/.ssh", "mode": "deny" }
              ],
              "envVars": [
                { "name": "GITHUB_TOKEN", "mode": "deny" },
                { "name": "NPM_TOKEN", "mode": "deny" }
              ]
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
    fn test_sandbox_credentials_mode_must_be_deny() {
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
}
