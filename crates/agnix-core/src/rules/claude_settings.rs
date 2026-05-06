//! Claude Code `.claude/settings.json` top-level field validation (CC-SET-*).
//!
//! This validator complements the hooks-focused rules in `hooks/mod.rs` by
//! checking top-level settings fields documented at
//! <https://code.claude.com/docs/en/settings>.
//!
//! Rules:
//! - `prUrlTemplate` (CC-SET-001, added in Claude Code v2.1.119).
//! - `channelsEnabled` boolean check (CC-SET-002, added in Claude Code v2.1.128).
//!
//! Runs on FileType::Hooks (which covers `.claude/settings.json` -
//! see `file_types/detection.rs`). Skips non-Claude Code settings paths
//! (e.g. `.amp/settings.json`) via a parent-directory check.

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["CC-SET-001", "CC-SET-002"];

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

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
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

fn describe_json_type(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::String(_) => "string",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
        // Bool and Null are handled by the caller.
        serde_json::Value::Bool(_) | serde_json::Value::Null => "other",
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
        let content =
            "{\n  \"model\": \"claude-sonnet-4\",\n  \"channelsEnabled\": \"true\"\n}";
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
        let diagnostics =
            validator.validate(&path, r#"{"channelsEnabled": "true"}"#, &config);
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
}
