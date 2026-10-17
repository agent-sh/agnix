//! Kiro configuration files that no Kiro engine loads (KIRO-026).
//!
//! File-type detection routes near misses in Kiro configuration locations to
//! `FileType::KiroNotLoaded`: a wrong-case or unsupported extension where Kiro
//! lists a directory (`.kiro/agents/reviewer.JSON`, `.kiro/hooks/lint.yaml`),
//! a settings file name Kiro does not open (`permissions.yml`,
//! ` permissions.yaml`), a settings file placed in `.kiro/` instead of
//! `.kiro/settings/`, a hook document in a subdirectory of `.kiro/hooks/`
//! (hook files are not discovered recursively), and `.kiro/settings.json`,
//! which Kiro CLI does not read (its settings live in `.kiro/settings/cli.json`).
//! The last two are reported only when the content shows the intent: a hook
//! document, or a Kiro CLI setting key.

use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    file_types::{KiroNotLoadedReason, kiro_not_loaded_reason},
    rules::{Validator, ValidatorMetadata},
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["KIRO-026"];

/// Setting keys Kiro CLI 2.32 accepts in `settings/cli.json`
/// (`kiro-cli settings list --all`).
pub(crate) const KIRO_CLI_SETTING_KEYS: &[&str] = &[
    "api.codewhisperer.service",
    "api.cps.service",
    "api.kiroauth.service",
    "api.krs.service",
    "api.oidc.scopePrefix",
    "api.q.service",
    "api.streamIdleHardTimeout",
    "api.streamIdleSoftTimeout",
    "api.subagentTimeout",
    "api.timeout",
    "app.disableAutoupdates",
    "chat.agentEngine",
    "chat.allowAnimations",
    "chat.allowAsciiArt",
    "chat.allowIcons",
    "chat.autoExpandToolOutput",
    "chat.autocompletionKey",
    "chat.defaultAgent",
    "chat.defaultInterruptBehavior",
    "chat.defaultModel",
    "chat.delegateModeKey",
    "chat.diffTool",
    "chat.disableAutoCompaction",
    "chat.disableGranularTrust",
    "chat.disableInheritingDefaultResources",
    "chat.disableMarkdownRendering",
    "chat.disableTrustAllConfirmation",
    "chat.disableWrap",
    "chat.disabledModelFallbacks",
    "chat.editMode",
    "chat.enableAutoAgentUpgrade",
    "chat.enableC2s",
    "chat.enableCheckpoint",
    "chat.enableCodeIntelligence",
    "chat.enableContextUsageIndicator",
    "chat.enableCustomSpinnerVerbs",
    "chat.enableDelegate",
    "chat.enableHistoryHints",
    "chat.enableInfraSafetyEnforce",
    "chat.enableInfraSafetyMonitor",
    "chat.enableKnowledge",
    "chat.enableMainAgentSubagentTool",
    "chat.enableNotifications",
    "chat.enablePromptHints",
    "chat.enableSubagent",
    "chat.enableTangentMode",
    "chat.enableThinking",
    "chat.enableTodoList",
    "chat.enableWorkflows",
    "chat.fullscreenCopyOnSelect",
    "chat.fullscreenWheelRows",
    "chat.greeting.enabled",
    "chat.hasSeenLogo",
    "chat.highPerformanceMode",
    "chat.historyMode",
    "chat.keybindings.cancelStream",
    "chat.keybindings.closeMenu",
    "chat.keybindings.moveToBackground",
    "chat.keybindings.quit",
    "chat.keybindings.toggleInterruptBehavior",
    "chat.keybindings.toggleSessionDashboard",
    "chat.modelDefaults",
    "chat.modelPicker.lastUsed",
    "chat.modelPicker.sortBy",
    "chat.notificationMethod",
    "chat.outputStyle",
    "chat.preserveScrollback",
    "chat.screenReader",
    "chat.sessionDashboard.groupBy",
    "chat.sessionDashboard.indexResponses",
    "chat.sessionDashboard.scope",
    "chat.sessionDashboard.sortBy",
    "chat.showTasks",
    "chat.showThinking",
    "chat.showThinkingTips",
    "chat.skimCommandKey",
    "chat.specReview.mouseEnabled",
    "chat.spinnerVerbs",
    "chat.startFullscreen",
    "chat.statusLine.lite",
    "chat.statusLine.tui",
    "chat.subagent.showDeps",
    "chat.subagent.showPipeline",
    "chat.subagent.showPrompts",
    "chat.subagent.showResponses",
    "chat.subagent.showRoles",
    "chat.tangentModeKey",
    "chat.terminalTitle",
    "chat.tools.argsMaxChars",
    "chat.tools.argsMaxLines",
    "chat.tools.argsMode",
    "chat.tools.filters",
    "chat.tools.outputMaxChars",
    "chat.tools.outputMaxLines",
    "chat.tools.persistOutput",
    "chat.tools.showElapsed",
    "chat.tools.showReasoning",
    "chat.tools.showWriteDiffs",
    "chat.turnUsage.thisSession",
    "chat.turnUsage.thisTurn",
    "chat.turnUsage.time",
    "chat.ui",
    "chat.uiMode",
    "chat.verbosity.lite",
    "chat.verbosity.tui",
    "chat.workflow.continueInterrupted",
    "chat.workflowMonitor.mouseEnabled",
    "cleanup.periodDays",
    "codeWhisperer.shareCodeWhispererContentWithAWS",
    "compaction.excludeContextWindowPercent",
    "compaction.excludeMessages",
    "hooks.showStatus",
    "introspect.progressiveMode",
    "introspect.tangentMode",
    "knowledge.chunkOverlap",
    "knowledge.chunkSize",
    "knowledge.defaultExcludePatterns",
    "knowledge.defaultIncludePatterns",
    "knowledge.indexType",
    "knowledge.maxFiles",
    "mcp.initTimeout",
    "mcp.loadedBefore",
    "mcp.noInteractiveTimeout",
    "memory.mode",
    "memory.reflection",
    "telemetry.enabled",
    "telemetryClientId",
    "toolSearch.enabled",
    "toolSearch.minPct",
    "toolSearch.minTokens",
    "trust.classifier.shadow",
    "trust.classifier.shadowLogPath",
    "voice.autoSubmit",
    "voice.language",
    "voice.maxSessionTime",
    "voice.modelSize",
    "voice.partialPause",
    "voice.seenWelcome",
    "voice.serverUrl",
    "voice.silenceTimeout",
];

pub struct KiroNotLoadedValidator;

impl Validator for KiroNotLoadedValidator {
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
        if !config.is_rule_enabled("KIRO-026") {
            return Vec::new();
        }
        let Some(reason) = kiro_not_loaded_reason(path) else {
            return Vec::new();
        };
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let (line, column, message, suggestion) = match reason {
            KiroNotLoadedReason::ExtensionCase { expected, dir } => (
                1,
                0,
                t!(
                    "rules.kiro_026.extension_case",
                    expected = expected,
                    dir = dir,
                    name = name
                ),
                t!(
                    "rules.kiro_026.rename_suggestion",
                    fixed = replace_extension(name, expected)
                ),
            ),
            KiroNotLoadedReason::UnsupportedExtension { expected, dir } => {
                let fixed = replace_extension(name, expected);
                let yaml = extension(name).is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml")
                });
                let suggestion = if yaml {
                    t!("rules.kiro_026.convert_suggestion", fixed = fixed)
                } else {
                    t!("rules.kiro_026.rename_suggestion", fixed = fixed)
                };
                (
                    1,
                    0,
                    t!(
                        "rules.kiro_026.unsupported_extension",
                        expected = expected,
                        dir = dir,
                        name = name
                    ),
                    suggestion,
                )
            }
            KiroNotLoadedReason::NestedHookFile => {
                if !is_hook_document(content) {
                    return Vec::new();
                }
                (
                    1,
                    0,
                    t!("rules.kiro_026.nested_hook"),
                    t!("rules.kiro_026.nested_hook_suggestion"),
                )
            }
            KiroNotLoadedReason::Misnamed { expected } => (
                1,
                0,
                t!("rules.kiro_026.misnamed", expected = expected, name = name),
                t!("rules.kiro_026.rename_suggestion", fixed = expected),
            ),
            KiroNotLoadedReason::Misplaced { expected } => (
                1,
                0,
                t!("rules.kiro_026.misplaced", expected = expected, name = name),
                t!("rules.kiro_026.move_suggestion", expected = expected),
            ),
            KiroNotLoadedReason::LegacySettings => {
                let Some((key, line, column)) = first_cli_setting_key(content) else {
                    return Vec::new();
                };
                (
                    line,
                    column,
                    t!("rules.kiro_026.legacy_settings", key = key),
                    t!("rules.kiro_026.legacy_settings_suggestion"),
                )
            }
        };

        vec![
            Diagnostic::warning(path.to_path_buf(), line, column, "KIRO-026", message)
                .with_suggestion(suggestion),
        ]
    }
}

fn extension(name: &str) -> Option<&str> {
    name.rsplit_once('.').map(|(_, ext)| ext)
}

/// `reviewer.JSON` -> `reviewer.json`, `lint.yaml` -> `lint.json`.
fn replace_extension(name: &str, expected: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => format!("{stem}{expected}"),
        _ => format!("{name}{expected}"),
    }
}

/// A V3 hook document (`{"hooks": [...]}`) or a legacy IDE hook saved as JSON
/// (`when` and `then`).
fn is_hook_document(content: &str) -> bool {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(content)
    else {
        return false;
    };
    map.get("hooks").is_some_and(serde_json::Value::is_array)
        || (map.contains_key("when") && map.contains_key("then"))
}

/// The first top-level key of a JSON object that is a Kiro CLI setting, with
/// its 1-based line and column.
fn first_cli_setting_key(content: &str) -> Option<(String, usize, usize)> {
    let serde_json::Value::Object(map) = serde_json::from_str::<serde_json::Value>(content).ok()?
    else {
        return None;
    };
    let key = map
        .keys()
        .find(|key| KIRO_CLI_SETTING_KEYS.contains(&key.as_str()))?
        .clone();
    let (line, column) = top_level_key_position(content, &key).unwrap_or((1, 0));
    Some((key, line, column))
}

/// 1-based position of a key at nesting depth 1 of a JSON object, skipping
/// strings and nested values, so a nested key with the same name is never
/// chosen.
fn top_level_key_position(content: &str, key: &str) -> Option<(usize, usize)> {
    let bytes = content.as_bytes();
    let mut depth = 0usize;
    let mut i = 0;
    let mut expect_key = false;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                depth += 1;
                expect_key = depth == 1;
            }
            b'[' => depth += 1,
            b'}' | b']' => depth = depth.saturating_sub(1),
            b',' => expect_key = depth == 1,
            b'"' => {
                let start = i;
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                if expect_key && depth == 1 {
                    let raw = content.get(start + 1..i)?;
                    let decoded: Option<String> = serde_json::from_str(&format!("\"{raw}\"")).ok();
                    if decoded.as_deref() == Some(key) {
                        return Some(crate::rules::line_col_at_offset(content, start));
                    }
                }
                expect_key = false;
            }
            _ => {}
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;

    fn validate(path: &str, content: &str) -> Vec<Diagnostic> {
        let config = LintConfig::default();
        KiroNotLoadedValidator.validate_per_file(
            Path::new(path),
            content,
            &config.for_path(Path::new(path)),
        )
    }

    fn only(diagnostics: &[Diagnostic]) -> &Diagnostic {
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        &diagnostics[0]
    }

    #[test]
    fn wrong_case_agent_extension_is_reported_with_the_loaded_name() {
        let diagnostics = validate(".kiro/agents/reviewer.JSON", "{\"name\": \"r\"}");
        let diagnostic = only(&diagnostics);
        assert_eq!(diagnostic.rule, "KIRO-026");
        assert_eq!(
            diagnostic.level,
            crate::diagnostics::DiagnosticLevel::Warning
        );
        assert!(
            diagnostic.message.contains("'.json'"),
            "{}",
            diagnostic.message
        );
        assert!(
            diagnostic.message.contains(".kiro/agents"),
            "{}",
            diagnostic.message
        );
        assert!(
            diagnostic
                .suggestion
                .as_deref()
                .is_some_and(|s| s.contains("reviewer.json")),
            "{:?}",
            diagnostic.suggestion
        );
    }

    #[test]
    fn wrong_case_steering_and_hook_extensions_are_reported() {
        only(&validate(".kiro/steering/Style.MD", "# Style\n"));
        only(&validate(".kiro/hooks/lint.Json", "{}"));
        only(&validate(
            "/home/u/.kiro/agents/team/planner.Md",
            "---\n---\n",
        ));
    }

    #[test]
    fn yaml_hook_file_gets_a_convert_suggestion() {
        let diagnostics = validate(".kiro/hooks/lint.yaml", "version: v1\n");
        let diagnostic = only(&diagnostics);
        let suggestion = diagnostic.suggestion.as_deref().unwrap();
        assert!(suggestion.contains("lint.json"), "{suggestion}");
        assert!(suggestion.contains("JSON"), "{suggestion}");
        let jsonc = validate(".kiro/agents/reviewer.jsonc", "{}");
        assert!(
            only(&jsonc)
                .suggestion
                .as_deref()
                .is_some_and(|s| s.contains("reviewer.json") && !s.contains("Convert"))
        );
    }

    #[test]
    fn misnamed_and_misplaced_settings_files_are_reported() {
        for (path, expected) in [
            (".kiro/settings/permissions.yml", "permissions.yaml"),
            (".kiro/settings/ permissions.yaml", "permissions.yaml"),
            (".kiro/settings/mcp.json ", "mcp.json"),
            (".kiro/settings/mcp.jsonc", "mcp.json"),
            (
                "/home/u/.kiro/workspace-roots/abc/permissions.yml",
                "permissions.yaml",
            ),
            (".kiro/mcp.json", ".kiro/settings/mcp.json"),
            (".kiro/cli.json", ".kiro/settings/cli.json"),
            (
                ".kiro/permissions.yaml",
                "~/.kiro/settings/permissions.yaml",
            ),
        ] {
            let diagnostics = validate(path, "{}");
            let diagnostic = only(&diagnostics);
            assert!(
                diagnostic.message.contains(expected),
                "{path}: {}",
                diagnostic.message
            );
        }
    }

    #[test]
    fn nested_hook_file_is_reported_only_for_hook_documents() {
        let hook = r#"{"version":"v1","hooks":[{"name":"a","trigger":"Stop","action":{"type":"command","command":"x"}}]}"#;
        only(&validate(".kiro/hooks/team/lint.json", hook));
        only(&validate(
            ".kiro/hooks/legacy/save.json",
            r#"{"when":{"type":"fileEdited"},"then":{"type":"askAgent","prompt":"x"}}"#,
        ));
        assert!(
            validate(
                ".kiro/hooks/scripts/package.json",
                r#"{"name":"x","version":"1.0.0"}"#
            )
            .is_empty()
        );
        assert!(validate(".kiro/hooks/scripts/broken.json", "{").is_empty());
    }

    #[test]
    fn legacy_settings_json_is_reported_only_with_cli_setting_keys() {
        let content = "{\n  \"editor\": {\"chat.defaultModel\": \"nested\"},\n  \"chat.defaultModel\": \"claude-sonnet-4.6\"\n}\n";
        let diagnostics = validate(".kiro/settings.json", content);
        let diagnostic = only(&diagnostics);
        assert!(diagnostic.message.contains("chat.defaultModel"));
        // Points at the top-level key, not the nested copy on line 2.
        assert_eq!((diagnostic.line, diagnostic.column), (3, 3));

        // Editor preferences and Claude Code keys are not Kiro CLI settings.
        assert!(validate(".kiro/settings.json", r#"{"editor": {"tabSize": 2}}"#).is_empty());
        assert!(
            validate(
                ".kiro/settings.json",
                r#"{"statusLine": {"type": "command"}}"#
            )
            .is_empty()
        );
        assert!(validate(".kiro/settings.json", "not json").is_empty());
    }

    #[test]
    fn loaded_files_are_not_reported() {
        for path in [
            ".kiro/agents/reviewer.json",
            ".kiro/agents/reviewer.md",
            ".kiro/hooks/lint.json",
            ".kiro/steering/style.md",
            ".kiro/settings/permissions.yaml",
            ".kiro/settings/cli.json",
        ] {
            assert!(validate(path, "{}").is_empty(), "{path}");
        }
    }

    #[test]
    fn rule_can_be_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KIRO-026".to_string()];
        let path = Path::new(".kiro/agents/reviewer.JSON");
        assert!(
            KiroNotLoadedValidator
                .validate_per_file(path, "{}", &config.for_path(path))
                .is_empty()
        );
        config.rules_mut().disabled_rules.clear();
        config.rules_mut().kiro_steering = false;
        assert!(
            KiroNotLoadedValidator
                .validate_per_file(path, "{}", &config.for_path(path))
                .is_empty()
        );
    }

    #[test]
    fn cli_setting_keys_are_unique_and_sorted() {
        let mut sorted = KIRO_CLI_SETTING_KEYS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, KIRO_CLI_SETTING_KEYS);
        assert_eq!(KIRO_CLI_SETTING_KEYS.len(), 140);
    }
}
