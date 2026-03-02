//! Kiro IDE hook validation rules (KR-HK-001 to KR-HK-004).
//!
//! Validates `.kiro/hooks/*.kiro.hook` files:
//! - KR-HK-001: Invalid hook event type
//! - KR-HK-002: File event hook missing patterns
//! - KR-HK-003: Hook missing action
//! - KR-HK-004: Pre/Post tool hook missing toolTypes

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
    schemas::kiro_hook::{VALID_KIRO_HOOK_EVENTS, parse_kiro_hook},
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["KR-HK-001", "KR-HK-002", "KR-HK-003", "KR-HK-004"];

fn is_file_event(event: &str) -> bool {
    matches!(event, "fileEdited" | "fileCreate" | "fileDelete")
}

fn is_tool_event(event: &str) -> bool {
    matches!(event, "preToolUse" | "postToolUse")
}

fn has_non_blank_entries(values: &[String]) -> bool {
    values.iter().any(|value| !value.trim().is_empty())
}

pub struct KiroHookValidator;

impl Validator for KiroHookValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let parsed = parse_kiro_hook(content);

        if config.is_rule_enabled("KR-HK-001")
            && let Some(parse_error) = parsed.parse_error.as_ref()
        {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    parse_error.line.max(1),
                    parse_error.column,
                    "KR-HK-001",
                    t!(
                        "rules.kr_hk_001_parse.message",
                        error = parse_error.message.as_str()
                    ),
                )
                .with_suggestion(t!("rules.kr_hk_001_parse.suggestion")),
            );
            return diagnostics;
        }

        let Some(hook) = parsed.hook else {
            return diagnostics;
        };

        let event = hook.event.as_deref().unwrap_or("").trim();
        let event_valid = !event.is_empty() && VALID_KIRO_HOOK_EVENTS.contains(&event);

        if config.is_rule_enabled("KR-HK-001") && !event_valid {
            let display = if event.is_empty() { "<missing>" } else { event };
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "KR-HK-001",
                    t!("rules.kr_hk_001.message", event = display),
                )
                .with_suggestion(t!("rules.kr_hk_001.suggestion")),
            );
        }

        if event_valid {
            if config.is_rule_enabled("KR-HK-002")
                && is_file_event(event)
                && hook
                    .patterns
                    .as_ref()
                    .is_none_or(|patterns| !has_non_blank_entries(patterns))
            {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "KR-HK-002",
                        t!("rules.kr_hk_002.message", event = event),
                    )
                    .with_suggestion(t!("rules.kr_hk_002.suggestion")),
                );
            }

            if config.is_rule_enabled("KR-HK-004")
                && is_tool_event(event)
                && hook
                    .tool_types
                    .as_ref()
                    .is_none_or(|tool_types| !has_non_blank_entries(tool_types))
            {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "KR-HK-004",
                        t!("rules.kr_hk_004.message", event = event),
                    )
                    .with_suggestion(t!("rules.kr_hk_004.suggestion")),
                );
            }
        }

        if config.is_rule_enabled("KR-HK-003") && !hook.has_action() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "KR-HK-003",
                    t!("rules.kr_hk_003.message"),
                )
                .with_suggestion(t!("rules.kr_hk_003.suggestion")),
            );
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = KiroHookValidator;
        validator.validate(
            Path::new(".kiro/hooks/example.kiro.hook"),
            content,
            &LintConfig::default(),
        )
    }

    #[test]
    fn test_kr_hk_001_invalid_event() {
        let diagnostics = validate(include_str!(
            "../../../../tests/fixtures/kiro-hooks/.kiro/hooks/invalid-event.kiro.hook"
        ));
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-001"));
    }

    #[test]
    fn test_kr_hk_002_missing_patterns_for_file_event() {
        let diagnostics = validate(include_str!(
            "../../../../tests/fixtures/kiro-hooks/.kiro/hooks/missing-patterns.kiro.hook"
        ));
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-002"));
    }

    #[test]
    fn test_kr_hk_002_blank_patterns_for_file_event() {
        let diagnostics = validate(
            r#"{
  "event": "fileEdited",
  "patterns": ["   "],
  "runCommand": "echo changed"
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-002"));
    }

    #[test]
    fn test_kr_hk_003_missing_action() {
        let diagnostics = validate(include_str!(
            "../../../../tests/fixtures/kiro-hooks/.kiro/hooks/missing-action.kiro.hook"
        ));
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-003"));
    }

    #[test]
    fn test_kr_hk_003_blank_action() {
        let diagnostics = validate(
            r#"{
  "event": "promptSubmit",
  "runCommand": "   "
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-003"));
    }

    #[test]
    fn test_kr_hk_004_missing_tool_types_for_tool_event() {
        let diagnostics = validate(include_str!(
            "../../../../tests/fixtures/kiro-hooks/.kiro/hooks/missing-tool-types.kiro.hook"
        ));
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-004"));
    }

    #[test]
    fn test_kr_hk_004_blank_tool_types_for_tool_event() {
        let diagnostics = validate(
            r#"{
  "event": "preToolUse",
  "toolTypes": ["   "],
  "runCommand": "echo changed"
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-004"));
    }

    #[test]
    fn test_kr_hk_parse_error_reports_diagnostic() {
        let diagnostics = validate(r#"{"event":"fileEdited","patterns":[}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-001"));
    }

    #[test]
    fn test_valid_hooks_have_no_kr_hk_diagnostics() {
        let fixtures = [
            include_str!(
                "../../../../tests/fixtures/kiro-hooks/.kiro/hooks/valid-file-save.kiro.hook"
            ),
            include_str!(
                "../../../../tests/fixtures/kiro-hooks/.kiro/hooks/valid-prompt-submit.kiro.hook"
            ),
            include_str!(
                "../../../../tests/fixtures/kiro-hooks/.kiro/hooks/valid-pre-tool.kiro.hook"
            ),
        ];

        for fixture in fixtures {
            let diagnostics = validate(fixture);
            assert!(diagnostics.iter().all(|d| !d.rule.starts_with("KR-HK-")));
        }
    }

    #[test]
    fn test_metadata() {
        let validator = KiroHookValidator;
        let metadata = validator.metadata();
        assert_eq!(metadata.name, "KiroHookValidator");
        assert_eq!(
            metadata.rule_ids,
            &["KR-HK-001", "KR-HK-002", "KR-HK-003", "KR-HK-004"]
        );
    }
}
