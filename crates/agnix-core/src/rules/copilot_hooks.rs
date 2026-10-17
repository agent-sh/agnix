//! GitHub Copilot hook files: COP-017 (schema), COP-031 (entry constraints),
//! COP-036 (cloud agent and script reachability) and COP-037 (literal
//! secrets).
//!
//! Copilot CLI and cloud agent load every `.github/hooks/*.json` file, and the
//! CLI also loads `~/.copilot/hooks/*.json` ("You define hooks in JSON files,
//! stored in your repository at `.github/hooks/*.json`"). The Copilot format
//! has a numeric `version` of 1 and lower camelCase events. VS Code's Local
//! harness reads the same folder and also accepts its native format:
//! "PascalCase events without a numeric `version` property".
//!
//! Copilot settings files (`.github/copilot/settings.json`,
//! `~/.copilot/settings.json`) carry the same event map inline under `hooks`.
//! "Hooks defined inline in `settings.json` remain strict: any item-level
//! validation error rejects the whole `hooks` field", while a hook file only
//! drops the malformed item.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    rules::config_json::{JsonCx, key_span, node_span, str_field},
    schemas::kiro_json::{JsonNode, JsonValue, parse_json_document},
};
use rust_i18n::t;
use std::path::Path;

/// Hook events of the Copilot format (hooks reference, "Hook events").
pub(crate) const COPILOT_HOOK_EVENTS: &[&str] = &[
    "agentStop",
    "errorOccurred",
    "notification",
    "permissionRequest",
    "postToolUse",
    "postToolUseFailure",
    "preCompact",
    "preToolUse",
    "sessionEnd",
    "sessionStart",
    "subagentStart",
    "subagentStop",
    "userPromptSubmitted",
    "userPromptTransformed",
];

/// PascalCase event names: the aliases the Copilot hooks reference documents
/// ("Configure the event name in PascalCase (for example, `SessionStart`)"),
/// which select the VS Code-compatible payload, and the events of VS Code's
/// native format.
pub(crate) const PASCAL_CASE_HOOK_EVENTS: &[&str] = &[
    "SessionStart",
    "SessionEnd",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "PostToolUseFailure",
    "PermissionRequest",
    "Stop",
    "SubagentStart",
    "SubagentStop",
    "ErrorOccurred",
    "PreCompact",
];

/// Events whose HTTP response can grant tool permissions, so HTTP hooks on
/// them must use https:// even for localhost.
const PERMISSION_EVENTS: &[&str] = &[
    "preToolUse",
    "permissionRequest",
    "PreToolUse",
    "PermissionRequest",
];

/// PascalCase events whose matchers follow Claude's matcher semantics:
/// "`*`, `**`, or an empty `matcher` value fires for every tool", and a
/// `|`-separated list of names matches by name.
const CLAUDE_MATCHER_EVENTS: &[&str] = &["PreToolUse", "PermissionRequest"];

/// Fields of a command hook entry in the Copilot format, plus the OS
/// overrides of the VS Code format, which the Local harness maps.
const COMMAND_FIELDS: &[&str] = &["bash", "powershell", "command", "windows", "linux", "osx"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    /// A `.github/hooks/*.json` or `~/.copilot/hooks/*.json` file.
    HookFile,
    /// The inline `hooks` field of a Copilot settings file.
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    /// `version: 1` and camelCase events.
    Copilot,
    /// VS Code native: PascalCase events and no `version`.
    Native,
}

/// Validate a Copilot hook file.
pub(crate) fn validate_hook_file(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    validate(path, content, config, Source::HookFile)
}

/// Validate the inline `hooks` field of a Copilot settings file.
pub(crate) fn validate_settings_file(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    validate(path, content, config, Source::Settings)
}

fn validate(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
    source: Source,
) -> Vec<Diagnostic> {
    let mut cx = JsonCx::new(path, content, config);
    let schema = cx.enabled("COP-017");

    // Copilot reads these files with JSON.parse; settings.json files are
    // JSON too. Comments are accepted only for settings files, which other
    // tools also write.
    let document = match parse_json_document(content, source == Source::Settings) {
        Ok(document) => document,
        Err(error) => {
            if schema && source == Source::HookFile {
                cx.push_at(
                    DiagnosticLevel::Error,
                    "COP-017",
                    (error.line, error.column),
                    t!("rules.cop_017.parse_error", error = error.message.as_str()),
                    t!("rules.cop_017.suggestion"),
                );
            }
            return cx.diagnostics;
        }
    };
    let root = &document.root;
    let Some(members) = root.value.as_object() else {
        if schema && source == Source::HookFile {
            cx.push(
                DiagnosticLevel::Error,
                "COP-017",
                node_span(root),
                t!("rules.cop_017.root_not_object"),
                t!("rules.cop_017.suggestion"),
            );
        }
        return cx.diagnostics;
    };

    let hooks_member = root.value.get("hooks");
    if source == Source::Settings && hooks_member.is_none() {
        // A settings file without inline hooks has nothing for these rules.
        return cx.diagnostics;
    }

    let event_names: Vec<&str> = hooks_member
        .and_then(|m| m.value.value.as_object())
        .map(|events| events.iter().map(|e| e.key.as_str()).collect())
        .unwrap_or_default();
    let version = root.value.get("version");
    let all_pascal = !event_names.is_empty()
        && event_names
            .iter()
            .all(|name| name.chars().next().is_some_and(|c| c.is_ascii_uppercase()));
    let format = if source == Source::HookFile && version.is_none() && all_pascal {
        Format::Native
    } else {
        Format::Copilot
    };

    if schema && source == Source::HookFile {
        match version {
            None if format == Format::Native => {
                // Documented VS Code configuration: report only that the Copilot
                // CLI and cloud agent, which read the same folder, skip it.
                let span = hooks_member.map_or(node_span(root), key_span);
                cx.push(
                    DiagnosticLevel::Info,
                    "COP-017",
                    span,
                    t!("rules.cop_017.native_no_version"),
                    t!("rules.cop_017.native_no_version_suggestion"),
                );
            }
            None => {
                let span = members.first().map_or(node_span(root), key_span);
                cx.push(
                    DiagnosticLevel::Error,
                    "COP-017",
                    span,
                    t!("rules.cop_017.missing_version"),
                    t!("rules.cop_017.suggestion"),
                );
            }
            Some(member) => {
                let is_one = member
                    .value
                    .value
                    .as_number()
                    .and_then(|n| n.as_f64())
                    .is_some_and(|n| n == 1.0);
                if !is_one {
                    let got = describe(&member.value);
                    cx.push(
                        DiagnosticLevel::Error,
                        "COP-017",
                        node_span(&member.value),
                        t!("rules.cop_017.invalid_version", got = got.as_str()),
                        t!("rules.cop_017.suggestion"),
                    );
                }
            }
        }
    }

    let Some(hooks_member) = hooks_member else {
        if schema {
            cx.push(
                DiagnosticLevel::Error,
                "COP-017",
                node_span(root),
                t!("rules.cop_017.missing_hooks"),
                t!("rules.cop_017.suggestion"),
            );
        }
        return cx.diagnostics;
    };
    let Some(events) = hooks_member.value.value.as_object() else {
        if schema {
            cx.push(
                DiagnosticLevel::Error,
                "COP-017",
                node_span(&hooks_member.value),
                t!(
                    "rules.cop_017.hooks_not_object",
                    got = hooks_member.value.value.type_name()
                ),
                t!("rules.cop_017.suggestion"),
            );
        }
        return cx.diagnostics;
    };

    let repo_root = repository_root(path, source);
    for event in events {
        let name = event.key.as_str();
        let known = match format {
            Format::Copilot => {
                COPILOT_HOOK_EVENTS.contains(&name) || PASCAL_CASE_HOOK_EVENTS.contains(&name)
            }
            Format::Native => PASCAL_CASE_HOOK_EVENTS.contains(&name),
        };
        if !known && schema {
            let suggestion = match suggest_event(name) {
                Some(close) => t!("rules.cop_017.unknown_event_suggestion", event = close),
                None => t!("rules.cop_017.suggestion"),
            };
            cx.push(
                DiagnosticLevel::Error,
                "COP-017",
                key_span(event),
                t!("rules.cop_017.unknown_event", event = name),
                suggestion,
            );
        }
        let Some(entries) = event.value.value.as_array() else {
            if schema {
                cx.push(
                    DiagnosticLevel::Error,
                    "COP-017",
                    node_span(&event.value),
                    t!(
                        "rules.cop_017.event_not_array",
                        event = name,
                        got = event.value.value.type_name()
                    ),
                    t!("rules.cop_017.suggestion"),
                );
            }
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let at = EntryAt {
                event: name,
                index,
                format,
                source,
            };
            check_entry(&mut cx, entry, &at, repo_root.as_deref());
        }
    }

    cx.diagnostics
}

struct EntryAt<'a> {
    event: &'a str,
    index: usize,
    format: Format,
    source: Source,
}

impl EntryAt<'_> {
    fn suggestion(&self) -> String {
        match self.source {
            Source::Settings => t!("rules.cop_017.settings_suggestion").to_string(),
            Source::HookFile => t!("rules.cop_017.suggestion").to_string(),
        }
    }
}

fn check_entry(cx: &mut JsonCx<'_>, entry: &JsonNode, at: &EntryAt<'_>, repo_root: Option<&Path>) {
    let schema = cx.enabled("COP-017");
    let event = at.event;
    let index = at.index.to_string();
    let Some(fields) = entry.value.as_object() else {
        if schema {
            cx.push(
                DiagnosticLevel::Error,
                "COP-017",
                node_span(entry),
                t!(
                    "rules.cop_017.entry_not_object",
                    event = event,
                    index = index.as_str()
                ),
                at.suggestion(),
            );
        }
        return;
    };

    let kind = match entry.value.get("type") {
        None => Some("command"),
        Some(member) => match member.value.value.as_str() {
            Some(kind @ ("command" | "http" | "prompt")) if at.format == Format::Copilot => {
                Some(kind)
            }
            Some("command") => Some("command"),
            other => {
                if schema {
                    let got = other.map_or_else(|| describe(&member.value), str::to_string);
                    let message = if at.format == Format::Native {
                        t!("rules.cop_017.native_type", got = got.as_str())
                    } else {
                        t!(
                            "rules.cop_017.invalid_type",
                            event = event,
                            index = index.as_str(),
                            got = got.as_str()
                        )
                    };
                    cx.push(
                        DiagnosticLevel::Error,
                        "COP-017",
                        node_span(&member.value),
                        message,
                        at.suggestion(),
                    );
                }
                None
            }
        },
    };

    // Typed optional fields: a wrong type makes Copilot drop the entry.
    if schema {
        for field in fields {
            let expected = match field.key.as_str() {
                "bash" | "powershell" | "cwd" | "exec" | "url" | "prompt" | "matcher"
                | "windows" | "linux" | "osx" => {
                    Some(("a string", field.value.value.as_str().is_some()))
                }
                "env" | "headers" => Some((
                    "an object of strings",
                    field.value.value.as_object().is_some_and(|items| {
                        items.iter().all(|item| item.value.value.as_str().is_some())
                    }),
                )),
                "args" | "allowedEnvVars" => Some((
                    "an array of strings",
                    field
                        .value
                        .value
                        .as_array()
                        .is_some_and(|items| items.iter().all(|i| i.value.as_str().is_some())),
                )),
                "timeoutSec" | "timeout" => Some((
                    "a positive number of seconds",
                    field
                        .value
                        .value
                        .as_number()
                        .and_then(|n| n.as_f64())
                        .is_some_and(|n| n > 0.0),
                )),
                // `command` is a string; the nested `{ "bash": ... }` object
                // is reported below with its own message.
                _ => None,
            };
            if let Some((expected, true)) = expected.map(|(e, ok)| (e, !ok)) {
                cx.push(
                    DiagnosticLevel::Error,
                    "COP-017",
                    node_span(&field.value),
                    t!(
                        "rules.cop_017.field_type",
                        field = field.key.as_str(),
                        event = event,
                        index = index.as_str(),
                        expected = expected,
                        got = field.value.value.type_name()
                    ),
                    at.suggestion(),
                );
            }
        }
    }

    match kind {
        Some("command") => check_command_entry(cx, entry, at, repo_root),
        Some("http") => check_http_entry(cx, entry, at),
        Some("prompt") => {
            let has_prompt = str_field(entry, "prompt").is_some_and(|p| !p.trim().is_empty());
            if schema && !has_prompt {
                cx.push(
                    DiagnosticLevel::Error,
                    "COP-017",
                    node_span(entry),
                    t!(
                        "rules.cop_017.missing_prompt",
                        event = event,
                        index = index.as_str()
                    ),
                    at.suggestion(),
                );
            }
            if cx.enabled("COP-031") && !matches!(event, "sessionStart" | "SessionStart") {
                let span = entry.value.get("type").map_or(node_span(entry), key_span);
                cx.push(
                    DiagnosticLevel::Error,
                    "COP-031",
                    span,
                    t!("rules.cop_031.prompt_event", event = event),
                    t!("rules.cop_031.prompt_event_suggestion"),
                );
            }
        }
        _ => {}
    }

    if let Some(matcher) = entry.value.get("matcher")
        && let Some(pattern) = matcher.value.value.as_str()
        && cx.enabled("COP-031")
        && let Some(error) = matcher_error(event, pattern)
    {
        cx.push(
            DiagnosticLevel::Error,
            "COP-031",
            node_span(&matcher.value),
            t!(
                "rules.cop_031.invalid_matcher",
                matcher = pattern,
                error = error.as_str()
            ),
            t!("rules.cop_031.invalid_matcher_suggestion"),
        );
    }

    if cx.enabled("COP-037") {
        check_secrets(cx, entry);
    }
}

fn check_command_entry(
    cx: &mut JsonCx<'_>,
    entry: &JsonNode,
    at: &EntryAt<'_>,
    repo_root: Option<&Path>,
) {
    let schema = cx.enabled("COP-017");
    let event = at.event;
    let index = at.index.to_string();
    let non_empty = |field: &str| str_field(entry, field).is_some_and(|v| !v.trim().is_empty());

    if let Some(command) = entry.value.get("command")
        && command.value.value.as_object().is_some()
    {
        if schema {
            cx.push(
                DiagnosticLevel::Error,
                "COP-017",
                node_span(&command.value),
                t!(
                    "rules.cop_017.command_object",
                    event = event,
                    index = index.as_str()
                ),
                at.suggestion(),
            );
        }
        return;
    }

    let has_exec = at.format == Format::Copilot && non_empty("exec");
    let has_shell = COMMAND_FIELDS.iter().any(|field| non_empty(field));
    if schema && !has_exec && !has_shell {
        let message = if at.format == Format::Native {
            t!(
                "rules.cop_017.native_missing_command",
                event = event,
                index = index.as_str()
            )
        } else {
            t!(
                "rules.cop_017.missing_command",
                event = event,
                index = index.as_str()
            )
        };
        cx.push(
            DiagnosticLevel::Error,
            "COP-017",
            node_span(entry),
            message,
            at.suggestion(),
        );
    }

    // COP-031: "Do not combine `exec` with `bash`, `powershell`, or `command`."
    if at.format == Format::Copilot && entry.value.get("exec").is_some() && cx.enabled("COP-031") {
        for field in ["bash", "powershell", "command"] {
            if let Some(member) = entry.value.get(field) {
                cx.push(
                    DiagnosticLevel::Error,
                    "COP-031",
                    key_span(member),
                    t!("rules.cop_031.exec_combined", field = field),
                    t!("rules.cop_031.exec_combined_suggestion"),
                );
            }
        }
    }

    // COP-036: cloud agent honors only `bash`, with `command` as fallback.
    if at.format == Format::Copilot && at.source == Source::HookFile && cx.enabled("COP-036") {
        if let Some(powershell) = entry.value.get("powershell")
            && !non_empty("bash")
            && !non_empty("command")
            && !has_exec
        {
            cx.push(
                DiagnosticLevel::Info,
                "COP-036",
                key_span(powershell),
                t!(
                    "rules.cop_036.powershell_only",
                    event = event,
                    index = index.as_str()
                ),
                t!("rules.cop_036.powershell_only_suggestion"),
            );
        }
        if let Some(root) = repo_root {
            check_script(cx, entry, root);
        }
    }
}

/// COP-036: a relative script path in `bash` or `command` that does not exist
/// at the repository root (where hook commands run unless `cwd` is set), or
/// exists without a shebang line.
fn check_script(cx: &mut JsonCx<'_>, entry: &JsonNode, root: &Path) {
    if entry.value.get("cwd").is_some() {
        return;
    }
    for field in ["bash", "command"] {
        let Some(member) = entry.value.get(field) else {
            continue;
        };
        let Some(command) = member.value.value.as_str() else {
            continue;
        };
        let Some(script) = relative_script(command) else {
            continue;
        };
        let full = root.join(script.trim_start_matches("./"));
        let fs = cx.config.fs();
        if !fs.exists(&full) {
            cx.push(
                DiagnosticLevel::Info,
                "COP-036",
                node_span(&member.value),
                t!("rules.cop_036.script_missing", script = script),
                t!("rules.cop_036.script_missing_suggestion"),
            );
        } else if fs.is_file(&full)
            && let Ok(text) = fs.read_to_string(&full)
            && !text.starts_with("#!")
        {
            cx.push(
                DiagnosticLevel::Info,
                "COP-036",
                node_span(&member.value),
                t!("rules.cop_036.no_shebang", script = script),
                t!("rules.cop_036.no_shebang_suggestion"),
            );
        }
        return;
    }
}

/// The first word of a shell command when it is a relative path to a script
/// file: it has a `/`, no shell syntax, and a script extension (`.sh`,
/// `.bash`, `.py`, ...). Interpreter invocations such as `bash x.sh` and
/// commands with variables are skipped.
pub(crate) fn relative_script(command: &str) -> Option<&str> {
    let first = command.split_whitespace().next()?;
    let is_path = first.contains('/')
        && !first.starts_with('/')
        && !first.starts_with('~')
        && !first.contains([
            '$', '`', '"', '\'', '*', '?', '(', ')', ';', '&', '|', '<', '>',
        ]);
    let script_ext = [
        ".sh", ".bash", ".zsh", ".py", ".js", ".mjs", ".ts", ".rb", ".pl",
    ]
    .iter()
    .any(|ext| first.ends_with(ext));
    (is_path && script_ext).then_some(first)
}

fn check_http_entry(cx: &mut JsonCx<'_>, entry: &JsonNode, at: &EntryAt<'_>) {
    let event = at.event;
    let index = at.index.to_string();
    let Some(url_member) = entry.value.get("url") else {
        if cx.enabled("COP-017") {
            cx.push(
                DiagnosticLevel::Error,
                "COP-017",
                node_span(entry),
                t!(
                    "rules.cop_017.missing_url",
                    event = event,
                    index = index.as_str()
                ),
                at.suggestion(),
            );
        }
        return;
    };
    let Some(url) = url_member.value.value.as_str() else {
        return;
    };
    if !cx.enabled("COP-031") {
        return;
    }
    let span = node_span(&url_member.value);
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("https://") {
        return;
    }
    if !lower.starts_with("http://") {
        cx.push(
            DiagnosticLevel::Error,
            "COP-031",
            span,
            t!("rules.cop_031.url_scheme", url = url),
            t!("rules.cop_031.https_suggestion"),
        );
        return;
    }
    let host = lower["http://".len()..]
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("");
    let host_name = if host.starts_with('[') {
        host.split(']')
            .next()
            .map(|h| format!("{h}]"))
            .unwrap_or_default()
    } else {
        host.split(':').next().unwrap_or("").to_string()
    };
    let localhost =
        host_name == "localhost" || host_name.starts_with("127.") || host_name == "[::1]";
    if PERMISSION_EVENTS.contains(&event) {
        cx.push(
            DiagnosticLevel::Error,
            "COP-031",
            span,
            t!("rules.cop_031.http_permission", event = event),
            t!("rules.cop_031.https_suggestion"),
        );
    } else if let Some(allowed) = entry.value.get("allowedEnvVars") {
        cx.push(
            DiagnosticLevel::Error,
            "COP-031",
            key_span(allowed),
            t!("rules.cop_031.allowed_env_vars"),
            t!("rules.cop_031.https_suggestion"),
        );
    } else if localhost {
        cx.push(
            DiagnosticLevel::Info,
            "COP-031",
            span,
            t!("rules.cop_031.http_localhost"),
            t!("rules.cop_031.http_localhost_suggestion"),
        );
    } else {
        cx.push(
            DiagnosticLevel::Error,
            "COP-031",
            span,
            t!("rules.cop_031.http_scheme", url = url),
            t!("rules.cop_031.https_suggestion"),
        );
    }
}

/// Why Copilot skips an entry with this matcher, or `None` when it compiles.
/// Copilot compiles the matcher as the JavaScript regex `^(?:PATTERN)$`.
/// PascalCase `PreToolUse` and `PermissionRequest` hooks use Claude's matcher
/// semantics instead, where `*`, `**`, an empty value and `|`-separated names
/// are not regular expressions.
fn matcher_error(event: &str, pattern: &str) -> Option<String> {
    if CLAUDE_MATCHER_EVENTS.contains(&event) {
        let literal_names = pattern.split('|').all(|token| {
            token
                .trim()
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        });
        if matches!(pattern.trim(), "" | "*" | "**") || literal_names {
            return None;
        }
    }
    regress::Regex::new(&format!("^(?:{pattern})$"))
        .err()
        .map(|error| error.to_string())
}

/// COP-037: literal secrets in `env` values and `headers` values.
fn check_secrets(cx: &mut JsonCx<'_>, entry: &JsonNode) {
    for field in ["env", "headers"] {
        let Some(items) = entry
            .value
            .get(field)
            .and_then(|member| member.value.value.as_object())
        else {
            continue;
        };
        for item in items {
            let Some(value) = item.value.value.as_str() else {
                continue;
            };
            if is_secret_entry(&item.key, value) {
                cx.push(
                    DiagnosticLevel::Warning,
                    "COP-037",
                    node_span(&item.value),
                    t!(
                        "rules.cop_037.message",
                        field = field,
                        key = item.key.as_str()
                    ),
                    t!("rules.cop_037.suggestion"),
                );
            }
        }
    }
}

/// A name that suggests a credential: token, secret, password, API key,
/// authorization header and similar.
pub(crate) fn is_sensitive_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [
        "token",
        "secret",
        "password",
        "passwd",
        "api_key",
        "apikey",
        "api-key",
        "access_key",
        "private_key",
        "authorization",
        "credential",
    ]
    .iter()
    .any(|word| lower.contains(word))
}

/// A `key: value` pair whose value looks like a literal credential rather
/// than a variable reference or placeholder.
pub(crate) fn is_secret_entry(key: &str, value: &str) -> bool {
    if !is_sensitive_name(key) {
        return false;
    }
    let bare = ["Bearer ", "bearer ", "Basic ", "basic ", "Token ", "token "]
        .iter()
        .find_map(|scheme| value.strip_prefix(scheme))
        .unwrap_or(value);
    crate::rules::seems_plaintext_secret(bare)
}

/// The repository root for a repository hook file (`.github/hooks/x.json` is
/// three levels below it). User-level files have no repository root.
fn repository_root(path: &Path, source: Source) -> Option<std::path::PathBuf> {
    if source != Source::HookFile {
        return None;
    }
    let hooks_dir = path.parent()?;
    let dot_github = hooks_dir.parent()?;
    let name = dot_github.file_name()?.to_str()?;
    // A bare relative path (`.github/hooks/x.json`) has no known root; the
    // check would depend on the process working directory.
    name.eq_ignore_ascii_case(".github")
        .then(|| dot_github.parent().map(Path::to_path_buf))
        .flatten()
        .filter(|root| !root.as_os_str().is_empty())
}

/// A documented event that differs only by case or by a small edit.
fn suggest_event(name: &str) -> Option<&'static str> {
    let all: Vec<&'static str> = COPILOT_HOOK_EVENTS
        .iter()
        .chain(PASCAL_CASE_HOOK_EVENTS)
        .copied()
        .collect();
    crate::schemas::kiro_json::closest_name(name, &all)
}

fn describe(node: &JsonNode) -> String {
    match &node.value {
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => format!("\"{s}\""),
        other => other.type_name().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use crate::config::LintConfig;
    use crate::diagnostics::{Diagnostic, DiagnosticLevel};
    use crate::rules::Validator;
    use crate::rules::copilot::CopilotValidator;
    use std::path::Path;

    fn validate_at(path: &str, content: &str) -> Vec<Diagnostic> {
        CopilotValidator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn validate(content: &str) -> Vec<Diagnostic> {
        validate_at(".github/hooks/policy.json", content)
    }

    fn rules(diagnostics: &[Diagnostic], rule: &str) -> Vec<Diagnostic> {
        diagnostics
            .iter()
            .filter(|d| d.rule == rule)
            .cloned()
            .collect()
    }

    // The rule knew 6 events, required `type: "command"` and a nested
    // `command: {"bash": ...}` object. The hooks reference documents 14
    // events, `type` "Defaults to `\"command\"` when omitted", and `bash`,
    // `powershell`, `command` and `exec` strings on the entry.
    #[test]
    fn documented_copilot_format_is_clean() {
        let content = r#"{
  "version": 1,
  "disableAllHooks": false,
  "hooks": {
    "sessionStart": [{ "type": "prompt", "prompt": "/status" }],
    "preToolUse": [
      { "bash": "./scripts/check.sh", "powershell": "./scripts/check.ps1", "timeoutSec": 10, "matcher": "bash|edit" },
      { "type": "command", "exec": "node", "args": ["guard.js"] }
    ],
    "postToolUse": [{ "type": "http", "url": "https://hooks.example.com/copilot", "headers": { "X-Source": "copilot-cli" }, "allowedEnvVars": ["GITHUB_TOKEN"] }],
    "agentStop": [{ "command": "echo done" }],
    "errorOccurred": [{ "bash": "echo err" }],
    "notification": [{ "bash": "echo n", "matcher": "permission_prompt" }],
    "permissionRequest": [{ "bash": "echo p" }],
    "postToolUseFailure": [{ "bash": "echo f" }],
    "preCompact": [{ "bash": "echo c", "matcher": "auto" }],
    "sessionEnd": [{ "bash": "echo e", "timeout": 5 }],
    "subagentStart": [{ "bash": "echo s" }],
    "subagentStop": [{ "bash": "echo s" }],
    "userPromptSubmitted": [{ "bash": "echo u" }],
    "userPromptTransformed": [{ "bash": "echo t" }],
    "PreToolUse": [{ "command": "echo vscode", "matcher": "*" }]
  }
}"#;
        let diagnostics = validate(content);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }

    #[test]
    fn vs_code_native_format_is_a_note_only() {
        let content = r#"{
  "hooks": {
    "PreToolUse": [{ "type": "command", "command": "./scripts/validate-tool.sh", "windows": "powershell -File scripts\\validate-tool.ps1", "timeout": 15 }],
    "PostToolUse": [{ "type": "command", "command": "npx prettier --write ." }]
  }
}"#;
        let diagnostics = validate(content);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
        assert_eq!(diagnostics[0].rule, "COP-017");
        assert_eq!(diagnostics[0].level, DiagnosticLevel::Info);
    }

    #[test]
    fn structural_errors() {
        let missing_version = validate(r#"{ "hooks": { "preToolUse": [{ "bash": "x" }] } }"#);
        assert_eq!(
            rules(&missing_version, "COP-017")[0].level,
            DiagnosticLevel::Error
        );

        let bad_version = validate(r#"{ "version": 2, "hooks": {} }"#);
        let cop_017 = rules(&bad_version, "COP-017");
        assert_eq!(cop_017.len(), 1);
        assert_eq!(cop_017[0].line, 1);

        // The old array form with `events` lists is not the documented shape.
        let array_form = validate(
            r#"{ "version": 1, "hooks": [{ "type": "command", "events": ["sessionStart"], "command": { "bash": "echo" } }] }"#,
        );
        assert!(
            rules(&array_form, "COP-017")
                .iter()
                .any(|d| d.message.contains("must be an object"))
        );

        let not_array = validate(r#"{ "version": 1, "hooks": { "preToolUse": {} } }"#);
        assert!(!rules(&not_array, "COP-017").is_empty());
    }

    #[test]
    fn entry_errors_point_at_the_value() {
        let content = "{\n  \"version\": 1,\n  \"hooks\": {\n    \"preToolUse\": [\n      { \"type\": \"script\", \"bash\": \"x\" },\n      { \"command\": { \"bash\": \"echo\" } },\n      { \"timeoutSec\": \"30\", \"bash\": \"x\" },\n      { \"type\": \"http\" },\n      {}\n    ],\n    \"sessionStrat\": [{ \"bash\": \"x\" }]\n  }\n}";
        let diagnostics = validate(content);
        let cop_017 = rules(&diagnostics, "COP-017");
        assert!(
            cop_017
                .iter()
                .any(|d| d.line == 5 && d.message.contains("type"))
        );
        assert!(
            cop_017
                .iter()
                .any(|d| d.line == 6 && d.message.contains("must be a string"))
        );
        assert!(
            cop_017
                .iter()
                .any(|d| d.line == 7 && d.message.contains("timeoutSec"))
        );
        assert!(
            cop_017
                .iter()
                .any(|d| d.line == 8 && d.message.contains("url"))
        );
        assert!(
            cop_017
                .iter()
                .any(|d| d.line == 9 && d.message.contains("needs a non-empty"))
        );
        let unknown = cop_017
            .iter()
            .find(|d| d.message.contains("sessionStrat"))
            .unwrap();
        assert_eq!(unknown.line, 11);
        assert!(
            unknown
                .suggestion
                .as_deref()
                .unwrap()
                .contains("sessionStart")
        );
    }

    #[test]
    fn cop_031_constraints() {
        let content = r#"{
  "version": 1,
  "hooks": {
    "postToolUse": [
      { "type": "http", "url": "http://hooks.example.com" },
      { "type": "http", "url": "http://localhost:8080/hook" },
      { "type": "http", "url": "http://127.0.0.1/hook", "allowedEnvVars": ["TOKEN"] },
      { "type": "http", "url": "ftp://example.com" }
    ],
    "preToolUse": [
      { "type": "http", "url": "http://localhost:8080/hook" },
      { "exec": "node", "bash": "node x.js" },
      { "bash": "x", "matcher": "*" },
      { "bash": "x", "matcher": "(unclosed" }
    ],
    "PreToolUse": [{ "command": "x", "matcher": "Edit|Write" }, { "command": "x", "matcher": "**" }],
    "userPromptSubmitted": [{ "type": "prompt", "prompt": "/x" }]
  }
}"#;
        let diagnostics = validate(content);
        let cop_031 = rules(&diagnostics, "COP-031");
        let errors: Vec<_> = cop_031
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();
        let infos: Vec<_> = cop_031
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Info)
            .collect();
        // http non-TLS, allowedEnvVars, bad scheme, preToolUse http, exec+bash,
        // "*" and "(unclosed" matchers, prompt off sessionStart.
        assert_eq!(errors.len(), 8, "{cop_031:#?}");
        assert_eq!(infos.len(), 1, "localhost note: {cop_031:#?}");
        assert!(errors.iter().any(|d| d.message.contains("sessionStart")));
        // Claude-format matchers on PascalCase PreToolUse are not regexes.
        assert!(!errors.iter().any(|d| d.message.contains("Edit|Write")));
    }

    #[test]
    fn cop_036_cloud_agent_and_scripts() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join(".github/hooks")).unwrap();
        std::fs::create_dir_all(root.join("scripts")).unwrap();
        std::fs::write(root.join("scripts/no-shebang.sh"), "echo hi\n").unwrap();
        std::fs::write(root.join("scripts/ok.sh"), "#!/bin/bash\necho hi\n").unwrap();
        let path = root.join(".github/hooks/hooks.json");
        let content = r#"{
  "version": 1,
  "hooks": {
    "preToolUse": [
      { "powershell": "./scripts/check.ps1" },
      { "bash": "./scripts/missing.sh" },
      { "bash": "./scripts/no-shebang.sh" },
      { "bash": "scripts/ok.sh --flag" },
      { "bash": "bash scripts/missing.sh" },
      { "bash": "./hooks/x.sh", "cwd": "tools" }
    ]
  }
}"#;
        let diagnostics = CopilotValidator.validate(&path, content, &LintConfig::default());
        let cop_036 = rules(&diagnostics, "COP-036");
        assert_eq!(cop_036.len(), 3, "{cop_036:#?}");
        assert!(cop_036.iter().all(|d| d.level == DiagnosticLevel::Info));
        assert!(cop_036.iter().any(|d| d.message.contains("powershell")));
        assert!(cop_036.iter().any(|d| d.message.contains("missing.sh")));
        assert!(cop_036.iter().any(|d| d.message.contains("shebang")));
    }

    #[test]
    fn cop_037_literal_secrets() {
        let content = r#"{
  "version": 1,
  "hooks": {
    "postToolUse": [
      { "bash": "x", "env": { "API_TOKEN": "ghp_1234567890abcdefghijklmnop", "MODE": "strict", "SECRET": "$MY_SECRET" } },
      { "type": "http", "url": "https://h.example.com", "headers": { "Authorization": "Bearer abcdef1234567890abcdef", "X-Api-Key": "${KEY}" } }
    ]
  }
}"#;
        let diagnostics = validate(content);
        let cop_037 = rules(&diagnostics, "COP-037");
        assert_eq!(cop_037.len(), 2, "{cop_037:#?}");
        assert!(cop_037.iter().all(|d| d.line == 5 || d.line == 6));
    }

    // Inline hooks in settings.json are strict, and a settings file without
    // hooks has nothing to check (it is not a Claude settings file).
    #[test]
    fn settings_files() {
        let path = ".github/copilot/settings.json";
        assert_eq!(
            crate::detect_file_type(Path::new(path)),
            crate::FileType::CopilotHooks
        );
        assert!(validate_at(path, r#"{ "enabledPlugins": {} }"#).is_empty());
        let diagnostics = validate_at(
            path,
            r#"{ "hooks": { "preToolUse": [{ "type": "nope", "bash": "x" }] } }"#,
        );
        let cop_017 = rules(&diagnostics, "COP-017");
        assert_eq!(cop_017.len(), 1);
        assert!(
            cop_017[0]
                .suggestion
                .as_deref()
                .unwrap()
                .contains("whole hooks field")
        );
        // No version is needed in settings files.
        assert!(
            validate_at(path, r#"{ "hooks": { "preToolUse": [{ "bash": "x" }] } }"#).is_empty()
        );
    }

    #[test]
    fn relative_script_detection() {
        assert_eq!(super::relative_script("./x.sh"), Some("./x.sh"));
        assert_eq!(
            super::relative_script("scripts/x.py --a"),
            Some("scripts/x.py")
        );
        assert_eq!(super::relative_script("bash scripts/x.sh"), None);
        assert_eq!(super::relative_script("/abs/x.sh"), None);
        assert_eq!(super::relative_script("$HOME/x.sh"), None);
        assert_eq!(super::relative_script("npx prettier"), None);
    }
}
