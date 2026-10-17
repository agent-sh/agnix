//! Kiro IDE 0.x hook validation rules.
//!
//! Validates `.kiro/hooks/*.kiro.hook` files. Kiro CLI never reads them; Kiro
//! IDE 1.0 lists them as legacy hooks and offers to upgrade each one to the
//! `.kiro/hooks/*.json` v1 format.
//! - KR-HK-001: Invalid hook event type
//! - KR-HK-002: File event hook missing patterns
//! - KR-HK-003: Hook missing action
//! - KR-HK-004: Pre/Post tool hook missing toolTypes
//! - KR-HK-009: Command uses absolute path
//! - KR-HK-016: v1 hook document saved as `.kiro.hook`
//! - KR-HK-026: invalid JSON or wrongly typed fields
//!
//! KR-HK-010 (secrets in a hook command) covers both hook formats, so the V3
//! validator owns it; see `validate_legacy_migration`.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::{Validator, ValidatorMetadata},
    schemas::{
        kiro_hook::{KiroIdeHook, is_known_ide_event, looks_like_v1_hook},
        kiro_hook_v3::{JsonSpans, Seg, is_match_all_glob, resolve_trigger},
    },
};
use rust_i18n::t;
use serde_json::Value;
use std::{cell::OnceCell, path::Path};

const RULE_IDS: &[&str] = &[
    "KR-HK-001",
    "KR-HK-002",
    "KR-HK-003",
    "KR-HK-004",
    "KR-HK-009",
    "KR-HK-016",
    "KR-HK-026",
];

/// CLI trigger spellings that IDE 0.x never used. They get the KR-HK-014
/// migration hint instead of KR-HK-001.
const CLI_ONLY_EVENTS: &[&str] = &["SessionEnd", "AgentSpawn", "userPromptSubmit", "stop"];

fn is_file_event(event: &str) -> bool {
    matches!(
        event,
        "fileEdited" | "fileCreate" | "fileDelete" | "fileCreated" | "fileDeleted"
    )
}

fn is_tool_event(event: &str) -> bool {
    matches!(event, "preToolUse" | "postToolUse")
}

fn has_non_blank_entries(values: &[&str]) -> bool {
    values.iter().any(|value| !value.trim().is_empty())
}

/// The V3 trigger a CLI spelling in a legacy file corresponds to.
/// `AgentSpawn` is not a V3 alias (only `agentSpawn` is), but its intent is.
fn cli_event_canonical(event: &str) -> &str {
    match event {
        "AgentSpawn" => "SessionStart",
        other => resolve_trigger(other).unwrap_or(other),
    }
}

/// Do not infer migration from a filename alone: the replacement must contain
/// an enabled v1 hook with the same canonical event and action. An unrelated or
/// malformed JSON sibling must not hide the hint. Read failures are inconclusive.
fn has_corresponding_v3_hook(
    path: &Path,
    event: &str,
    legacy: &KiroIdeHook<'_>,
    config: &PerFileLintConfig<'_>,
) -> bool {
    let Some(parent) = path.parent() else {
        return true;
    };
    let Ok(entries) = config.fs().read_dir(parent) else {
        return true;
    };
    let event = cli_event_canonical(event);
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
        let Ok(doc) = serde_json::from_str::<Value>(&content) else {
            continue;
        };
        if doc.get("version").and_then(Value::as_str) != Some("v1") {
            continue;
        }
        let Some(hooks) = doc.get("hooks").and_then(Value::as_array) else {
            continue;
        };
        for hook in hooks {
            let Some(trigger) = hook.get("trigger").and_then(Value::as_str) else {
                continue;
            };
            if resolve_trigger(trigger) != Some(event)
                || hook.get("enabled").and_then(Value::as_bool) == Some(false)
                || hook
                    .get("name")
                    .and_then(Value::as_str)
                    .is_none_or(|n| n.trim().is_empty())
            {
                continue;
            }
            // A narrower matcher is not evidence that the old hook is covered.
            if hook
                .get("matcher")
                .and_then(Value::as_str)
                .is_some_and(|m| !m.is_empty() && m != ".*")
            {
                continue;
            }
            let Some(action) = hook.get("action") else {
                continue;
            };
            let text = |key: &str| {
                action
                    .get(key)
                    .and_then(Value::as_str)
                    .filter(|text| !text.trim().is_empty())
            };
            match action.get("type").and_then(Value::as_str) {
                Some("command")
                    if text("command").is_some()
                        && text("command") == legacy.effective_run_command() =>
                {
                    return true;
                }
                Some("agent")
                    if text("prompt").is_some()
                        && text("prompt") == legacy.effective_ask_agent() =>
                {
                    return true;
                }
                _ => {}
            }
        }
    }
    false
}

/// Legacy-file checks owned by the V3 validator: the KR-HK-010 secret check,
/// the KR-HK-014 placement hint and the KR-HK-028 broad file pattern note.
/// IDE legacy manual hooks remain runnable, and agentSpawn appears in the IDE
/// migration mapping: neither is evidence of a misplaced CLI-only hook.
pub(super) fn validate_legacy_migration(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let secrets = config.is_rule_enabled("KR-HK-010");
    let hint = config.is_rule_enabled("KR-HK-014");
    let broad = config.is_rule_enabled("KR-HK-028");
    if !secrets && !hint && !broad {
        return Vec::new();
    }
    let Ok(root) = serde_json::from_str::<Value>(content) else {
        return Vec::new();
    };
    if looks_like_v1_hook(&root) {
        return Vec::new(); // KR-HK-016 covers it
    }
    let hook = KiroIdeHook::from_value(&root);
    let locator = Locator {
        content,
        spans: OnceCell::new(),
    };
    let mut diagnostics = Vec::new();
    // A committed credential leaks whether or not the hook is enabled.
    if secrets
        && let Some(command) = hook.run_command.as_ref()
        && command_has_secret(command.value)
    {
        let (line, column) = locator.at(&command.path);
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                column,
                "KR-HK-010",
                t!("rules.kr_hk_010.message"),
            )
            .with_suggestion(t!("rules.kr_hk_010.suggestion")),
        );
    }
    if hook.enabled == Some(false) {
        return diagnostics;
    }
    let Some(event) = hook.event.as_ref() else {
        return diagnostics;
    };
    if hint
        && CLI_ONLY_EVENTS.contains(&event.value)
        && !has_corresponding_v3_hook(path, event.value, &hook, config)
    {
        let (line, column) = locator.at(&event.path);
        diagnostics.push(
            Diagnostic::info(
                path.to_path_buf(),
                line,
                column,
                "KR-HK-014",
                t!("rules.kr_hk_014.legacy", trigger = event.value),
            )
            .with_suggestion(t!("rules.kr_hk_014.legacy_suggestion")),
        );
    }
    // Missing patterns are KR-HK-002; here every pattern selects every file.
    if broad
        && is_file_event(event.value.trim())
        && let Some(patterns) = hook.patterns.as_ref()
        && !patterns.malformed
        && !patterns.values.is_empty()
        && patterns
            .values
            .iter()
            .all(|pattern| is_match_all_glob(pattern))
    {
        let (line, column) = locator.at(&patterns.path);
        diagnostics.push(
            Diagnostic::info(
                path.to_path_buf(),
                line,
                column,
                "KR-HK-028",
                t!("rules.kr_hk_028.legacy", event = event.value.trim()),
            )
            .with_suggestion(t!("rules.kr_hk_028.suggestion")),
        );
    }
    diagnostics
}

pub struct KiroHookValidator;

struct Locator<'a> {
    content: &'a str,
    spans: OnceCell<JsonSpans>,
}

impl Locator<'_> {
    fn spans(&self) -> &JsonSpans {
        self.spans.get_or_init(|| JsonSpans::new(self.content))
    }

    fn at(&self, path: &[Seg<'_>]) -> (usize, usize) {
        self.spans()
            .nearest(path)
            .map_or((1, 1), |loc| (loc.line, loc.column))
    }
}

/// Undocumented event spellings agnix used to accept, with the documented
/// IDE 0.x name. Whether IDE 0.x also accepted them is unverified, so they
/// are warnings rather than errors.
fn documented_event_for(event: &str) -> Option<&'static str> {
    match event {
        "fileCreate" => Some("fileCreated"),
        "fileDelete" => Some("fileDeleted"),
        "manual" => Some("userTriggered"),
        _ => None,
    }
}

fn display_path(path: &[Seg<'_>]) -> String {
    let mut out = String::new();
    for seg in path {
        match seg {
            Seg::Key(key) => {
                if !out.is_empty() {
                    out.push('.');
                }
                out.push_str(key);
            }
            Seg::Index(index) => out.push_str(&format!("[{index}]")),
        }
    }
    out
}

fn found(value: Option<&Value>) -> &'static str {
    match value {
        None | Some(Value::Null) => "null",
        Some(Value::Bool(_)) => "a boolean",
        Some(Value::Number(_)) => "a number",
        Some(Value::String(_)) => "a string",
        Some(Value::Array(_)) => "an array",
        Some(Value::Object(_)) => "an object",
    }
}

fn value_at<'v>(root: &'v Value, path: &[Seg<'_>]) -> Option<&'v Value> {
    path.iter().try_fold(root, |value, seg| match seg {
        Seg::Key(key) => value.get(*key),
        Seg::Index(index) => value.get(*index),
    })
}

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
        let root = match serde_json::from_str::<Value>(content) {
            Ok(root) => root,
            Err(error) => {
                if config.is_rule_enabled("KR-HK-026") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            error.line().max(1),
                            error.column(),
                            "KR-HK-026",
                            t!("rules.kr_hk_026.parse_error", error = error.to_string()),
                        )
                        .with_suggestion(t!("rules.kr_hk_026.suggestion")),
                    );
                }
                return diagnostics;
            }
        };
        let locator = Locator {
            content,
            spans: OnceCell::new(),
        };
        if !root.is_object() {
            if config.is_rule_enabled("KR-HK-026") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        1,
                        "KR-HK-026",
                        t!("rules.kr_hk_026.root_type", found = found(Some(&root))),
                    )
                    .with_suggestion(t!("rules.kr_hk_026.suggestion")),
                );
            }
            return diagnostics;
        }
        if looks_like_v1_hook(&root) {
            if config.is_rule_enabled("KR-HK-016") {
                let key = ["hooks", "trigger", "action"]
                    .into_iter()
                    .find(|key| root.get(*key).is_some())
                    .unwrap_or("hooks");
                let (line, column) = locator.at(&[Seg::Key(key)]);
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        column,
                        "KR-HK-016",
                        t!("rules.kr_hk_016.message"),
                    )
                    .with_suggestion(t!("rules.kr_hk_016.suggestion")),
                );
            }
            return diagnostics;
        }

        let hook = KiroIdeHook::from_value(&root);
        if config.is_rule_enabled("KR-HK-026") {
            for error in &hook.type_errors {
                let (line, column) = locator.at(&error.path);
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        column,
                        "KR-HK-026",
                        t!(
                            "rules.kr_hk_026.field_type",
                            field = display_path(&error.path),
                            found = found(value_at(&root, &error.path)),
                            expected = error.expected
                        ),
                    )
                    .with_suggestion(t!("rules.kr_hk_026.suggestion")),
                );
            }
        }

        if config.is_rule_enabled("KR-HK-026") {
            for timeout_path in &hook.negative_timeouts {
                let (line, column) = locator.at(timeout_path);
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        column,
                        "KR-HK-026",
                        t!(
                            "rules.kr_hk_026.negative_timeout",
                            field = display_path(timeout_path)
                        ),
                    )
                    .with_suggestion(t!("rules.kr_hk_026.suggestion")),
                );
            }
        }

        // IDE 0.x whitespace handling is unverified, so surrounding spaces are tolerated.
        let event = hook.effective_event().unwrap_or("").trim();
        let event_at = |locator: &Locator<'_>| match hook.event.as_ref() {
            Some(event) => locator.at(&event.path),
            None => locator.at(&[Seg::Key("when")]),
        };
        let cli_only = CLI_ONLY_EVENTS.contains(&event);
        let event_valid = is_known_ide_event(event);

        if config.is_rule_enabled("KR-HK-001")
            && let Some(documented) = documented_event_for(event)
            && let Some(located) = hook.event.as_ref()
        {
            let (line, column) = locator.at(&located.path);
            let mut diagnostic = Diagnostic::warning(
                path.to_path_buf(),
                line,
                column,
                "KR-HK-001",
                t!(
                    "rules.kr_hk_001.undocumented",
                    event = event,
                    documented = documented
                ),
            )
            .with_suggestion(t!("rules.kr_hk_001.suggestion"));
            // Unsafe: the IDE's handling of the old spelling is unverified.
            if located.value == event
                && let Some(loc) = locator.spans().unique(&located.path)
            {
                diagnostic = diagnostic.with_fix(Fix::replace(
                    loc.start,
                    loc.end,
                    format!("\"{documented}\""),
                    t!("rules.kr_hk_001.fix", documented = documented),
                    false,
                ));
            }
            diagnostics.push(diagnostic);
        }

        // A malformed event field is already reported as KR-HK-026.
        if config.is_rule_enabled("KR-HK-001") && !event_valid && !cli_only && !hook.event_malformed
        {
            let (line, column) = event_at(&locator);
            let message = if event.is_empty() && hook.event.is_none() {
                t!("rules.kr_hk_001.missing")
            } else {
                t!("rules.kr_hk_001.message", event = event)
            };
            diagnostics.push(
                Diagnostic::error(path.to_path_buf(), line, column, "KR-HK-001", message)
                    .with_suggestion(t!("rules.kr_hk_001.suggestion")),
            );
        }

        if event_valid {
            if config.is_rule_enabled("KR-HK-002")
                && is_file_event(event)
                && hook
                    .patterns
                    .as_ref()
                    .is_none_or(|list| !list.malformed && !has_non_blank_entries(&list.values))
            {
                let (line, column) = event_at(&locator);
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        column,
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
                    .is_none_or(|list| !list.malformed && !has_non_blank_entries(&list.values))
            {
                let (line, column) = event_at(&locator);
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        column,
                        "KR-HK-004",
                        t!("rules.kr_hk_004.message", event = event),
                    )
                    .with_suggestion(t!("rules.kr_hk_004.suggestion")),
                );
            }
        }

        if config.is_rule_enabled("KR-HK-003") && !hook.has_action() && !hook.action_malformed {
            let (line, column) = locator.at(&[Seg::Key("then")]);
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    column,
                    "KR-HK-003",
                    t!("rules.kr_hk_003.message"),
                )
                .with_suggestion(t!("rules.kr_hk_003.suggestion")),
            );
        }

        if let Some(command) = hook.run_command.as_ref() {
            // A portability note, not a Kiro requirement: commands run in the
            // project root, so relative paths work on every machine.
            if config.is_rule_enabled("KR-HK-009") && is_absolute_command(command.value) {
                let (line, column) = locator.at(&command.path);
                diagnostics.push(
                    Diagnostic::info(
                        path.to_path_buf(),
                        line,
                        column,
                        "KR-HK-009",
                        t!("rules.kr_hk_009.message"),
                    )
                    .with_suggestion(t!("rules.kr_hk_009.suggestion")),
                );
            }
        }

        diagnostics
    }
}

/// KR-HK-009: a command that starts with `/` or a drive path.
fn is_absolute_command(command: &str) -> bool {
    let trimmed = command.trim();
    trimmed.starts_with('/')
        || (trimmed.len() >= 3
            && trimmed.as_bytes()[0].is_ascii_alphabetic()
            && trimmed.as_bytes().get(1) == Some(&b':')
            && matches!(trimmed.as_bytes().get(2), Some(b'\\') | Some(b'/')))
}

/// Token prefixes and the minimum length of the token body after them.
const TOKEN_PREFIXES: &[(&str, usize)] = &[
    ("sk-", 16),
    ("ghp_", 30),
    ("gho_", 30),
    ("github_pat_", 30),
    ("xoxb-", 10),
    ("xoxp-", 10),
    ("glpat-", 20),
];

const SECRET_ASSIGNMENT_KEYS: &[&str] = &[
    "api_key=",
    "apikey=",
    "api-key=",
    "token=",
    "password=",
    "secret=",
];

fn is_token_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
}

/// A value that the shell or Kiro resolves at run time rather than a literal.
fn is_reference(value: &str) -> bool {
    value.starts_with('$')
        || value.starts_with('%')
        || value.starts_with('<')
        || value.starts_with("{{")
}

/// KR-HK-010: literal credentials in a hook command, in either hook format.
/// Prefixes must start a word and be followed by a token-shaped body (so
/// `flask-lint` and `task-check` are not keys), and `key=value` /
/// `Bearer value` only count when the value is a literal, not a `$VAR`,
/// `${VAR}`, `$(cmd)` or `%VAR%` reference.
pub(super) fn command_has_secret(command: &str) -> bool {
    let bytes = command.as_bytes();
    let lower = command.to_ascii_lowercase();
    let word_start = |at: usize| at == 0 || !bytes[at - 1].is_ascii_alphanumeric();

    for (prefix, min_body) in TOKEN_PREFIXES {
        for (at, _) in lower.match_indices(prefix) {
            let body = bytes[at + prefix.len()..]
                .iter()
                .take_while(|b| is_token_char(**b))
                .count();
            if word_start(at) && body >= *min_body {
                return true;
            }
        }
    }
    for (at, _) in command.match_indices("AKIA") {
        let body = &bytes[at + 4..];
        if word_start(at)
            && body.len() >= 16
            && body[..16]
                .iter()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
            && body.get(16).is_none_or(|b| !b.is_ascii_alphanumeric())
        {
            return true;
        }
    }
    let literal_after = |start: usize| {
        let rest = command[start..].trim_start_matches(['"', '\'']);
        let value: String = rest
            .chars()
            .take_while(|ch| !ch.is_whitespace() && !matches!(ch, '"' | '\'' | '&' | ';' | '|'))
            .collect();
        !is_reference(&value) && super::seems_plaintext_secret(&value)
    };
    for key in SECRET_ASSIGNMENT_KEYS {
        for (at, _) in lower.match_indices(key) {
            if literal_after(at + key.len()) {
                return true;
            }
        }
    }
    lower
        .match_indices("bearer ")
        .any(|(at, _)| literal_after(at + "bearer ".len()))
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

    fn rules(diagnostics: &[Diagnostic]) -> Vec<&str> {
        diagnostics.iter().map(|d| d.rule.as_str()).collect()
    }

    /// Both validators the registry runs on a `.kiro.hook` file.
    fn validate_both(content: &str) -> Vec<Diagnostic> {
        let mut diagnostics = validate(content);
        diagnostics.extend(crate::rules::kiro_hook_v3::KiroV3HookValidator.validate(
            Path::new(".kiro/hooks/example.kiro.hook"),
            content,
            &LintConfig::default(),
        ));
        diagnostics
    }

    #[test]
    fn test_kr_hk_001_invalid_event() {
        let diagnostics = validate(include_str!(
            "../../../../tests/fixtures/kiro-hooks/.kiro/hooks/invalid-event.kiro.hook"
        ));
        assert!(diagnostics.iter().any(|d| d.rule == "KR-HK-001"));
    }

    #[test]
    fn test_kr_hk_001_points_at_when_type() {
        let diagnostics = validate(
            "{\n  \"when\": {\n    \"type\": \"userPrompt\"\n  },\n  \"then\": {\"type\": \"askAgent\", \"prompt\": \"x\"}\n}",
        );
        assert_eq!(rules(&diagnostics), ["KR-HK-001"]);
        assert_eq!((diagnostics[0].line, diagnostics[0].column), (3, 13));
        let missing = validate(r#"{"then": {"type": "askAgent", "prompt": "x"}}"#);
        assert!(
            missing[0].message.contains("no event"),
            "{}",
            missing[0].message
        );
    }

    #[test]
    fn test_kr_hk_001_undocumented_spellings_warn_with_unsafe_fix() {
        for (event, documented) in [
            ("manual", "userTriggered"),
            ("fileCreate", "fileCreated"),
            ("fileDelete", "fileDeleted"),
        ] {
            let content = serde_json::json!({"when": {"type": event, "patterns": ["src/**"]}, "then": {"type": "askAgent", "prompt": "x"}}).to_string();
            let diagnostics = validate(&content);
            assert_eq!(rules(&diagnostics), ["KR-HK-001"], "{event}");
            let d = &diagnostics[0];
            assert_eq!(d.level, crate::diagnostics::DiagnosticLevel::Warning);
            let fix = &d.fixes[0];
            assert!(!fix.is_safe());
            assert_eq!(
                &content[fix.start_byte..fix.end_byte],
                format!("\"{event}\"")
            );
            assert_eq!(fix.replacement, format!("\"{documented}\""));
        }
    }

    #[test]
    fn test_kr_hk_026_legacy_timeouts_are_seconds() {
        for (then, expected) in [
            (
                serde_json::json!({"type": "runCommand", "command": "npm test", "timeout": 30}),
                0,
            ),
            (
                serde_json::json!({"type": "runCommand", "command": "npm test", "timeout": 0}),
                0,
            ),
            (
                serde_json::json!({"type": "runCommand", "command": "npm test", "timeout": 600000}),
                0,
            ),
            (
                serde_json::json!({"type": "runCommand", "command": "npm test", "timeout": -1}),
                1,
            ),
            (
                serde_json::json!({"type": "runCommand", "command": "npm test", "timeout": "30"}),
                1,
            ),
        ] {
            let content =
                serde_json::json!({"when": {"type": "agentStop"}, "then": then}).to_string();
            let diagnostics = validate(&content);
            assert_eq!(diagnostics.len(), expected, "{then}: {diagnostics:?}");
            assert!(diagnostics.iter().all(|d| d.rule == "KR-HK-026"));
        }
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
    fn test_kr_hk_026_parse_error_reports_diagnostic() {
        // JSON syntax errors used to be reported as KR-HK-001 (event type).
        let diagnostics = validate(r#"{"event":"fileEdited","patterns":[}"#);
        assert_eq!(rules(&diagnostics), ["KR-HK-026"]);
        assert_eq!(
            diagnostics[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
    }

    #[test]
    fn test_kr_hk_026_type_errors_are_located_and_do_not_hide_other_checks() {
        // Type errors used to be a KR-HK-001 "invalid JSON structure" that
        // stopped every other check.
        let content = "{\n  \"when\": {\"type\": \"fileEdited\", \"patterns\": \"*.ts\"},\n  \"then\": {\"type\": \"runCommand\", \"command\": \"/usr/bin/lint\"}\n}";
        let diagnostics = validate(content);
        assert_eq!(rules(&diagnostics), ["KR-HK-026", "KR-HK-009"]);
        assert_eq!((diagnostics[0].line, diagnostics[0].column), (2, 46));
        assert!(diagnostics[0].message.contains("when.patterns"));
        assert_eq!(
            diagnostics[0].level,
            crate::diagnostics::DiagnosticLevel::Warning
        );
    }

    #[test]
    fn test_kr_hk_016_v1_document_in_kiro_hook_file() {
        for content in [
            r#"{"version":"v1","hooks":[{"name":"a","trigger":"Stop","action":{"type":"command","command":"echo"}}]}"#,
            r#"{"name":"a","trigger":"Stop","action":{"type":"command","command":"echo"}}"#,
        ] {
            assert_eq!(rules(&validate(content)), ["KR-HK-016"], "{content}");
        }
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
            r#"{"enabled":true,"name":"Lint","version":"1","when":{"type":"fileEdited","patterns":["src/**/*.ts"]},"then":{"type":"askAgent","prompt":"Lint"}}"#,
            r#"{"name":"Manual","version":"1","when":{"type":"userTriggered"},"then":{"type":"runCommand","command":"npm test","timeout":30}}"#,
        ];

        for fixture in fixtures {
            let diagnostics = validate(fixture);
            assert!(
                diagnostics.iter().all(|d| !d.rule.starts_with("KR-HK-")),
                "{fixture}: {diagnostics:?}"
            );
        }
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
        // A portability note with no Kiro requirement behind it.
        assert!(
            diagnostics
                .iter()
                .all(|d| d.level == crate::diagnostics::DiagnosticLevel::Info)
        );
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
    fn test_kr_hk_010_secrets_in_command() {
        for command in [
            "curl -H 'Bearer sk-secret123'",
            "curl -d api_key=mysecretkey123",
            "mysql -p password=hunter2secret",
            "OPENAI_API_KEY=sk-proj-abcdefghijklmnopqrstuv npm test",
            "git clone https://ghp_abcdefghijklmnopqrstuvwxyz0123456789@github.com/x/y",
            "aws s3 ls # AKIAIOSFODNN7EXAMPLE",
            "curl -H \"Authorization: Bearer abcdefgh12345678\" https://x",
        ] {
            assert!(command_has_secret(command), "{command}");
        }
    }

    #[test]
    fn test_kr_hk_010_no_false_positives_on_words_and_references() {
        for command in [
            "echo hello",
            "python -m flask-lint && npm run task-check",
            "npx eslint --fix && disk-usage-report --mask-sensitive-values-for-logs",
            "deploy --token=$DEPLOY_TOKEN",
            "curl --api-key=${API_KEY} https://x",
            "curl -H \"Authorization: Bearer $GITHUB_TOKEN\" https://x",
            "curl -H 'Authorization: Bearer ${TOKEN}'",
            "login --password=%PASSWORD%",
            "AKIAS_REPORT=1 make",
        ] {
            assert!(!command_has_secret(command), "{command}");
        }
        let diagnostics = validate_both(
            r#"{"when":{"type":"promptSubmit"},"then":{"type":"runCommand","command":"python -m flask-lint && npm run task-check --token=$TOKEN"}}"#,
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn test_kr_hk_010_points_at_command() {
        let content = "{\n  \"when\": {\"type\": \"promptSubmit\"},\n  \"then\": {\"type\": \"runCommand\", \"command\": \"curl -d api_key=mysecretkey123\"}\n}";
        // The V3 validator owns KR-HK-010 for both formats.
        assert!(validate(content).is_empty());
        let diagnostics = validate_both(content);
        assert_eq!(rules(&diagnostics), ["KR-HK-010"]);
        assert_eq!(diagnostics[0].line, 3);
    }

    #[test]
    fn test_kr_hk_010_reports_disabled_hooks() {
        // A committed credential leaks whether or not the hook runs.
        let diagnostics = validate_both(
            r#"{"enabled":false,"when":{"type":"agentStop"},"then":{"type":"runCommand","command":"curl -d api_key=mysecretkey123"}}"#,
        );
        assert_eq!(rules(&diagnostics), ["KR-HK-010"]);
    }

    #[test]
    fn test_metadata() {
        let validator = KiroHookValidator;
        let metadata = validator.metadata();
        assert_eq!(metadata.name, "KiroHookValidator");
        assert_eq!(metadata.rule_ids, RULE_IDS);
        // KR-HK-007 (timeout range) and KR-HK-008 (duplicate handlers) are retired.
        assert!(!metadata.rule_ids.contains(&"KR-HK-007"));
        assert!(!metadata.rule_ids.contains(&"KR-HK-008"));
    }
}
