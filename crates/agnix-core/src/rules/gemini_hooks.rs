//! Gemini CLI hook validation (GM-004, GM-018, GM-019, GM-026).
//!
//! Gemini CLI reads the same `hooks` block from two places
//! (geminicli.com/docs/hooks, "Configuration schema"):
//!
//! - `settings.json` (`.gemini/settings.json`, `~/.gemini/settings.json`):
//!   the top-level `hooks` object, next to `hooksConfig` (`enabled`,
//!   `disabled`, `notifications`).
//! - an extension's `hooks/hooks.json`: "Define hooks in a `hooks/hooks.json`
//!   file within your extension directory" (docs/extensions/reference).
//!   `loadExtensionHooks` (extension-manager.ts) reads it with `JSON.parse`,
//!   ignores the file unless its `hooks` property is an object, and hydrates
//!   `${extensionPath}`, `${workspacePath}` and `${/}` in its strings.
//!
//! `hooks.<Event>` is an array of groups `{matcher?, sequential?, hooks:
//! [{type: "command", command, name?, timeout?, description?}]}`
//! (docs/hooks/reference, "Configuration schema").

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    fs::FileSystem,
    rules::{Validator, ValidatorMetadata},
    schemas::{
        gemini_settings::{VALID_HOOK_EVENTS, parse_gemini_settings},
        kiro_json::{
            JsonMember, JsonNode, JsonValue, LineIndex, closest_name, parse_json_document,
        },
    },
};
use rust_i18n::t;
use std::path::{Component, Path, PathBuf};

const RULE_IDS: &[&str] = &["GM-004", "GM-018", "GM-019", "GM-026"];

/// `hooksConfig` properties in settingsSchema.ts.
const HOOKS_CONFIG_KEYS: &[&str] = &["enabled", "disabled", "notifications"];

/// "Tool events (`BeforeTool`, `AfterTool`): Matchers are Regular
/// Expressions"; lifecycle matchers are exact strings (docs/hooks "Matchers").
const TOOL_EVENTS: &[&str] = &["BeforeTool", "AfterTool"];

/// Claude Code event names copied into Gemini hooks, with the Gemini event
/// that fires at the same point.
const CLAUDE_EVENT_EQUIVALENTS: &[(&str, &str)] = &[
    ("PreToolUse", "BeforeTool"),
    ("PostToolUse", "AfterTool"),
    ("UserPromptSubmit", "BeforeAgent"),
    ("Stop", "AfterAgent"),
    ("PreCompact", "PreCompress"),
];

/// Project-root variables Gemini CLI sets for hooks: "`GEMINI_PROJECT_DIR`:
/// The absolute path to the project root" and "`CLAUDE_PROJECT_DIR`: (Alias)
/// Provided for compatibility" (docs/hooks "Environment variables").
const PROJECT_DIR_VARS: &[&str] = &[
    "$GEMINI_PROJECT_DIR",
    "${GEMINI_PROJECT_DIR}",
    "$CLAUDE_PROJECT_DIR",
    "${CLAUDE_PROJECT_DIR}",
];

/// Which file the hooks come from.
#[derive(Clone, Copy, PartialEq, Eq)]
enum HookFile {
    Settings,
    Extension,
}

impl HookFile {
    fn of(path: &Path) -> Self {
        if path.file_name().is_some_and(|name| name == "hooks.json") {
            Self::Extension
        } else {
            Self::Settings
        }
    }
}

pub struct GeminiHooksValidator;

impl Validator for GeminiHooksValidator {
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
        if !RULE_IDS.iter().any(|id| config.is_rule_enabled(id)) {
            return Vec::new();
        }
        let kind = HookFile::of(path);
        let mut cx = Cx {
            path,
            lines: LineIndex::new(content),
            config,
            scripts: ScriptBase::for_file(path, kind),
            diagnostics: Vec::new(),
        };
        match kind {
            HookFile::Settings => {
                // GM-009 (GeminiSettingsValidator) reports syntax errors.
                if parse_gemini_settings(content).parse_error.is_some() {
                    return Vec::new();
                }
                let Ok(doc) = parse_json_document(content, true) else {
                    return Vec::new();
                };
                let Some(members) = doc.root.value.as_object() else {
                    return Vec::new();
                };
                for member in effective_members(members) {
                    match member.key.as_str() {
                        "hooksConfig" => cx.check_hooks_config(member),
                        "hooks" => cx.check_hooks(member),
                        _ => {}
                    }
                }
            }
            HookFile::Extension => match parse_json_document(content, false) {
                Err(error) => {
                    let message = t!("rules.gm_004.parse_error", error = error.message.as_str());
                    cx.push_at(error.line, error.column, message);
                }
                Ok(doc) => match doc.root.value.get("hooks") {
                    Some(member) => cx.check_hooks(member),
                    None => cx.warn(
                        doc.root.start,
                        doc.root.start + 1,
                        t!("rules.gm_004.no_hooks_object"),
                    ),
                },
            },
        }
        cx.diagnostics
    }
}

/// Members in source order, without the earlier copies of a repeated key
/// (`JSON.parse` keeps the last one).
fn effective_members(members: &[JsonMember]) -> impl Iterator<Item = &JsonMember> {
    members
        .iter()
        .enumerate()
        .filter(|(index, member)| !members[index + 1..].iter().any(|m| m.key == member.key))
        .map(|(_, member)| member)
}

struct Cx<'a, 'c> {
    path: &'a Path,
    lines: LineIndex<'a>,
    config: &'a PerFileLintConfig<'c>,
    scripts: Option<ScriptBase>,
    diagnostics: Vec<Diagnostic>,
}

impl Cx<'_, '_> {
    fn push(
        &mut self,
        level: DiagnosticLevel,
        rule: &str,
        span: (usize, usize),
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) {
        if !self.config.is_rule_enabled(rule) {
            return;
        }
        let (start, end) = span;
        let (line, column) = self.lines.line_col(start);
        let (end_line, end_column) = self.lines.line_col(end.max(start));
        let path = self.path.to_path_buf();
        let diagnostic = match level {
            DiagnosticLevel::Error => Diagnostic::error(path, line, column, rule, message),
            DiagnosticLevel::Warning => Diagnostic::warning(path, line, column, rule, message),
            DiagnosticLevel::Info => Diagnostic::info(path, line, column, rule, message),
        };
        self.diagnostics.push(
            diagnostic
                .with_span(end_line, end_column)
                .with_suggestion(suggestion),
        );
    }

    /// GM-004 warning over `start..end`.
    fn warn(&mut self, start: usize, end: usize, message: impl Into<String>) {
        self.push(
            DiagnosticLevel::Warning,
            "GM-004",
            (start, end),
            message,
            t!("rules.gm_004.suggestion"),
        );
    }

    /// GM-004 warning at a line and column (JSON syntax errors).
    fn push_at(&mut self, line: usize, column: usize, message: impl Into<String>) {
        if self.config.is_rule_enabled("GM-004") {
            self.diagnostics.push(
                Diagnostic::warning(self.path.to_path_buf(), line, column, "GM-004", message)
                    .with_suggestion(t!("rules.gm_004.suggestion")),
            );
        }
    }

    fn key_span(member: &JsonMember) -> (usize, usize) {
        (member.key_start, member.key_end)
    }

    fn value_span(node: &JsonNode) -> (usize, usize) {
        (node.start, node.end)
    }

    /// `hooksConfig` holds only `enabled` (boolean), `disabled` (array of hook
    /// names) and `notifications` (boolean).
    fn check_hooks_config(&mut self, member: &JsonMember) {
        let node = &member.value;
        let JsonValue::Object(members) = &node.value else {
            let message = t!(
                "rules.gm_004.hooks_config_not_object",
                found = node.value.type_name()
            );
            self.warn(node.start, node.end, message);
            return;
        };
        for entry in effective_members(members) {
            let key = entry.key.as_str();
            let value = &entry.value;
            match key {
                "enabled" | "notifications" => {
                    if value.value.as_bool().is_none() {
                        let message = t!(
                            "rules.gm_004.hooks_config_type",
                            key = key,
                            expected = "a boolean",
                            found = value.value.type_name()
                        );
                        self.warn(value.start, value.end, message);
                    }
                }
                "disabled" => match &value.value {
                    JsonValue::Array(items) => {
                        for (index, item) in items.iter().enumerate() {
                            if item.value.as_str().is_none() {
                                let message = t!(
                                    "rules.gm_004.hooks_config_item_type",
                                    index = index.to_string(),
                                    found = item.value.type_name()
                                );
                                self.warn(item.start, item.end, message);
                            }
                        }
                    }
                    other => {
                        let message = t!(
                            "rules.gm_004.hooks_config_type",
                            key = key,
                            expected = "an array of hook names",
                            found = other.type_name()
                        );
                        self.warn(value.start, value.end, message);
                    }
                },
                _ if is_event_like(key) => {
                    let (start, end) = Self::key_span(entry);
                    self.warn(start, end, t!("rules.gm_004.hooks_config_event", key = key));
                }
                _ => {
                    let (start, end) = Self::key_span(entry);
                    self.warn(
                        start,
                        end,
                        t!("rules.gm_004.hooks_config_unknown", key = key),
                    );
                }
            }
        }
    }

    /// The `hooks` object: event name -> array of hook groups.
    fn check_hooks(&mut self, member: &JsonMember) {
        let node = &member.value;
        let JsonValue::Object(events) = &node.value else {
            let message = t!(
                "rules.gm_004.hooks_not_object",
                found = node.value.type_name()
            );
            self.warn(node.start, node.end, message);
            return;
        };
        for event in effective_members(events) {
            let name = event.key.as_str();
            let (key_start, key_end) = Self::key_span(event);
            if HOOKS_CONFIG_KEYS.contains(&name) {
                self.warn(
                    key_start,
                    key_end,
                    t!("rules.gm_004.hooks_config_key", key = name),
                );
                continue;
            }
            if !VALID_HOOK_EVENTS.contains(&name) {
                let hint =
                    claude_equivalent(name).or_else(|| closest_name(name, VALID_HOOK_EVENTS));
                let message = match hint {
                    Some(suggestion) => t!(
                        "rules.gm_004.unknown_event_hint",
                        event = name,
                        suggestion = suggestion
                    ),
                    None => t!("rules.gm_004.unknown_event", event = name),
                };
                self.warn(key_start, key_end, message);
                continue;
            }
            let JsonValue::Array(groups) = &event.value.value else {
                let message = t!(
                    "rules.gm_004.event_not_array",
                    event = name,
                    found = event.value.value.type_name()
                );
                self.warn(event.value.start, event.value.end, message);
                continue;
            };
            for (index, group) in groups.iter().enumerate() {
                self.check_group(name, index, group);
            }
        }
    }

    /// One hook group: `{matcher?, sequential?, hooks: [...]}`.
    fn check_group(&mut self, event: &str, index: usize, group: &JsonNode) {
        let location = format!("hooks.{event}[{index}]");
        if group.value.as_object().is_none() {
            let message = t!(
                "rules.gm_004.group_not_object",
                location = location.as_str(),
                found = group.value.type_name()
            );
            self.warn(group.start, group.end, message);
            return;
        }
        let tool_event = TOOL_EVENTS.contains(&event);
        match group.value.get("matcher") {
            Some(matcher) => match &matcher.value.value {
                JsonValue::String(pattern) if tool_event => {
                    self.check_tool_matcher(event, pattern, &matcher.value);
                }
                JsonValue::String(_) => {}
                other => {
                    let message = t!(
                        "rules.gm_004.field_type",
                        location = location.as_str(),
                        field = "matcher",
                        expected = "a string",
                        found = other.type_name()
                    );
                    self.warn(matcher.value.start, matcher.value.end, message);
                }
            },
            // GM-026: no matcher runs the group for every tool call.
            None if tool_event => self.push(
                DiagnosticLevel::Info,
                "GM-026",
                (group.start, group.start + 1),
                t!("rules.gm_026.no_matcher", event = event),
                t!("rules.gm_026.suggestion"),
            ),
            None => {}
        }
        if let Some(sequential) = group.value.get("sequential")
            && sequential.value.value.as_bool().is_none()
        {
            let message = t!(
                "rules.gm_004.field_type",
                location = location.as_str(),
                field = "sequential",
                expected = "a boolean",
                found = sequential.value.value.type_name()
            );
            self.warn(sequential.value.start, sequential.value.end, message);
        }
        match group.value.get("hooks") {
            Some(hooks) => match &hooks.value.value {
                JsonValue::Array(items) => {
                    for (item_index, item) in items.iter().enumerate() {
                        let item_location = format!("{location}.hooks[{item_index}]");
                        self.check_hook(&item_location, item);
                    }
                }
                other => {
                    let message = t!(
                        "rules.gm_004.field_type",
                        location = location.as_str(),
                        field = "hooks",
                        expected = "an array of hook objects",
                        found = other.type_name()
                    );
                    self.warn(hooks.value.start, hooks.value.end, message);
                }
            },
            None => {
                // The flat `{type, command}` entry (agnix's former reading
                // of hooksConfig) is the usual cause.
                let flat = ["command", "type"]
                    .into_iter()
                    .find(|field| group.value.get(field).is_some());
                let message = match flat {
                    Some(field) => t!(
                        "rules.gm_004.flat_hook",
                        location = location.as_str(),
                        field = field
                    ),
                    None => t!("rules.gm_004.missing_hooks", location = location.as_str()),
                };
                self.warn(group.start, group.start + 1, message);
            }
        }
    }

    /// GM-026 for a tool-event matcher. Gemini trims the matcher; an empty
    /// string or `*` matches every tool before any regex is built, and an
    /// invalid regex falls back to an exact tool-name comparison
    /// (hookPlanner.ts `matchesContext`, `matchesToolName`).
    fn check_tool_matcher(&mut self, event: &str, pattern: &str, node: &JsonNode) {
        let trimmed = pattern.trim();
        if trimmed.is_empty() || trimmed == "*" || trimmed == ".*" {
            let shown = format!("\"{pattern}\"");
            self.push(
                DiagnosticLevel::Info,
                "GM-026",
                Self::value_span(node),
                t!(
                    "rules.gm_026.catch_all",
                    event = event,
                    matcher = shown.as_str()
                ),
                t!("rules.gm_026.suggestion"),
            );
        } else if let Err(error) = regress::Regex::new(trimmed) {
            self.push(
                DiagnosticLevel::Info,
                "GM-026",
                Self::value_span(node),
                t!(
                    "rules.gm_026.invalid_regex",
                    event = event,
                    matcher = trimmed,
                    error = error.to_string()
                ),
                t!("rules.gm_026.invalid_regex_suggestion"),
            );
        }
    }

    /// One hook: `{type: "command", command, name?, timeout?, description?}`.
    fn check_hook(&mut self, location: &str, item: &JsonNode) {
        if item.value.as_object().is_none() {
            let message = t!(
                "rules.gm_004.hook_not_object",
                location = location,
                found = item.value.type_name()
            );
            self.warn(item.start, item.end, message);
            return;
        }
        let hook_type = item.value.get("type");
        match hook_type.map(|member| (&member.value, member.value.value.as_str())) {
            None => {
                let message = t!(
                    "rules.gm_004.missing_field",
                    location = location,
                    field = "type"
                );
                self.warn(item.start, item.start + 1, message);
            }
            Some((_, Some("command"))) => {}
            Some((node, Some(other))) => {
                let message = t!(
                    "rules.gm_004.invalid_type",
                    location = location,
                    value = other
                );
                self.warn(node.start, node.end, message);
            }
            Some((node, None)) => {
                let message = t!(
                    "rules.gm_004.field_type",
                    location = location,
                    field = "type",
                    expected = "a string",
                    found = node.value.type_name()
                );
                self.warn(node.start, node.end, message);
            }
        }
        // hookRunner.ts runs every hook that is not an internal runtime hook
        // as a command, so `command` is needed unless `type` is something else.
        let runs_command =
            hook_type.is_none_or(|member| member.value.value.as_str() == Some("command"));
        match item.value.get("command") {
            None if runs_command => {
                let message = t!(
                    "rules.gm_004.missing_field",
                    location = location,
                    field = "command"
                );
                self.warn(item.start, item.start + 1, message);
            }
            None => {}
            Some(member) => match member.value.value.as_str() {
                Some(command) if command.trim().is_empty() => {
                    if runs_command {
                        let message = t!("rules.gm_004.empty_command", location = location);
                        self.warn(member.value.start, member.value.end, message);
                    }
                }
                Some(command) => {
                    if runs_command {
                        self.check_script(command, &member.value);
                    }
                }
                None => {
                    let message = t!(
                        "rules.gm_004.field_type",
                        location = location,
                        field = "command",
                        expected = "a string",
                        found = member.value.value.type_name()
                    );
                    self.warn(member.value.start, member.value.end, message);
                }
            },
        }
        for field in ["name", "description"] {
            if let Some(member) = item.value.get(field)
                && member.value.value.as_str().is_none()
            {
                let message = t!(
                    "rules.gm_004.field_type",
                    location = location,
                    field = field,
                    expected = "a string",
                    found = member.value.value.type_name()
                );
                self.warn(member.value.start, member.value.end, message);
            }
        }
        if let Some(member) = item.value.get("timeout") {
            self.check_timeout(location, &member.value);
        }
    }

    /// `timeout` is "Execution timeout in milliseconds (default: 60000)."
    fn check_timeout(&mut self, location: &str, node: &JsonNode) {
        let Some(value) = node.value.as_number().and_then(serde_json::Number::as_f64) else {
            let message = t!(
                "rules.gm_004.field_type",
                location = location,
                field = "timeout",
                expected = "a number of milliseconds",
                found = node.value.type_name()
            );
            self.warn(node.start, node.end, message);
            return;
        };
        let shown = format_number(value);
        if value <= 0.0 {
            let message = t!(
                "rules.gm_004.timeout_not_positive",
                location = location,
                value = shown.as_str()
            );
            self.warn(node.start, node.end, message);
        } else if (1.0..1000.0).contains(&value) {
            // GM-018: a seconds value (Claude Code hook timeouts are seconds).
            let suggested = format_number(value * 1000.0);
            self.push(
                DiagnosticLevel::Warning,
                "GM-018",
                Self::value_span(node),
                t!(
                    "rules.gm_018.message",
                    value = shown.as_str(),
                    suggested = suggested.as_str()
                ),
                t!(
                    "rules.gm_018.suggestion",
                    value = shown.as_str(),
                    suggested = suggested.as_str()
                ),
            );
        }
    }

    /// GM-019: the script a command runs does not exist (or, for the command
    /// itself, is not executable on Unix).
    fn check_script(&mut self, command: &str, node: &JsonNode) {
        if !self.config.is_rule_enabled("GM-019") {
            return;
        }
        let Some(scripts) = &self.scripts else {
            return;
        };
        let words = shell_words(command);
        let Some(candidate) = script_candidate(&words) else {
            return;
        };
        let Some(resolved) = scripts.resolve(&candidate.text) else {
            return;
        };
        let fs = self.config.fs().clone();
        if !fs.exists(&resolved) {
            let resolved = resolved.display().to_string();
            self.push(
                DiagnosticLevel::Warning,
                "GM-019",
                Self::value_span(node),
                t!(
                    "rules.gm_019.not_found",
                    script = candidate.text.as_str(),
                    resolved = resolved.as_str()
                ),
                t!("rules.gm_019.not_found_suggestion"),
            );
        } else if candidate.is_command && lacks_exec_bit(&resolved, fs.as_ref()) {
            self.push(
                DiagnosticLevel::Warning,
                "GM-019",
                Self::value_span(node),
                t!(
                    "rules.gm_019.not_executable",
                    script = candidate.text.as_str()
                ),
                t!("rules.gm_019.not_executable_suggestion"),
            );
        }
    }
}

/// A key that reads like a hook event (Gemini or Claude Code spelling).
fn is_event_like(key: &str) -> bool {
    VALID_HOOK_EVENTS.contains(&key) || claude_equivalent(key).is_some()
}

fn claude_equivalent(event: &str) -> Option<&'static str> {
    CLAUDE_EVENT_EQUIVALENTS
        .iter()
        .find(|(claude, _)| *claude == event)
        .map(|(_, gemini)| *gemini)
}

/// `30` stays `30`, `1.5` stays `1.5`.
fn format_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

// ---------------------------------------------------------------------------
// GM-019 script resolution
// ---------------------------------------------------------------------------

/// How a hook's script paths resolve for one hooks file.
struct ScriptBase {
    /// Variable spellings that Gemini replaces with `root` before running.
    vars: &'static [&'static str],
    /// The directory those variables stand for.
    root: PathBuf,
    /// Directory relative paths resolve against: hooks run in the project
    /// root (hookRunner.ts spawns with `cwd: input.cwd`), which is only known
    /// for project settings.
    cwd: Option<PathBuf>,
    /// `${/}` is hydrated to the path separator (extension hooks only).
    separator_var: bool,
}

/// Extension hook strings are hydrated with `${extensionPath}`
/// (extensions/reference "Variables").
const EXTENSION_PATH_VARS: &[&str] = &["${extensionPath}"];

impl ScriptBase {
    fn for_file(path: &Path, kind: HookFile) -> Option<Self> {
        match kind {
            HookFile::Settings => {
                let project = path.parent()?.parent()?.to_path_buf();
                if is_home_dir(&project) {
                    // ~/.gemini/settings.json: the project directory and the
                    // working directory are those of whatever project is open.
                    return Some(Self {
                        vars: &[],
                        root: project,
                        cwd: None,
                        separator_var: false,
                    });
                }
                Some(Self {
                    vars: PROJECT_DIR_VARS,
                    root: project.clone(),
                    cwd: Some(project),
                    separator_var: false,
                })
            }
            HookFile::Extension => Some(Self {
                vars: EXTENSION_PATH_VARS,
                root: path.parent()?.parent()?.to_path_buf(),
                // Extension hooks also run in the open project, not the
                // extension directory, so relative paths are not checked.
                cwd: None,
                separator_var: true,
            }),
        }
    }

    /// The file a script token names, or `None` when agnix cannot know it.
    fn resolve(&self, token: &str) -> Option<PathBuf> {
        let token = if self.separator_var {
            token.replace("${/}", "/").replace("${pathSeparator}", "/")
        } else {
            token.to_string()
        };
        if !token.contains('/') {
            return None;
        }
        let resolved = if let Some(rest) = self.vars.iter().find_map(|var| strip_var(&token, var)) {
            if has_unresolved_parts(rest) {
                return None;
            }
            normalize(&self.root.join(rest.trim_start_matches('/')))
        } else if has_unresolved_parts(&token) || token.starts_with('~') {
            return None;
        } else if is_absolute_token(&token) {
            // A literal absolute path is checked only inside the project
            // (or extension) directory; system tools are not agnix's business.
            let path = normalize(Path::new(&token));
            if !(self.root.is_absolute() && path.starts_with(&self.root)) {
                return None;
            }
            path
        } else {
            normalize(&self.cwd.as_ref()?.join(&token))
        };
        // Dependencies are installed after checkout.
        if resolved
            .components()
            .any(|component| component.as_os_str() == "node_modules")
        {
            return None;
        }
        Some(resolved)
    }
}

/// The rest of `token` after a leading `var`, when it starts a path.
fn strip_var<'t>(token: &'t str, var: &str) -> Option<&'t str> {
    let rest = token.strip_prefix(var)?;
    (rest.is_empty() || rest.starts_with('/')).then_some(rest)
}

/// Variables, command substitution and glob characters agnix does not expand.
fn has_unresolved_parts(text: &str) -> bool {
    text.contains(['$', '`', '%', '*', '?', '[', '{'])
}

fn is_absolute_token(token: &str) -> bool {
    let bytes = token.as_bytes();
    token.starts_with('/')
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\'))
}

/// Resolve `.` and `..` without touching the file system.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match out.components().next_back() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                Some(Component::RootDir | Component::Prefix(_)) => {}
                _ => out.push(".."),
            },
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// `dir` is the user's home directory (where `~/.gemini/` lives).
#[cfg(feature = "filesystem")]
pub(crate) fn is_home_dir(dir: &Path) -> bool {
    dirs::home_dir().is_some_and(|home| home == dir)
}

#[cfg(not(feature = "filesystem"))]
pub(crate) fn is_home_dir(_dir: &Path) -> bool {
    false
}

#[cfg(unix)]
fn lacks_exec_bit(path: &Path, fs: &dyn FileSystem) -> bool {
    use std::os::unix::fs::PermissionsExt;
    // The FileSystem trait has no permission bits; a mock file that is not
    // also on disk is never reported.
    fs.is_file(path)
        && std::fs::metadata(path)
            .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 == 0)
}

#[cfg(not(unix))]
fn lacks_exec_bit(_path: &Path, _fs: &dyn FileSystem) -> bool {
    false
}

/// A shell word, or an operator (`;`, `&`, `|`, `<`, `>`, `(`, `)`).
#[derive(Debug, PartialEq, Eq)]
struct Word {
    text: String,
    operator: bool,
}

/// Split a command into shell words: quotes are removed, `\` escapes the
/// next character, and a `#` that starts a word ends the command.
fn shell_words(command: &str) -> Vec<Word> {
    fn flush(words: &mut Vec<Word>, current: &mut String, started: &mut bool) {
        if *started {
            words.push(Word {
                text: std::mem::take(current),
                operator: false,
            });
            *started = false;
        }
    }
    let mut words = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                started = true;
                for c in chars.by_ref() {
                    if c == '\'' {
                        break;
                    }
                    current.push(c);
                }
            }
            '"' => {
                started = true;
                while let Some(c) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' if chars
                            .peek()
                            .is_some_and(|next| matches!(next, '"' | '\\' | '$' | '`')) =>
                        {
                            current.extend(chars.next());
                        }
                        _ => current.push(c),
                    }
                }
            }
            '\\' => {
                started = true;
                current.extend(chars.next());
            }
            ';' | '&' | '|' | '<' | '>' | '(' | ')' => {
                flush(&mut words, &mut current, &mut started);
                words.push(Word {
                    text: c.to_string(),
                    operator: true,
                });
            }
            '#' if !started => break,
            c if c.is_whitespace() => flush(&mut words, &mut current, &mut started),
            c => {
                started = true;
                current.push(c);
            }
        }
    }
    flush(&mut words, &mut current, &mut started);
    words
}

/// The path a command runs: its first word when that contains a `/`, or the
/// first path argument of a known interpreter (`bash script.sh`).
#[derive(Debug, PartialEq, Eq)]
struct Candidate {
    text: String,
    /// The path is the command itself, so it must be executable.
    is_command: bool,
}

#[derive(Clone, Copy)]
enum Interpreter {
    Shell,
    Node,
    Python,
}

impl Interpreter {
    fn of(word: &str) -> Option<Self> {
        let name = word.rsplit(['/', '\\']).next().unwrap_or(word);
        let name = name.to_ascii_lowercase();
        let name = name.strip_suffix(".exe").unwrap_or(&name);
        match name {
            "bash" | "sh" | "zsh" | "dash" | "ksh" => Some(Self::Shell),
            "node" | "nodejs" => Some(Self::Node),
            _ if name
                .strip_prefix("python")
                .is_some_and(|v| v.chars().all(|c| c.is_ascii_digit() || c == '.')) =>
            {
                Some(Self::Python)
            }
            _ => None,
        }
    }

    /// The flag makes the interpreter run inline code or a module instead
    /// of a script file.
    fn runs_inline(self, flag: &str) -> bool {
        match self {
            Self::Shell => !flag.starts_with("--") && flag[1..].contains(['c', 's']),
            Self::Node => {
                matches!(flag, "-e" | "--eval" | "-p" | "--print")
                    || flag.starts_with("--eval=")
                    || flag.starts_with("--print=")
            }
            Self::Python => matches!(flag, "-c" | "-m"),
        }
    }

    /// The flag consumes the next word as its value.
    fn takes_value(self, flag: &str) -> bool {
        match self {
            Self::Shell => matches!(flag, "-o" | "-O" | "--rcfile" | "--init-file"),
            Self::Node => matches!(
                flag,
                "-r" | "--require"
                    | "--import"
                    | "--loader"
                    | "--experimental-loader"
                    | "-C"
                    | "--conditions"
                    | "--input-type"
                    | "--env-file"
            ),
            Self::Python => matches!(flag, "-W" | "-X"),
        }
    }
}

fn script_candidate(words: &[Word]) -> Option<Candidate> {
    let first = words.first().filter(|word| !word.operator)?;
    // `VAR=value command ...`: not analyzed.
    if first.text.contains('=') {
        return None;
    }
    if let Some(interpreter) = Interpreter::of(&first.text) {
        let mut rest = words[1..].iter();
        while let Some(word) = rest.next() {
            if word.operator {
                return None;
            }
            let arg = word.text.as_str();
            if arg.len() > 1 && arg.starts_with('-') {
                if interpreter.runs_inline(arg) {
                    return None;
                }
                if interpreter.takes_value(arg) {
                    rest.next();
                }
                continue;
            }
            return arg.contains('/').then(|| Candidate {
                text: arg.to_string(),
                is_command: false,
            });
        }
        return None;
    }
    first.text.contains('/').then(|| Candidate {
        text: first.text.clone(),
        is_command: true,
    })
}

#[cfg(test)]
mod tests;
