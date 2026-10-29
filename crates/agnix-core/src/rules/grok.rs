//! Grok Build native contracts, reconciled with the released CLI changelog.
use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    file_types::{FileType, native},
    parsers::frontmatter::{check_yaml_depth, split_frontmatter},
    rules::{
        Validator, ValidatorMetadata,
        config_json::JsonCx,
        config_span::{Seg, TomlSpans, yaml_key_pos},
        tool_contract::{Schema, json_document, pointer_span},
    },
    schemas::kiro_json::JsonNode,
};
use serde_json::Value;
use std::path::Path;

const RULES: &[&str] = &[
    "GRK-CFG-001",
    "GRK-CFG-002",
    "GRK-CFG-003",
    "GRK-CFG-004",
    "GRK-CFG-005",
    "GRK-CFG-006",
    "GRK-SBX-001",
    "GRK-SBX-002",
    "GRK-MCP-001",
    "GRK-HK-001",
    "GRK-HK-002",
    "GRK-HK-003",
    "GRK-PL-001",
    "GRK-PL-002",
    "GRK-AG-001",
    "GRK-AG-002",
    "GRK-AG-003",
    "GRK-AG-004",
    "GRK-SK-001",
    "GRK-SK-002",
];

pub struct GrokValidator;
impl Validator for GrokValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULES,
        }
    }
    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        match crate::pipeline::resolve_file_type(path, config) {
            FileType::GrokAgent => return agent(path, content, config),
            FileType::GrokRule => return Vec::new(),
            FileType::Skill => return skill(path, content, config),
            _ => {}
        }
        let resource = native::resource_owner(
            path,
            config.fs().as_ref(),
            native::selected(config),
            config.root_dir().map(|root| root.as_path()),
        );
        let filename = match resource {
            Some(native::Resource::GrokMcp) => ".mcp.json",
            Some(native::Resource::GrokLsp) => ".lsp.json",
            Some(native::Resource::GrokHooks) => "hooks.json",
            _ => path.file_name().and_then(|s| s.to_str()).unwrap_or(""),
        };
        let mut cx = JsonCx::new(path, content, config);
        if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            let value = match toml::from_str::<toml::Value>(content) {
                Ok(value) => serde_json::to_value(value).unwrap_or(Value::Null),
                Err(error) => {
                    if cx.enabled("GRK-CFG-001") {
                        cx.push(
                            DiagnosticLevel::Error,
                            "GRK-CFG-001",
                            error.span().map_or((0, 0), |s| (s.start, s.end)),
                            "Invalid Grok TOML configuration.",
                            "Correct the TOML syntax before Grok loads this file.",
                        );
                    }
                    return cx.diagnostics;
                }
            };
            let Some(spans) = TomlSpans::parse(content) else {
                return cx.diagnostics;
            };
            let source = Source::Toml(&spans, &value);
            let parts = native::grok_relative(path);
            if filename == "sandbox.toml" {
                schema(
                    &mut cx,
                    &source,
                    &value,
                    Schema::GrokSandbox,
                    "GRK-SBX-002",
                    DiagnosticLevel::Error,
                );
                if let Some(profiles) = value.get("profiles").and_then(Value::as_object) {
                    for name in profiles.keys() {
                        if ["off", "workspace", "read-only", "strict", "devbox"]
                            .contains(&name.as_str())
                        {
                            push(
                                &mut cx,
                                &source,
                                "GRK-SBX-001",
                                &format!("/profiles/{}", escape(name)),
                                DiagnosticLevel::Error,
                                "Grok custom profiles cannot redefine a built-in sandbox profile.",
                                "Choose a custom name and extend a supported built-in profile.",
                            );
                        }
                    }
                }
            } else if parts
                .first()
                .is_some_and(|p| p == "personas" || p == "roles")
            {
                let role = parts.first().is_some_and(|p| p == "roles");
                schema(
                    &mut cx,
                    &source,
                    &value,
                    if role {
                        Schema::GrokRole
                    } else {
                        Schema::GrokPersona
                    },
                    "GRK-AG-003",
                    DiagnosticLevel::Error,
                );
            } else {
                configuration(&mut cx, &source, &value, path);
            }
        } else {
            let Some((root, value)) = json_document(&mut cx, content, "GRK-CFG-001", false) else {
                return cx.diagnostics;
            };
            let source = Source::Json(&root);
            let index_fallback = filename == "plugin.json"
                && value.get("plugins").is_some()
                && path
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| matches!(name, ".grok-plugin" | ".claude-plugin"));
            match filename {
                "plugin.json" if index_fallback => schema(
                    &mut cx,
                    &source,
                    &value,
                    Schema::GrokMarketplace,
                    "GRK-PL-001",
                    DiagnosticLevel::Error,
                ),
                "marketplace.json" => schema(
                    &mut cx,
                    &source,
                    &value,
                    Schema::GrokMarketplace,
                    "GRK-PL-001",
                    DiagnosticLevel::Error,
                ),
                "lsp.json" | ".lsp.json" => schema(
                    &mut cx,
                    &source,
                    &value,
                    Schema::GrokLsp,
                    "GRK-CFG-003",
                    DiagnosticLevel::Error,
                ),
                "plugin.json" => plugin(&mut cx, &source, &value),
                ".mcp.json" => mcp(&mut cx, &source, &value, "/mcpServers"),
                _ => hooks(&mut cx, &source, &value),
            }
        }
        cx.diagnostics
    }
}

enum Source<'a> {
    Json(&'a JsonNode),
    Toml(&'a TomlSpans<'a>, &'a Value),
    Yaml(&'a str, usize),
}
fn escape(s: &str) -> String {
    s.replace('~', "~0").replace('/', "~1")
}
fn components(pointer: &str) -> Vec<String> {
    pointer
        .strip_prefix('/')
        .into_iter()
        .flat_map(|s| s.split('/'))
        .map(|s| s.replace("~1", "/").replace("~0", "~"))
        .collect()
}
fn push(
    cx: &mut JsonCx<'_>,
    source: &Source<'_>,
    rule: &str,
    pointer: &str,
    level: DiagnosticLevel,
    message: &str,
    suggestion: &str,
) {
    if !cx.enabled(rule) {
        return;
    }
    match source {
        Source::Json(root) => cx.push(
            level,
            rule,
            pointer_span(root, pointer),
            message,
            suggestion,
        ),
        Source::Toml(spans, root) => {
            let parts = components(pointer);
            let mut current = *root;
            let mut segments = Vec::new();
            for part in &parts {
                if let Some(array) = current.as_array() {
                    let Ok(index) = part.parse::<usize>() else {
                        break;
                    };
                    segments.push(Seg::Index(index));
                    let Some(next) = array.get(index) else {
                        break;
                    };
                    current = next;
                } else {
                    segments.push(Seg::Key(part));
                    let Some(next) = current.get(part) else {
                        break;
                    };
                    current = next;
                }
            }
            let offset = spans.offset(&segments).unwrap_or(0);
            cx.push(level, rule, (offset, offset), message, suggestion);
        }
        Source::Yaml(header, start) => {
            let parts = components(pointer);
            let keys: Vec<_> = parts.iter().map(String::as_str).collect();
            let position = (1..=keys.len())
                .rev()
                .find_map(|len| yaml_key_pos(header, &keys[..len]))
                .unwrap_or((1, 1));
            cx.push_at(
                level,
                rule,
                (start + position.0 - 1, position.1),
                message,
                suggestion,
            );
        }
    }
}
fn schema(
    cx: &mut JsonCx<'_>,
    source: &Source<'_>,
    value: &Value,
    contract: Schema,
    rule: &str,
    level: DiagnosticLevel,
) {
    for issue in contract.issues(value) {
        // The native MCP validator owns these fields, including transport
        // semantics and u64 timeouts. Avoid a second generic schema report.
        if matches!(contract, Schema::GrokConfig) && issue.pointer.starts_with("/mcp_servers") {
            continue;
        }
        push(
            cx,
            source,
            rule,
            &issue.pointer,
            level,
            &format!(
                "Grok field violates the documented {} constraint.",
                issue.keyword
            ),
            "Use the native Grok configuration field types and values.",
        );
    }
}
fn strings(v: &Value) -> bool {
    v.as_array()
        .is_some_and(|items| items.iter().all(Value::is_string))
}
fn string_map(v: &Value) -> bool {
    v.as_object()
        .is_some_and(|map| map.values().all(Value::is_string))
}

fn configuration(cx: &mut JsonCx<'_>, source: &Source<'_>, value: &Value, path: &Path) {
    schema(
        cx,
        source,
        value,
        Schema::GrokConfig,
        "GRK-CFG-003",
        DiagnosticLevel::Warning,
    );
    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let default_home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .map(|home| home.join(".grok"));
    let override_home = std::env::var_os("GROK_HOME").map(std::path::PathBuf::from);
    let user = path.parent().is_some_and(|parent| {
        Some(parent) == default_home.as_deref() || Some(parent) == override_home.as_deref()
    });
    if filename == "config.toml" && !user {
        if let Some(fields) = value.as_object() {
            for key in fields.keys() {
                if key == "mcp" {
                    if let Some(options) = value.get(key).and_then(Value::as_object) {
                        for option in options
                            .keys()
                            .filter(|key| key.as_str() != "max_output_bytes")
                        {
                            push(
                                cx,
                                source,
                                "GRK-CFG-002",
                                &format!("/mcp/{}", escape(option)),
                                DiagnosticLevel::Warning,
                                "Grok ignores this MCP option in project config.toml.",
                                "Only mcp.max_output_bytes is loaded from project configuration.",
                            );
                        }
                    }
                    continue;
                }
                if !["mcp_servers", "plugins", "permission"].contains(&key.as_str()) {
                    push(
                        cx,
                        source,
                        "GRK-CFG-002",
                        &format!("/{}", escape(key)),
                        DiagnosticLevel::Warning,
                        "Grok ignores this setting in project config.toml.",
                        "Put user settings in $GROK_HOME/config.toml; project files contribute mcp_servers, plugins, permission and mcp.max_output_bytes.",
                    );
                }
            }
        }
    }
    for (pointer, minimum, message) in [
        (
            "/ui/simple_mode",
            "1.0.69",
            "Grok 1.0.69 retired ui.simple_mode; it no longer changes the editor.",
        ),
        (
            "/ui/vim_mode",
            "1.0.68",
            "Grok 1.0.68 replaced ui.vim_mode with keys.vim; the old key is a legacy alias.",
        ),
        (
            "/memory_v2/compact_index_enabled",
            "1.0.63",
            "Grok 1.0.63 retired memory_v2.compact_index_enabled.",
        ),
    ] {
        let qualified = cx
            .config
            .tool_versions()
            .grok_code
            .as_deref()
            .and_then(|version| semver::Version::parse(version.trim_start_matches('v')).ok())
            .is_none_or(|version| {
                version >= semver::Version::parse(minimum).expect("fixed Grok release version")
            });
        if qualified && value.pointer(pointer).is_some() {
            push(
                cx,
                source,
                "GRK-CFG-004",
                pointer,
                DiagnosticLevel::Warning,
                message,
                "Remove retired settings; use keys.vim = off or nav for current vim navigation.",
            );
        }
    }
    if value
        .pointer("/ui/disable_bypass_permissions_mode")
        .is_some()
        && path != Path::new("/etc/grok/requirements.toml")
    {
        push(
            cx,
            source,
            "GRK-CFG-005",
            "/ui/disable_bypass_permissions_mode",
            DiagnosticLevel::Warning,
            "Grok honors the bypass-permissions lock only from root-owned system policy.",
            "Set the lock in /etc/grok/requirements.toml through your system administrator.",
        );
    }
    version_overrides(cx, source, value);
    mcp(cx, source, value, "/mcp_servers");
    if let Some(hook_config) = value.get("hooks") {
        hooks_map(cx, source, hook_config, "/hooks");
    }
}

fn version_overrides(cx: &mut JsonCx<'_>, source: &Source<'_>, value: &Value) {
    let Some(items) = value.get("version_overrides").and_then(Value::as_array) else {
        return;
    };
    for (index, item) in items.iter().enumerate() {
        let mut bounds = Vec::new();
        for key in ["minimum_version", "maximum_version"] {
            if let Some(value) = item.get(key) {
                let bound = value
                    .as_str()
                    .and_then(|s| semver::Version::parse(s.trim()).ok());
                if bound.is_none() {
                    push(
                        cx,
                        source,
                        "GRK-CFG-006",
                        &format!("/version_overrides/{index}/{key}"),
                        DiagnosticLevel::Error,
                        "Grok version override bound must be a semantic version.",
                        "Use a complete major.minor.patch version string.",
                    );
                }
                bounds.push(bound);
            } else {
                bounds.push(None);
            }
        }
        if bounds
            .first()
            .and_then(Option::as_ref)
            .zip(bounds.get(1).and_then(Option::as_ref))
            .is_some_and(|(min, max)| min > max)
        {
            push(
                cx,
                source,
                "GRK-CFG-006",
                &format!("/version_overrides/{index}"),
                DiagnosticLevel::Warning,
                "This Grok version range can never apply.",
                "Put minimum_version at or below maximum_version.",
            );
        }
    }
}

fn mcp(cx: &mut JsonCx<'_>, source: &Source<'_>, value: &Value, pointer: &str) {
    let Some(servers) = value.pointer(pointer) else {
        return;
    };
    let Some(servers) = servers.as_object() else {
        push(
            cx,
            source,
            "GRK-MCP-001",
            pointer,
            DiagnosticLevel::Error,
            "Grok MCP server declarations must be a map.",
            "Map server names to stdio command or remote URL configurations.",
        );
        return;
    };
    for (name, server) in servers {
        let path = format!("{pointer}/{}", escape(name));
        mcp_server(cx, source, server, &path);
    }
}

fn mcp_server(cx: &mut JsonCx<'_>, source: &Source<'_>, server: &Value, path: &str) {
    let Some(object) = server.as_object() else {
        push(
            cx,
            source,
            "GRK-MCP-001",
            path,
            DiagnosticLevel::Error,
            "Grok MCP server must be an object.",
            "Configure command/args/env or url/headers.",
        );
        return;
    };
    if object.get("enabled") == Some(&Value::Bool(false)) {
        return;
    }
    let command = object.get("command").and_then(Value::as_str);
    let remote = object.get("url").and_then(Value::as_str);
    if (command.is_none() && remote.is_none())
        || command.is_some_and(|s| s.trim().is_empty())
        || remote.is_some_and(|s| s.trim().is_empty())
    {
        push(
            cx,
            source,
            "GRK-MCP-001",
            path,
            DiagnosticLevel::Error,
            "Grok MCP server needs a non-empty command or url.",
            "Use command for stdio, or url for a remote server.",
        );
    }
    for (key, entry) in object {
        let valid = match key.as_str() {
            "command" | "url" | "cwd" | "bearer_token_env_var" | "bearer_token_file" => {
                entry.is_string()
            }
            "args" => strings(entry),
            "env" | "headers" => string_map(entry),
            "enabled" => entry.is_boolean(),
            "startup_timeout_sec" | "tool_timeout_sec" => entry.as_u64().is_some(),
            "tool_timeouts" => entry
                .as_object()
                .is_some_and(|values| values.values().all(|timeout| timeout.as_u64().is_some())),
            _ => true,
        };
        if !valid {
            push(
                cx,
                source,
                "GRK-MCP-001",
                &format!("{path}/{}", escape(key)),
                DiagnosticLevel::Error,
                "Invalid Grok MCP server option type.",
                "Use the types documented in the Grok MCP configuration reference.",
            );
        }
    }
}

const EVENTS: &[&str] = &[
    "SessionStart",
    "session_start",
    "sessionStart",
    "SessionEnd",
    "session_end",
    "sessionEnd",
    "UserPromptSubmit",
    "user_prompt_submit",
    "beforeSubmitPrompt",
    "PreToolUse",
    "pre_tool_use",
    "preToolUse",
    "beforeShellExecution",
    "beforeMCPExecution",
    "beforeReadFile",
    "PostToolUse",
    "post_tool_use",
    "postToolUse",
    "afterShellExecution",
    "afterMCPExecution",
    "afterFileEdit",
    "afterAgentResponse",
    "afterAgentThought",
    "PostToolUseFailure",
    "post_tool_use_failure",
    "postToolUseFailure",
    "PermissionDenied",
    "permission_denied",
    "permissionDenied",
    "Stop",
    "stop",
    "StopFailure",
    "stop_failure",
    "stopFailure",
    "StopCancelled",
    "stop_cancelled",
    "stopCancelled",
    "Notification",
    "notification",
    "SubagentStart",
    "subagent_start",
    "subagentStart",
    "SubagentStop",
    "subagent_stop",
    "subagentStop",
    "SubagentEnd",
    "subagent_end",
    "subagentEnd",
    "PreCompact",
    "pre_compact",
    "preCompact",
    "PostCompact",
    "post_compact",
    "postCompact",
];
fn hooks(cx: &mut JsonCx<'_>, source: &Source<'_>, value: &Value) {
    let Some(hooks) = value.get("hooks") else {
        push(
            cx,
            source,
            "GRK-HK-001",
            "",
            DiagnosticLevel::Error,
            "Grok hook files need a hooks object.",
            "Put lifecycle events inside the hooks map.",
        );
        return;
    };
    hooks_map(cx, source, hooks, "/hooks");
}
fn hooks_map(cx: &mut JsonCx<'_>, source: &Source<'_>, value: &Value, base: &str) {
    let Some(events) = value.as_object() else {
        push(
            cx,
            source,
            "GRK-HK-001",
            base,
            DiagnosticLevel::Error,
            "Grok hooks must map event names to matcher groups.",
            "Use the documented hooks object.",
        );
        return;
    };
    for (event, groups) in events {
        let path = format!("{base}/{}", escape(event));
        if !EVENTS.contains(&event.as_str()) {
            push(
                cx,
                source,
                "GRK-HK-002",
                &path,
                DiagnosticLevel::Warning,
                "Grok skips this unknown hook event.",
                "Use a documented lifecycle event or supported compatibility alias.",
            );
            continue;
        }
        let Some(groups) = groups.as_array() else {
            push(
                cx,
                source,
                "GRK-HK-001",
                &path,
                DiagnosticLevel::Error,
                "Hook event must contain a matcher-group array.",
                "Put handlers in each group's hooks array.",
            );
            continue;
        };
        for (i, group) in groups.iter().enumerate() {
            let path = format!("{path}/{i}");
            if let Some(matcher) = group.get("matcher") {
                if !matcher.is_null() && !matcher.is_string() {
                    push(
                        cx,
                        source,
                        "GRK-HK-001",
                        &path,
                        DiagnosticLevel::Error,
                        "Hook matcher must be a string or null.",
                        "Use a tool matcher expression, or omit it.",
                    );
                }
                if let Some(pattern) = matcher.as_str() {
                    if !matches!(
                        event.as_str(),
                        "Stop"
                            | "stop"
                            | "UserPromptSubmit"
                            | "user_prompt_submit"
                            | "beforeSubmitPrompt"
                    ) && !pattern.is_empty()
                        && pattern != "*"
                        && regex::Regex::new(pattern).is_err()
                    {
                        push(
                            cx,
                            source,
                            "GRK-HK-003",
                            &format!("{path}/matcher"),
                            DiagnosticLevel::Warning,
                            "Invalid Grok hook matcher regular expression.",
                            "Use a valid expression in Grok's regex dialect, or * to match all tools.",
                        );
                    }
                }
            }
            let Some(handlers) = group.get("hooks").and_then(Value::as_array) else {
                push(
                    cx,
                    source,
                    "GRK-HK-001",
                    &path,
                    DiagnosticLevel::Error,
                    "Hook matcher group needs a hooks array.",
                    "Add command or HTTP hook handlers.",
                );
                continue;
            };
            for (j, handler) in handlers.iter().enumerate() {
                let path = format!("{path}/hooks/{j}");
                let kind = handler.get("type").and_then(Value::as_str);
                let valid = match kind {
                    Some("command") => handler
                        .get("command")
                        .and_then(Value::as_str)
                        .is_some_and(|s| !s.trim().is_empty()),
                    Some("http") => handler
                        .get("url")
                        .and_then(Value::as_str)
                        .is_some_and(|s| !s.trim().is_empty()),
                    _ => false,
                };
                let options = handler
                    .get("timeout")
                    .is_none_or(|v| v.is_null() || v.as_u64().is_some())
                    && handler.get("args").is_none_or(strings)
                    && handler
                        .get("env")
                        .is_none_or(|v| v.is_null() || string_map(v))
                    && ["command", "url"].iter().all(|key| {
                        handler
                            .get(*key)
                            .is_none_or(|v| v.is_null() || v.is_string())
                    });
                if !valid || !options {
                    push(
                        cx,
                        source,
                        "GRK-HK-001",
                        &path,
                        DiagnosticLevel::Error,
                        "Invalid Grok hook handler.",
                        "Use type command with command, or type http with url; timeout is unsigned seconds and args is a string list.",
                    );
                }
            }
        }
    }
}

fn escapes(path: &str, cx: &JsonCx<'_>) -> bool {
    if Path::new(path).is_absolute() {
        let Some(parent) = cx.path.parent() else {
            return false;
        };
        let root = if parent
            .file_name()
            .and_then(|p| p.to_str())
            .is_some_and(|p| matches!(p, ".grok-plugin" | ".claude-plugin"))
        {
            parent.parent().unwrap_or(parent)
        } else {
            parent
        };
        let root = cx
            .config
            .fs()
            .canonicalize(root)
            .unwrap_or_else(|_| root.to_path_buf());
        let candidate = cx
            .config
            .fs()
            .canonicalize(Path::new(path))
            .unwrap_or_else(|_| Path::new(path).to_path_buf());
        // An unanchored in-memory document cannot establish an absolute root.
        return root.is_absolute() && !candidate.starts_with(root);
    }
    let mut depth = 0;
    for part in path.replace('\\', "/").split('/') {
        match part {
            ".." => {
                if depth == 0 {
                    return true;
                }
                depth -= 1;
            }
            "" | "." => {}
            _ => depth += 1,
        }
    }
    false
}
fn plugin(cx: &mut JsonCx<'_>, source: &Source<'_>, value: &Value) {
    schema(
        cx,
        source,
        value,
        Schema::GrokPlugin,
        "GRK-PL-001",
        DiagnosticLevel::Error,
    );
    for field in [
        "skills",
        "commands",
        "agents",
        "hooks",
        "mcpServers",
        "lspServers",
    ] {
        if let Some(paths) = value.get(field) {
            let paths: Vec<_> = match paths {
                Value::String(s) => vec![s.as_str()],
                Value::Array(v) => v.iter().filter_map(Value::as_str).collect(),
                _ => Vec::new(),
            };
            if paths.iter().any(|path| escapes(path, cx)) {
                push(
                    cx,
                    source,
                    "GRK-PL-002",
                    &format!("/{field}"),
                    DiagnosticLevel::Warning,
                    "Grok skips plugin component paths outside the plugin root.",
                    "Keep component paths within the plugin directory.",
                );
            }
        }
    }
    if let Some(hook) = value.get("hooks").filter(|v| v.is_object()) {
        if let Some(map) = hook.get("hooks") {
            hooks_map(cx, source, map, "/hooks/hooks");
        } else {
            hooks_map(cx, source, hook, "/hooks");
        }
    }
    if value.get("mcpServers").is_some_and(Value::is_object) {
        mcp(cx, source, value, "/mcpServers");
    }
    if let Some(lsp) = value.get("lspServers").filter(|v| v.is_object()) {
        for issue in Schema::GrokLsp.issues(lsp) {
            push(
                cx,
                source,
                "GRK-PL-001",
                &format!("/lspServers{}", issue.pointer),
                DiagnosticLevel::Error,
                "Invalid Grok plugin LSP server configuration.",
                "Use the native LSP command, argument, transport and timeout types.",
            );
        }
    }
}

fn agent(path: &Path, content: &str, config: &PerFileLintConfig<'_>) -> Vec<Diagnostic> {
    let mut cx = JsonCx::new(path, content, config);
    let parts = split_frontmatter(content.trim_start());
    let value = if parts.has_frontmatter
        && parts.has_closing
        && check_yaml_depth(&parts.frontmatter).is_ok()
    {
        serde_yaml::from_str::<Value>(&parts.frontmatter)
            .ok()
            .filter(Value::is_object)
    } else {
        None
    };
    let Some(value) = value else {
        if cx.enabled("GRK-AG-001") {
            cx.push_at(
                DiagnosticLevel::Error,
                "GRK-AG-001",
                (1, 1),
                "Grok agent needs valid YAML frontmatter.",
                "Add a complete agent metadata object with name and description.",
            );
        }
        return cx.diagnostics;
    };
    let start = crate::rules::line_col_at_offset(
        content,
        crate::rules::frontmatter_content_offset(content, parts.frontmatter_start),
    )
    .0;
    let source = Source::Yaml(&parts.frontmatter, start);
    let mut schema_value = value.clone();
    normalize_agent_yaml_strings(&mut schema_value);
    schema(
        &mut cx,
        &source,
        &schema_value,
        Schema::GrokAgent,
        "GRK-AG-002",
        DiagnosticLevel::Error,
    );
    if let Some(inline) = value.get("mcpServers").and_then(Value::as_array) {
        for (index, item) in inline.iter().enumerate() {
            let pointer = format!("/mcpServers/{index}");
            if let Some(object) = item.as_object() {
                if object.contains_key("name") {
                    mcp_server(&mut cx, &source, item, &pointer);
                } else if object.len() == 1 {
                    for (name, server) in object {
                        mcp_server(
                            &mut cx,
                            &source,
                            server,
                            &format!("{pointer}/{}", escape(name)),
                        );
                    }
                }
            }
            let valid = item.is_string()
                || item.as_object().is_some_and(|o| {
                    if o.len() == 1 {
                        o.values().all(Value::is_object)
                    } else {
                        o.get("name").is_some_and(Value::is_string)
                    }
                });
            if !valid {
                push(
                    &mut cx,
                    &source,
                    "GRK-AG-002",
                    &format!("/mcpServers/{index}"),
                    DiagnosticLevel::Error,
                    "Invalid Grok inline MCP server reference.",
                    "Use a server name, a single name-to-config map, or a server object with name.",
                );
            }
        }
    }
    let parts = native::grok_relative(path);
    if parts.first().is_some_and(|p| p == "plugins")
        || native::plugin_owner(path, config.fs().as_ref(), native::selected(config))
            .is_some_and(|(tool, _, _)| tool == native::Tool::Grok)
    {
        for field in ["mcpServers", "hooks", "permissionMode"] {
            let restricted = match field {
                "permissionMode" => {
                    value.get(field).and_then(Value::as_str) == Some("bypassPermissions")
                }
                _ => value.get(field).is_some_and(|v| !v.is_null()),
            };
            if restricted {
                push(
                    &mut cx,
                    &source,
                    "GRK-AG-004",
                    &format!("/{field}"),
                    DiagnosticLevel::Warning,
                    "Grok plugin agents cannot use this privileged frontmatter setting.",
                    "Configure trusted plugin/session MCP and hooks outside the agent; plugin agents cannot bypass permissions.",
                );
            }
        }
    }
    if let Some(hooks) = value.get("hooks").filter(|v| v.is_object()) {
        hooks_map(&mut cx, &source, hooks, "/hooks");
    }
    cx.diagnostics
}

fn skill(path: &Path, content: &str, config: &PerFileLintConfig<'_>) -> Vec<Diagnostic> {
    if crate::rules::per_client_skill::resolve_skill_client(path, config)
        != crate::rules::per_client_skill::SkillClient::Grok
    {
        return Vec::new();
    }
    let parts = split_frontmatter(content);
    if !parts.has_frontmatter || !parts.has_closing || check_yaml_depth(&parts.frontmatter).is_err()
    {
        return Vec::new();
    }
    let Ok(value) = serde_yaml::from_str::<Value>(&parts.frontmatter) else {
        return Vec::new();
    };
    let mut cx = JsonCx::new(path, content, config);
    let source = Source::Yaml(&parts.frontmatter, 2);
    if value.get("allowed-tools").is_some() {
        push(
            &mut cx,
            &source,
            "GRK-SK-001",
            "/allowed-tools",
            DiagnosticLevel::Info,
            "Grok skill allowed-tools does not grant or restrict tools.",
            "Use Grok permission rules or an agent tool policy for tool restrictions.",
        );
    }
    for key in ["user-invocable", "disable-model-invocation"] {
        if value.get(key).is_some_and(|v| !v.is_boolean()) {
            push(
                &mut cx,
                &source,
                "GRK-SK-002",
                &format!("/{key}"),
                DiagnosticLevel::Warning,
                "Grok treats only literal true as enabled for this skill field.",
                "Use true or false, not yes/no or quoted boolean strings.",
            );
        }
    }
    cx.diagnostics
}

/// serde_yaml's typed String fields accept scalar numbers and booleans.
/// Normalize only fields whose native deserializer has that behavior; opaque
/// MCP JSON values and enum fields retain their original types.
fn normalize_agent_yaml_strings(value: &mut Value) {
    fn scalar(value: &mut Value) {
        if value.is_number() || value.is_boolean() {
            *value = Value::String(value.to_string());
        }
    }
    for key in ["name", "description", "model", "initialPrompt"] {
        if let Some(entry) = value.get_mut(key) {
            scalar(entry);
        }
    }
    for key in ["skills", "tools", "disallowedTools"] {
        if let Some(items) = value.get_mut(key).and_then(Value::as_array_mut) {
            for item in items {
                scalar(item);
            }
        }
    }
    if let Some(inheritance) = value
        .get_mut("mcpInheritance")
        .and_then(Value::as_object_mut)
    {
        for items in inheritance.values_mut().filter_map(Value::as_array_mut) {
            for item in items {
                scalar(item);
            }
        }
    }
    if let Some(requirement) = value.get_mut("completionRequirement") {
        for key in ["tool", "reminder"] {
            if let Some(entry) = requirement.get_mut(key) {
                scalar(entry);
            }
        }
    }
}
