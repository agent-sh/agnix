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
    matches!(
        event,
        "fileEdited" | "fileCreate" | "fileDelete" | "fileCreated" | "fileDeleted"
    )
}

fn is_tool_event(event: &str) -> bool {
    matches!(event, "preToolUse" | "postToolUse")
}

fn has_non_blank_entries(values: &[String]) -> bool {
    values.iter().any(|value| !value.trim().is_empty())
}

/// Do not infer migration from a filename alone: the replacement must contain
/// an enabled v1 hook with the same canonical event and action. An unrelated or
/// malformed JSON sibling must not hide the hint. Read failures are inconclusive.
fn has_corresponding_v3_hook(
    path: &Path,
    event: &str,
    legacy: &crate::schemas::kiro_hook::KiroIdeHook,
    config: &PerFileLintConfig<'_>,
) -> bool {
    use crate::schemas::kiro_hook_v3::{canonical_trigger, parse_kiro_v3_hook_doc};
    let Some(parent) = path.parent() else {
        return true;
    };
    let Ok(entries) = config.fs().read_dir(parent) else {
        return true;
    };
    let event = canonical_trigger(event).unwrap_or(event);
    for entry in entries {
        if !entry.metadata.is_file
            || entry.metadata.is_symlink
            || entry.path.extension().and_then(|s| s.to_str()) != Some("json")
        {
            continue;
        }
        let Ok(content) = config.fs().read_to_string(&entry.path) else {
            return true;
        };
        let Some(doc) = parse_kiro_v3_hook_doc(&content).doc else {
            continue;
        };
        if doc.version.as_deref() != Some("v1") {
            continue;
        }
        for hook in doc.hooks.unwrap_or_default() {
            let Some(trigger) = hook.trigger.as_deref() else {
                continue;
            };
            if canonical_trigger(trigger).unwrap_or(trigger) != event
                || hook.enabled == Some(false)
                || hook.name.as_deref().is_none_or(|n| n.trim().is_empty())
            {
                continue;
            }
            // A narrower matcher is not evidence that the old hook is covered.
            if hook
                .matcher
                .as_deref()
                .is_some_and(|m| !m.is_empty() && m != ".*")
            {
                continue;
            }
            let Some(action) = hook.action else {
                continue;
            };
            if (action.r#type.as_deref() == Some("command")
                && action.command.as_deref().is_some_and(|cmd| {
                    !cmd.trim().is_empty() && Some(cmd) == legacy.effective_run_command()
                }))
                || (action.r#type.as_deref() == Some("agent")
                    && action.prompt.as_deref().is_some_and(|prompt| {
                        !prompt.trim().is_empty() && Some(prompt) == legacy.effective_ask_agent()
                    }))
            {
                return true;
            }
        }
    }
    false
}

/// Low-certainty placement advice owned by the V3 validator. IDE legacy manual
/// hooks remain runnable, and agentSpawn appears in the IDE migration mapping:
/// neither is evidence of a misplaced CLI-only hook.
pub(super) fn validate_legacy_migration(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    if !config.is_rule_enabled("KR-HK-014") {
        return Vec::new();
    }
    let Some(hook) = parse_kiro_hook(content).hook else {
        return Vec::new();
    };
    if hook
        .extra
        .get("enabled")
        .and_then(serde_json::Value::as_bool)
        == Some(false)
    {
        return Vec::new();
    }
    let event = hook.effective_event().unwrap_or("");
    if matches!(
        event,
        "SessionEnd" | "AgentSpawn" | "userPromptSubmit" | "stop"
    ) && !has_corresponding_v3_hook(path, event, &hook, config)
    {
        vec![
            Diagnostic::info(
                path.to_path_buf(),
                1,
                0,
                "KR-HK-014",
                t!("rules.kr_hk_014.legacy", trigger = event),
            )
            .with_suggestion(t!("rules.kr_hk_014.legacy_suggestion")),
        ]
    } else {
        Vec::new()
    }
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

        let event = hook.effective_event().unwrap_or("").trim();
        let cli_only = matches!(
            event,
            "SessionEnd" | "AgentSpawn" | "userPromptSubmit" | "stop"
        );
        let event_valid = !event.is_empty() && VALID_KIRO_HOOK_EVENTS.contains(&event);

        if config.is_rule_enabled("KR-HK-001") && !event_valid && !cli_only {
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
                    .effective_patterns()
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
                    .effective_tool_types()
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
