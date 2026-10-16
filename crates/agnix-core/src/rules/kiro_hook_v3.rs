//! Kiro CLI V3 hook validation and migration rules (KR-HK-011 to KR-HK-014).
//! KR-HK-014 also checks CLI-only spellings in legacy `.kiro.hook` files.
//!
//! Validates `.kiro/hooks/*.json` documents (CLI V3 / IDE 1.0 standalone
//! format: `{ "version": "v1", "hooks": [...] }`):
//! - KR-HK-011: Invalid or missing `version` value (MEDIUM)
//! - KR-HK-012: Invalid hook `trigger` name, or entry missing
//!   `name`/`action` (HIGH)
//! - KR-HK-013: `action.type` must be `command` or `agent`, with the matching
//!   non-empty command/prompt field (HIGH)
//!
//! Source: <https://kiro.dev/docs/hooks/> and
//! <https://kiro.dev/docs/cli/v3/hooks-migration/>.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::{Validator, ValidatorMetadata, line_col_at_offset},
    schemas::kiro_hook_v3::{
        VALID_KIRO_V3_HOOK_DOC_VERSIONS, VALID_KIRO_V3_HOOK_TRIGGERS, canonical_trigger,
        json_value_span, parse_kiro_v3_hook_doc,
    },
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["KR-HK-011", "KR-HK-012", "KR-HK-013", "KR-HK-014"];

/// `timeout` of 0 disables the timeout, so 0 is valid. The documented
/// default is 60 seconds (kept here as the reference value the range check
/// is anchored to; the docs do not define a hard maximum, so the ceiling
/// below is an agnix sanity bound, not a vendor limit).
const MAX_TIMEOUT_SECONDS: u64 = 3_600;

pub struct KiroV3HookValidator;

impl Validator for KiroV3HookValidator {
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
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("hook"))
        {
            return super::kiro_hook::validate_legacy_migration(path, content, config);
        }
        let mut diagnostics = Vec::new();
        let parsed = parse_kiro_v3_hook_doc(content);

        // JSON parse errors surface here: this file type has no other
        // validator that would report malformed JSON.
        if let Some(parse_error) = parsed.parse_error.as_ref()
            && config.is_rule_enabled("KR-HK-012")
        {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    parse_error.line.max(1),
                    parse_error.column,
                    "KR-HK-012",
                    t!(
                        "rules.kr_hk_012.parse_error",
                        error = parse_error.message.as_str()
                    ),
                )
                .with_suggestion(t!("rules.kr_hk_012.suggestion")),
            );
            return diagnostics;
        }

        let Some(doc) = parsed.doc else {
            return diagnostics;
        };

        if config.is_rule_enabled("KR-HK-011") {
            validate_version(path, content, &doc, &mut diagnostics);
        }

        if !config.is_rule_enabled("KR-HK-012")
            && !config.is_rule_enabled("KR-HK-013")
            && !config.is_rule_enabled("KR-HK-014")
        {
            return diagnostics;
        }

        let Some(hooks) = doc.hooks.as_ref() else {
            if config.is_rule_enabled("KR-HK-012") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "KR-HK-012",
                        t!("rules.kr_hk_012.missing_hooks"),
                    )
                    .with_suggestion(t!("rules.kr_hk_012.suggestion")),
                );
            }
            return diagnostics;
        };

        for (index, hook) in hooks.iter().enumerate() {
            validate_entry(path, index, hook, config, &mut diagnostics);
            if config.is_rule_enabled("KR-HK-014") {
                validate_alias(
                    path,
                    content,
                    index,
                    hook.trigger.as_deref(),
                    &mut diagnostics,
                );
            }
        }

        diagnostics
    }
}

/// KR-HK-011: `version` is required by the standalone document format and the
/// docs fix it at `"v1"`.
fn validate_version(
    path: &Path,
    content: &str,
    doc: &crate::schemas::kiro_hook_v3::KiroV3HookDoc,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match doc.version.as_deref() {
        Some(version) if VALID_KIRO_V3_HOOK_DOC_VERSIONS.contains(&version) => {}
        Some(version) => {
            let span = json_value_span(content, &["version"]);
            let (line, column) =
                span.map_or((1, 0), |(start, _)| line_col_at_offset(content, start));
            let mut diagnostic = Diagnostic::warning(
                path.to_path_buf(),
                line,
                column,
                "KR-HK-011",
                t!("rules.kr_hk_011.invalid_version", version = version),
            )
            .with_suggestion(t!("rules.kr_hk_011.suggestion"));
            // This can activate a previously ignored hook, so never auto-apply
            // it as a safe fix. Unknown/future/missing versions are not guessed.
            if matches!(version, "1" | "1.0")
                && let Some((start, end)) = span
            {
                diagnostic = diagnostic.with_fix(Fix::replace(
                    start,
                    end,
                    "\"v1\"",
                    t!("rules.kr_hk_011.fix"),
                    false,
                ));
            }
            diagnostics.push(diagnostic);
        }
        None => diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                1,
                0,
                "KR-HK-011",
                t!("rules.kr_hk_011.missing_version"),
            )
            .with_suggestion(t!("rules.kr_hk_011.suggestion")),
        ),
    }
}

/// Style advice only: aliases remain valid, including /upgrade-agent output.
fn validate_alias(
    path: &Path,
    content: &str,
    index: usize,
    trigger: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(trigger) = trigger else {
        return;
    };
    let Some(canonical) = canonical_trigger(trigger) else {
        return;
    };
    let span = json_value_span(content, &["hooks", &index.to_string(), "trigger"]);
    let (line, column) = span.map_or((1, 0), |(start, _)| line_col_at_offset(content, start));
    let mut diagnostic = Diagnostic::info(
        path.to_path_buf(),
        line,
        column,
        "KR-HK-014",
        t!(
            "rules.kr_hk_014.alias",
            trigger = trigger,
            canonical = canonical
        ),
    )
    .with_suggestion(t!("rules.kr_hk_014.alias_suggestion"));
    if let Some((start, end)) = span {
        diagnostic = diagnostic.with_fix(Fix::replace(
            start,
            end,
            format!("\"{canonical}\""),
            t!("rules.kr_hk_014.fix", canonical = canonical),
            true,
        ));
    }
    diagnostics.push(diagnostic);
}

fn validate_entry(
    path: &Path,
    index: usize,
    hook: &crate::schemas::kiro_hook_v3::KiroV3HookEntry,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if config.is_rule_enabled("KR-HK-012") {
        // `name` is required per the hooks reference field table.
        let name_missing = hook
            .name
            .as_deref()
            .is_none_or(|name| name.trim().is_empty());
        if name_missing {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "KR-HK-012",
                    t!("rules.kr_hk_012.missing_name", index = index),
                )
                .with_suggestion(t!("rules.kr_hk_012.suggestion")),
            );
        }

        // `trigger` is required and must be a documented trigger.
        match hook.trigger.as_deref().map(str::trim) {
            Some(trigger) if !trigger.is_empty() => {
                if !VALID_KIRO_V3_HOOK_TRIGGERS.contains(&trigger) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "KR-HK-012",
                            t!("rules.kr_hk_012.invalid_trigger", trigger = trigger),
                        )
                        .with_suggestion(t!("rules.kr_hk_012.suggestion")),
                    );
                }
            }
            _ => diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "KR-HK-012",
                    t!("rules.kr_hk_012.missing_trigger", index = index),
                )
                .with_suggestion(t!("rules.kr_hk_012.suggestion")),
            ),
        }

        // `action` is required.
        if hook.action.is_none() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "KR-HK-012",
                    t!("rules.kr_hk_012.missing_action", index = index),
                )
                .with_suggestion(t!("rules.kr_hk_012.suggestion")),
            );
        }
    }

    if config.is_rule_enabled("KR-HK-013")
        && let Some(action) = hook.action.as_ref()
    {
        validate_action(path, index, action, diagnostics);
        validate_timeout(path, index, hook, diagnostics);
    }
}

/// KR-HK-013: `action.type` is `command` or `agent`; the matching payload
/// field must be a non-empty string.
fn validate_action(
    path: &Path,
    index: usize,
    action: &crate::schemas::kiro_hook_v3::KiroV3HookAction,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let is_non_empty = |value: Option<&str>| value.is_some_and(|v| !v.trim().is_empty());

    match action.r#type.as_deref() {
        Some("command") => {
            if !is_non_empty(action.command.as_deref()) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "KR-HK-013",
                        t!("rules.kr_hk_013.command_required", index = index),
                    )
                    .with_suggestion(t!("rules.kr_hk_013.suggestion")),
                );
            }
        }
        Some("agent") => {
            if !is_non_empty(action.prompt.as_deref()) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "KR-HK-013",
                        t!("rules.kr_hk_013.prompt_required", index = index),
                    )
                    .with_suggestion(t!("rules.kr_hk_013.suggestion")),
                );
            }
        }
        Some(other) => diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "KR-HK-013",
                t!("rules.kr_hk_013.invalid_type", kind = other, index = index),
            )
            .with_suggestion(t!("rules.kr_hk_013.suggestion")),
        ),
        None => diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "KR-HK-013",
                t!("rules.kr_hk_013.missing_type", index = index),
            )
            .with_suggestion(t!("rules.kr_hk_013.suggestion")),
        ),
    }
}

/// KR-HK-013 (timeout facet): optional `timeout` in seconds, default 60,
/// `0` disables, and command-hook timeouts beyond an hour are almost
/// certainly a mistake (KR-HK-007 caps legacy hooks at 300_000 ms).
fn validate_timeout(
    path: &Path,
    index: usize,
    hook: &crate::schemas::kiro_hook_v3::KiroV3HookEntry,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(timeout) = hook.timeout.as_ref() else {
        return;
    };
    // Validate only numeric timeouts; other shapes keep the parse-level
    // schema-agnostic behavior (the docs type it as a number of seconds).
    if let Some(seconds) = timeout.as_u64() {
        if seconds > MAX_TIMEOUT_SECONDS {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "KR-HK-013",
                    t!(
                        "rules.kr_hk_013.timeout_range",
                        index = index,
                        value = &seconds.to_string(),
                        limit = &MAX_TIMEOUT_SECONDS.to_string()
                    ),
                )
                .with_suggestion(t!("rules.kr_hk_013.suggestion")),
            );
        }
    } else if let Some(seconds) = timeout.as_i64()
        && seconds < 0
    {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                1,
                0,
                "KR-HK-013",
                t!(
                    "rules.kr_hk_013.timeout_negative",
                    index = index,
                    value = &seconds.to_string()
                ),
            )
            .with_suggestion(t!("rules.kr_hk_013.suggestion")),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = KiroV3HookValidator;
        validator.validate(
            Path::new(".kiro/hooks/lint-on-save.json"),
            content,
            &LintConfig::default(),
        )
    }

    #[test]
    fn test_valid_standalone_hook_doc_produces_no_diagnostics() {
        let content = r#"{
  "version": "v1",
  "hooks": [
    {
      "name": "lint-on-save",
      "trigger": "PostFileSave",
      "matcher": "\\.(ts|tsx)$",
      "action": { "type": "command", "command": "npx eslint --fix" },
      "timeout": 30,
      "enabled": true
    },
    { "name": "ctx", "trigger": "UserPromptSubmit", "action": { "type": "agent", "prompt": "Add context" } }
  ]
}"#;
        let diagnostics = validate(content);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn test_kr_hk_011_missing_version() {
        let diagnostics = validate(
            r#"{ "hooks": [ { "name": "a", "trigger": "Stop", "action": { "type": "command", "command": "echo" } } ] }"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-011"));
    }

    #[test]
    fn test_kr_hk_011_unknown_version() {
        let diagnostics = validate(
            r#"{ "version": "v2", "hooks": [ { "name": "a", "trigger": "Stop", "action": { "type": "command", "command": "echo" } } ] }"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-011"));
    }

    #[test]
    fn test_kr_hk_012_invalid_trigger() {
        let diagnostics = validate(
            r#"{ "version": "v1", "hooks": [ { "name": "a", "trigger": "OnFileChange", "action": { "type": "command", "command": "echo" } } ] }"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-012"));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("OnFileChange"))
        );
    }

    #[test]
    fn test_kr_hk_012_legacy_camelcase_alias_accepted() {
        // /upgrade-agent keeps the camelCase trigger alias; V3 accepts it.
        let diagnostics = validate(
            r#"{ "version": "v1", "hooks": [ { "name": "a", "trigger": "agentSpawn", "action": { "type": "command", "command": "git status" } } ] }"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-HK-012"));
    }

    #[test]
    fn test_kr_hk_012_missing_name_and_action() {
        let diagnostics = validate(r#"{ "version": "v1", "hooks": [ { "trigger": "Stop" } ] }"#);
        let count = diagnostics.iter().filter(|d| d.rule == "KR-HK-012").count();
        assert_eq!(count, 2, "{diagnostics:?}");
    }

    #[test]
    fn test_kr_hk_012_missing_hooks_array() {
        let diagnostics = validate(r#"{ "version": "v1" }"#);
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-012"));
    }

    #[test]
    fn test_kr_hk_013_invalid_action_type() {
        let diagnostics = validate(
            r#"{ "version": "v1", "hooks": [ { "name": "a", "trigger": "Stop", "action": { "type": "http", "url": "https://x" } } ] }"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-013"));
    }

    #[test]
    fn test_kr_hk_013_command_action_missing_command() {
        let diagnostics = validate(
            r#"{ "version": "v1", "hooks": [ { "name": "a", "trigger": "Stop", "action": { "type": "command" } } ] }"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-013"));
    }

    #[test]
    fn test_kr_hk_013_agent_action_missing_prompt() {
        let diagnostics = validate(
            r#"{ "version": "v1", "hooks": [ { "name": "a", "trigger": "Stop", "action": { "type": "agent", "prompt": "  " } } ] }"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-013"));
    }

    #[test]
    fn test_kr_hk_013_timeout_over_limit_warns() {
        let diagnostics = validate(
            r#"{ "version": "v1", "hooks": [ { "name": "a", "trigger": "Stop", "action": { "type": "command", "command": "echo" }, "timeout": 7200 } ] }"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-013"));
    }

    #[test]
    fn test_kr_hk_013_zero_timeout_is_fine() {
        let diagnostics = validate(
            r#"{ "version": "v1", "hooks": [ { "name": "a", "trigger": "Stop", "action": { "type": "command", "command": "echo" }, "timeout": 0 } ] }"#,
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn test_parse_error_surfaces_as_kr_hk_012() {
        let diagnostics = validate(r#"{ "version": "v1", "hooks": [ }"#);
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-012"));
    }
}
