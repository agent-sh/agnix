//! Kiro CLI settings validation (KR-SET-*).
//!
//! Validates `.kiro/settings.json` (and `~/.kiro/settings.json`) against the
//! documented CLI settings fields. Today focuses on the Tool Search feature
//! added in Kiro CLI 2.1:
//!
//! - `toolSearch.enabled` (boolean, default false) - master toggle
//! - `toolSearch.minPct` (number, default 5) - % of context window threshold
//! - `toolSearch.minTokens` (number, default 50000) - token count threshold
//!
//! Source: <https://kiro.dev/docs/cli/mcp/tool-search/>
//!
//! Settings are stored as a flat JSON object with dotted keys, e.g.
//! `{"toolSearch.enabled": true, "toolSearch.minPct": 10}`. This mirrors how
//! `kiro-cli settings toolSearch.enabled true` writes them.

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["KR-SET-001", "KR-SET-002", "KR-SET-003"];

pub struct KiroSettingsValidator;

impl Validator for KiroSettingsValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Parse JSON; bail silently on parse errors (other validators in the
        // pipeline surface malformed JSON with a dedicated rule).
        let Ok(value) = serde_json::from_str::<serde_json::Value>(content) else {
            return diagnostics;
        };

        if config.is_rule_enabled("KR-SET-001") {
            validate_tool_search_enabled(path, content, &value, &mut diagnostics);
        }
        if config.is_rule_enabled("KR-SET-002") {
            validate_tool_search_min_pct(path, content, &value, &mut diagnostics);
        }
        if config.is_rule_enabled("KR-SET-003") {
            validate_tool_search_min_tokens(path, content, &value, &mut diagnostics);
        }

        diagnostics
    }
}

/// KR-SET-001: `toolSearch.enabled` must be a boolean when present.
fn validate_tool_search_enabled(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field) = value.get("toolSearch.enabled") else {
        return;
    };
    if field.as_bool().is_none() {
        let line = find_key_line(content, "toolSearch.enabled").unwrap_or(1);
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                0,
                "KR-SET-001",
                t!("rules.kr_set_001.type_error"),
            )
            .with_suggestion(t!("rules.kr_set_001.suggestion")),
        );
    }
}

/// KR-SET-002: `toolSearch.minPct` must be a non-negative number when present.
/// Kiro treats 0 as "always active" so negatives are the only invalid numbers.
/// The docs don't enforce an upper bound, so we only flag obvious misuses
/// (non-number types, negative, or > 100 which would never trigger).
fn validate_tool_search_min_pct(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field) = value.get("toolSearch.minPct") else {
        return;
    };
    let line = find_key_line(content, "toolSearch.minPct").unwrap_or(1);
    match field.as_f64() {
        None => {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    0,
                    "KR-SET-002",
                    t!("rules.kr_set_002.type_error"),
                )
                .with_suggestion(t!("rules.kr_set_002.suggestion")),
            );
        }
        Some(n) if n < 0.0 => {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    0,
                    "KR-SET-002",
                    t!("rules.kr_set_002.negative"),
                )
                .with_suggestion(t!("rules.kr_set_002.suggestion")),
            );
        }
        Some(n) if n > 100.0 => {
            // A percentage over 100 would never trigger Tool Search since the
            // spec tokens can't exceed the context window. Warn (not error)
            // so power users setting it as a disable-by-default knob aren't
            // blocked.
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    "KR-SET-002",
                    t!("rules.kr_set_002.over_100"),
                )
                .with_suggestion(t!("rules.kr_set_002.suggestion")),
            );
        }
        _ => {}
    }
}

/// KR-SET-003: `toolSearch.minTokens` must be a non-negative integer number
/// when present. 0 is valid (means "always active").
fn validate_tool_search_min_tokens(
    path: &Path,
    content: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(field) = value.get("toolSearch.minTokens") else {
        return;
    };
    let line = find_key_line(content, "toolSearch.minTokens").unwrap_or(1);
    match field.as_f64() {
        None => {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    0,
                    "KR-SET-003",
                    t!("rules.kr_set_003.type_error"),
                )
                .with_suggestion(t!("rules.kr_set_003.suggestion")),
            );
        }
        Some(n) if n < 0.0 => {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    0,
                    "KR-SET-003",
                    t!("rules.kr_set_003.negative"),
                )
                .with_suggestion(t!("rules.kr_set_003.suggestion")),
            );
        }
        Some(n) if n.fract() != 0.0 => {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    0,
                    "KR-SET-003",
                    t!("rules.kr_set_003.not_integer"),
                )
                .with_suggestion(t!("rules.kr_set_003.suggestion")),
            );
        }
        _ => {}
    }
}

/// 1-indexed line of the first occurrence of `"<key>":` in a JSON document,
/// skipping matches inside string literals. Matches ASCII keys only.
///
/// Shares the same byte-slice-safe + full-JSON-whitespace implementation as
/// `claude_settings::find_key_line`. Duplicated here (rather than shared via
/// a common helper) to keep this validator self-contained - the two scanners
/// diverge on what counts as a valid key character (Kiro uses dotted keys
/// like `toolSearch.enabled`, which the scanner must accept as-is).
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
            if !in_string
                && i + needle_len <= bytes.len()
                && &bytes[i..i + needle_len] == needle_bytes
            {
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
        let validator = KiroSettingsValidator;
        validator.validate(
            &PathBuf::from(".kiro/settings.json"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = KiroSettingsValidator;
        validator.validate(&PathBuf::from(".kiro/settings.json"), content, config)
    }

    // ===== KR-SET-001: toolSearch.enabled =====

    #[test]
    fn test_kr_set_001_absent_field_is_fine() {
        let diagnostics = validate(r#"{"chat.ui": "prose"}"#);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_kr_set_001_true_is_fine() {
        let diagnostics = validate(r#"{"toolSearch.enabled": true}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SET-001")
            .collect();
        assert!(hits.is_empty());
    }

    #[test]
    fn test_kr_set_001_false_is_fine() {
        let diagnostics = validate(r#"{"toolSearch.enabled": false}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SET-001")
            .collect();
        assert!(hits.is_empty());
    }

    #[test]
    fn test_kr_set_001_string_flags() {
        let diagnostics = validate(r#"{"toolSearch.enabled": "true"}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SET-001")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].level, crate::diagnostics::DiagnosticLevel::Error);
    }

    #[test]
    fn test_kr_set_001_number_flags() {
        let diagnostics = validate(r#"{"toolSearch.enabled": 1}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SET-001")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn test_kr_set_001_null_flags() {
        let diagnostics = validate(r#"{"toolSearch.enabled": null}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SET-001")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    // ===== KR-SET-002: toolSearch.minPct =====

    #[test]
    fn test_kr_set_002_valid_percentage_is_fine() {
        let diagnostics = validate(r#"{"toolSearch.minPct": 5}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SET-002")
            .collect();
        assert!(hits.is_empty());
    }

    #[test]
    fn test_kr_set_002_zero_is_fine() {
        // Kiro documents 0 as "always active".
        let diagnostics = validate(r#"{"toolSearch.minPct": 0}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SET-002")
            .collect();
        assert!(hits.is_empty());
    }

    #[test]
    fn test_kr_set_002_float_is_fine() {
        // Percentages can be fractional (e.g. 2.5%).
        let diagnostics = validate(r#"{"toolSearch.minPct": 2.5}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SET-002")
            .collect();
        assert!(hits.is_empty());
    }

    #[test]
    fn test_kr_set_002_negative_flags() {
        let diagnostics = validate(r#"{"toolSearch.minPct": -1}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SET-002")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].level, crate::diagnostics::DiagnosticLevel::Error);
    }

    #[test]
    fn test_kr_set_002_over_100_warns() {
        let diagnostics = validate(r#"{"toolSearch.minPct": 150}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SET-002")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].level, crate::diagnostics::DiagnosticLevel::Warning);
    }

    #[test]
    fn test_kr_set_002_string_flags() {
        let diagnostics = validate(r#"{"toolSearch.minPct": "5"}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SET-002")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    // ===== KR-SET-003: toolSearch.minTokens =====

    #[test]
    fn test_kr_set_003_valid_is_fine() {
        let diagnostics = validate(r#"{"toolSearch.minTokens": 50000}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SET-003")
            .collect();
        assert!(hits.is_empty());
    }

    #[test]
    fn test_kr_set_003_zero_is_fine() {
        let diagnostics = validate(r#"{"toolSearch.minTokens": 0}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SET-003")
            .collect();
        assert!(hits.is_empty());
    }

    #[test]
    fn test_kr_set_003_negative_flags() {
        let diagnostics = validate(r#"{"toolSearch.minTokens": -10}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SET-003")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn test_kr_set_003_fractional_flags() {
        // Token counts must be whole numbers.
        let diagnostics = validate(r#"{"toolSearch.minTokens": 100.5}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SET-003")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn test_kr_set_003_string_flags() {
        let diagnostics = validate(r#"{"toolSearch.minTokens": "50000"}"#);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SET-003")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    // ===== Combined + disable + edge cases =====

    #[test]
    fn test_all_three_rules_fire_on_combined_bad_config() {
        let content = r#"{
            "toolSearch.enabled": "true",
            "toolSearch.minPct": -5,
            "toolSearch.minTokens": "lots"
        }"#;
        let diagnostics = validate(content);
        let ids: Vec<&str> = diagnostics.iter().map(|d| d.rule.as_str()).collect();
        assert!(ids.contains(&"KR-SET-001"));
        assert!(ids.contains(&"KR-SET-002"));
        assert!(ids.contains(&"KR-SET-003"));
    }

    #[test]
    fn test_rules_are_independently_disableable() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KR-SET-002".to_string()];
        let content = r#"{
            "toolSearch.enabled": "true",
            "toolSearch.minPct": -5
        }"#;
        let diagnostics = validate_with_config(content, &config);
        assert!(diagnostics.iter().any(|d| d.rule == "KR-SET-001"));
        assert!(!diagnostics.iter().any(|d| d.rule == "KR-SET-002"));
    }

    #[test]
    fn test_malformed_json_is_silent() {
        let diagnostics = validate(r#"{"toolSearch.enabled": tr"#);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_line_reporting_for_toolsearch_enabled() {
        let content = "{\n  \"chat.ui\": \"prose\",\n  \"toolSearch.enabled\": \"true\"\n}";
        let diagnostics = validate(content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "KR-SET-001")
            .expect("KR-SET-001 diagnostic");
        assert_eq!(hit.line, 3);
    }

    #[test]
    fn test_prefix_typo_does_not_match_scanner() {
        // toolSearch.enabledX should NOT be matched when searching for
        // toolSearch.enabled.
        let content = "{\n  \"toolSearch.enabledX\": true,\n  \"toolSearch.enabled\": \"bad\"\n}";
        let diagnostics = validate(content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "KR-SET-001")
            .expect("KR-SET-001 diagnostic");
        assert_eq!(hit.line, 3);
    }

    #[test]
    fn test_does_not_panic_on_non_ascii_content() {
        // Byte-slice comparison guards against UTF-8 boundary panics.
        let content = "{\n  \"chat.ui\": \"\u{1F525}prose\u{4e2d}\u{6587}\",\n  \"toolSearch.enabled\": 42\n}";
        let diagnostics = validate(content);
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "KR-SET-001")
            .expect("KR-SET-001 diagnostic");
        assert_eq!(hit.line, 3);
    }
}
