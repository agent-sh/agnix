//! Gemini CLI MCP server checks shared by the settings and extension
//! manifest validators (GM-011, GM-014, GM-015, GM-023), and the diagnostic
//! sink the Gemini JSON validators use.
//!
//! `mcpServers` is the same map in `.gemini/settings.json` and in
//! `gemini-extension.json`: "These servers will be loaded on startup just
//! like MCP servers defined in a `settings.json` file"
//! (geminicli.com/docs/extensions/reference). The differences are what each
//! file may carry: an extension's servers ignore `trust` ("all MCP server
//! configuration options are supported except for `trust`") and should use
//! `${extensionPath}`, while a settings entry can be a partial override of
//! an extension's server (geminicli.com/docs/tools/mcp-server, "Overriding
//! extension configurations").

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    schemas::kiro_json::{JsonMember, JsonNode, JsonValue, LineIndex},
};
use rust_i18n::t;
use std::path::{Path, PathBuf};

/// Server keys a settings.json entry may carry to adjust a server that an
/// extension defines, without a transport of its own: the tool lists, `env`
/// and scalar properties such as `timeout` ("Tool lists are merged
/// securely", "The `env` objects are merged", "Properties like `command`,
/// `url`, and `timeout` are replaced by your local values if provided").
const OVERRIDE_ONLY_KEYS: &[&str] = &[
    "includeTools",
    "excludeTools",
    "env",
    "timeout",
    "trust",
    "description",
];

/// Transports in Gemini's order of precedence: "If multiple are specified,
/// the order of precedence is `httpUrl`, then `url`, then `command`."
const TRANSPORTS: [&str; 3] = ["httpUrl", "url", "command"];

/// The variable an extension uses for its own directory.
const EXTENSION_PATH_VAR: &str = "${extensionPath}";

/// Diagnostics for one JSON file, positioned with a [`LineIndex`].
pub(crate) struct JsonSink<'a, 'c> {
    pub path: &'a Path,
    pub lines: LineIndex<'a>,
    pub config: &'a PerFileLintConfig<'c>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'a, 'c> JsonSink<'a, 'c> {
    pub fn new(path: &'a Path, content: &'a str, config: &'a PerFileLintConfig<'c>) -> Self {
        Self {
            path,
            lines: LineIndex::new(content),
            config,
            diagnostics: Vec::new(),
        }
    }

    pub fn push(
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
        let (line, column) = self.lines.line_col(span.0);
        let (end_line, end_column) = self.lines.line_col(span.1.max(span.0));
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
}

/// Which file an `mcpServers` map comes from.
pub(crate) enum McpSource {
    /// `.gemini/settings.json` of a project.
    ProjectSettings {
        /// `mcp.serverCommand` is set ("A global command to start an MCP
        /// server"), so agnix cannot tell which entries it serves.
        server_command: bool,
    },
    /// `~/.gemini/settings.json`.
    UserSettings { server_command: bool },
    /// `gemini-extension.json`; `root` is the extension directory.
    Extension { root: PathBuf },
}

/// The dashed alternative of an MCP server name that contains `_`.
///
/// geminicli.com/docs/tools/mcp-server ("3. Tool naming and namespaces"):
/// "Do not use underscores (`_`) in your MCP server names (for example, use
/// `my-server` rather than `my_server`). The policy parser splits Fully
/// Qualified Names (`mcp_server_tool`) on the _first_ underscore following
/// the `mcp_` prefix."
pub(crate) fn underscore_free_name(name: &str) -> Option<String> {
    name.contains('_').then(|| name.replace('_', "-"))
}

/// GM-011 message and suggestion for a server name with `_`.
pub(crate) fn gm_011_texts(name: &str, suggested: &str) -> (String, String) {
    (
        t!("rules.gm_011.message", name = name, suggested = suggested).to_string(),
        t!("rules.gm_011.suggestion").to_string(),
    )
}

/// Members of an object as `JSON.parse` keeps them: the last of each key.
pub(crate) fn last_members(members: &[JsonMember]) -> impl Iterator<Item = &JsonMember> {
    members
        .iter()
        .enumerate()
        .filter(|(index, member)| !members[index + 1..].iter().any(|m| m.key == member.key))
        .map(|(_, member)| member)
}

/// JavaScript truthiness of a string property (`if (config.httpUrl)`).
fn truthy_string(server: &JsonValue, key: &str) -> bool {
    server
        .get(key)
        .and_then(|member| member.value.value.as_str())
        .is_some_and(|value| !value.is_empty())
}

/// Check every server of an `mcpServers` member.
pub(crate) fn check_mcp_servers(
    sink: &mut JsonSink<'_, '_>,
    mcp_servers: &JsonMember,
    source: &McpSource,
) {
    let Some(servers) = mcp_servers.value.value.as_object() else {
        return;
    };
    for server in last_members(servers) {
        if let Some(suggested) = underscore_free_name(&server.key) {
            let (message, suggestion) = gm_011_texts(&server.key, &suggested);
            sink.push(
                DiagnosticLevel::Warning,
                "GM-011",
                (server.key_start, server.key_end),
                message,
                suggestion,
            );
        }
        if server.value.value.as_object().is_none() {
            continue;
        }
        check_transport(sink, server, source);
        check_trust(sink, server, source);
        if let McpSource::Extension { root } = source {
            check_extension_portability(sink, server, root);
        }
    }
}

/// GM-014: "At least one of `command`, `url`, or `httpUrl` must be
/// provided. If multiple are specified, the order of precedence is
/// `httpUrl`, then `url`, then `command`."
fn check_transport(sink: &mut JsonSink<'_, '_>, server: &JsonMember, source: &McpSource) {
    let value = &server.value.value;
    let set: Vec<&str> = TRANSPORTS
        .iter()
        .copied()
        .filter(|key| truthy_string(value, key))
        .collect();
    let key_span = (server.key_start, server.key_end);
    if set.is_empty() {
        // `tcp` (websocket) is a transport of MCPServerConfig, though the
        // settings reference does not list it.
        if truthy_string(value, "tcp") {
            return;
        }
        let possible_override = match source {
            McpSource::ProjectSettings { server_command }
            | McpSource::UserSettings { server_command } => {
                *server_command
                    || value.as_object().is_some_and(|members| {
                        members
                            .iter()
                            .all(|m| OVERRIDE_ONLY_KEYS.contains(&m.key.as_str()))
                    })
            }
            McpSource::Extension { .. } => false,
        };
        if !possible_override {
            sink.push(
                DiagnosticLevel::Error,
                "GM-014",
                key_span,
                t!("rules.gm_014.missing", name = server.key.as_str()),
                t!("rules.gm_014.missing_suggestion"),
            );
        }
    } else if set.len() > 1 {
        sink.push(
            DiagnosticLevel::Info,
            "GM-014",
            key_span,
            t!(
                "rules.gm_014.multiple",
                name = server.key.as_str(),
                fields = set.join(", "),
                winner = set[0]
            ),
            t!("rules.gm_014.multiple_suggestion"),
        );
    }
}

/// GM-015: "The `trust` option bypasses all confirmation dialogs. Use
/// cautiously and only for servers you completely control"; extensions:
/// "all MCP server configuration options are supported except for `trust`".
fn check_trust(sink: &mut JsonSink<'_, '_>, server: &JsonMember, source: &McpSource) {
    let Some(trust) = server.value.value.get("trust") else {
        return;
    };
    if trust.value.value.as_bool() != Some(true) {
        return;
    }
    let span = (trust.value.start, trust.value.end);
    let name = server.key.as_str();
    match source {
        McpSource::ProjectSettings { .. } => sink.push(
            DiagnosticLevel::Warning,
            "GM-015",
            span,
            t!("rules.gm_015.project", name = name),
            t!("rules.gm_015.suggestion"),
        ),
        McpSource::UserSettings { .. } => sink.push(
            DiagnosticLevel::Info,
            "GM-015",
            span,
            t!("rules.gm_015.user", name = name),
            t!("rules.gm_015.suggestion"),
        ),
        McpSource::Extension { .. } => sink.push(
            DiagnosticLevel::Info,
            "GM-015",
            span,
            t!("rules.gm_015.extension", name = name),
            t!("rules.gm_015.extension_suggestion"),
        ),
    }
}

/// A path value that is relative to the working directory (not absolute,
/// not a variable, not `~`).
fn is_relative_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    !(value.is_empty()
        || value.starts_with(['/', '\\', '$', '~', '%'])
        || (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'))
}

/// GM-023: "Separate your executable and its arguments using `command` and
/// `args` instead of putting them both in `command`." and "For portability,
/// you should use `${extensionPath}` to refer to files within your
/// extension directory." (extensions/reference, "`gemini-extension.json`").
fn check_extension_portability(sink: &mut JsonSink<'_, '_>, server: &JsonMember, root: &Path) {
    let value = &server.value.value;
    let name = server.key.as_str();
    let fs = sink.config.fs().clone();
    let cwd = value
        .get("cwd")
        .and_then(|m| m.value.value.as_str().map(|s| (m, s)));
    // Relative paths resolve against the server's working directory, which
    // is the extension only when `cwd` says so.
    let cwd_is_extension = cwd.is_some_and(|(_, cwd)| cwd.starts_with(EXTENSION_PATH_VAR));
    let inside_extension = |text: &str| -> bool {
        let candidate = Path::new(text);
        if candidate.is_absolute() {
            return root.is_absolute() && candidate.starts_with(root);
        }
        false
    };
    let flag = |sink: &mut JsonSink<'_, '_>, node: &JsonNode, field: &str, text: &str| {
        sink.push(
            DiagnosticLevel::Info,
            "GM-023",
            (node.start, node.end),
            t!(
                "rules.gm_023.extension_path",
                name = name,
                field = field,
                value = text
            ),
            t!("rules.gm_023.extension_path_suggestion"),
        );
    };

    if let Some(command) = value.get("command")
        && let Some(text) = command.value.value.as_str()
    {
        let trimmed = text.trim();
        if trimmed.contains(char::is_whitespace) {
            sink.push(
                DiagnosticLevel::Info,
                "GM-023",
                (command.value.start, command.value.end),
                t!("rules.gm_023.split_command", name = name, command = text),
                t!("rules.gm_023.split_command_suggestion"),
            );
        } else if (is_relative_path(trimmed) && trimmed.contains(['/', '\\']) && !cwd_is_extension)
            || inside_extension(trimmed)
        {
            flag(sink, &command.value, "command", text);
        }
    }

    if let Some(args) = value.get("args").and_then(|m| m.value.value.as_array()) {
        for arg in args {
            let Some(text) = arg.value.as_str() else {
                continue;
            };
            let relative_file = is_relative_path(text)
                && !text.starts_with('-')
                && !text.contains("://")
                && !cwd_is_extension
                && fs.is_file(&root.join(text));
            if relative_file || inside_extension(text) {
                flag(sink, arg, "args", text);
            }
        }
    }

    if let Some((member, text)) = cwd {
        let relative_dir =
            is_relative_path(text) && !matches!(text, "." | "./") && fs.is_dir(&root.join(text));
        if relative_dir || inside_extension(text) {
            flag(sink, &member.value, "cwd", text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn underscore_names_get_a_dashed_alternative() {
        assert_eq!(underscore_free_name("my_server"), Some("my-server".into()));
        assert_eq!(underscore_free_name("a_b_c"), Some("a-b-c".into()));
        assert_eq!(underscore_free_name("my-server"), None);
    }

    #[test]
    fn relative_paths() {
        for relative in ["server.js", "./bin/server", "dist/index.js", "..\\x"] {
            assert!(is_relative_path(relative), "{relative}");
        }
        for other in [
            "",
            "/usr/bin/node",
            "${extensionPath}/s.js",
            "$HOME/x",
            "~/x",
            "C:\\x",
            "%APPDATA%\\x",
        ] {
            assert!(!is_relative_path(other), "{other}");
        }
    }
}
