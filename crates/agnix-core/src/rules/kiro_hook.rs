//! Kiro IDE hook validation rules (KR-HK-001 to KR-HK-010).
//!
//! Validates `.kiro/hooks/*.kiro.hook` files:
//! - KR-HK-001: Invalid hook event type
//! - KR-HK-002: File event hook missing patterns
//! - KR-HK-003: Hook missing action
//! - KR-HK-004: Pre/Post tool hook missing toolTypes
//! - KR-HK-007: Hook timeout out of range
//! - KR-HK-008: Duplicate event handlers
//! - KR-HK-009: Command uses absolute path
//! - KR-HK-010: Secrets in hook command

use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
    schemas::kiro_hook::{VALID_KIRO_HOOK_EVENTS, parse_kiro_hook},
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &[
    "KR-HK-001",
    "KR-HK-002",
    "KR-HK-003",
    "KR-HK-004",
    "KR-HK-007",
    "KR-HK-008",
    "KR-HK-009",
    "KR-HK-010",
];

const MAX_HOOK_TIMEOUT_MS: u64 = 300_000; // 5 minutes

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

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
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

        // KR-HK-007: Hook timeout out of range
        if config.is_rule_enabled("KR-HK-007") {
            if let Some(timeout_val) = hook
                .extra
                .get("timeout")
                .or_else(|| hook.extra.get("timeoutMs"))
            {
                if let Some(timeout) = timeout_val.as_u64() {
                    if timeout == 0 || timeout > MAX_HOOK_TIMEOUT_MS {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "KR-HK-007",
                                t!(
                                    "rules.kr_hk_007.message",
                                    value = &timeout.to_string(),
                                    limit = &MAX_HOOK_TIMEOUT_MS.to_string()
                                ),
                            )
                            .with_suggestion(t!("rules.kr_hk_007.suggestion")),
                        );
                    }
                } else if let Some(timeout) = timeout_val.as_i64() {
                    if timeout <= 0 {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "KR-HK-007",
                                t!(
                                    "rules.kr_hk_007.message",
                                    value = &timeout.to_string(),
                                    limit = &MAX_HOOK_TIMEOUT_MS.to_string()
                                ),
                            )
                            .with_suggestion(t!("rules.kr_hk_007.suggestion")),
                        );
                    }
                }
            }
        }

        // Note: KR-HK-008 (duplicate event handlers) is a project-level check
        // requiring cross-file analysis; registered in RULE_IDS but checked
        // at the project validator layer.

        // KR-HK-009: Command uses absolute path
        if config.is_rule_enabled("KR-HK-009") {
            if let Some(cmd) = hook.effective_run_command() {
                let trimmed = cmd.trim();
                if trimmed.starts_with('/')
                    || (trimmed.len() >= 3
                        && trimmed.as_bytes()[0].is_ascii_alphabetic()
                        && trimmed.as_bytes().get(1) == Some(&b':')
                        && matches!(trimmed.as_bytes().get(2), Some(b'\\') | Some(b'/')))
                {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            1,
                            0,
                            "KR-HK-009",
                            t!("rules.kr_hk_009.message"),
                        )
                        .with_suggestion(t!("rules.kr_hk_009.suggestion")),
                    );
                }
            }
        }

        // KR-HK-010: Secrets in hook command
        if config.is_rule_enabled("KR-HK-010") {
            if let Some(cmd) = hook.effective_run_command() {
                let lower = cmd.to_ascii_lowercase();
                // Known secret token prefixes (always suspicious in commands)
                let has_token_prefix = ["sk-", "ghp_", "gho_", "xoxb-", "xoxp-", "glpat-"]
                    .iter()
                    .any(|pfx| lower.contains(pfx));
                // AWS key prefix (4+ uppercase after AKIA)
                let has_aws_prefix = lower.contains("akia") && cmd.contains("AKIA");
                // Key=value assignments with sensitive key names
                let has_secret_assignment = [
                    "api_key=",
                    "apikey=",
                    "token=",
                    "password=",
                    "secret=",
                    "--token=",
                    "--password=",
                    "--api-key=",
                ]
                .iter()
                .any(|kv| lower.contains(kv));
                // Bearer token pattern
                let has_bearer = lower.contains("bearer ") && {
                    // Only flag if followed by a value that looks like a secret
                    if let Some(idx) = lower.find("bearer ") {
                        let after = cmd[idx + 7..].trim();
                        super::seems_plaintext_secret(after)
                    } else {
                        false
                    }
                };
                if has_token_prefix || has_aws_prefix || has_secret_assignment || has_bearer {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "KR-HK-010",
                            t!("rules.kr_hk_010.message"),
                        )
                        .with_suggestion(t!("rules.kr_hk_010.suggestion")),
                    );
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;

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
    fn test_kr_hk_007_timeout_exceeds_limit() {
        let diagnostics = validate(
            r#"{
  "event": "promptSubmit",
  "runCommand": "echo test",
  "timeout": 600000
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-007"));
    }

    #[test]
    fn test_kr_hk_007_negative_timeout() {
        let diagnostics = validate(
            r#"{
  "event": "promptSubmit",
  "runCommand": "echo test",
  "timeout": -1
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-007"));
    }

    #[test]
    fn test_kr_hk_007_valid_timeout_no_diagnostic() {
        let diagnostics = validate(
            r#"{
  "event": "promptSubmit",
  "runCommand": "echo test",
  "timeout": 30000
}"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-HK-007"));
    }

    #[test]
    fn test_kr_hk_009_absolute_path_in_command() {
        let diagnostics = validate(
            r#"{
  "event": "promptSubmit",
  "runCommand": "/usr/local/bin/lint"
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-009"));
    }

    #[test]
    fn test_kr_hk_009_relative_path_no_diagnostic() {
        let diagnostics = validate(
            r#"{
  "event": "promptSubmit",
  "runCommand": "npm run lint"
}"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-HK-009"));
    }

    #[test]
    fn test_kr_hk_010_secrets_in_command() {
        let diagnostics = validate(
            r#"{
  "event": "promptSubmit",
  "runCommand": "curl -H 'Bearer sk-secret123'"
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-010"));
    }

    #[test]
    fn test_kr_hk_010_no_secret_no_diagnostic() {
        let diagnostics = validate(
            r#"{
  "event": "promptSubmit",
  "runCommand": "echo hello"
}"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-HK-010"));
    }

    #[test]
    fn test_kr_hk_008_is_registered_in_metadata() {
        // KR-HK-008 (duplicate event handlers) requires project-level context;
        // verify it is registered in metadata.
        let validator = KiroHookValidator;
        let metadata = validator.metadata();
        assert!(metadata.rule_ids.contains(&"KR-HK-008"));
    }

    #[test]
    fn test_kr_hk_007_timeout_ms_key_variant() {
        let diagnostics = validate(
            r#"{
  "event": "promptSubmit",
  "runCommand": "echo test",
  "timeoutMs": 600000
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-007"));
    }

    #[test]
    fn test_kr_hk_007_boundary_at_limit_no_diagnostic() {
        let diagnostics = validate(
            r#"{
  "event": "promptSubmit",
  "runCommand": "echo test",
  "timeout": 300000
}"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-HK-007"));
    }

    #[test]
    fn test_kr_hk_007_boundary_above_limit() {
        let diagnostics = validate(
            r#"{
  "event": "promptSubmit",
  "runCommand": "echo test",
  "timeout": 300001
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-007"));
    }

    #[test]
    fn test_kr_hk_009_windows_absolute_path() {
        let diagnostics = validate(
            r#"{
  "event": "promptSubmit",
  "runCommand": "C:\\lint.exe"
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-009"));
    }

    #[test]
    fn test_kr_hk_010_api_key_secret() {
        let diagnostics = validate(
            r#"{
  "event": "promptSubmit",
  "runCommand": "curl -d api_key=mysecretkey123"
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-010"));
    }

    #[test]
    fn test_kr_hk_010_password_secret() {
        let diagnostics = validate(
            r#"{
  "event": "promptSubmit",
  "runCommand": "mysql -p password=hunter2secret"
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-010"));
    }

    #[test]
    fn test_metadata() {
        let validator = KiroHookValidator;
        let metadata = validator.metadata();
        assert_eq!(metadata.name, "KiroHookValidator");
        assert_eq!(
            metadata.rule_ids,
            &[
                "KR-HK-001",
                "KR-HK-002",
                "KR-HK-003",
                "KR-HK-004",
                "KR-HK-007",
                "KR-HK-008",
                "KR-HK-009",
                "KR-HK-010",
            ]
        );
    }
}
