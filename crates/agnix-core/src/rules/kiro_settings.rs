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
    diagnostics::{Diagnostic, Fix},
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
        let mut diagnostic = Diagnostic::error(
            path.to_path_buf(),
            line,
            0,
            "KR-SET-001",
            t!("rules.kr_set_001.type_error"),
        )
        .with_suggestion(t!("rules.kr_set_001.suggestion"));

        // Auto-fix: when the user wrote "true" / "false" / "True" / "FALSE"
        // as a quoted string, strip the quotes + normalize case. Safe
        // because the fix preserves the user's clearly-intended value:
        // Kiro's toolSearch.enabled field expects a boolean, not a string,
        // so the quoted form would be rejected at config load anyway.
        if let Some(s) = field.as_str()
            && let Some(parsed) = parse_string_as_bool(s)
            && let Some((start, end)) = find_value_span(content, "toolSearch.enabled")
        {
            diagnostic = diagnostic.with_fix(Fix::replace(
                start,
                end,
                if parsed { "true" } else { "false" }.to_string(),
                format!("Remove quotes: \"{s}\" -> {parsed}"),
                true,
            ));
        }

        diagnostics.push(diagnostic);
    }
}

/// Parse a string like "true"/"True"/"false"/"FALSE" as a boolean.
/// Returns None for anything ambiguous ("1"/"0"/"yes"/"no" etc.) to keep
/// the auto-fix conservative.
fn parse_string_as_bool(s: &str) -> Option<bool> {
    match s.to_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// Parse a string as a JSON number (integer or float) per RFC 8259
/// section 6. Stricter than `f64::from_str` - rejects leading `+`,
/// leading zeros on multi-digit integers, leading/trailing `.`, and
/// negative values (since KR-SET-002/003 both expect non-negative).
///
/// Returns the canonical JSON representation so the auto-fix rewrites
/// exactly what Kiro would have accepted. Returns None if the string
/// isn't a valid non-negative JSON number. This prevents auto-fix from
/// emitting invalid JSON like `05` or from auto-correcting a negative
/// string that would just re-flag on the negative rule.
fn parse_string_as_number(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if !is_json_nonneg_number(trimmed) {
        return None;
    }
    Some(trimmed.to_string())
}

/// Parse a string as a JSON non-negative integer. Rejects everything
/// `parse_string_as_number` rejects plus any fractional/exponent form.
fn parse_string_as_integer(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if !is_json_nonneg_integer(trimmed) {
        return None;
    }
    Some(trimmed.to_string())
}

/// JSON RFC 8259 number grammar, restricted to non-negative values:
///   number = int [ frac ] [ exp ]
///   int    = "0" | ( digit1-9 *DIGIT )       ; no leading zeros on multi-digit
///   frac   = "." 1*DIGIT                     ; requires digits after .
///   exp    = ("e"|"E") [ "+" | "-" ] 1*DIGIT
/// Rejects negative numbers, leading `+`, `.5`, `5.`, `05`, empty string.
fn is_json_nonneg_number(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let mut i;
    // int part
    match bytes.first() {
        Some(b'0') => {
            i = 1;
            // Must be followed by end, `.`, or exponent - not another digit.
            if let Some(b) = bytes.get(i)
                && b.is_ascii_digit()
            {
                return false; // leading zero
            }
        }
        Some(b) if (b'1'..=b'9').contains(b) => {
            i = 1;
            while let Some(c) = bytes.get(i)
                && c.is_ascii_digit()
            {
                i += 1;
            }
        }
        _ => return false, // empty, `-`, `+`, `.`, or other
    }
    // frac part
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        let frac_start = i;
        while let Some(c) = bytes.get(i)
            && c.is_ascii_digit()
        {
            i += 1;
        }
        if i == frac_start {
            return false; // `5.` with no fraction digits
        }
    }
    // exp part
    if let Some(c) = bytes.get(i)
        && (*c == b'e' || *c == b'E')
    {
        i += 1;
        if let Some(s) = bytes.get(i)
            && (*s == b'+' || *s == b'-')
        {
            i += 1;
        }
        let exp_start = i;
        while let Some(c) = bytes.get(i)
            && c.is_ascii_digit()
        {
            i += 1;
        }
        if i == exp_start {
            return false; // `5e` with no exponent digits
        }
    }
    i == bytes.len()
}

/// Non-negative JSON integer: digits only, no leading zeros on multi-digit,
/// no fraction, no exponent.
fn is_json_nonneg_integer(s: &str) -> bool {
    let bytes = s.as_bytes();
    match bytes.first() {
        Some(b'0') => bytes.len() == 1,
        Some(b) if (b'1'..=b'9').contains(b) => bytes.iter().all(|c| c.is_ascii_digit()),
        _ => false,
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
            let mut diagnostic = Diagnostic::error(
                path.to_path_buf(),
                line,
                0,
                "KR-SET-002",
                t!("rules.kr_set_002.type_error"),
            )
            .with_suggestion(t!("rules.kr_set_002.suggestion"));

            // Auto-fix: string-that-parses-as-number -> bare number. Safe
            // because the user's numeric intent is clearly preserved and
            // Kiro rejects quoted numbers anyway.
            if let Some(s) = field.as_str()
                && let Some(parsed) = parse_string_as_number(s)
                && let Some((start, end)) = find_value_span(content, "toolSearch.minPct")
            {
                diagnostic = diagnostic.with_fix(Fix::replace(
                    start,
                    end,
                    parsed.clone(),
                    format!("Remove quotes: \"{s}\" -> {parsed}"),
                    true,
                ));
            }

            diagnostics.push(diagnostic);
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
            let mut diagnostic = Diagnostic::error(
                path.to_path_buf(),
                line,
                0,
                "KR-SET-003",
                t!("rules.kr_set_003.type_error"),
            )
            .with_suggestion(t!("rules.kr_set_003.suggestion"));

            // Auto-fix: string-that-parses-as-integer -> bare integer. Only
            // integers qualify here since minTokens is a token count; a
            // fractional string like "1.5" falls through to manual fix.
            if let Some(s) = field.as_str()
                && let Some(parsed) = parse_string_as_integer(s)
                && let Some((start, end)) = find_value_span(content, "toolSearch.minTokens")
            {
                diagnostic = diagnostic.with_fix(Fix::replace(
                    start,
                    end,
                    parsed.clone(),
                    format!("Remove quotes: \"{s}\" -> {parsed}"),
                    true,
                ));
            }

            diagnostics.push(diagnostic);
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

/// Locate the byte span of the JSON *value* that follows `"<key>":`.
///
/// Returns `(start, end)` covering the value token exactly - no leading or
/// trailing whitespace and no comma. For a string value `"abc"` the span
/// includes both surrounding quotes. This drives auto-fix replacements like
/// `"true"` -> `true` where we want to swap the whole token.
///
/// Returns `None` when the key isn't present, when the value is an object/
/// array (which would need bracket-matching), or when parsing ambiguity
/// would make the span unsafe to rewrite. Callers fall back to "manual fix"
/// in that case.
fn find_value_span(content: &str, key: &str) -> Option<(usize, usize)> {
    debug_assert!(
        key.is_ascii() && !key.contains('"') && !key.contains('\\'),
        "find_value_span expects ASCII key without quotes or backslashes"
    );
    let needle = format!("\"{key}\"");
    let needle_bytes = needle.as_bytes();
    let needle_len = needle_bytes.len();
    let bytes = content.as_bytes();
    let mut in_string = false;
    let mut escape = false;
    let mut i = 0;

    // Phase 1: walk to the `:` after the key.
    let mut colon_pos: Option<usize> = None;
    while i < bytes.len() {
        let b = bytes[i];
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
                    colon_pos = Some(j);
                    break;
                }
            }
            in_string = !in_string;
        }
        i += 1;
    }
    let colon = colon_pos?;

    // Phase 2: skip whitespace after `:` to find the value start.
    let mut start = colon + 1;
    while start < bytes.len() && matches!(bytes[start], b' ' | b'\t' | b'\n' | b'\r') {
        start += 1;
    }
    if start >= bytes.len() {
        return None;
    }

    // Phase 3: scan the value token. We only handle string, number, true,
    // false, null - the shapes the type-coercion fixes care about.
    // Objects and arrays would need bracket-matching; return None so callers
    // skip auto-fix rather than guessing.
    match bytes[start] {
        b'"' => {
            // String literal. Walk until closing `"` respecting escapes.
            let mut k = start + 1;
            let mut esc = false;
            while k < bytes.len() {
                let b = bytes[k];
                if esc {
                    esc = false;
                } else if b == b'\\' {
                    esc = true;
                } else if b == b'"' {
                    return Some((start, k + 1));
                }
                k += 1;
            }
            None // unterminated string
        }
        b't' | b'f' | b'n' => {
            // true / false / null literal. Match the canonical spelling.
            for lit in ["true", "false", "null"] {
                let lit_bytes = lit.as_bytes();
                if start + lit_bytes.len() <= bytes.len()
                    && &bytes[start..start + lit_bytes.len()] == lit_bytes
                {
                    return Some((start, start + lit_bytes.len()));
                }
            }
            None
        }
        b'-' | b'+' | b'0'..=b'9' => {
            // Number token. Walk while the next byte is a valid number
            // character (JSON number grammar: digits, ., e, +, -). This is
            // a lenient scan - good enough for rewriting a string back to
            // a bare number form.
            let mut k = start + 1;
            while k < bytes.len()
                && matches!(bytes[k], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
            {
                k += 1;
            }
            Some((start, k))
        }
        _ => None, // object, array, or other unhandled shape
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

    // ===== Auto-fix: string-to-typed coercion =====

    /// Apply the first fix on the first matching diagnostic to `content`
    /// and return the new string. Panics if no matching diagnostic or fix
    /// exists so failing tests point at the missing autofix.
    fn apply_first_fix(content: &str, rule: &str) -> String {
        let diagnostics = validate(content);
        let diag = diagnostics
            .iter()
            .find(|d| d.rule == rule)
            .unwrap_or_else(|| panic!("expected {rule} diagnostic, got {:?}", diagnostics));
        let fix = diag
            .fixes
            .first()
            .unwrap_or_else(|| panic!("{rule} diagnostic had no fix attached: {:?}", diag));
        let mut out = content.to_string();
        out.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        out
    }

    #[test]
    fn test_kr_set_001_autofix_string_true_to_bool() {
        let content = r#"{"toolSearch.enabled": "true"}"#;
        let fixed = apply_first_fix(content, "KR-SET-001");
        assert_eq!(fixed, r#"{"toolSearch.enabled": true}"#);
        // And re-validation of the fixed output should be clean.
        assert!(validate(&fixed).iter().all(|d| d.rule != "KR-SET-001"));
    }

    #[test]
    fn test_kr_set_001_autofix_string_false_to_bool() {
        let content = r#"{"toolSearch.enabled": "false"}"#;
        let fixed = apply_first_fix(content, "KR-SET-001");
        assert_eq!(fixed, r#"{"toolSearch.enabled": false}"#);
    }

    #[test]
    fn test_kr_set_001_autofix_case_insensitive() {
        // "True" / "FALSE" / "TRUE" etc. should all normalize to lowercase bool.
        let content = r#"{"toolSearch.enabled": "TRUE"}"#;
        let fixed = apply_first_fix(content, "KR-SET-001");
        assert_eq!(fixed, r#"{"toolSearch.enabled": true}"#);
    }

    #[test]
    fn test_kr_set_001_no_autofix_for_ambiguous_string() {
        // "1" / "yes" / "on" are NOT unambiguously bool - no fix offered.
        let content = r#"{"toolSearch.enabled": "yes"}"#;
        let diagnostics = validate(content);
        let diag = diagnostics
            .iter()
            .find(|d| d.rule == "KR-SET-001")
            .expect("KR-SET-001");
        assert!(
            diag.fixes.is_empty(),
            "ambiguous string must not get an auto-fix, got {:?}",
            diag.fixes
        );
    }

    #[test]
    fn test_kr_set_001_no_autofix_for_non_string_type() {
        // Number/array/object get the diagnostic but no auto-fix - we
        // can't guess what value the user meant.
        for fixture in [
            r#"{"toolSearch.enabled": 1}"#,
            r#"{"toolSearch.enabled": 0}"#,
            r#"{"toolSearch.enabled": null}"#,
            r#"{"toolSearch.enabled": []}"#,
        ] {
            let diagnostics = validate(fixture);
            let diag = diagnostics
                .iter()
                .find(|d| d.rule == "KR-SET-001")
                .unwrap_or_else(|| panic!("fixture {fixture} did not flag"));
            assert!(
                diag.fixes.is_empty(),
                "no fix expected for {fixture}, got {:?}",
                diag.fixes
            );
        }
    }

    #[test]
    fn test_kr_set_002_autofix_string_to_number() {
        let content = r#"{"toolSearch.minPct": "5"}"#;
        let fixed = apply_first_fix(content, "KR-SET-002");
        assert_eq!(fixed, r#"{"toolSearch.minPct": 5}"#);
    }

    #[test]
    fn test_kr_set_002_autofix_string_float_to_number() {
        let content = r#"{"toolSearch.minPct": "2.5"}"#;
        let fixed = apply_first_fix(content, "KR-SET-002");
        assert_eq!(fixed, r#"{"toolSearch.minPct": 2.5}"#);
    }

    #[test]
    fn test_kr_set_002_no_autofix_for_non_numeric_string() {
        let content = r#"{"toolSearch.minPct": "five"}"#;
        let diagnostics = validate(content);
        let diag = diagnostics
            .iter()
            .find(|d| d.rule == "KR-SET-002")
            .expect("KR-SET-002");
        assert!(diag.fixes.is_empty());
    }

    #[test]
    fn test_kr_set_002_no_autofix_for_negative_number() {
        // Negative gets the rule but not type_error - no fix path (negative
        // is a semantic mistake, can't mechanically correct).
        let content = r#"{"toolSearch.minPct": -5}"#;
        let diagnostics = validate(content);
        let diag = diagnostics
            .iter()
            .find(|d| d.rule == "KR-SET-002")
            .expect("KR-SET-002");
        assert!(diag.fixes.is_empty());
    }

    #[test]
    fn test_kr_set_003_autofix_string_to_integer() {
        let content = r#"{"toolSearch.minTokens": "50000"}"#;
        let fixed = apply_first_fix(content, "KR-SET-003");
        assert_eq!(fixed, r#"{"toolSearch.minTokens": 50000}"#);
    }

    #[test]
    fn test_kr_set_003_no_autofix_for_fractional_string() {
        // Integer parser rejects "50000.5" so no fix - and the "not_integer"
        // rule message would immediately re-flag anyway.
        let content = r#"{"toolSearch.minTokens": "50000.5"}"#;
        let diagnostics = validate(content);
        let diag = diagnostics
            .iter()
            .find(|d| d.rule == "KR-SET-003")
            .expect("KR-SET-003");
        assert!(diag.fixes.is_empty());
    }

    #[test]
    fn test_autofix_line_reporting_stays_intact() {
        // Autofix replaces only the value span - the line number of the
        // diagnostic should still point at the key line.
        let content = "{\n  \"chat.ui\": \"prose\",\n  \"toolSearch.enabled\": \"true\"\n}";
        let diagnostics = validate(content);
        let diag = diagnostics
            .iter()
            .find(|d| d.rule == "KR-SET-001")
            .expect("KR-SET-001");
        assert_eq!(diag.line, 3);
        let fix = diag.fixes.first().expect("fix attached");
        // The fix replacement points at the value on line 3, not the key.
        assert_eq!(&content[fix.start_byte..fix.end_byte], "\"true\"");
        assert_eq!(fix.replacement, "true");
    }

    // ===== find_value_span unit tests =====

    #[test]
    fn test_find_value_span_string() {
        let content = r#"{"k": "v"}"#;
        let (s, e) = find_value_span(content, "k").unwrap();
        assert_eq!(&content[s..e], r#""v""#);
    }

    #[test]
    fn test_find_value_span_number() {
        let content = r#"{"k": 42}"#;
        let (s, e) = find_value_span(content, "k").unwrap();
        assert_eq!(&content[s..e], "42");
    }

    #[test]
    fn test_find_value_span_bool() {
        let content = r#"{"k": true}"#;
        let (s, e) = find_value_span(content, "k").unwrap();
        assert_eq!(&content[s..e], "true");
    }

    #[test]
    fn test_find_value_span_null() {
        let content = r#"{"k": null}"#;
        let (s, e) = find_value_span(content, "k").unwrap();
        assert_eq!(&content[s..e], "null");
    }

    #[test]
    fn test_find_value_span_object_returns_none() {
        // Object/array need bracket-matching; return None so callers skip
        // auto-fix rather than mis-slicing.
        let content = r#"{"k": {"nested": 1}}"#;
        assert!(find_value_span(content, "k").is_none());
    }

    #[test]
    fn test_find_value_span_missing_key() {
        let content = r#"{"k": 1}"#;
        assert!(find_value_span(content, "other").is_none());
    }

    #[test]
    fn test_find_value_span_key_inside_string_literal_ignored() {
        // "k" mentioned in a prose value must not be treated as the key.
        let content = r#"{"note": "k is an important key", "k": 42}"#;
        let (s, e) = find_value_span(content, "k").unwrap();
        assert_eq!(&content[s..e], "42");
    }

    // ===== JSON number grammar (is_json_nonneg_number / _integer) =====

    #[test]
    fn test_json_number_accepts_valid_forms() {
        for v in [
            "0", "5", "42", "100", "2.5", "0.5", "1e10", "1E10", "1.5e-3", "1.5E+3",
        ] {
            assert!(is_json_nonneg_number(v), "should accept {v}");
        }
    }

    #[test]
    fn test_json_number_rejects_leading_zero() {
        // "05" is invalid JSON per RFC 8259. Auto-fix must not emit it.
        for v in ["05", "05.5", "007", "00"] {
            assert!(!is_json_nonneg_number(v), "should reject {v}");
        }
    }

    #[test]
    fn test_json_number_rejects_negative_and_leading_plus() {
        // Rust parses these fine but JSON doesn't accept `+`, and
        // negatives re-flag on the .negative rule so auto-fix shouldn't
        // produce them.
        for v in ["-5", "+5", "-0.5", "+100"] {
            assert!(!is_json_nonneg_number(v), "should reject {v}");
        }
    }

    #[test]
    fn test_json_number_rejects_malformed_fraction_or_exponent() {
        for v in [".5", "5.", "5.e3", "5e", "5e+", "5.e", ""] {
            assert!(!is_json_nonneg_number(v), "should reject {v}");
        }
    }

    #[test]
    fn test_json_integer_accepts_valid_forms() {
        for v in ["0", "5", "42", "50000", "9999999"] {
            assert!(is_json_nonneg_integer(v), "should accept {v}");
        }
    }

    #[test]
    fn test_json_integer_rejects_fractional_and_exponent_and_leading_zero() {
        for v in ["5.5", "1e10", "05", "00", "-5", "+5", "", "5."] {
            assert!(!is_json_nonneg_integer(v), "should reject {v}");
        }
    }

    // ===== Regression: parse_string_as_* declines shapes that would
    //       break the invariant "autofix produces valid JSON" =====

    #[test]
    fn test_parse_string_as_number_declines_leading_zero() {
        assert!(parse_string_as_number("050").is_none());
        assert!(parse_string_as_number("007.5").is_none());
    }

    #[test]
    fn test_parse_string_as_number_declines_negative() {
        // Negative strings on minPct would auto-fix to a negative number
        // and immediately re-flag on .negative. Better to stay manual.
        assert!(parse_string_as_number("-5").is_none());
        assert!(parse_string_as_number("+5").is_none());
    }

    #[test]
    fn test_parse_string_as_integer_declines_fractional_string() {
        assert!(parse_string_as_integer("5.5").is_none());
        assert!(parse_string_as_integer("1e10").is_none());
    }

    #[test]
    fn test_kr_set_002_no_autofix_for_leading_zero_string() {
        // Even though "05" looks number-ish, emitting it as bare `05`
        // would produce invalid JSON. Stay manual.
        let content = r#"{"toolSearch.minPct": "05"}"#;
        let diagnostics = validate(content);
        let diag = diagnostics
            .iter()
            .find(|d| d.rule == "KR-SET-002")
            .expect("KR-SET-002");
        assert!(
            diag.fixes.is_empty(),
            "leading-zero string must not auto-fix to invalid JSON, got {:?}",
            diag.fixes
        );
    }

    #[test]
    fn test_kr_set_002_no_autofix_for_negative_string() {
        // Auto-fixing "-5" to `-5` would immediately re-flag on the
        // negative rule. Keep it manual.
        let content = r#"{"toolSearch.minPct": "-5"}"#;
        let diagnostics = validate(content);
        let diag = diagnostics
            .iter()
            .find(|d| d.rule == "KR-SET-002")
            .expect("KR-SET-002");
        assert!(diag.fixes.is_empty());
    }
}
