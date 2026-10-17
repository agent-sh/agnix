//! Kiro CLI settings validation (KR-SET-001 to KR-SET-012).
//!
//! Validates Kiro CLI settings files: `~/.kiro/settings/cli.json` (global,
//! `$KIRO_HOME/settings/cli.json` when set) and `<workspace>/.kiro/settings/cli.json`.
//! Both are a flat JSON object keyed by dotted setting names, for example
//! `{"chat.defaultModel": "claude-sonnet-4.6", "toolSearch.enabled": true}`;
//! `kiro-cli settings <key> <value>` and `/settings` write them.
//!
//! Behavior observed with kiro-cli 2.32.1 (`kiro-cli settings`, offline):
//! - the file is parsed with strict serde_json: invalid JSON, comments,
//!   trailing commas, an empty file or a non-object root make every
//!   kiro-cli command, chat included, exit with "failed to parse";
//! - only flat dotted keys are read (`{"chat": {"defaultAgent": "x"}}` leaves
//!   `chat.defaultAgent` unset), a repeated key keeps its last value, and
//!   `kiro-cli settings` refuses to write unknown keys;
//! - global-only keys (for example `chat.uiMode`) are ignored in a workspace
//!   file (`settings --workspace` refuses them: "cannot be overridden at
//!   workspace level").
//!
//! The key list and value types come from `kiro-cli settings list --all`
//! 2.32.1 (140 keys). A value of the wrong type is not coerced: the Q
//! Developer CLI lineage reads settings with typed getters that return no
//! value, so Kiro falls back to the default.
//!
//! Sources: <https://kiro.dev/docs/reference/settings/>,
//! <https://kiro.dev/docs/mcp/tool-search/>.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel, Fix},
    rules::{Validator, ValidatorMetadata},
    schemas::kiro_json::{
        JsonMember, JsonNode, JsonValue, LineIndex, closest_name, parse_json_document,
    },
};
use rust_i18n::t;
use std::path::{Path, PathBuf};

const RULE_IDS: &[&str] = &[
    "KR-SET-001",
    "KR-SET-002",
    "KR-SET-003",
    "KR-SET-004",
    "KR-SET-005",
    "KR-SET-006",
    "KR-SET-007",
    "KR-SET-008",
    "KR-SET-009",
    "KR-SET-010",
    "KR-SET-011",
    "KR-SET-012",
];

/// Value type of a setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SettingKind {
    Bool,
    Number,
    String,
    StringArray,
    Object,
}

/// One Kiro CLI setting.
#[derive(Clone, Copy, Debug)]
struct SettingSpec {
    key: &'static str,
    kind: SettingKind,
    /// `kiro-cli settings --workspace <key>` refuses the key, and a workspace
    /// value is ignored.
    global_only: bool,
    /// Documented string values; empty when free-form.
    values: &'static [&'static str],
}

const fn spec(
    key: &'static str,
    kind: SettingKind,
    global_only: bool,
    values: &'static [&'static str],
) -> SettingSpec {
    SettingSpec {
        key,
        kind,
        global_only,
        values,
    }
}

use SettingKind::{Bool, Number, Object, String as Str, StringArray};

/// Every key `kiro-cli settings list --all` reports in 2.32.1, with the type
/// from its description and the global-only flag from
/// `kiro-cli settings --workspace`. String values come from the descriptions
/// and kiro.dev/docs/reference/settings.
const SETTINGS: &[SettingSpec] = &[
    spec("telemetry.enabled", Bool, true, &[]),
    spec("telemetryClientId", Str, true, &[]),
    spec(
        "codeWhisperer.shareCodeWhispererContentWithAWS",
        Bool,
        true,
        &[],
    ),
    spec("chat.enableThinking", Bool, false, &[]),
    spec("chat.enableKnowledge", Bool, false, &[]),
    spec("chat.enableCodeIntelligence", Bool, false, &[]),
    spec("knowledge.defaultIncludePatterns", StringArray, false, &[]),
    spec("knowledge.defaultExcludePatterns", StringArray, false, &[]),
    spec("knowledge.maxFiles", Number, false, &[]),
    spec("knowledge.chunkSize", Number, false, &[]),
    spec("knowledge.chunkOverlap", Number, false, &[]),
    spec("knowledge.indexType", Str, false, &[]),
    spec("chat.skimCommandKey", Str, false, &[]),
    spec("chat.autocompletionKey", Str, false, &[]),
    spec("chat.enableTangentMode", Bool, false, &[]),
    spec("chat.tangentModeKey", Str, false, &[]),
    spec("chat.enableSubagent", Bool, false, &[]),
    spec("chat.delegateModeKey", Str, false, &[]),
    spec("introspect.tangentMode", Bool, false, &[]),
    spec("introspect.progressiveMode", Bool, false, &[]),
    spec("chat.greeting.enabled", Bool, false, &[]),
    spec("api.timeout", Number, false, &[]),
    spec("api.streamIdleSoftTimeout", Number, false, &[]),
    spec("api.streamIdleHardTimeout", Number, false, &[]),
    spec("api.subagentTimeout", Number, false, &[]),
    spec("chat.editMode", Bool, false, &[]),
    spec("chat.enableNotifications", Bool, false, &[]),
    spec(
        "chat.notificationMethod",
        Str,
        false,
        &["auto", "bel", "osc9"],
    ),
    spec("api.oidc.scopePrefix", Str, true, &[]),
    spec("api.codewhisperer.service", Str, true, &[]),
    spec("api.krs.service", Object, true, &[]),
    spec("api.cps.service", Object, true, &[]),
    spec("api.q.service", Str, true, &[]),
    spec("api.kiroauth.service", Str, true, &[]),
    spec("mcp.initTimeout", Number, false, &[]),
    spec("mcp.noInteractiveTimeout", Number, false, &[]),
    spec("mcp.loadedBefore", Bool, true, &[]),
    spec("chat.enableContextUsageIndicator", Bool, false, &[]),
    spec("chat.defaultModel", Str, false, &[]),
    spec("chat.disableMarkdownRendering", Bool, false, &[]),
    spec("chat.defaultAgent", Str, false, &[]),
    spec("chat.disableAutoCompaction", Bool, false, &[]),
    spec("compaction.excludeContextWindowPercent", Number, false, &[]),
    spec("compaction.excludeMessages", Number, false, &[]),
    spec("chat.enableHistoryHints", Bool, false, &[]),
    spec("chat.enablePromptHints", Bool, false, &[]),
    spec("chat.enableTodoList", Bool, false, &[]),
    spec("chat.enableCheckpoint", Bool, false, &[]),
    spec("chat.enableDelegate", Bool, false, &[]),
    spec("chat.uiMode", Str, true, &["lite", "tui", "structured"]),
    spec("chat.diffTool", Str, false, &[]),
    spec("chat.ui", Str, true, &["legacy", "classic", "tui"]),
    spec("cleanup.periodDays", Number, true, &[]),
    spec("chat.disableGranularTrust", Bool, false, &[]),
    spec("app.disableAutoupdates", Bool, true, &[]),
    spec("hooks.showStatus", Bool, false, &[]),
    spec("chat.autoExpandToolOutput", Bool, false, &[]),
    spec("chat.disableTrustAllConfirmation", Bool, true, &[]),
    spec("toolSearch.enabled", Bool, false, &[]),
    spec("toolSearch.minPct", Number, false, &[]),
    spec("toolSearch.minTokens", Number, false, &[]),
    spec("chat.disableWrap", Bool, false, &[]),
    spec("chat.preserveScrollback", Bool, true, &[]),
    spec("chat.startFullscreen", Bool, true, &[]),
    spec("chat.fullscreenWheelRows", Number, true, &[]),
    spec("chat.fullscreenCopyOnSelect", Bool, true, &[]),
    spec("chat.modelDefaults", Object, false, &[]),
    spec("chat.disabledModelFallbacks", Object, true, &[]),
    spec("chat.spinnerVerbs", Object, true, &[]),
    spec("chat.enableCustomSpinnerVerbs", Bool, true, &[]),
    spec("chat.verbosity.lite", Object, true, &[]),
    spec("chat.verbosity.tui", Object, true, &[]),
    spec("chat.tools.filters", Object, true, &[]),
    spec("chat.tools.showReasoning", Bool, true, &[]),
    spec("chat.tools.argsMode", Str, true, &[]),
    spec("chat.tools.showElapsed", Bool, true, &[]),
    spec("chat.tools.argsMaxLines", Number, true, &[]),
    spec("chat.tools.outputMaxLines", Number, true, &[]),
    spec("chat.tools.argsMaxChars", Number, true, &[]),
    spec("chat.tools.outputMaxChars", Number, true, &[]),
    spec("chat.tools.persistOutput", Bool, true, &[]),
    spec("chat.tools.showWriteDiffs", Bool, true, &[]),
    spec("chat.showTasks", Bool, true, &[]),
    spec("chat.subagent.showPipeline", Bool, true, &[]),
    spec("chat.subagent.showPrompts", Bool, true, &[]),
    spec("chat.subagent.showRoles", Bool, true, &[]),
    spec("chat.subagent.showDeps", Bool, true, &[]),
    spec("chat.subagent.showResponses", Bool, true, &[]),
    spec("chat.statusLine.tui", Object, true, &[]),
    spec("chat.statusLine.lite", Object, true, &[]),
    spec("chat.keybindings.toggleSessionDashboard", Str, true, &[]),
    spec("chat.keybindings.moveToBackground", Str, true, &[]),
    spec("chat.sessionDashboard.groupBy", Str, true, &[]),
    spec("chat.sessionDashboard.sortBy", Str, true, &[]),
    spec(
        "chat.sessionDashboard.scope",
        Str,
        true,
        &["current", "all"],
    ),
    spec("chat.sessionDashboard.indexResponses", Bool, true, &[]),
    spec(
        "chat.modelPicker.sortBy",
        Str,
        true,
        &["last-used", "cost", "company"],
    ),
    spec("chat.modelPicker.lastUsed", Object, true, &[]),
    spec("chat.enableInfraSafetyMonitor", Bool, true, &[]),
    spec("chat.enableInfraSafetyEnforce", Bool, true, &[]),
    spec("trust.classifier.shadow", Bool, true, &[]),
    spec("trust.classifier.shadowLogPath", Str, true, &[]),
    spec("chat.workflowMonitor.mouseEnabled", Bool, true, &[]),
    spec("chat.workflow.continueInterrupted", Bool, true, &[]),
    spec("chat.specReview.mouseEnabled", Bool, true, &[]),
    spec("chat.turnUsage.thisTurn", Bool, true, &[]),
    spec("chat.turnUsage.thisSession", Bool, true, &[]),
    spec("chat.turnUsage.time", Bool, true, &[]),
    spec("chat.keybindings.cancelStream", Str, true, &[]),
    spec("chat.keybindings.closeMenu", Str, true, &[]),
    spec("chat.keybindings.quit", Str, true, &[]),
    spec("chat.agentEngine", Str, true, &["v1", "v2", "v3"]),
    spec("chat.allowAnimations", Bool, true, &[]),
    spec("chat.allowAsciiArt", Bool, true, &[]),
    spec("chat.allowIcons", Bool, true, &[]),
    spec("chat.screenReader", Str, true, &["auto", "on", "off"]),
    spec("chat.hasSeenLogo", Bool, true, &[]),
    spec("chat.showThinking", Bool, false, &[]),
    spec("chat.showThinkingTips", Bool, false, &[]),
    spec("chat.terminalTitle", Bool, true, &[]),
    spec("chat.historyMode", Str, false, &["session", "global"]),
    spec("chat.outputStyle", Str, false, &[]),
    spec(
        "chat.defaultInterruptBehavior",
        Str,
        false,
        &["steer", "queue"],
    ),
    spec("chat.keybindings.toggleInterruptBehavior", Str, false, &[]),
    spec("chat.disableInheritingDefaultResources", Bool, false, &[]),
    spec("chat.enableAutoAgentUpgrade", Bool, true, &[]),
    spec("chat.enableWorkflows", Bool, false, &[]),
    spec("chat.enableMainAgentSubagentTool", Bool, false, &[]),
    spec("chat.highPerformanceMode", Bool, true, &[]),
    spec("chat.enableC2s", Bool, true, &[]),
    spec(
        "memory.mode",
        Str,
        true,
        &["read_write", "read_only", "disabled"],
    ),
    spec("memory.reflection", Bool, true, &[]),
    spec("voice.language", Str, false, &[]),
    spec("voice.modelSize", Str, false, &[]),
    spec("voice.silenceTimeout", Number, false, &[]),
    spec("voice.partialPause", Number, false, &[]),
    spec("voice.maxSessionTime", Number, false, &[]),
    spec("voice.serverUrl", Str, true, &[]),
    spec("voice.seenWelcome", Bool, false, &[]),
    spec("voice.autoSubmit", Bool, false, &[]),
];

/// Global-only keys Kiro writes on its own (startup prompts, pickers, MCP
/// bookkeeping), never through `kiro-cli settings --workspace`.
const GLOBAL_STATE_KEYS: &[&str] = &[
    "mcp.loadedBefore",
    "chat.modelPicker.lastUsed",
    "chat.hasSeenLogo",
    "telemetryClientId",
    "chat.enableAutoAgentUpgrade",
    "chat.disabledModelFallbacks",
];

fn setting(key: &str) -> Option<&'static SettingSpec> {
    SETTINGS.iter().find(|s| s.key == key)
}

fn setting_keys() -> Vec<&'static str> {
    SETTINGS.iter().map(|s| s.key).collect()
}

/// Keys of other Kiro files that people paste into cli.json, with the file
/// Kiro reads them from.
fn misplaced_section(key: &str) -> Option<&'static str> {
    match key {
        "mcpServers" | "powers" => Some(".kiro/settings/mcp.json"),
        "hooks" => Some("a custom agent (.kiro/agents/) or .kiro/hooks/*.json"),
        "rules" | "permissions" | "policies" => Some("~/.kiro/settings/permissions.yaml"),
        "tools" | "allowedTools" | "toolsSettings" | "resources" | "prompt" | "includeMcpJson" => {
            Some("a custom agent file in .kiro/agents/")
        }
        _ => None,
    }
}

/// Undotted spellings of common settings.
fn alias(key: &str) -> Option<&'static str> {
    match key {
        "model" | "defaultModel" => Some("chat.defaultModel"),
        "agent" | "defaultAgent" => Some("chat.defaultAgent"),
        "agentEngine" | "engine" => Some("chat.agentEngine"),
        "autoCompact" | "chat.autoCompact" | "chat.autoCompaction" => {
            Some("chat.disableAutoCompaction")
        }
        "telemetry" => Some("telemetry.enabled"),
        _ => None,
    }
}

pub struct KiroSettingsValidator;

impl Validator for KiroSettingsValidator {
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
        let mut cx = Cx {
            path,
            lines: LineIndex::new(content),
            config,
            diagnostics: Vec::new(),
        };
        // `.kiro/settings.json` is where Kiro CLI 1.23 saved the default model;
        // current Kiro CLI does not read it. Many such files belong to other
        // tools, so only one carrying Kiro settings is reported (KR-SET-012).
        let legacy_file = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case("settings.json"));
        if legacy_file {
            let doc = parse_json_document(content, false).ok();
            let first_setting = doc.as_ref().and_then(|doc| {
                doc.root
                    .value
                    .as_object()?
                    .iter()
                    .find(|m| setting(&m.key).is_some())
                    .map(|m| (m.key_start, m.key_end))
            });
            let Some((start, end)) = first_setting else {
                return cx.diagnostics;
            };
            if cx.enabled("KR-SET-012") {
                cx.push(
                    DiagnosticLevel::Warning,
                    "KR-SET-012",
                    start,
                    end,
                    t!("rules.kr_set_012.message").to_string(),
                    t!("rules.kr_set_012.suggestion").to_string(),
                );
            }
        }
        let doc = match parse_json_document(content, false) {
            Ok(doc) => doc,
            Err(error) => {
                if cx.enabled("KR-SET-004") {
                    let offset = offset_of(content, error.line, error.column);
                    cx.push(
                        DiagnosticLevel::Error,
                        "KR-SET-004",
                        offset,
                        offset,
                        t!("rules.kr_set_004.message", error = error.message.as_str()).to_string(),
                        t!("rules.kr_set_004.suggestion").to_string(),
                    );
                }
                return cx.diagnostics;
            }
        };
        let Some(members) = doc.root.value.as_object() else {
            if cx.enabled("KR-SET-004") {
                cx.push(
                    DiagnosticLevel::Error,
                    "KR-SET-004",
                    doc.root.start,
                    doc.root.end,
                    t!("rules.kr_set_004.root", found = doc.root.value.type_name()).to_string(),
                    t!("rules.kr_set_004.suggestion").to_string(),
                );
            }
            return cx.diagnostics;
        };
        // Keys only Kiro itself writes, and only to the global file: a file
        // carrying them is a copy of ~/.kiro/settings/cli.json (for example in
        // a dotfiles repository), not a workspace file.
        let global_copy = members
            .iter()
            .any(|m| GLOBAL_STATE_KEYS.contains(&m.key.as_str()));
        let workspace_file = !global_copy && is_workspace_settings_file(path) == Some(true);
        for (index, member) in members.iter().enumerate() {
            // A repeated key: Kiro (serde_json) keeps the last value.
            if let Some(later) = members[index + 1..].iter().find(|m| m.key == member.key) {
                if cx.enabled("KR-SET-010") {
                    let (later_line, _) = cx.lines.line_col(later.key_start);
                    cx.push(
                        DiagnosticLevel::Warning,
                        "KR-SET-010",
                        member.key_start,
                        member.key_end,
                        t!(
                            "rules.kr_set_010.message",
                            key = member.key.as_str(),
                            line = later_line
                        )
                        .to_string(),
                        t!("rules.kr_set_010.suggestion").to_string(),
                    );
                }
                continue;
            }
            validate_member(&mut cx, content, member, workspace_file);
        }
        cx.diagnostics
    }
}

struct Cx<'a> {
    path: &'a Path,
    lines: LineIndex<'a>,
    config: &'a PerFileLintConfig<'a>,
    diagnostics: Vec<Diagnostic>,
}

impl Cx<'_> {
    fn enabled(&self, rule: &str) -> bool {
        self.config.is_rule_enabled(rule)
    }

    fn diagnostic(
        &self,
        level: DiagnosticLevel,
        rule: &str,
        start: usize,
        end: usize,
        message: String,
        suggestion: String,
    ) -> Diagnostic {
        let (line, column) = self.lines.line_col(start);
        let (end_line, end_column) = self.lines.line_col(end.max(start));
        let path = self.path.to_path_buf();
        let diagnostic = match level {
            DiagnosticLevel::Error => Diagnostic::error(path, line, column, rule, message),
            DiagnosticLevel::Warning => Diagnostic::warning(path, line, column, rule, message),
            DiagnosticLevel::Info => Diagnostic::info(path, line, column, rule, message),
        };
        diagnostic
            .with_span(end_line, end_column)
            .with_suggestion(suggestion)
    }

    fn push(
        &mut self,
        level: DiagnosticLevel,
        rule: &str,
        start: usize,
        end: usize,
        message: String,
        suggestion: String,
    ) {
        let diagnostic = self.diagnostic(level, rule, start, end, message, suggestion);
        self.diagnostics.push(diagnostic);
    }
}

/// Byte offset of a 1-based (line, column) position, clamped to the text.
fn offset_of(content: &str, line: usize, column: usize) -> usize {
    let mut offset = 0;
    for (index, text) in content.split_inclusive('\n').enumerate() {
        if index + 1 == line {
            let col_offset = text
                .char_indices()
                .nth(column.saturating_sub(1))
                .map_or(text.len(), |(i, _)| i);
            return (offset + col_offset).min(content.len());
        }
        offset += text.len();
    }
    content.len()
}

fn validate_member(cx: &mut Cx<'_>, content: &str, member: &JsonMember, workspace_file: bool) {
    let key = member.key.as_str();
    let Some(spec) = setting(key) else {
        report_unknown_key(cx, member);
        return;
    };
    if workspace_file && spec.global_only && cx.enabled("KR-SET-011") {
        cx.push(
            DiagnosticLevel::Info,
            "KR-SET-011",
            member.key_start,
            member.key_end,
            t!("rules.kr_set_011.message", key = key).to_string(),
            t!("rules.kr_set_011.suggestion").to_string(),
        );
    }
    match key {
        "toolSearch.enabled" => validate_tool_search_enabled(cx, content, member),
        "toolSearch.minPct" => validate_tool_search_min_pct(cx, content, member),
        "toolSearch.minTokens" => validate_tool_search_min_tokens(cx, content, member),
        _ => validate_value(cx, content, member, spec),
    }
}

fn report_unknown_key(cx: &mut Cx<'_>, member: &JsonMember) {
    let key = member.key.as_str();
    if let Some(home) = misplaced_section(key) {
        if cx.enabled("KR-SET-006") {
            cx.push(
                DiagnosticLevel::Warning,
                "KR-SET-006",
                member.key_start,
                member.key_end,
                t!("rules.kr_set_006.message", key = key, file = home).to_string(),
                t!("rules.kr_set_006.suggestion", file = home).to_string(),
            );
        }
        return;
    }
    // A nested object whose dotted paths are settings: Kiro reads only the
    // flat dotted keys, so the nested values are ignored.
    if let Some(members) = member.value.value.as_object() {
        let mut nested = Vec::new();
        collect_nested_settings(key, members, &mut nested);
        if !nested.is_empty() {
            if cx.enabled("KR-SET-007") {
                let list = nested.join(", ");
                cx.push(
                    DiagnosticLevel::Warning,
                    "KR-SET-007",
                    member.key_start,
                    member.key_end,
                    t!(
                        "rules.kr_set_007.message",
                        key = key,
                        settings = list.as_str()
                    )
                    .to_string(),
                    t!("rules.kr_set_007.suggestion", example = nested[0].as_str()).to_string(),
                );
            }
            return;
        }
    }
    if !cx.enabled("KR-SET-005") {
        return;
    }
    let keys = setting_keys();
    let suggestion = match alias(key).or_else(|| closest_name(key, &keys)) {
        Some(close) => t!("rules.kr_set_005.did_you_mean", key = close).to_string(),
        None => t!("rules.kr_set_005.suggestion").to_string(),
    };
    cx.push(
        DiagnosticLevel::Info,
        "KR-SET-005",
        member.key_start,
        member.key_end,
        t!("rules.kr_set_005.message", key = key).to_string(),
        suggestion,
    );
}

fn collect_nested_settings(prefix: &str, members: &[JsonMember], out: &mut Vec<String>) {
    for member in members {
        let dotted = format!("{prefix}.{}", member.key);
        if setting(&dotted).is_some() {
            out.push(dotted);
        } else if let Some(children) = member.value.value.as_object() {
            collect_nested_settings(&dotted, children, out);
        }
    }
}

fn kind_name(kind: SettingKind) -> &'static str {
    match kind {
        Bool => "a boolean (true or false)",
        Number => "a number",
        Str => "a string",
        StringArray => "an array of strings",
        Object => "an object",
    }
}

fn kind_matches(kind: SettingKind, value: &JsonValue) -> bool {
    match kind {
        Bool => value.as_bool().is_some(),
        Number => value.as_number().is_some(),
        Str => value.as_str().is_some(),
        StringArray => value
            .as_array()
            .is_some_and(|items| items.iter().all(|i| i.value.as_str().is_some())),
        Object => value.as_object().is_some(),
    }
}

/// KR-SET-008 / KR-SET-009 for every setting except toolSearch.*.
fn validate_value(cx: &mut Cx<'_>, content: &str, member: &JsonMember, spec: &SettingSpec) {
    let node = &member.value;
    if !kind_matches(spec.kind, &node.value) {
        if !cx.enabled("KR-SET-008") {
            return;
        }
        let mut diagnostic = cx.diagnostic(
            DiagnosticLevel::Warning,
            "KR-SET-008",
            node.start,
            node.end,
            t!(
                "rules.kr_set_008.message",
                key = spec.key,
                expected = kind_name(spec.kind),
                found = node.value.type_name()
            )
            .to_string(),
            t!("rules.kr_set_008.suggestion").to_string(),
        );
        let fix = match spec.kind {
            Bool => quoted_bool_fix(content, node),
            // Integers only: most numeric settings are counts or seconds,
            // and a fractional value would be ignored again.
            Number => quoted_number_fix(content, node, true),
            _ => None,
        };
        if let Some(fix) = fix {
            diagnostic = diagnostic.with_fix(fix);
        }
        cx.diagnostics.push(diagnostic);
        return;
    }
    if !cx.enabled("KR-SET-009") {
        return;
    }
    let invalid = match (&node.value, spec.key) {
        (JsonValue::String(value), _) if !spec.values.is_empty() => {
            (!spec.values.contains(&value.as_str())).then(|| spec.values.join(", "))
        }
        (JsonValue::Number(n), "compaction.excludeContextWindowPercent") => n
            .as_f64()
            .is_none_or(|v| !(0.0..=100.0).contains(&v))
            .then(|| "a number from 0 to 100".to_string()),
        (JsonValue::Number(n), "chat.fullscreenWheelRows") => {
            (!matches!(n.as_u64(), Some(1..=3))).then(|| "1, 2, 3".to_string())
        }
        _ => None,
    };
    if let Some(allowed) = invalid {
        cx.push(
            DiagnosticLevel::Warning,
            "KR-SET-009",
            node.start,
            node.end,
            t!(
                "rules.kr_set_009.message",
                key = spec.key,
                value = content.get(node.start..node.end).unwrap_or_default(),
                allowed = allowed.as_str()
            )
            .to_string(),
            t!("rules.kr_set_009.suggestion").to_string(),
        );
    }
}

/// Unsafe fix for `"true"` / `"false"` (any case): replace the exact string
/// token with a JSON boolean. Unsafe because Kiro ignores the quoted value
/// today, so converting it activates the setting.
fn quoted_bool_fix(content: &str, node: &JsonNode) -> Option<Fix> {
    let text = node.value.as_str()?;
    let parsed = match text.to_ascii_lowercase().as_str() {
        "true" => true,
        "false" => false,
        _ => return None,
    };
    let token = content.get(node.start..node.end)?;
    Some(Fix::replace(
        node.start,
        node.end,
        if parsed { "true" } else { "false" },
        t!("rules.boolean_unwrap.json_fix", value = token).to_string(),
        false,
    ))
}

/// Unsafe fix for a quoted plain decimal (`"5"`, `"2.5"`; `"50000"` when
/// `integer`): the replacement is the same digits as a JSON number, so the
/// file still parses. Exponents, signs, leading zeros and values a JSON
/// number cannot hold exactly are left for a manual fix.
fn quoted_number_fix(content: &str, node: &JsonNode, integer: bool) -> Option<Fix> {
    let text = node.value.as_str()?;
    if !is_plain_decimal(text, integer) {
        return None;
    }
    // Stay within the range both serde_json and JavaScript read exactly.
    let value: f64 = text.parse().ok()?;
    if !(value.is_finite() && value <= 9_007_199_254_740_991.0) {
        return None;
    }
    let token = content.get(node.start..node.end)?;
    Some(Fix::replace(
        node.start,
        node.end,
        text.to_string(),
        t!("rules.kr_set_008.number_fix", value = token).to_string(),
        false,
    ))
}

/// `0`, `42`, `2.5` (no sign, exponent, leading zero or bare dot).
fn is_plain_decimal(text: &str, integer: bool) -> bool {
    let (int, frac) = match text.split_once('.') {
        Some(_) if integer => return false,
        Some((int, frac)) => (int, Some(frac)),
        None => (text, None),
    };
    let int_ok = !int.is_empty()
        && int.bytes().all(|b| b.is_ascii_digit())
        && (int == "0" || !int.starts_with('0'));
    let frac_ok = frac.is_none_or(|f| !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()));
    int_ok && frac_ok
}

/// KR-SET-001: `toolSearch.enabled` must be a boolean.
fn validate_tool_search_enabled(cx: &mut Cx<'_>, content: &str, member: &JsonMember) {
    let node = &member.value;
    if node.value.as_bool().is_some() || !cx.enabled("KR-SET-001") {
        return;
    }
    let mut diagnostic = cx.diagnostic(
        DiagnosticLevel::Warning,
        "KR-SET-001",
        node.start,
        node.end,
        t!("rules.kr_set_001.type_error").to_string(),
        t!("rules.kr_set_001.suggestion").to_string(),
    );
    if let Some(fix) = quoted_bool_fix(content, node) {
        diagnostic = diagnostic.with_fix(fix);
    }
    cx.diagnostics.push(diagnostic);
}

/// KR-SET-002: `toolSearch.minPct` must be a number from 0 to 100.
fn validate_tool_search_min_pct(cx: &mut Cx<'_>, content: &str, member: &JsonMember) {
    if !cx.enabled("KR-SET-002") {
        return;
    }
    let node = &member.value;
    let (message, fix) = match node.value.as_number().and_then(serde_json::Number::as_f64) {
        None => (
            t!("rules.kr_set_002.type_error"),
            quoted_number_fix(content, node, false),
        ),
        Some(n) if n < 0.0 => (t!("rules.kr_set_002.negative"), None),
        Some(n) if n > 100.0 => (t!("rules.kr_set_002.over_100"), None),
        Some(_) => return,
    };
    let mut diagnostic = cx.diagnostic(
        DiagnosticLevel::Warning,
        "KR-SET-002",
        node.start,
        node.end,
        message.to_string(),
        t!("rules.kr_set_002.suggestion").to_string(),
    );
    if let Some(fix) = fix {
        diagnostic = diagnostic.with_fix(fix);
    }
    cx.diagnostics.push(diagnostic);
}

/// KR-SET-003: `toolSearch.minTokens` must be a non-negative integer.
fn validate_tool_search_min_tokens(cx: &mut Cx<'_>, content: &str, member: &JsonMember) {
    if !cx.enabled("KR-SET-003") {
        return;
    }
    let node = &member.value;
    let (message, fix) = match node.value.as_number() {
        None => (
            t!("rules.kr_set_003.type_error"),
            quoted_number_fix(content, node, true),
        ),
        Some(n) if n.as_f64().is_some_and(|v| v < 0.0) => (t!("rules.kr_set_003.negative"), None),
        Some(n) if n.as_u64().is_none() => (t!("rules.kr_set_003.not_integer"), None),
        Some(_) => return,
    };
    let mut diagnostic = cx.diagnostic(
        DiagnosticLevel::Warning,
        "KR-SET-003",
        node.start,
        node.end,
        message.to_string(),
        t!("rules.kr_set_003.suggestion").to_string(),
    );
    if let Some(fix) = fix {
        diagnostic = diagnostic.with_fix(fix);
    }
    cx.diagnostics.push(diagnostic);
}

/// Whether `path` is a workspace `.kiro/settings/cli.json` rather than the
/// global one (`~/.kiro/settings/cli.json` or `$KIRO_HOME/settings/cli.json`).
/// `None` when it cannot be told (no home directory available).
fn is_workspace_settings_file(path: &Path) -> Option<bool> {
    let kiro_home = std::env::var_os("KIRO_HOME").map(PathBuf::from);
    workspace_settings_decision(path, user_home().as_deref(), kiro_home.as_deref())
}

#[cfg(feature = "filesystem")]
fn user_home() -> Option<PathBuf> {
    dirs::home_dir()
}

#[cfg(not(feature = "filesystem"))]
fn user_home() -> Option<PathBuf> {
    None
}

fn workspace_settings_decision(
    path: &Path,
    home: Option<&Path>,
    kiro_home: Option<&Path>,
) -> Option<bool> {
    let settings_dir = path.parent()?;
    let config_dir = settings_dir.parent()?;
    let absolute = std::path::absolute(config_dir).ok()?;
    // KIRO_HOME counts only when absolute, not "/", and not $HOME or one of
    // its ancestors (otherwise Kiro ignores it and uses ~/.kiro).
    let kiro_home = kiro_home.filter(|k| {
        k.is_absolute() && k.parent().is_some() && home.is_none_or(|h| !h.starts_with(k))
    });
    let global_dirs: Vec<PathBuf> = kiro_home
        .map(Path::to_path_buf)
        .into_iter()
        .chain(home.map(|h| h.join(".kiro")))
        .collect();
    if global_dirs.is_empty() {
        return None;
    }
    Some(!global_dirs.contains(&absolute))
}

#[cfg(test)]
mod tests;
