//! Codex CLI lifecycle hooks (CDX-HK-000 to CDX-HK-005).
//!
//! Codex reads hooks from `hooks.json` next to each config layer
//! (`.codex/hooks.json`, `~/.codex/hooks.json`), from inline `[hooks]` tables
//! in `config.toml` and `requirements.toml`, and from a plugin's default
//! `hooks/hooks.json`. The checks follow the Codex loader
//! (openai/codex `codex-rs/config/src/hook_config.rs` and
//! `codex-rs/hooks/src/engine/discovery.rs`):
//!
//! - `hooks.json` is `HooksFile` with `deny_unknown_fields`: any top-level key
//!   other than `description` and `hooks`, or any value of the wrong type,
//!   fails deserialization and Codex skips every hook in the file
//!   (CDX-HK-000). A bad inline `[hooks]` table drops all inline hooks of
//!   that layer the same way.
//! - Event names are fixed struct fields without `deny_unknown_fields`, so an
//!   unknown event (`Notification`) is ignored without a warning (CDX-HK-005).
//! - `SessionEnd` and `Interrupt` timeouts are clamped to 1..=3 seconds, and
//!   a timeout of 0 becomes 1 second for every event (CDX-HK-001).
//! - `prompt` and `agent` handlers, `mcp_tool` handlers on `SessionEnd`,
//!   blank commands, blank MCP server or tool names, and matcher groups whose
//!   regex does not compile are skipped at load (CDX-HK-002).
//! - `additionalContextLimit` is ignored (with a warning) on events that cannot
//!   emit additional context; `0` disables spilling entirely (CDX-HK-004).
//! - A layer with both `hooks.json` and inline `[hooks]` loads both and warns
//!   (CDX-HK-003).

use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    rules::config_span::{Seg, TomlSpans, json_offset, line_col},
    rules::{Validator, ValidatorMetadata},
    schemas::kiro_json::{closest_name, parse_json_document},
};
use rust_i18n::t;
use serde_json::Value;
use std::path::Path;

const RULE_IDS: &[&str] = &[
    "CDX-HK-000",
    "CDX-HK-001",
    "CDX-HK-002",
    "CDX-HK-003",
    "CDX-HK-004",
    "CDX-HK-005",
];

/// Hook events Codex reads (`HookEventsToml`).
pub(crate) const CODEX_HOOK_EVENTS: &[&str] = &[
    "PreToolUse",
    "PermissionRequest",
    "PostToolUse",
    "PreCompact",
    "PostCompact",
    "SessionStart",
    "SessionEnd",
    "UserPromptSubmit",
    "SubagentStart",
    "SubagentStop",
    "Stop",
    "Interrupt",
];

/// Events whose command hooks can return `additionalContext`; Codex ignores
/// `additionalContextLimit` everywhere else.
const CONTEXT_EVENTS: &[&str] = &[
    "PreToolUse",
    "PostToolUse",
    "SessionStart",
    "UserPromptSubmit",
    "SubagentStart",
];

/// Events whose `matcher` Codex compiles; it ignores the matcher of the rest.
const MATCHER_EVENTS: &[&str] = &[
    "PreToolUse",
    "PermissionRequest",
    "PostToolUse",
    "SessionStart",
    "SessionEnd",
    "SubagentStart",
    "SubagentStop",
    "PreCompact",
    "PostCompact",
];

/// Maximum timeout, in seconds, Codex allows for `SessionEnd` and `Interrupt`.
const SHORT_EVENT_MAX_TIMEOUT: u64 = 3;

/// Where a set of hook events was read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HookLayer {
    /// `.codex/hooks.json` or `~/.codex/hooks.json`.
    HooksJson,
    /// A Codex plugin's `hooks/hooks.json`. `shared_with_claude` is set when
    /// the plugin also has a Claude Code manifest, so Claude reads the file
    /// too.
    PluginHooksJson { shared_with_claude: bool },
    /// Inline `[hooks]` in a Codex `config.toml`.
    ConfigToml,
    /// Inline `[hooks]` in managed `requirements.toml`.
    RequirementsToml,
}

impl HookLayer {
    fn is_toml(self) -> bool {
        matches!(self, Self::ConfigToml | Self::RequirementsToml)
    }

    fn shared_with_claude(self) -> bool {
        matches!(
            self,
            Self::PluginHooksJson {
                shared_with_claude: true
            }
        )
    }

    /// Non-event keys Codex accepts in the hooks map of this layer.
    fn extra_keys(self) -> &'static [&'static str] {
        match self {
            Self::ConfigToml => &["state"],
            Self::RequirementsToml => &["managed_dir", "windows_managed_dir"],
            Self::HooksJson | Self::PluginHooksJson { .. } => &[],
        }
    }
}

/// Positions for a hooks document, in either TOML or JSON.
pub(crate) enum HookSpans<'a> {
    Toml(&'a TomlSpans<'a>),
    Json(&'a crate::schemas::kiro_json::JsonNode),
    None,
}

impl HookSpans<'_> {
    fn offset(&self, path: &[Seg<'_>]) -> Option<usize> {
        match self {
            Self::Toml(spans) => spans.offset(path),
            Self::Json(root) => json_offset(root, path),
            Self::None => None,
        }
    }
}

/// Shared context for one hooks document.
pub(crate) struct HookCheck<'a> {
    pub path: &'a Path,
    pub content: &'a str,
    pub spans: HookSpans<'a>,
    pub layer: HookLayer,
}

impl HookCheck<'_> {
    fn pos(&self, path: &[Seg<'_>]) -> (usize, usize) {
        self.spans
            .offset(path)
            .map_or((1, 1), |offset| line_col(self.content, offset))
    }

    fn file_scope(&self) -> String {
        if self.layer.is_toml() {
            t!("rules.cdx_hk_000.scope_inline").to_string()
        } else {
            t!("rules.cdx_hk_000.scope_file").to_string()
        }
    }
}

pub struct CodexHooksValidator;

impl Validator for CodexHooksValidator {
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
        if !RULE_IDS.iter().any(|id| config.is_rule_enabled(id)) {
            return diagnostics;
        }
        let Some(layer) = hook_layer(path, config) else {
            return diagnostics;
        };

        let value: Value = match serde_json::from_str(content) {
            Ok(value) => value,
            Err(error) => {
                if config.is_rule_enabled("CDX-HK-000") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            error.line().max(1),
                            error.column(),
                            "CDX-HK-000",
                            t!("rules.cdx_hk_000.parse_error", error = error.to_string()),
                        )
                        .with_suggestion(t!("rules.cdx_hk_000.parse_suggestion")),
                    );
                }
                return diagnostics;
            }
        };
        let document = parse_json_document(content, false).ok();
        let check = HookCheck {
            path,
            content,
            spans: document
                .as_ref()
                .map_or(HookSpans::None, |doc| HookSpans::Json(&doc.root)),
            layer,
        };

        let Some(root) = value.as_object() else {
            push_skip(
                &check,
                config,
                &[],
                t!("rules.cdx_hk_000.root_type").to_string(),
                &mut diagnostics,
            );
            return diagnostics;
        };
        for (key, field) in root {
            match key.as_str() {
                "hooks" => {}
                "description" if field.is_string() => {}
                "description" => push_skip(
                    &check,
                    config,
                    &[Seg::Key("description")],
                    t!(
                        "rules.cdx_hk_000.wrong_type",
                        field = "description",
                        expected = "a string"
                    )
                    .to_string(),
                    &mut diagnostics,
                ),
                other => push_skip(
                    &check,
                    config,
                    &[Seg::Key(other)],
                    t!("rules.cdx_hk_000.unknown_top_level", key = other).to_string(),
                    &mut diagnostics,
                ),
            }
        }
        if let Some(events) = root.get("hooks") {
            check_hook_events(
                events,
                &[Seg::Key("hooks")],
                &check,
                config,
                &mut diagnostics,
            );
            if layer == HookLayer::HooksJson
                && config.is_rule_enabled("CDX-HK-003")
                && has_hook_handlers(events)
                && inline_hooks_in_sibling_config(path, config)
            {
                let (line, column) = check.pos(&[Seg::Key("hooks")]);
                diagnostics.push(
                    Diagnostic::info(
                        path.to_path_buf(),
                        line,
                        column,
                        "CDX-HK-003",
                        t!("rules.cdx_hk_003.message"),
                    )
                    .with_suggestion(t!("rules.cdx_hk_003.suggestion")),
                );
            }
        }
        diagnostics
    }
}

/// The layer `path` belongs to, or `None` when Codex does not read it as a
/// hooks file (a `hooks/hooks.json` outside a Codex plugin, or one the plugin
/// manifest replaces with its own `hooks` entry).
fn hook_layer(path: &Path, config: &PerFileLintConfig<'_>) -> Option<HookLayer> {
    let parent = path.parent()?;
    let parent_name = parent.file_name().and_then(|name| name.to_str());
    if parent_name == Some(".codex") {
        return Some(HookLayer::HooksJson);
    }
    if parent_name != Some("hooks") {
        return None;
    }
    let plugin_root = parent.parent()?;
    let fs = config.fs();
    let manifest_path = plugin_root.join(".codex-plugin").join("plugin.json");
    if !fs.is_file(&manifest_path) {
        return None;
    }
    // "If a manifest defines `hooks`, Codex uses those manifest entries
    // instead of the default `hooks/hooks.json`."
    if let Ok(manifest) = fs.read_to_string(&manifest_path)
        && let Ok(manifest) = serde_json::from_str::<Value>(&manifest)
        && let Some(hooks) = manifest.get("hooks")
        && !manifest_hooks_include_default(hooks)
    {
        return None;
    }
    let shared_with_claude = fs.is_file(&plugin_root.join(".claude-plugin").join("plugin.json"));
    Some(HookLayer::PluginHooksJson { shared_with_claude })
}

fn manifest_hooks_include_default(hooks: &Value) -> bool {
    let is_default = |value: &Value| {
        value.as_str().is_some_and(|path| {
            matches!(
                path.trim().trim_start_matches("./"),
                "hooks/hooks.json" | "hooks\\hooks.json"
            )
        })
    };
    match hooks {
        Value::Null => true,
        Value::Array(items) => items.iter().any(is_default),
        value => is_default(value),
    }
}

fn has_hook_handlers(events: &Value) -> bool {
    events.as_object().is_some_and(|events| {
        CODEX_HOOK_EVENTS.iter().any(|event| {
            events
                .get(*event)
                .and_then(Value::as_array)
                .is_some_and(|groups| !groups.is_empty())
        })
    })
}

/// Whether the `config.toml` next to this `hooks.json` has inline hooks.
fn inline_hooks_in_sibling_config(path: &Path, config: &PerFileLintConfig<'_>) -> bool {
    let Some(dir) = path.parent() else {
        return false;
    };
    let Ok(content) = config.fs().read_to_string(&dir.join("config.toml")) else {
        return false;
    };
    let Ok(table) = toml::from_str::<toml::Table>(&content) else {
        return false;
    };
    let Ok(value) = serde_json::to_value(table) else {
        return false;
    };
    value.get("hooks").is_some_and(has_hook_handlers)
}

fn push_skip(
    check: &HookCheck<'_>,
    config: &PerFileLintConfig<'_>,
    path: &[Seg<'_>],
    reason: String,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !config.is_rule_enabled("CDX-HK-000") {
        return;
    }
    let (line, column) = check.pos(path);
    diagnostics.push(
        Diagnostic::error(
            check.path.to_path_buf(),
            line,
            column,
            "CDX-HK-000",
            t!(
                "rules.cdx_hk_000.message",
                reason = reason.as_str(),
                scope = check.file_scope()
            ),
        )
        .with_suggestion(t!("rules.cdx_hk_000.suggestion")),
    );
}

fn push(
    level: fn(std::path::PathBuf, usize, usize, &str, String) -> Diagnostic,
    check: &HookCheck<'_>,
    rule: &'static str,
    path: &[Seg<'_>],
    message: String,
    suggestion: String,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (line, column) = check.pos(path);
    diagnostics.push(
        level(check.path.to_path_buf(), line, column, rule, message).with_suggestion(suggestion),
    );
}

fn non_negative_integer(value: &Value) -> Option<u64> {
    value.as_u64()
}

/// Check one hooks map (`hooks` in a `hooks.json`, or an inline `[hooks]`
/// table). `base` is the path to the map.
pub(crate) fn check_hook_events(
    events: &Value,
    base: &[Seg<'_>],
    check: &HookCheck<'_>,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(events) = events.as_object() else {
        push_skip(
            check,
            config,
            base,
            t!(
                "rules.cdx_hk_000.wrong_type",
                field = "hooks",
                expected = "an object"
            )
            .to_string(),
            diagnostics,
        );
        return;
    };
    let layer = check.layer;
    for (event, groups) in events {
        let mut event_path = base.to_vec();
        event_path.push(Seg::Key(event.as_str()));
        if !CODEX_HOOK_EVENTS.contains(&event.as_str()) {
            if layer.extra_keys().contains(&event.as_str()) {
                continue;
            }
            // A shared plugin file also feeds Claude Code, which has events
            // Codex lacks; only report clear misspellings there.
            let suggestion = closest_name(event, CODEX_HOOK_EVENTS);
            if config.is_rule_enabled("CDX-HK-005")
                && (!layer.shared_with_claude() || suggestion.is_some_and(|s| s != event))
            {
                let message = match suggestion {
                    Some(name) => t!(
                        "rules.cdx_hk_005.near_miss",
                        event = event.as_str(),
                        suggestion = name
                    ),
                    None => t!("rules.cdx_hk_005.message", event = event.as_str()),
                };
                push(
                    Diagnostic::warning,
                    check,
                    "CDX-HK-005",
                    &event_path,
                    message.to_string(),
                    t!(
                        "rules.cdx_hk_005.suggestion",
                        events = CODEX_HOOK_EVENTS.join(", ")
                    )
                    .to_string(),
                    diagnostics,
                );
            }
            continue;
        }
        let Some(groups) = groups.as_array() else {
            push_skip(
                check,
                config,
                &event_path,
                t!(
                    "rules.cdx_hk_000.wrong_type",
                    field = event.as_str(),
                    expected = "an array of matcher groups"
                )
                .to_string(),
                diagnostics,
            );
            continue;
        };
        for (group_index, group) in groups.iter().enumerate() {
            let mut group_path = event_path.clone();
            group_path.push(Seg::Index(group_index));
            check_matcher_group(event, group, &group_path, check, config, diagnostics);
        }
    }
}

fn check_matcher_group(
    event: &str,
    group: &Value,
    group_path: &[Seg<'_>],
    check: &HookCheck<'_>,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(group) = group.as_object() else {
        push_skip(
            check,
            config,
            group_path,
            t!(
                "rules.cdx_hk_000.wrong_type",
                field = format!("{event} matcher group"),
                expected = "an object"
            )
            .to_string(),
            diagnostics,
        );
        return;
    };
    let mut matcher_path = group_path.to_vec();
    matcher_path.push(Seg::Key("matcher"));
    match group.get("matcher") {
        None | Some(Value::Null) => {}
        Some(Value::String(matcher)) => {
            if MATCHER_EVENTS.contains(&event)
                && !matcher.is_empty()
                && matcher != "*"
                && !matcher
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '|')
                && let Err(error) = regex::Regex::new(matcher)
                && config.is_rule_enabled("CDX-HK-002")
            {
                push(
                    Diagnostic::warning,
                    check,
                    "CDX-HK-002",
                    &matcher_path,
                    t!(
                        "rules.cdx_hk_002.bad_matcher",
                        matcher = matcher.as_str(),
                        event = event,
                        error = first_line(&error.to_string())
                    )
                    .to_string(),
                    t!("rules.cdx_hk_002.bad_matcher_suggestion").to_string(),
                    diagnostics,
                );
            }
        }
        Some(_) => push_skip(
            check,
            config,
            &matcher_path,
            t!(
                "rules.cdx_hk_000.wrong_type",
                field = "matcher",
                expected = "a string"
            )
            .to_string(),
            diagnostics,
        ),
    }
    let mut hooks_path = group_path.to_vec();
    hooks_path.push(Seg::Key("hooks"));
    let handlers = match group.get("hooks") {
        None => return,
        Some(Value::Array(handlers)) => handlers,
        Some(_) => {
            push_skip(
                check,
                config,
                &hooks_path,
                t!(
                    "rules.cdx_hk_000.wrong_type",
                    field = "hooks",
                    expected = "an array of handlers"
                )
                .to_string(),
                diagnostics,
            );
            return;
        }
    };
    for (index, handler) in handlers.iter().enumerate() {
        let mut handler_path = hooks_path.clone();
        handler_path.push(Seg::Index(index));
        check_handler(event, handler, &handler_path, check, config, diagnostics);
    }
}

fn check_handler(
    event: &str,
    handler: &Value,
    handler_path: &[Seg<'_>],
    check: &HookCheck<'_>,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let field_path = |field: &'static str| {
        let mut path = handler_path.to_vec();
        path.push(Seg::Key(field));
        path
    };
    let Some(handler) = handler.as_object() else {
        push_skip(
            check,
            config,
            handler_path,
            t!(
                "rules.cdx_hk_000.wrong_type",
                field = "hook handler",
                expected = "an object"
            )
            .to_string(),
            diagnostics,
        );
        return;
    };
    let kind = match handler.get("type") {
        Some(Value::String(kind)) => kind.as_str(),
        Some(_) => {
            push_skip(
                check,
                config,
                &field_path("type"),
                t!(
                    "rules.cdx_hk_000.wrong_type",
                    field = "type",
                    expected = "a string"
                )
                .to_string(),
                diagnostics,
            );
            return;
        }
        None => {
            push_skip(
                check,
                config,
                handler_path,
                t!("rules.cdx_hk_000.missing_type").to_string(),
                diagnostics,
            );
            return;
        }
    };
    let skipped = |reason: String, diagnostics: &mut Vec<Diagnostic>, path: &[Seg<'_>]| {
        if config.is_rule_enabled("CDX-HK-002") {
            push(
                Diagnostic::warning,
                check,
                "CDX-HK-002",
                path,
                reason,
                t!("rules.cdx_hk_002.suggestion").to_string(),
                diagnostics,
            );
        }
    };
    match kind {
        "command" => {
            let command = match handler.get("command") {
                Some(Value::String(command)) => Some(command),
                Some(_) => {
                    push_skip(
                        check,
                        config,
                        &field_path("command"),
                        t!(
                            "rules.cdx_hk_000.wrong_type",
                            field = "command",
                            expected = "a string"
                        )
                        .to_string(),
                        diagnostics,
                    );
                    None
                }
                None => {
                    push_skip(
                        check,
                        config,
                        handler_path,
                        t!(
                            "rules.cdx_hk_000.missing_field",
                            field = "command",
                            kind = "command"
                        )
                        .to_string(),
                        diagnostics,
                    );
                    None
                }
            };
            for field in ["commandWindows", "command_windows", "statusMessage"] {
                if handler
                    .get(field)
                    .is_some_and(|value| !value.is_string() && !value.is_null())
                {
                    push_skip(
                        check,
                        config,
                        &field_path(field),
                        t!(
                            "rules.cdx_hk_000.wrong_type",
                            field = field,
                            expected = "a string"
                        )
                        .to_string(),
                        diagnostics,
                    );
                }
            }
            if handler
                .get("async")
                .is_some_and(|value| !value.is_boolean())
            {
                push_skip(
                    check,
                    config,
                    &field_path("async"),
                    t!(
                        "rules.cdx_hk_000.wrong_type",
                        field = "async",
                        expected = "a boolean"
                    )
                    .to_string(),
                    diagnostics,
                );
            }
            if let Some(command) = command
                && command.trim().is_empty()
            {
                skipped(
                    t!("rules.cdx_hk_002.blank_command", event = event).to_string(),
                    diagnostics,
                    &field_path("command"),
                );
            }
            check_timeout(
                event,
                handler,
                &field_path("timeout"),
                check,
                config,
                diagnostics,
            );
            check_context_limit(
                event,
                handler,
                &field_path("additionalContextLimit"),
                check,
                config,
                diagnostics,
            );
        }
        "mcp_tool" => {
            let mut blank = Vec::new();
            for field in ["server", "tool"] {
                match handler.get(field) {
                    Some(Value::String(value)) => {
                        if value.trim().is_empty() {
                            blank.push(field);
                        }
                    }
                    Some(_) => push_skip(
                        check,
                        config,
                        &field_path(field),
                        t!(
                            "rules.cdx_hk_000.wrong_type",
                            field = field,
                            expected = "a string"
                        )
                        .to_string(),
                        diagnostics,
                    ),
                    None => push_skip(
                        check,
                        config,
                        handler_path,
                        t!(
                            "rules.cdx_hk_000.missing_field",
                            field = field,
                            kind = "mcp_tool"
                        )
                        .to_string(),
                        diagnostics,
                    ),
                }
            }
            match handler.get("input") {
                None | Some(Value::Object(_)) => {}
                Some(_) => push_skip(
                    check,
                    config,
                    &field_path("input"),
                    t!(
                        "rules.cdx_hk_000.wrong_type",
                        field = "input",
                        expected = "an object"
                    )
                    .to_string(),
                    diagnostics,
                ),
            }
            if handler
                .get("statusMessage")
                .is_some_and(|value| !value.is_string() && !value.is_null())
            {
                push_skip(
                    check,
                    config,
                    &field_path("statusMessage"),
                    t!(
                        "rules.cdx_hk_000.wrong_type",
                        field = "statusMessage",
                        expected = "a string"
                    )
                    .to_string(),
                    diagnostics,
                );
            }
            if event == "SessionEnd" {
                skipped(
                    t!("rules.cdx_hk_002.session_end_mcp").to_string(),
                    diagnostics,
                    &field_path("type"),
                );
            } else if !blank.is_empty() {
                skipped(
                    t!("rules.cdx_hk_002.blank_mcp", fields = blank.join(" and ")).to_string(),
                    diagnostics,
                    handler_path,
                );
            }
            check_timeout(
                event,
                handler,
                &field_path("timeout"),
                check,
                config,
                diagnostics,
            );
        }
        "prompt" | "agent" => {
            // Claude Code runs prompt and agent hooks from a shared plugin
            // file; Codex skipping them there is expected.
            if !check.layer.shared_with_claude() {
                skipped(
                    t!(
                        "rules.cdx_hk_002.unsupported_type",
                        kind = kind,
                        event = event
                    )
                    .to_string(),
                    diagnostics,
                    &field_path("type"),
                );
            }
        }
        other => push_skip(
            check,
            config,
            &field_path("type"),
            t!("rules.cdx_hk_000.unknown_type", kind = other).to_string(),
            diagnostics,
        ),
    }
}

fn check_timeout(
    event: &str,
    handler: &serde_json::Map<String, Value>,
    path: &[Seg<'_>],
    check: &HookCheck<'_>,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(value) = handler.get("timeout") else {
        return;
    };
    if value.is_null() {
        return;
    }
    let Some(timeout) = non_negative_integer(value) else {
        push_skip(
            check,
            config,
            path,
            t!(
                "rules.cdx_hk_000.wrong_type",
                field = "timeout",
                expected = "a non-negative whole number of seconds"
            )
            .to_string(),
            diagnostics,
        );
        return;
    };
    if !config.is_rule_enabled("CDX-HK-001") {
        return;
    }
    let short_event = matches!(event, "SessionEnd" | "Interrupt");
    let message = if short_event && timeout > SHORT_EVENT_MAX_TIMEOUT {
        Some(t!(
            "rules.cdx_hk_001.too_long",
            event = event,
            timeout = timeout,
            max = SHORT_EVENT_MAX_TIMEOUT
        ))
    } else if timeout == 0 {
        Some(t!("rules.cdx_hk_001.zero", event = event))
    } else {
        None
    };
    if let Some(message) = message {
        push(
            Diagnostic::warning,
            check,
            "CDX-HK-001",
            path,
            message.to_string(),
            t!("rules.cdx_hk_001.suggestion").to_string(),
            diagnostics,
        );
    }
}

fn check_context_limit(
    event: &str,
    handler: &serde_json::Map<String, Value>,
    path: &[Seg<'_>],
    check: &HookCheck<'_>,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(value) = handler.get("additionalContextLimit") else {
        return;
    };
    let Some(limit) = non_negative_integer(value) else {
        push_skip(
            check,
            config,
            path,
            t!(
                "rules.cdx_hk_000.wrong_type",
                field = "additionalContextLimit",
                expected = "a non-negative whole number"
            )
            .to_string(),
            diagnostics,
        );
        return;
    };
    if !config.is_rule_enabled("CDX-HK-004") {
        return;
    }
    let message = if !CONTEXT_EVENTS.contains(&event) {
        Some(t!("rules.cdx_hk_004.no_context", event = event))
    } else if limit == 0 {
        Some(t!("rules.cdx_hk_004.unbounded", event = event))
    } else {
        None
    };
    if let Some(message) = message {
        push(
            Diagnostic::info,
            check,
            "CDX-HK-004",
            path,
            message.to_string(),
            t!("rules.cdx_hk_004.suggestion").to_string(),
            diagnostics,
        );
    }
}

fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(text)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::fs::MockFileSystem;
    use std::sync::Arc;

    fn run(path: &str, content: &str) -> Vec<Diagnostic> {
        CodexHooksValidator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn rules(diagnostics: &[Diagnostic]) -> Vec<&str> {
        let mut rules: Vec<&str> = diagnostics.iter().map(|d| d.rule.as_str()).collect();
        rules.sort_unstable();
        rules
    }

    #[test]
    fn documented_example_is_clean() {
        let content = r#"{
  "description": "Optional lifecycle hooks for this workspace.",
  "hooks": {
    "SessionStart": [{"matcher": "startup|resume", "hooks": [{"type": "command", "command": "python3 ~/.codex/hooks/session_start.py", "statusMessage": "Loading session notes", "additionalContextLimit": 5000}]}],
    "SessionEnd": [{"hooks": [{"type": "command", "command": "python3 ~/.codex/hooks/session_end.py", "timeout": 3}]}],
    "PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "x", "statusMessage": "Checking Bash command"}]}],
    "PostToolUse": [{"matcher": "Write|Edit", "hooks": [{"type": "mcp_tool", "server": "scanner", "tool": "scan_patch", "input": {"patch": "${tool_input.command}"}, "timeout": 30}]}],
    "Stop": [{"hooks": [{"type": "command", "command": "y", "timeout": 30}]}]
  }
}"#;
        assert!(run(".codex/hooks.json", content).is_empty());
    }

    #[test]
    fn session_end_timeout_over_three_seconds_is_clamped() {
        let content = "{\n  \"hooks\": {\n    \"SessionEnd\": [{\"hooks\": [{\"type\": \"command\", \"command\": \"x\",\n      \"timeout\": 60}]}]\n  }\n}";
        let diagnostics = run(".codex/hooks.json", content);
        assert_eq!(rules(&diagnostics), ["CDX-HK-001"]);
        assert_eq!((diagnostics[0].line, diagnostics[0].column), (4, 7));
        assert!(diagnostics[0].message.contains("3"));
        let zero = r#"{"hooks": {"PreToolUse": [{"hooks": [{"type": "command", "command": "x", "timeout": 0}]}]}}"#;
        assert_eq!(rules(&run(".codex/hooks.json", zero)), ["CDX-HK-001"]);
        let ok = r#"{"hooks": {"Interrupt": [{"hooks": [{"type": "command", "command": "x", "timeout": 2}]}]}}"#;
        assert!(run(".codex/hooks.json", ok).is_empty());
    }

    #[test]
    fn parse_and_shape_errors_skip_the_whole_file() {
        let diagnostics = run(".codex/hooks.json", "{\"hooks\": ");
        assert_eq!(rules(&diagnostics), ["CDX-HK-000"]);
        for content in [
            r#"{"version": 1, "hooks": {}}"#,
            r#"{"hooks": {"Stop": {"hooks": []}}}"#,
            r#"{"hooks": {"Stop": [{"hooks": [{"command": "x"}]}]}}"#,
            r#"{"hooks": {"Stop": [{"hooks": [{"type": "http", "url": "x"}]}]}}"#,
            r#"{"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "x", "timeout": "30"}]}]}}"#,
            r#"{"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "x", "timeout": -1}]}]}}"#,
            r#"{"hooks": {"Stop": [{"hooks": [{"type": "command"}]}]}}"#,
            r#"{"hooks": {"Stop": [{"hooks": [{"type": "mcp_tool", "server": "s"}]}]}}"#,
        ] {
            assert!(
                rules(&run(".codex/hooks.json", content)).contains(&"CDX-HK-000"),
                "{content}"
            );
        }
    }

    #[test]
    fn skipped_handlers_are_reported() {
        for (content, needle) in [
            (
                r#"{"hooks": {"Stop": [{"hooks": [{"type": "prompt", "prompt": "check"}]}]}}"#,
                "prompt",
            ),
            (
                r#"{"hooks": {"Stop": [{"hooks": [{"type": "agent"}]}]}}"#,
                "agent",
            ),
            (
                r#"{"hooks": {"SessionEnd": [{"hooks": [{"type": "mcp_tool", "server": "s", "tool": "t"}]}]}}"#,
                "SessionEnd",
            ),
            (
                r#"{"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "  "}]}]}}"#,
                "empty",
            ),
            (
                r#"{"hooks": {"PreToolUse": [{"matcher": "Bash(", "hooks": [{"type": "command", "command": "x"}]}]}}"#,
                "Bash(",
            ),
        ] {
            let diagnostics = run(".codex/hooks.json", content);
            assert_eq!(rules(&diagnostics), ["CDX-HK-002"], "{content}");
            assert!(
                diagnostics[0].message.contains(needle),
                "{}",
                diagnostics[0].message
            );
        }
        // Matchers on events that ignore them are never compiled.
        let ignored = r#"{"hooks": {"Stop": [{"matcher": "Bash(", "hooks": [{"type": "command", "command": "x"}]}]}}"#;
        assert!(run(".codex/hooks.json", ignored).is_empty());
    }

    #[test]
    fn unknown_events_and_context_limits() {
        let diagnostics = run(
            ".codex/hooks.json",
            r#"{"hooks": {"Notification": [], "PreToolUSe": []}}"#,
        );
        assert_eq!(rules(&diagnostics), ["CDX-HK-005", "CDX-HK-005"]);
        assert!(diagnostics.iter().any(|d| d.message.contains("PreToolUse")));
        let limits = r#"{"hooks": {
            "Stop": [{"hooks": [{"type": "command", "command": "x", "additionalContextLimit": 100}]}],
            "SessionStart": [{"hooks": [{"type": "command", "command": "x", "additionalContextLimit": 0}]}]
        }}"#;
        assert_eq!(
            rules(&run(".codex/hooks.json", limits)),
            ["CDX-HK-004", "CDX-HK-004"]
        );
    }

    #[test]
    fn plugin_hooks_need_a_codex_manifest_and_respect_overrides() {
        let fs = Arc::new(MockFileSystem::new());
        fs.add_file("/p/.codex-plugin/plugin.json", r#"{"name": "p"}"#);
        fs.add_file(
            "/q/.codex-plugin/plugin.json",
            r#"{"name": "q", "hooks": "./other.json"}"#,
        );
        fs.add_file("/s/.codex-plugin/plugin.json", r#"{"name": "s"}"#);
        fs.add_file("/s/.claude-plugin/plugin.json", r#"{"name": "s"}"#);
        let mut config = LintConfig::default();
        config.set_fs(fs);
        let content = r#"{"hooks": {"Notification": [], "Stop": [{"hooks": [{"type": "prompt", "prompt": "x"}]}], "SessionEnd": [{"hooks": [{"type": "command", "command": "x", "timeout": 30}]}]}}"#;
        let run = |path: &str| CodexHooksValidator.validate(Path::new(path), content, &config);
        assert_eq!(
            rules(&run("/p/hooks/hooks.json")),
            ["CDX-HK-001", "CDX-HK-002", "CDX-HK-005"]
        );
        assert!(run("/q/hooks/hooks.json").is_empty());
        assert!(run("/x/hooks/hooks.json").is_empty());
        // Shared with Claude Code: Claude-only events and handlers are expected.
        assert_eq!(rules(&run("/s/hooks/hooks.json")), ["CDX-HK-001"]);
    }

    #[test]
    fn hooks_json_next_to_inline_hooks_warns() {
        let fs = Arc::new(MockFileSystem::new());
        fs.add_file(
            "/r/.codex/config.toml",
            "[[hooks.Stop]]\n[[hooks.Stop.hooks]]\ntype = \"command\"\ncommand = \"x\"\n",
        );
        let mut config = LintConfig::default();
        config.set_fs(fs);
        let content = r#"{"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "y"}]}]}}"#;
        let diagnostics =
            CodexHooksValidator.validate(Path::new("/r/.codex/hooks.json"), content, &config);
        assert_eq!(rules(&diagnostics), ["CDX-HK-003"]);
    }
}
