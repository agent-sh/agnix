//! Handler and matcher checks shared by every place Claude Code reads hooks:
//! settings files, plugin `hooks/hooks.json`, and skill or subagent
//! frontmatter.
//!
//! - CC-HK-010: SessionEnd timeout above the 60-second budget cap, or a plugin
//!   SessionEnd timeout, which never raises the 1.5-second budget
//! - CC-HK-029: `onFailure` value that is invalid or has no effect
//! - CC-HK-030: path placeholder outside double quotes in a shell-form command,
//!   and (LOW) a script run by a relative path
//! - CC-HK-031: exec-form `command` that carries its own arguments, and
//!   `shell` set together with `args`
//! - CC-HK-032: bare `$CLAUDE_PROJECT_DIR` or a single-quoted placeholder in a
//!   PowerShell shell-form hook
//! - CC-HK-033: MCP matcher written as a bare server prefix
//! - CC-HK-034: plugin hook that names the plugin's own MCP server by its bare
//!   key
//! - CC-HK-035: PermissionRequest hook with no narrow matcher
//! - CC-HK-025 (extension): tool-event matcher that differs from a built-in
//!   tool name only in letter case
//!
//! The checks work on `serde_json::Value` so YAML frontmatter hooks (converted
//! with serde_json) and JSON hook files share one implementation. Each finding
//! carries the JSON path of the offending key or value; the caller resolves it
//! to a line and column.

use crate::config::PerFileLintConfig;
use crate::diagnostics::DiagnosticLevel;
use crate::schemas::hooks::HooksSchema;
use rust_i18n::t;
use serde_json::Value;

/// Where the hooks being checked were declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HookOrigin {
    /// `settings.json`, `settings.local.json` or managed settings
    Settings,
    /// A plugin's `hooks/hooks.json`
    PluginHooksFile,
    /// Hooks in skill or subagent frontmatter
    Frontmatter,
}

/// The plugin a hooks file belongs to, for CC-HK-034.
#[derive(Debug, Clone, Default)]
pub(crate) struct PluginServers {
    /// Plugin name used in scoped MCP names (`plugin.json` `name`, or the
    /// plugin directory name).
    pub(crate) plugin: String,
    /// Keys of the MCP servers the plugin itself declares.
    pub(crate) servers: Vec<String>,
}

/// An owned JSON path segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PathSeg {
    Key(String),
    Index(usize),
}

/// One finding; `path` names the key or value it is about, relative to the
/// event map, and `on_key` says whether to point at the key or the value.
#[derive(Debug, Clone)]
pub(crate) struct HookFinding {
    pub(crate) rule: &'static str,
    pub(crate) level: DiagnosticLevel,
    pub(crate) path: Vec<PathSeg>,
    pub(crate) on_key: bool,
    pub(crate) message: String,
    pub(crate) suggestion: String,
    /// Replacement text for the value at `path` (a JSON string value), offered
    /// as an unsafe fix where the caller can locate the exact span.
    pub(crate) replacement: Option<String>,
}

/// Built-in tool names, for the CC-HK-025 case check. Matches the tool names
/// the hooks reference lists for PreToolUse plus the other built-in tools.
const BUILTIN_TOOLS: &[&str] = &[
    "Agent",
    "AskUserQuestion",
    "Bash",
    "Edit",
    "EndConversation",
    "EnterPlanMode",
    "ExitPlanMode",
    "Glob",
    "Grep",
    "LSP",
    "MultiEdit",
    "NotebookEdit",
    "PowerShell",
    "Read",
    "Skill",
    "Task",
    "TodoWrite",
    "WebFetch",
    "WebSearch",
    "Workflow",
    "Write",
];

const PLACEHOLDERS: &[&str] = &[
    "CLAUDE_PROJECT_DIR",
    "CLAUDE_PLUGIN_ROOT",
    "CLAUDE_PLUGIN_DATA",
];

/// Events where `onFailure` has no effect: exit code 2 already sends Claude
/// back to work there.
const NO_ON_FAILURE_EVENTS: &[&str] = &["Stop", "SubagentStop", "TaskCompleted", "TeammateIdle"];

fn location(event: &str, matcher_idx: usize, hook_idx: Option<usize>) -> String {
    match hook_idx {
        Some(hook_idx) => format!("hooks.{event}[{matcher_idx}].hooks[{hook_idx}]"),
        None => format!("hooks.{event}[{matcher_idx}]"),
    }
}

fn hook_path(event: &str, matcher_idx: usize, hook_idx: usize, key: &str) -> Vec<PathSeg> {
    vec![
        PathSeg::Key(event.to_string()),
        PathSeg::Index(matcher_idx),
        PathSeg::Key("hooks".to_string()),
        PathSeg::Index(hook_idx),
        PathSeg::Key(key.to_string()),
    ]
}

fn matcher_path(event: &str, matcher_idx: usize) -> Vec<PathSeg> {
    vec![
        PathSeg::Key(event.to_string()),
        PathSeg::Index(matcher_idx),
        PathSeg::Key("matcher".to_string()),
    ]
}

/// The pinned Claude Code version when it is older than 2.`minor`.`patch`.
/// Only used for features released within the last few months (onFailure,
/// v2.1.295, 2026-10-08).
fn version_below(config: &PerFileLintConfig<'_>, minor: u64, patch: u64) -> Option<String> {
    let pinned = config.get_claude_code_version()?;
    let version = semver::Version::parse(pinned.trim_start_matches('v')).ok()?;
    (version < semver::Version::new(2, minor, patch)).then(|| pinned.to_string())
}

/// Whether a matcher takes Claude Code's exact-string path: only letters,
/// digits, `_`, `-`, spaces, `,` and `|`.
fn is_exact_matcher(matcher: &str) -> bool {
    matcher
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ' ' | ',' | '|'))
}

fn exact_alternatives(matcher: &str) -> Vec<&str> {
    matcher
        .split(['|', ','])
        .map(str::trim)
        .filter(|alt| !alt.is_empty())
        .collect()
}

/// Run every handler and matcher check over an event map (the value of
/// `hooks`).
pub(crate) fn check_hooks(
    events: &serde_json::Map<String, Value>,
    origin: HookOrigin,
    plugin: Option<&PluginServers>,
    config: &PerFileLintConfig<'_>,
) -> Vec<HookFinding> {
    let mut findings = Vec::new();
    for (event, groups) in events {
        let Some(groups) = groups.as_array() else {
            continue;
        };
        for (matcher_idx, group) in groups.iter().enumerate() {
            let matcher = group.get("matcher").and_then(Value::as_str);
            let hooks: &[Value] = group
                .get("hooks")
                .and_then(Value::as_array)
                .map_or(&[], Vec::as_slice);
            check_matcher(
                event,
                matcher_idx,
                matcher,
                hooks,
                plugin,
                config,
                &mut findings,
            );
            for (hook_idx, hook) in hooks.iter().enumerate() {
                if hook.is_object() {
                    check_handler(
                        event,
                        matcher_idx,
                        hook_idx,
                        hook,
                        origin,
                        plugin,
                        config,
                        &mut findings,
                    );
                }
            }
        }
    }
    findings
}

fn check_matcher(
    event: &str,
    matcher_idx: usize,
    matcher: Option<&str>,
    hooks: &[Value],
    plugin: Option<&PluginServers>,
    config: &PerFileLintConfig<'_>,
    findings: &mut Vec<HookFinding>,
) {
    let loc = location(event, matcher_idx, None);
    let tool_event = HooksSchema::is_tool_event(event);

    // CC-HK-035: a PermissionRequest hook with no narrow matcher can approve
    // every permission prompt. Background hooks can't decide, so a group of
    // only async hooks is exempt.
    if event == "PermissionRequest"
        && config.is_rule_enabled("CC-HK-035")
        && matcher.is_none_or(|m| matches!(m.trim(), "" | "*" | ".*"))
        && hooks.iter().any(|hook| {
            hook.get("async").and_then(Value::as_bool) != Some(true)
                && hook.get("asyncRewake").and_then(Value::as_bool) != Some(true)
        })
    {
        let shown = matcher.map_or_else(|| "(none)".to_string(), |m| format!("'{m}'"));
        findings.push(HookFinding {
            rule: "CC-HK-035",
            level: DiagnosticLevel::Warning,
            path: if matcher.is_some() {
                matcher_path(event, matcher_idx)
            } else {
                vec![PathSeg::Key(event.to_string()), PathSeg::Index(matcher_idx)]
            },
            on_key: false,
            message: t!(
                "rules.cc_hk_035.message",
                location = loc.as_str(),
                matcher = shown.as_str()
            )
            .to_string(),
            suggestion: t!("rules.cc_hk_035.suggestion").to_string(),
            replacement: None,
        });
    }

    let Some(matcher) = matcher else {
        return;
    };
    if !tool_event {
        return;
    }

    if is_exact_matcher(matcher) {
        let alternatives = exact_alternatives(matcher);
        // CC-HK-033: `mcp__<server>` on the exact path names no tool.
        if config.is_rule_enabled("CC-HK-033") {
            for alt in &alternatives {
                if let Some(rest) = alt.strip_prefix("mcp__")
                    && !rest.is_empty()
                    && !rest.contains("__")
                {
                    let fixed = format!("{alt}__.*");
                    findings.push(HookFinding {
                        rule: "CC-HK-033",
                        level: DiagnosticLevel::Error,
                        path: matcher_path(event, matcher_idx),
                        on_key: false,
                        message: t!(
                            "rules.cc_hk_033.message",
                            matcher = *alt,
                            location = loc.as_str()
                        )
                        .to_string(),
                        suggestion: t!("rules.cc_hk_033.suggestion", fixed = fixed.as_str())
                            .to_string(),
                        // Only a single-alternative matcher can be rewritten:
                        // the regex form changes how other alternatives match.
                        replacement: (alternatives.len() == 1 && matcher.trim() == *alt)
                            .then(|| fixed.clone()),
                    });
                }
            }
        }
        // CC-HK-025: matchers are case-sensitive.
        if config.is_rule_enabled("CC-HK-025") {
            for alt in &alternatives {
                if BUILTIN_TOOLS.contains(alt) {
                    continue;
                }
                if let Some(tool) = BUILTIN_TOOLS
                    .iter()
                    .find(|tool| tool.eq_ignore_ascii_case(alt))
                {
                    findings.push(HookFinding {
                        rule: "CC-HK-025",
                        level: DiagnosticLevel::Warning,
                        path: matcher_path(event, matcher_idx),
                        on_key: false,
                        message: t!(
                            "rules.cc_hk_025.case_message",
                            matcher = *alt,
                            location = loc.as_str(),
                            tool = *tool
                        )
                        .to_string(),
                        suggestion: t!("rules.cc_hk_025.case_suggestion", tool = *tool).to_string(),
                        replacement: None,
                    });
                }
            }
        }
    }

    // CC-HK-034: a matcher against the plugin's own server by its bare key.
    if let Some(plugin) = plugin
        && config.is_rule_enabled("CC-HK-034")
        && let Some(server) = bare_own_server_in_tool_pattern(matcher, plugin)
    {
        findings.push(own_server_finding(
            "rules.cc_hk_034.matcher_message",
            matcher_path(event, matcher_idx),
            matcher,
            &loc,
            server,
            plugin,
        ));
    }
}

/// The plugin server key that `pattern` (a matcher or `if` rule) names as
/// `mcp__<key>__...` or `mcp__<key>`, if any.
fn bare_own_server_in_tool_pattern<'p>(
    pattern: &str,
    plugin: &'p PluginServers,
) -> Option<&'p str> {
    plugin.servers.iter().map(String::as_str).find(|key| {
        let prefix = format!("mcp__{key}");
        pattern.match_indices(&prefix).any(|(start, _)| {
            let before_ok = start == 0
                || !pattern[..start]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
            let after = &pattern[start + prefix.len()..];
            before_ok
                && (after.is_empty()
                    || after.starts_with("__")
                    || !after
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        })
    })
}

fn own_server_finding(
    message_key: &str,
    path: Vec<PathSeg>,
    value: &str,
    loc: &str,
    server: &str,
    plugin: &PluginServers,
) -> HookFinding {
    let scoped_tool = format!("mcp__plugin_{}_{}__", plugin.plugin, server);
    let scoped_server = format!("plugin:{}:{}", plugin.plugin, server);
    HookFinding {
        rule: "CC-HK-034",
        level: DiagnosticLevel::Error,
        path,
        on_key: false,
        message: t!(
            message_key,
            value = value,
            location = loc,
            server = server,
            plugin = plugin.plugin.as_str()
        )
        .to_string(),
        suggestion: t!(
            "rules.cc_hk_034.suggestion",
            scoped_tool = scoped_tool.as_str(),
            scoped_server = scoped_server.as_str()
        )
        .to_string(),
        replacement: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn check_handler(
    event: &str,
    matcher_idx: usize,
    hook_idx: usize,
    hook: &Value,
    origin: HookOrigin,
    plugin: Option<&PluginServers>,
    config: &PerFileLintConfig<'_>,
    findings: &mut Vec<HookFinding>,
) {
    let loc = location(event, matcher_idx, Some(hook_idx));
    let hook_type = hook.get("type").and_then(Value::as_str).unwrap_or("");
    let command = hook.get("command").and_then(Value::as_str);
    let exec_form = hook.get("args").is_some();
    let shell = hook.get("shell").and_then(Value::as_str);
    let is_async = hook.get("async").and_then(Value::as_bool) == Some(true)
        || hook.get("asyncRewake").and_then(Value::as_bool) == Some(true);

    // CC-HK-010: "`SessionEnd` hooks share a 1.5-second budget; if your
    // settings set a longer per-hook `timeout`, Claude Code raises the budget
    // to match, up to 60 seconds", and "Timeouts set on plugin-provided hooks
    // don't raise the budget."
    if event == "SessionEnd"
        && config.is_rule_enabled("CC-HK-010")
        && let Some(timeout) = hook.get("timeout").and_then(Value::as_u64)
    {
        let path = hook_path(event, matcher_idx, hook_idx, "timeout");
        if origin == HookOrigin::PluginHooksFile && timeout > 1 {
            findings.push(HookFinding {
                rule: "CC-HK-010",
                level: DiagnosticLevel::Info,
                path,
                on_key: false,
                message: t!(
                    "rules.cc_hk_010.plugin_message",
                    location = loc.as_str(),
                    timeout = timeout
                )
                .to_string(),
                suggestion: t!("rules.cc_hk_010.plugin_suggestion").to_string(),
                replacement: None,
            });
        } else if origin != HookOrigin::PluginHooksFile && timeout > 60 {
            findings.push(HookFinding {
                rule: "CC-HK-010",
                level: DiagnosticLevel::Warning,
                path,
                on_key: false,
                message: t!(
                    "rules.cc_hk_010.message",
                    location = loc.as_str(),
                    timeout = timeout
                )
                .to_string(),
                suggestion: t!("rules.cc_hk_010.suggestion").to_string(),
                replacement: None,
            });
        }
    }

    // CC-HK-029: onFailure
    if config.is_rule_enabled("CC-HK-029")
        && let Some(on_failure) = hook.get("onFailure")
    {
        let path = hook_path(event, matcher_idx, hook_idx, "onFailure");
        let value = on_failure.as_str();
        if !matches!(value, Some("continue" | "block")) {
            let shown = value.map_or_else(|| on_failure.to_string(), str::to_string);
            findings.push(HookFinding {
                rule: "CC-HK-029",
                level: DiagnosticLevel::Warning,
                path,
                on_key: false,
                message: t!(
                    "rules.cc_hk_029.invalid",
                    location = loc.as_str(),
                    value = shown.as_str()
                )
                .to_string(),
                suggestion: t!("rules.cc_hk_029.invalid_suggestion").to_string(),
                replacement: None,
            });
        } else {
            let reason = if let Some(version) = version_below(config, 1, 295) {
                Some(t!("rules.cc_hk_029.version", version = version.as_str()))
            } else if !matches!(hook_type, "command" | "http") {
                Some(t!("rules.cc_hk_029.wrong_type", hook_type = hook_type))
            } else if NO_ON_FAILURE_EVENTS.contains(&event) {
                Some(t!("rules.cc_hk_029.non_blocking_event", event = event))
            } else if hook_type == "command" && is_async {
                Some(t!("rules.cc_hk_029.background"))
            } else {
                None
            };
            if let Some(reason) = reason {
                findings.push(HookFinding {
                    rule: "CC-HK-029",
                    level: DiagnosticLevel::Info,
                    path,
                    on_key: true,
                    message: t!(
                        "rules.cc_hk_029.no_effect",
                        location = loc.as_str(),
                        reason = reason.as_ref()
                    )
                    .to_string(),
                    suggestion: t!("rules.cc_hk_029.no_effect_suggestion").to_string(),
                    replacement: None,
                });
            }
        }
    }

    // CC-HK-034: an mcp_tool hook or `if` rule naming the plugin's own server
    // by its bare key.
    if let Some(plugin) = plugin
        && config.is_rule_enabled("CC-HK-034")
    {
        if hook_type == "mcp_tool"
            && let Some(server) = hook.get("server").and_then(Value::as_str)
            && plugin.servers.iter().any(|key| key == server)
        {
            findings.push(own_server_finding(
                "rules.cc_hk_034.server_message",
                hook_path(event, matcher_idx, hook_idx, "server"),
                server,
                &loc,
                server,
                plugin,
            ));
        }
        if let Some(rule) = hook.get("if").and_then(Value::as_str)
            && let Some(server) = bare_own_server_in_tool_pattern(rule, plugin)
        {
            findings.push(own_server_finding(
                "rules.cc_hk_034.if_message",
                hook_path(event, matcher_idx, hook_idx, "if"),
                rule,
                &loc,
                server,
                plugin,
            ));
        }
    }

    if hook_type != "command" {
        return;
    }
    let Some(command) = command else {
        return;
    };
    let command_path = hook_path(event, matcher_idx, hook_idx, "command");

    if exec_form {
        // CC-HK-031: exec form takes the executable only.
        if config.is_rule_enabled("CC-HK-031") {
            let trimmed = command.trim();
            if trimmed.chars().any(char::is_whitespace) && !trimmed.contains(['/', '\\']) {
                findings.push(HookFinding {
                    rule: "CC-HK-031",
                    level: DiagnosticLevel::Error,
                    path: command_path.clone(),
                    on_key: false,
                    message: t!(
                        "rules.cc_hk_031.message",
                        command = trimmed,
                        location = loc.as_str()
                    )
                    .to_string(),
                    suggestion: t!("rules.cc_hk_031.suggestion").to_string(),
                    replacement: None,
                });
            }
            if shell.is_some() {
                findings.push(HookFinding {
                    rule: "CC-HK-031",
                    level: DiagnosticLevel::Info,
                    path: hook_path(event, matcher_idx, hook_idx, "shell"),
                    on_key: true,
                    message: t!("rules.cc_hk_031.shell_ignored", location = loc.as_str())
                        .to_string(),
                    suggestion: t!("rules.cc_hk_031.shell_ignored_suggestion").to_string(),
                    replacement: None,
                });
            }
        }
        return;
    }

    if shell == Some("powershell") {
        // CC-HK-032: PowerShell resolves bare `$CLAUDE_PROJECT_DIR` to $null and
        // never expands inside single quotes.
        if config.is_rule_enabled("CC-HK-032")
            && let Some((spelling, single_quoted)) = powershell_placeholder_problem(command)
        {
            findings.push(HookFinding {
                rule: "CC-HK-032",
                level: DiagnosticLevel::Error,
                path: command_path,
                on_key: false,
                message: if single_quoted {
                    t!(
                        "rules.cc_hk_032.single_quoted",
                        placeholder = spelling.as_str(),
                        location = loc.as_str()
                    )
                } else {
                    t!(
                        "rules.cc_hk_032.bare",
                        placeholder = spelling.as_str(),
                        location = loc.as_str()
                    )
                }
                .to_string(),
                suggestion: t!("rules.cc_hk_032.suggestion").to_string(),
                replacement: None,
            });
        }
        return;
    }

    // CC-HK-030: shell form, bash.
    if config.is_rule_enabled("CC-HK-030") {
        if let Some(spelling) = unquoted_placeholder(command) {
            findings.push(HookFinding {
                rule: "CC-HK-030",
                level: DiagnosticLevel::Warning,
                path: command_path,
                on_key: false,
                message: t!(
                    "rules.cc_hk_030.message",
                    placeholder = spelling.as_str(),
                    location = loc.as_str()
                )
                .to_string(),
                suggestion: t!(
                    "rules.cc_hk_030.suggestion",
                    placeholder = spelling.as_str()
                )
                .to_string(),
                replacement: None,
            });
        } else if origin != HookOrigin::Frontmatter
            && let Some(script) = relative_script(command)
        {
            // Skill and agent frontmatter hooks are exempt: the hooks
            // reference's own frontmatter example runs `./scripts/...`.
            let anchor = if origin == HookOrigin::PluginHooksFile {
                "${CLAUDE_PLUGIN_ROOT}"
            } else {
                "${CLAUDE_PROJECT_DIR}"
            };
            findings.push(HookFinding {
                rule: "CC-HK-030",
                level: DiagnosticLevel::Info,
                path: command_path,
                on_key: false,
                message: t!(
                    "rules.cc_hk_030.relative",
                    script = script.as_str(),
                    location = loc.as_str()
                )
                .to_string(),
                suggestion: t!("rules.cc_hk_030.relative_suggestion", anchor = anchor).to_string(),
                replacement: None,
            });
        }
    }
}

/// The first `$NAME` / `${NAME}` placeholder in a shell-form command that is
/// outside double quotes. Single-quoted text counts as quoted.
pub(crate) fn unquoted_placeholder(command: &str) -> Option<String> {
    let bytes = command.as_bytes();
    let mut i = 0;
    let mut in_double = false;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'\\' => {
                i += 2;
                continue;
            }
            b'\'' if !in_double => {
                // Skip to the closing single quote.
                match command[i + 1..].find('\'') {
                    Some(end) => i += end + 2,
                    None => return None,
                }
                continue;
            }
            b'"' => in_double = !in_double,
            b'$' if !in_double => {
                if let Some(spelling) = placeholder_at(&command[i..]) {
                    return Some(spelling);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The placeholder spelling at the start of `text` (`${NAME}` or `$NAME`).
fn placeholder_at(text: &str) -> Option<String> {
    let rest = text.strip_prefix('$')?;
    for name in PLACEHOLDERS {
        if let Some(after) = rest.strip_prefix('{')
            && let Some(tail) = after.strip_prefix(name)
            && tail.starts_with('}')
        {
            return Some(format!("${{{name}}}"));
        }
        if let Some(tail) = rest.strip_prefix(name)
            && !tail
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Some(format!("${name}"));
        }
    }
    None
}

/// For a PowerShell shell-form command: a bare `$CLAUDE_*` spelling (not
/// `${...}` or `$env:`), or a `${CLAUDE_*}` placeholder inside single quotes.
fn powershell_placeholder_problem(command: &str) -> Option<(String, bool)> {
    let bytes = command.as_bytes();
    let mut i = 0;
    let mut in_single = false;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' => {
                // `''` inside a single-quoted string is an escaped quote.
                if in_single && bytes.get(i + 1) == Some(&b'\'') {
                    i += 2;
                    continue;
                }
                in_single = !in_single;
            }
            b'$' => {
                if let Some(spelling) = placeholder_at(&command[i..]) {
                    if in_single {
                        return Some((spelling, true));
                    }
                    if !spelling.starts_with("${") {
                        return Some((spelling, false));
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// A script the command runs by a path relative to the current directory.
fn relative_script(command: &str) -> Option<String> {
    super::helpers::extract_script_paths(command)
        .into_iter()
        .find(|script| {
            let s = script.as_str();
            !(s.starts_with('/')
                || s.starts_with('\\')
                || s.starts_with('~')
                || s.starts_with('$')
                || s.contains("${")
                || (s.len() > 2 && s.as_bytes()[1] == b':'))
                && (s.starts_with("./") || s.starts_with("../") || s.contains('/'))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unquoted_placeholder_detection() {
        assert_eq!(
            unquoted_placeholder("${CLAUDE_PLUGIN_ROOT}/hooks/x.sh"),
            Some("${CLAUDE_PLUGIN_ROOT}".to_string())
        );
        assert_eq!(
            unquoted_placeholder("node $CLAUDE_PROJECT_DIR/x.js"),
            Some("$CLAUDE_PROJECT_DIR".to_string())
        );
        assert_eq!(
            unquoted_placeholder(r#""${CLAUDE_PLUGIN_ROOT}"/scripts/x.sh"#),
            None
        );
        assert_eq!(
            unquoted_placeholder(r#"bash "${CLAUDE_PLUGIN_ROOT}/hooks/x.sh" arg"#),
            None
        );
        assert_eq!(
            unquoted_placeholder("bash -c 'cd ${CLAUDE_PLUGIN_ROOT} && make'"),
            None
        );
        assert_eq!(unquoted_placeholder("echo $CLAUDE_PROJECT_DIRX"), None);
        assert_eq!(unquoted_placeholder(r"echo \${CLAUDE_PLUGIN_ROOT}"), None);
    }

    #[test]
    fn powershell_problems() {
        assert_eq!(
            powershell_placeholder_problem("& \"$CLAUDE_PROJECT_DIR\\x.ps1\""),
            Some(("$CLAUDE_PROJECT_DIR".to_string(), false))
        );
        assert_eq!(
            powershell_placeholder_problem("& '${CLAUDE_PROJECT_DIR}\\x.ps1'"),
            Some(("${CLAUDE_PROJECT_DIR}".to_string(), true))
        );
        assert_eq!(
            powershell_placeholder_problem("& \"$env:CLAUDE_PROJECT_DIR\\x.ps1\""),
            None
        );
        assert_eq!(
            powershell_placeholder_problem("& \"${CLAUDE_PROJECT_DIR}\\x.ps1\""),
            None
        );
    }

    #[test]
    fn relative_script_detection() {
        assert_eq!(
            relative_script("./scripts/check.sh"),
            Some("./scripts/check.sh".to_string())
        );
        assert_eq!(
            relative_script("python3 .claude/hooks/x.py"),
            Some(".claude/hooks/x.py".to_string())
        );
        assert_eq!(relative_script("/abs/x.sh"), None);
        assert_eq!(relative_script("\"$CLAUDE_PROJECT_DIR\"/x.sh"), None);
        assert_eq!(relative_script("x.sh"), None);
    }

    #[test]
    fn own_server_patterns() {
        let plugin = PluginServers {
            plugin: "my-plugin".to_string(),
            servers: vec!["db".to_string()],
        };
        assert_eq!(
            bare_own_server_in_tool_pattern("mcp__db__.*", &plugin),
            Some("db")
        );
        assert_eq!(
            bare_own_server_in_tool_pattern("mcp__db", &plugin),
            Some("db")
        );
        assert_eq!(
            bare_own_server_in_tool_pattern("mcp__plugin_my-plugin_db__.*", &plugin),
            None
        );
        assert_eq!(
            bare_own_server_in_tool_pattern("mcp__dbx__.*", &plugin),
            None
        );
    }
}
