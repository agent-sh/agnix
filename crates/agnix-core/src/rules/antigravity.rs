//! Antigravity CLI configuration, native hooks, modular rules and custom agents.
use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    file_types::FileType,
    parsers::frontmatter::{check_yaml_depth, split_frontmatter},
    rules::{
        Validator, ValidatorMetadata,
        config_json::JsonCx,
        tool_contract::{Schema, json_document, pointer_span},
    },
    schemas::kiro_json::JsonNode,
};
use serde_json::Value;
use std::{collections::HashSet, path::Path};

const RULES: &[&str] = &[
    "AGY-CFG-001",
    "AGY-CFG-002",
    "AGY-CFG-003",
    "AGY-CFG-004",
    "AGY-CFG-005",
    "AGY-KB-001",
    "AGY-MCP-001",
    "AGY-MCP-002",
    "AGY-PL-001",
    "AGY-PL-002",
    "AGY-HK-001",
    "AGY-HK-002",
    "AGY-RL-001",
    "AGY-RL-002",
    "AGY-RL-003",
    "AGY-RL-004",
    "AGY-AG-001",
    "AGY-AG-002",
];

pub struct AntigravityValidator;

impl Validator for AntigravityValidator {
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
            FileType::ClaudeMd | FileType::GeminiMd => {
                let name = path.file_name().and_then(|name| name.to_str());
                if !matches!(name, Some("AGENTS.md" | "GEMINI.md")) {
                    return Vec::new();
                }
                let selected = crate::file_types::native::selected(config)
                    == Some(crate::file_types::native::Tool::Antigravity)
                    || config.tools().iter().any(|tool| {
                        matches!(tool.to_ascii_lowercase().as_str(), "antigravity" | "agy")
                    });
                let parts = crate::file_types::native::components(path);
                let global = parts
                    .iter()
                    .rev()
                    .nth(1)
                    .is_some_and(|parent| parent == ".gemini")
                    || crate::file_types::native::sequence(&parts, &[".gemini", "config"]);
                if (selected || global)
                    && config.is_rule_enabled("AGY-RL-003")
                    && expanded_rule_size(path, content, config).is_some_and(|size| size > 24_000)
                {
                    return vec![
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            1,
                            "AGY-RL-003",
                            "Antigravity truncates rule files after 24,000 expanded bytes.",
                        )
                        .with_suggestion("Split the instructions into smaller modular rules."),
                    ];
                }
                return Vec::new();
            }
            FileType::AntigravityRule => return markdown(path, content, config, false),
            FileType::AntigravityAgent => return markdown(path, content, config, true),
            _ => {}
        }
        let mut cx = JsonCx::new(path, content, config);
        let Some((root, value)) = json_document(&mut cx, content, "AGY-CFG-001", false) else {
            return cx.diagnostics;
        };
        let actual_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let filename =
            if crate::file_types::native::detect(path) == Some(FileType::AntigravityConfig) {
                actual_name
            } else {
                crate::file_types::native::hinted_basename(content).unwrap_or(actual_name)
            };
        match filename {
            "config.json" => {
                if value
                    .get("cliRemoteControlHostname")
                    .is_some_and(|hostname| !hostname.is_string())
                {
                    issue(
                        &mut cx,
                        &root,
                        "AGY-CFG-002",
                        "/cliRemoteControlHostname",
                        DiagnosticLevel::Error,
                        "Antigravity CLI remote-control hostname must be a string.",
                        "Use the daemon hostname setting documented in Remote Control.",
                    );
                }
            }
            "settings.json" => {
                let permission_advisories = settings_schema(&mut cx, &root, &value);
                for key in ["hooks", "mcpServers"] {
                    if value.get(key).is_some() {
                        issue(
                            &mut cx,
                            &root,
                            "AGY-CFG-003",
                            &format!("/{key}"),
                            DiagnosticLevel::Warning,
                            "Antigravity CLI does not load this customization from settings.json.",
                            "Move hooks to .agents/hooks.json or ~/.gemini/config/hooks.json; put servers in mcp_config.json.",
                        );
                    }
                }
                if let Some(permissions) = value.get("permissions").and_then(Value::as_object) {
                    for list in ["allow", "ask", "deny"] {
                        if let Some(items) = permissions.get(list).and_then(Value::as_array) {
                            for (index, item) in items.iter().enumerate() {
                                if item.as_str().is_some_and(|s| !permission_resource(s))
                                    && !permission_advisories
                                        .contains(&format!("/permissions/{list}/{index}"))
                                {
                                    issue(
                                        &mut cx,
                                        &root,
                                        "AGY-CFG-004",
                                        &format!("/permissions/{list}/{index}"),
                                        DiagnosticLevel::Warning,
                                        "Antigravity ignores this malformed permission resource.",
                                        "Use action(target), without a space before (, and a documented CLI action.",
                                    );
                                }
                            }
                        }
                    }
                }
            }
            "plugin.json" => {
                // Unknown keys are ignored and reported by `agy plugin validate`.
                // A name is needed for installation, while folder-loaded plugins
                // can derive it. Neither case is a runtime-fatal type error.
                for error in Schema::AntigravityPlugin.issues(&value) {
                    let advisory =
                        matches!(error.keyword.as_str(), "additionalProperties" | "required")
                            || (error.pointer == "/logo"
                                && value.get("logo").is_some_and(Value::is_string));
                    issue(
                        &mut cx,
                        &root,
                        if advisory { "AGY-PL-002" } else { "AGY-PL-001" },
                        &error.pointer,
                        if advisory {
                            DiagnosticLevel::Warning
                        } else {
                            DiagnosticLevel::Error
                        },
                        if advisory {
                            "Plugin metadata is ignored or incomplete for CLI installation."
                        } else {
                            "Invalid Antigravity plugin manifest field."
                        },
                        "Use the native plugin schema; name is required for CLI installation and version can be any string.",
                    );
                }
            }
            "mcp_config.json" => mcp(&mut cx, &root, &value),
            "hooks.json" => hooks(&mut cx, &root, &value),
            "keybindings.json" => keybindings(&mut cx, &root, &value),
            "rules.json" => rule_registry(&mut cx, &root, &value),
            _ => {}
        }
        cx.diagnostics
    }
}

fn issue(
    cx: &mut JsonCx<'_>,
    root: &JsonNode,
    rule: &str,
    pointer: &str,
    level: DiagnosticLevel,
    message: &str,
    suggestion: &str,
) {
    if cx.enabled(rule) {
        cx.push(
            level,
            rule,
            pointer_span(root, pointer),
            message,
            suggestion,
        );
    }
}
fn escaped(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}
fn strings(value: &Value) -> bool {
    value
        .as_array()
        .is_some_and(|items| items.iter().all(Value::is_string))
}
fn string_map(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|map| map.values().all(Value::is_string))
}
fn integer(value: &Value) -> bool {
    value
        .as_f64()
        .is_some_and(|n| n.is_finite() && n.fract() == 0.0)
}

fn permission_resource(text: &str) -> bool {
    let Some((action, target)) = text.split_once('(') else {
        return false;
    };
    [
        "command",
        "unsandboxed",
        "read_file",
        "write_file",
        "read_url",
        "execute_url",
        "mcp",
    ]
    .contains(&action)
        && target.ends_with(')')
        && target.len() > 1
}

fn settings_schema(cx: &mut JsonCx<'_>, root: &JsonNode, value: &Value) -> HashSet<String> {
    let mut permissions = HashSet::new();
    for error in Schema::AntigravitySettings.issues(value) {
        let permission_pattern =
            error.pointer.starts_with("/permissions/") && error.keyword == "pattern";
        let formatter_fallback =
            error.pointer.ends_with("/toolFormatterType") && error.keyword == "enum";
        if permission_pattern {
            permissions.insert(error.pointer.clone());
            issue(
                cx,
                root,
                "AGY-CFG-004",
                &error.pointer,
                DiagnosticLevel::Warning,
                "Antigravity ignores this malformed permission resource.",
                "Use a documented action(target) string without a space before (.",
            );
        } else if formatter_fallback {
            issue(
                cx,
                root,
                "AGY-CFG-005",
                &error.pointer,
                DiagnosticLevel::Warning,
                "Antigravity uses the XML tool formatter for this unrecognized formatter value.",
                "Use a documented formatter value, or omit the field to use the default.",
            );
        } else {
            issue(
                cx,
                root,
                "AGY-CFG-002",
                &error.pointer,
                DiagnosticLevel::Error,
                &format!(
                    "Antigravity setting violates the documented {} constraint.",
                    error.keyword
                ),
                "Use the types and values in the Antigravity CLI settings reference.",
            );
        }
    }
    permissions
}

fn mcp(cx: &mut JsonCx<'_>, root: &JsonNode, value: &Value) {
    let Some(servers) = value.get("mcpServers").and_then(Value::as_object) else {
        if value.get("mcpServers").is_some() {
            issue(
                cx,
                root,
                "AGY-MCP-001",
                "/mcpServers",
                DiagnosticLevel::Error,
                "mcpServers must be an object.",
                "Map server names to stdio or Streamable HTTP server definitions.",
            );
        }
        return;
    };
    for (name, server) in servers {
        let base = format!("/mcpServers/{}", escaped(name));
        let Some(object) = server.as_object() else {
            issue(
                cx,
                root,
                "AGY-MCP-001",
                &base,
                DiagnosticLevel::Error,
                "An Antigravity MCP server must be an object.",
                "Use command for stdio or serverUrl/url for Streamable HTTP.",
            );
            continue;
        };
        let command = object.get("command").and_then(Value::as_str);
        let remote = object
            .get("serverUrl")
            .or_else(|| object.get("url"))
            .and_then(Value::as_str);
        if object.contains_key("httpUrl")
            || object.get("type").and_then(Value::as_str) == Some("sse")
        {
            issue(
                cx,
                root,
                "AGY-MCP-002",
                &base,
                DiagnosticLevel::Error,
                "Antigravity CLI does not support httpUrl or legacy HTTP+SSE MCP transport.",
                "Use a Streamable HTTP endpoint under serverUrl or url.",
            );
        }
        if command.is_none() == remote.is_none() {
            issue(
                cx,
                root,
                "AGY-MCP-001",
                &base,
                DiagnosticLevel::Error,
                "An MCP server needs one stdio command or Streamable HTTP URL.",
                "Set command for a local process, or serverUrl/url for a remote endpoint.",
            );
        }
        for (key, entry) in object {
            let valid = match key.as_str() {
                "command" | "serverUrl" | "url" | "cwd" | "authProviderType" => entry.is_string(),
                "args" | "enabledTools" | "disabledTools" => strings(entry),
                "env" | "headers" => string_map(entry),
                "disabled" => entry.is_boolean(),
                "timeoutSeconds" => integer(entry),
                "oauth" => entry.as_object().is_some_and(|map| {
                    map.get("clientId").is_some_and(Value::is_string)
                        && map.get("clientSecret").is_none_or(Value::is_string)
                }),
                _ => true,
            };
            if !valid {
                issue(
                    cx,
                    root,
                    "AGY-MCP-001",
                    &format!("{base}/{}", escaped(key)),
                    DiagnosticLevel::Error,
                    "Invalid Antigravity MCP server option.",
                    "Use the documented types; timeoutSeconds accepts zero and negative integers.",
                );
            }
        }
        if let Some(endpoint) = remote {
            // Placeholders are expanded by the client; never fetch the URL.
            if !endpoint.contains("${")
                && !url::Url::parse(endpoint).is_ok_and(|u| matches!(u.scheme(), "http" | "https"))
            {
                issue(
                    cx,
                    root,
                    "AGY-MCP-001",
                    &base,
                    DiagnosticLevel::Error,
                    "MCP remote endpoint must be an HTTP or HTTPS URL.",
                    "Use a Streamable HTTP server URL or supported environment expansion.",
                );
            }
        }
    }
}

const EVENTS: &[&str] = &[
    "SessionStart",
    "PreToolUse",
    "PostToolUse",
    "PreInvocation",
    "PostInvocation",
    "Stop",
];

fn hooks(cx: &mut JsonCx<'_>, root: &JsonNode, value: &Value) {
    let Some(named) = value.as_object() else {
        return;
    };
    for (name, definition) in named {
        let base = format!("/{}", escaped(name));
        let Some(events) = definition.as_object() else {
            issue(
                cx,
                root,
                "AGY-HK-001",
                &base,
                DiagnosticLevel::Error,
                "An Antigravity named hook must map events to handlers.",
                "Use a named hook object with native Antigravity event keys.",
            );
            continue;
        };
        for (event, entries) in events {
            let pointer = format!("{base}/{}", escaped(event));
            if event == "enabled" {
                if !entries.is_boolean() {
                    issue(
                        cx,
                        root,
                        "AGY-HK-001",
                        &pointer,
                        DiagnosticLevel::Error,
                        "Hook enabled must be a boolean.",
                        "Use true or false.",
                    );
                }
                continue;
            }
            if !EVENTS.contains(&event.as_str()) {
                issue(
                    cx,
                    root,
                    "AGY-HK-002",
                    &pointer,
                    DiagnosticLevel::Warning,
                    "Unrecognized Antigravity lifecycle event.",
                    "Use SessionStart, PreToolUse, PostToolUse, PreInvocation, PostInvocation or Stop.",
                );
                continue;
            }
            let Some(items) = entries.as_array() else {
                issue(
                    cx,
                    root,
                    "AGY-HK-001",
                    &pointer,
                    DiagnosticLevel::Error,
                    "Hook event handlers must be an array.",
                    "Use matcher groups for tool events and direct handlers for other events.",
                );
                continue;
            };
            for (index, item) in items.iter().enumerate() {
                let location = format!("{pointer}/{index}");
                if matches!(event.as_str(), "PreToolUse" | "PostToolUse") {
                    if item
                        .get("matcher")
                        .is_some_and(|matcher| !matcher.is_string())
                    {
                        issue(
                            cx,
                            root,
                            "AGY-HK-001",
                            &location,
                            DiagnosticLevel::Error,
                            "A hook matcher must be a string.",
                            "Use a tool-name expression, *, or omit matcher.",
                        );
                    }
                    match item.get("hooks").and_then(Value::as_array) {
                        Some(handlers) => {
                            for (h, handler) in handlers.iter().enumerate() {
                                handler_config(cx, root, &format!("{location}/hooks/{h}"), handler);
                            }
                        }
                        None => issue(
                            cx,
                            root,
                            "AGY-HK-001",
                            &location,
                            DiagnosticLevel::Error,
                            "Tool hook matcher group needs a hooks array.",
                            "Put command handlers inside the matcher's hooks array.",
                        ),
                    }
                } else {
                    handler_config(cx, root, &location, item);
                }
            }
        }
    }
}

fn handler_config(cx: &mut JsonCx<'_>, root: &JsonNode, pointer: &str, value: &Value) {
    let valid = value.as_object().is_some_and(|object| {
        object
            .get("command")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.trim().is_empty())
            && object
                .get("type")
                .is_none_or(|v| v.as_str() == Some("command"))
            && object.get("timeout").is_none_or(integer)
    });
    if !valid {
        issue(
            cx,
            root,
            "AGY-HK-001",
            pointer,
            DiagnosticLevel::Error,
            "Invalid Antigravity command hook handler.",
            "Set a non-empty command, optional type command, and an integer timeout in seconds.",
        );
    }
}

fn keybindings(cx: &mut JsonCx<'_>, root: &JsonNode, value: &Value) {
    let Some(object) = value.as_object() else {
        return;
    };
    for (action, entry) in object {
        if action == "$schema" {
            continue;
        }
        let disables_protected = matches!(action.as_str(), "cli.exit" | "cli.enter")
            && entry.as_array().is_some_and(Vec::is_empty);
        if !strings(entry) || disables_protected {
            issue(
                cx,
                root,
                "AGY-KB-001",
                &format!("/{}", escaped(action)),
                DiagnosticLevel::Warning,
                "Invalid or protected Antigravity keybinding override.",
                "Map actions to arrays of hotkey strings; cli.exit and cli.enter cannot be disabled.",
            );
        }
    }
}

fn rule_registry(cx: &mut JsonCx<'_>, root: &JsonNode, value: &Value) {
    for key in ["inherits", "entries"] {
        let Some(entries) = value.get(key) else {
            continue;
        };
        let valid = entries.as_array().is_some_and(|items| {
            items.iter().all(|item| {
                item.as_object().is_some_and(|object| {
                    object
                        .get("path")
                        .and_then(Value::as_str)
                        .is_some_and(|s| !s.is_empty())
                        && ["include_only", "exclude"]
                            .iter()
                            .all(|field| object.get(*field).is_none_or(strings))
                })
            })
        });
        if !valid {
            issue(
                cx,
                root,
                "AGY-RL-004",
                &format!("/{key}"),
                DiagnosticLevel::Error,
                "Invalid Antigravity rule registry entry.",
                "Use arrays of objects with path and optional include_only/exclude string lists.",
            );
        }
    }
}

/// Count literal bytes and expanded includes iteratively. The analysis shares
/// the linter's existing aggregate read limit and never executes references.
/// An unavailable include or cycle leaves the result unknown.
fn expanded_rule_size(path: &Path, content: &str, config: &PerFileLintConfig<'_>) -> Option<usize> {
    enum Work {
        Text(std::path::PathBuf, String),
        Include(std::path::PathBuf),
        End(std::path::PathBuf),
        Bytes(usize),
    }
    static INCLUDES: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"@\[[^\]\r\n]*\]\(([^)\r\n]+)\)").expect("include pattern")
    });
    let fs = config.fs();
    let mut work = vec![Work::Text(path.to_path_buf(), content.to_owned())];
    let mut active = HashSet::new();
    let mut read_bytes = content.len() as u64;
    let mut size = 0usize;
    while let Some(item) = work.pop() {
        match item {
            Work::Bytes(bytes) => {
                size = size.saturating_add(bytes);
                if size > 24_000 {
                    return Some(size);
                }
            }
            Work::End(path) => {
                active.remove(&path);
            }
            Work::Include(path) => {
                let canonical = fs.canonicalize(&path).ok()?;
                if active.contains(&canonical) {
                    return None;
                }
                let metadata = fs.symlink_metadata(&path).ok()?;
                if metadata.is_symlink
                    || !metadata.is_file
                    || metadata.len > crate::file_utils::DEFAULT_MAX_FILE_SIZE
                {
                    return None;
                }
                let text = fs.read_to_string(&path).ok()?;
                read_bytes = read_bytes.checked_add(text.len() as u64)?;
                if read_bytes > crate::file_utils::DEFAULT_MAX_FILE_SIZE {
                    return None;
                }
                active.insert(canonical.clone());
                work.push(Work::End(canonical));
                let parts = split_frontmatter(&text);
                let body = if parts.has_frontmatter && parts.has_closing {
                    parts.body.to_string()
                } else {
                    text
                };
                work.push(Work::Text(path, body));
            }
            Work::Text(path, text) => {
                let mut items = Vec::new();
                let mut offset = 0;
                for capture in INCLUDES.captures_iter(&text) {
                    let whole = capture.get(0)?;
                    items.push(Work::Bytes(whole.start() - offset));
                    let target = capture.get(1)?.as_str();
                    let target = if let Some(tail) = target.strip_prefix("~/") {
                        crate::file_types::native::home_dir()?.join(tail)
                    } else {
                        path.parent().unwrap_or_else(|| Path::new(".")).join(target)
                    };
                    items.push(Work::Include(target));
                    offset = whole.end();
                }
                items.push(Work::Bytes(text.len() - offset));
                work.extend(items.into_iter().rev());
            }
        }
    }
    Some(size)
}

fn markdown(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
    agent: bool,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let syntax = if agent { "AGY-AG-001" } else { "AGY-RL-001" };
    let fields = if agent { "AGY-AG-002" } else { "AGY-RL-002" };
    let parts = split_frontmatter(content);
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
        if config.is_rule_enabled(syntax) {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    1,
                    syntax,
                    "Antigravity customization needs valid YAML frontmatter.",
                )
                .with_suggestion(
                    "Add a complete frontmatter object with the required native fields.",
                ),
            );
        }
        return diagnostics;
    };
    let mut emit = |rule: &str, key: &str, message: &str, suggestion: &str| {
        if !config.is_rule_enabled(rule) {
            return;
        }
        let (line, column) =
            crate::rules::config_span::yaml_key_pos(&parts.frontmatter, &[key]).unwrap_or((1, 1));
        let (start, _) = crate::rules::line_col_at_offset(
            content,
            crate::rules::frontmatter_content_offset(content, parts.frontmatter_start),
        );
        diagnostics.push(
            Diagnostic::error(path.to_path_buf(), start + line - 1, column, rule, message)
                .with_suggestion(suggestion),
        );
    };
    if agent {
        for key in ["name", "description"] {
            if !value
                .get(key)
                .and_then(Value::as_str)
                .is_some_and(|s| !s.trim().is_empty())
            {
                emit(
                    fields,
                    key,
                    "Antigravity agents need a non-empty name and description.",
                    "Set both fields in the agent frontmatter.",
                );
            }
        }
        for (key, entry) in value.as_object().into_iter().flatten() {
            let valid = match key.as_str() {
                "tools" | "skills" | "plugins" => strings(entry),
                "mainAgent" | "subagent" => entry.is_boolean(),
                "model" | "permissionMode" => entry.is_string(),
                "commandExecutionPolicy" => entry
                    .as_str()
                    .is_some_and(|s| ["off", "auto", "eager", "sandbox"].contains(&s)),
                "mcpServers" => {
                    entry.is_object()
                        || entry
                            .as_array()
                            .is_some_and(|items| items.iter().all(Value::is_object))
                }
                _ => true,
            };
            if !valid {
                emit(
                    fields,
                    key,
                    "Invalid Antigravity agent frontmatter field.",
                    "Use documented field types; model can be an alias or a full model identifier.",
                );
            }
        }
    } else {
        let trigger = value.get("trigger").and_then(Value::as_str);
        if !trigger.is_some_and(|s| ["always_on", "model_decision", "glob", "manual"].contains(&s))
        {
            emit(
                syntax,
                "trigger",
                "Antigravity ignores a rule without a recognized trigger.",
                "Use always_on, model_decision, glob or manual.",
            );
        }
        if trigger == Some("model_decision")
            && !value
                .get("description")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.trim().is_empty())
        {
            emit(
                fields,
                "description",
                "A model_decision rule needs a description.",
                "Describe when the agent should load the rule.",
            );
        }
        if trigger == Some("glob")
            && !value
                .get("globs")
                .or_else(|| value.get("glob"))
                .and_then(Value::as_str)
                .is_some_and(|s| !s.trim().is_empty())
        {
            emit(
                fields,
                "globs",
                "A glob rule needs globs or glob as a non-empty string.",
                "Use a quoted comma-separated string of file patterns.",
            );
        }
        for key in ["description", "globs", "glob"] {
            if value.get(key).is_some_and(|v| !v.is_string()) {
                emit(
                    fields,
                    key,
                    "Rule descriptions and glob expressions must be strings.",
                    "Use the native Antigravity rule frontmatter types.",
                );
            }
        }
        if expanded_rule_size(path, content, config).is_some_and(|size| size > 24_000) {
            emit(
                "AGY-RL-003",
                "trigger",
                "Antigravity truncates rule files after 24,000 bytes.",
                "Split the rule into smaller files or use model_decision disclosure.",
            );
        }
    }
    diagnostics
}
