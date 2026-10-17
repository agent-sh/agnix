//! Gemini CLI settings validation rules (GM-009, GM-010)
//!
//! Validates:
//! - GM-009: Settings.json parse error (HIGH) and unknown top-level keys
//!   (warning; the documented categories of geminicli.com/docs/reference/configuration)
//! - GM-010: `experimental.memoryManager`, which is not a setting since v0.40.0
//!
//! The `hooks` and `hooksConfig` blocks (GM-004, GM-018, GM-019, GM-026) are
//! checked by `rules::gemini_hooks`, which also runs on this file.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::{Validator, ValidatorMetadata},
    schemas::{
        gemini_settings::parse_gemini_settings,
        kiro_json::{JsonMember, LineIndex, parse_json_document},
    },
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["GM-009", "GM-010"];

pub struct GeminiSettingsValidator;

impl Validator for GeminiSettingsValidator {
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
        let path_buf = path.to_path_buf();

        let parsed = parse_gemini_settings(content);

        // GM-009: Parse error (ERROR)
        if let Some(ref error) = parsed.parse_error {
            if config.is_rule_enabled("GM-009") {
                diagnostics.push(
                    Diagnostic::error(
                        path_buf.clone(),
                        error.line,
                        error.column,
                        "GM-009",
                        t!("rules.gm_009.message", error = error.message.as_str()),
                    )
                    .with_suggestion(t!("rules.gm_009.suggestion")),
                );
            }
            return diagnostics;
        }

        let schema = match parsed.schema {
            Some(s) => s,
            None => return diagnostics,
        };

        // Key positions. The document parsed after comment stripping, so the
        // JSONC tree (comments blanked in place) has the same keys.
        let doc = parse_json_document(content, true).ok();
        let lines = LineIndex::new(content);
        let top_level = doc.as_ref().and_then(|doc| doc.root.value.as_object());
        let key_position = |members: Option<&[JsonMember]>, key: &str| {
            members
                .and_then(|members| members.iter().find(|m| m.key == key))
                .map(|m| {
                    let (line, column) = lines.line_col(m.key_start);
                    let (end_line, end_column) = lines.line_col(m.key_end);
                    (line, column, end_line, end_column)
                })
                .unwrap_or((1, 1, 1, 1))
        };

        // GM-009: Unknown top-level keys (WARNING)
        if config.is_rule_enabled("GM-009") {
            for key in &parsed.unknown_top_keys {
                let (line, column, end_line, end_column) = key_position(top_level, key);
                let mut diagnostic = Diagnostic::warning(
                    path_buf.clone(),
                    line,
                    column,
                    "GM-009",
                    t!("rules.gm_009.unknown_key", key = key.as_str()),
                )
                .with_span(end_line, end_column)
                .with_suggestion(t!("rules.gm_009.unknown_key_suggestion"));

                // Unsafe: the key may be a mistyped or misplaced setting
                // the user wants, or one a newer Gemini CLI reads.
                if let Some(members) = top_level
                    && let Some((start, end)) = member_deletion_span(content, members, key)
                {
                    diagnostic = diagnostic.with_fix(Fix::delete(
                        start,
                        end,
                        format!("Remove unknown settings key '{key}'"),
                        false,
                    ));
                }

                diagnostics.push(diagnostic);
            }
        }

        // GM-010: experimental.memoryManager is not a setting since v0.40.0
        // (WARNING). settingsSchema.ts has `memoryManager` up to v0.39.0 and
        // not from v0.40.0 on; `experimental.autoMemory` now drives
        // background extraction and the /memory inbox. Any value is
        // reported: Gemini ignores the key either way. agnix has no
        // gemini-cli version pin, so the rule is not version-gated.
        if config.is_rule_enabled("GM-010")
            && schema
                .experimental
                .as_ref()
                .is_some_and(|exp| exp.has_memory_manager)
        {
            let experimental = top_level
                .and_then(|members| members.iter().rev().find(|m| m.key == "experimental"))
                .and_then(|m| m.value.value.as_object());
            let (line, column, end_line, end_column) = key_position(experimental, "memoryManager");
            diagnostics.push(
                Diagnostic::warning(
                    path_buf.clone(),
                    line,
                    column,
                    "GM-010",
                    t!("rules.gm_010.message"),
                )
                .with_span(end_line, end_column)
                .with_suggestion(t!("rules.gm_010.suggestion")),
            );
        }

        diagnostics
    }
}

/// The bytes to delete to remove member `key` (and one separating comma)
/// from an object, so the result still parses. `None` when the key repeats.
fn member_deletion_span(
    content: &str,
    members: &[JsonMember],
    key: &str,
) -> Option<(usize, usize)> {
    let mut matching = members.iter().enumerate().filter(|(_, m)| m.key == key);
    let (index, member) = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    let bytes = content.as_bytes();
    // Start of the line when only blanks precede `pos` on it.
    let line_start = |pos: usize| {
        let mut start = pos;
        while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
            start -= 1;
        }
        if start == 0 || bytes[start - 1] == b'\n' {
            start
        } else {
            pos
        }
    };
    if let Some(next) = members.get(index + 1) {
        // `"key": value,` up to the next member.
        let start = line_start(member.key_start);
        let end = if start < member.key_start {
            line_start(next.key_start)
        } else {
            next.key_start
        };
        Some((start, end))
    } else if let Some(previous) = index.checked_sub(1).and_then(|i| members.get(i)) {
        // Last member: also remove the comma after the previous value.
        Some((previous.value.end, member.value.end))
    } else {
        Some((member.key_start, member.value.end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = GeminiSettingsValidator;
        validator.validate(
            Path::new(".gemini/settings.json"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = GeminiSettingsValidator;
        validator.validate(Path::new(".gemini/settings.json"), content, config)
    }

    fn rule<'d>(diagnostics: &'d [Diagnostic], id: &str) -> Vec<&'d Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == id).collect()
    }

    // ===== GM-009: Parse Error =====

    #[test]
    fn test_gm_009_invalid_json() {
        let diagnostics = validate("{ invalid }");
        let gm_009 = rule(&diagnostics, "GM-009");
        assert_eq!(gm_009.len(), 1);
        assert_eq!(gm_009[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_gm_009_unterminated_jsonc_comment_is_invalid() {
        let diagnostics = validate(r#"{"general": {}} /* "memoryManager": true"#);
        assert!(diagnostics.iter().any(|d| d.rule == "GM-009"));
        assert!(diagnostics.iter().all(|d| d.rule == "GM-009"));
    }

    #[test]
    fn test_gm_009_comment_removal_does_not_join_tokens() {
        let diagnostics = validate(r#"{"experimental":{"autoMemory":tr/*x*/ue}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "GM-009"));
        assert!(diagnostics.iter().all(|d| d.rule == "GM-009"));
    }

    #[test]
    fn test_gm_009_empty_content() {
        let diagnostics = validate("");
        assert_eq!(rule(&diagnostics, "GM-009").len(), 1);
    }

    #[test]
    fn test_gm_009_unknown_top_key() {
        let content = "{\"general\": {}, \"badKey\": true}";
        let diagnostics = validate(content);
        let gm_009 = rule(&diagnostics, "GM-009");
        assert_eq!(gm_009.len(), 1);
        assert_eq!(gm_009[0].level, DiagnosticLevel::Warning);
        // Points at the key.
        assert_eq!((gm_009[0].line, gm_009[0].column), (1, 17));
    }

    #[test]
    fn test_gm_009_documented_keys_are_known() {
        // agnix 0.58 warned on `hooks` and `mcpServers`, which the
        // configuration reference documents.
        let content = r#"{
  "$schema": "https://raw.githubusercontent.com/google-gemini/gemini-cli/main/schemas/settings.schema.json",
  "hooks": {},
  "mcpServers": {},
  "policyPaths": [],
  "privacy": {},
  "billing": {},
  "modelConfigs": {},
  "agents": {},
  "mcp": {},
  "useWriteTodos": true,
  "contextManagement": {},
  "admin": {},
  "telemetry": {},
  "extensions": {}
}"#;
        assert!(rule(&validate(content), "GM-009").is_empty());
    }

    #[test]
    fn test_gm_009_valid_json_no_issues() {
        assert!(rule(&validate(r#"{"general": {}}"#), "GM-009").is_empty());
    }

    #[test]
    fn test_gm_009_blocks_further_rules() {
        let diagnostics = validate("{ invalid }");
        assert!(diagnostics.iter().all(|d| d.rule == "GM-009"));
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_gemini_md_category() {
        let mut config = LintConfig::default();
        config.rules_mut().gemini_md = false;

        let diagnostics = validate_with_config("{ invalid }", &config);
        assert!(diagnostics.iter().all(|d| !d.rule.starts_with("GM-")));
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["GM-009".to_string()];

        let diagnostics = validate_with_config("{ invalid }", &config);
        assert!(rule(&diagnostics, "GM-009").is_empty());
    }

    // ===== Valid Settings =====

    #[test]
    fn test_valid_settings_no_issues() {
        let content = r#"{
  "general": {},
  "model": {},
  "hooksConfig": {"enabled": true},
  "hooks": {
    "SessionStart": [
      {"hooks": [{"type": "command", "command": "echo hello"}]}
    ]
  }
}"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got: {:?}",
            diagnostics
        );
    }

    // ===== GM-010: experimental.memoryManager =====

    #[test]
    fn test_gm_010_fires_for_any_memory_manager_value() {
        // The rule used to fire only for `true` without autoMemory; since
        // v0.40.0 the key is not a setting at all, so every value is stale.
        for value in ["true", "false", "null", "\"on\""] {
            for auto_memory in ["", ", \"autoMemory\": true", ", \"autoMemory\": false"] {
                let content = format!(
                    "{{\n  \"experimental\": {{\n    \"memoryManager\": {value}{auto_memory}\n  }}\n}}"
                );
                let diagnostics = validate(&content);
                let gm_010 = rule(&diagnostics, "GM-010");
                assert_eq!(gm_010.len(), 1, "{content}");
                assert_eq!(gm_010[0].level, DiagnosticLevel::Warning);
                assert_eq!((gm_010[0].line, gm_010[0].column), (3, 5), "{content}");
                assert!(
                    gm_010[0]
                        .message
                        .contains("not a Gemini CLI setting since v0.40.0")
                );
            }
        }
    }

    #[test]
    fn test_gm_010_auto_memory_only_no_diagnostic() {
        let content = r#"{"experimental": {"autoMemory": true}}"#;
        assert!(rule(&validate(content), "GM-010").is_empty());
    }

    #[test]
    fn test_gm_010_no_experimental_block_no_diagnostic() {
        assert!(rule(&validate(r#"{"general": {}}"#), "GM-010").is_empty());
    }

    #[test]
    fn test_gm_010_ignores_key_inside_block_comment() {
        // The string "memoryManager" appears only inside a block comment.
        let content = r#"{
  /* note: experimental.memoryManager was removed in v0.40,
     see https://github.com/google-gemini/gemini-cli/pull/25601 */
  "experimental": {}
}"#;
        assert!(rule(&validate(content), "GM-010").is_empty());
    }

    #[test]
    fn test_gm_010_ignores_key_inside_line_comment() {
        let content = r#"{
  // doc: https://example.com/gemini-cli/experimental.memoryManager
  "experimental": {}
}"#;
        assert!(rule(&validate(content), "GM-010").is_empty());
    }

    #[test]
    fn test_gm_010_memory_manager_outside_experimental_is_gm_009_only() {
        let content = r#"{"memoryManager": true}"#;
        let diagnostics = validate(content);
        assert!(rule(&diagnostics, "GM-010").is_empty());
        assert_eq!(rule(&diagnostics, "GM-009").len(), 1);
    }

    #[test]
    fn test_gm_010_disabled_by_config() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["GM-010".to_string()];

        let content = r#"{"experimental": {"memoryManager": true}}"#;
        let diagnostics = validate_with_config(content, &config);
        assert!(rule(&diagnostics, "GM-010").is_empty());
    }

    // ===== Autofix Tests =====

    #[test]
    fn test_gm_009_unknown_key_fix_is_unsafe_exact_and_reparses() {
        // The former line-based fix left a trailing comma when the unknown
        // key was the last member, which Gemini cannot parse.
        for (content, expected) in [
            (
                "{\n  \"general\": {},\n  \"badKey\": true\n}",
                "{\n  \"general\": {}\n}",
            ),
            (
                "{\n  \"badKey\": {\"a\": 1},\n  \"general\": {}\n}",
                "{\n  \"general\": {}\n}",
            ),
            (
                "{\"ui\": {}, \"badKey\": [1], \"general\": {}}",
                "{\"ui\": {}, \"general\": {}}",
            ),
            ("{\"badKey\": 1}", "{}"),
            (
                "{\n  // keep\n  \"general\": {}, // tail\n  \"badKey\": true\n}",
                "{\n  // keep\n  \"general\": {}\n}",
            ),
        ] {
            let diagnostics = validate(content);
            let gm_009 = rule(&diagnostics, "GM-009");
            assert_eq!(gm_009.len(), 1, "{content}");
            let fix = &gm_009[0].fixes[0];
            assert!(!fix.safe, "GM-009 fix should be unsafe");
            assert!(fix.is_deletion());
            let mut fixed = content.to_string();
            fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
            assert_eq!(fixed, expected, "{content}");
            assert!(
                crate::schemas::gemini_settings::parse_gemini_settings(&fixed)
                    .parse_error
                    .is_none(),
                "{fixed}"
            );
            assert!(validate(&fixed).is_empty(), "{fixed}");
        }
    }

    #[test]
    fn test_gm_009_duplicate_unknown_key_has_no_fix() {
        let diagnostics = validate(r#"{"badKey": 1, "badKey": 2}"#);
        let gm_009 = rule(&diagnostics, "GM-009");
        assert_eq!(gm_009.len(), 1);
        assert!(gm_009[0].fixes.is_empty());
    }
}
