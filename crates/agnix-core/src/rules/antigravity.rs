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
use rust_i18n::t;
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
                            t!("rules.agy_rl_003.antigravity_truncates_rule_files_after_24_000")
                                .as_ref(),
                        )
                        .with_suggestion(
                            t!(
                                "rules.agy_rl_003.split_the_instructions_into_smaller_modular_rules"
                            )
                            .as_ref(),
                        ),
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
                        t!("rules.agy_cfg_002.antigravity_cli_remote_control_hostname_must_be")
                            .as_ref(),
                        t!("rules.agy_cfg_002.use_the_daemon_hostname_setting_documented_in")
                            .as_ref(),
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
                            t!(
                                "rules.agy_cfg_003.antigravity_cli_does_not_load_this_customization"
                            )
                            .as_ref(),
                            t!("rules.agy_cfg_003.move_hooks_to_agents_hooks_json_or").as_ref(),
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
                                        t!("rules.agy_cfg_004.antigravity_ignores_this_malformed_permission_resource").as_ref(),
                                        t!("rules.agy_cfg_004.use_action_target_without_a_space_before").as_ref(),
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
                    let message = if advisory {
                        t!("rules.agy_pl_002.plugin_metadata_is_ignored_or_incomplete_for")
                    } else {
                        t!("rules.agy_pl_001.invalid_antigravity_plugin_manifest_field")
                    };
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
                        message.as_ref(),
                        t!("rules.agy_pl_001.use_the_native_plugin_schema_name_is").as_ref(),
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
                t!("rules.agy_cfg_004.antigravity_ignores_this_malformed_permission_resource")
                    .as_ref(),
                t!("rules.agy_cfg_004.use_a_documented_action_target_string_without").as_ref(),
            );
        } else if formatter_fallback {
            issue(
                cx,
                root,
                "AGY-CFG-005",
                &error.pointer,
                DiagnosticLevel::Warning,
                t!("rules.agy_cfg_005.antigravity_uses_the_xml_tool_formatter_for").as_ref(),
                t!("rules.agy_cfg_005.use_a_documented_formatter_value_or_omit").as_ref(),
            );
        } else {
            issue(
                cx,
                root,
                "AGY-CFG-002",
                &error.pointer,
                DiagnosticLevel::Error,
                t!(
                    "rules.agy_cfg_002.antigravity_setting_violates_the_documented_constraint",
                    keyword = error.keyword.as_str()
                )
                .as_ref(),
                t!("rules.agy_cfg_002.use_the_types_and_values_in_the").as_ref(),
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
                t!("rules.agy_mcp_001.mcpservers_must_be_an_object").as_ref(),
                t!("rules.agy_mcp_001.map_server_names_to_stdio_or_streamable").as_ref(),
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
                t!("rules.agy_mcp_001.an_antigravity_mcp_server_must_be_an").as_ref(),
                t!("rules.agy_mcp_001.use_command_for_stdio_or_serverurl_url").as_ref(),
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
                t!("rules.agy_mcp_002.antigravity_cli_does_not_support_httpurl_or").as_ref(),
                t!("rules.agy_mcp_002.use_a_streamable_http_endpoint_under_serverurl").as_ref(),
            );
        }
        if command.is_none() == remote.is_none() {
            issue(
                cx,
                root,
                "AGY-MCP-001",
                &base,
                DiagnosticLevel::Error,
                t!("rules.agy_mcp_001.an_mcp_server_needs_one_stdio_command").as_ref(),
                t!("rules.agy_mcp_001.set_command_for_a_local_process_or").as_ref(),
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
                    t!("rules.agy_mcp_001.invalid_antigravity_mcp_server_option").as_ref(),
                    t!("rules.agy_mcp_001.use_the_documented_types_timeoutseconds_accepts_zero")
                        .as_ref(),
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
                    t!("rules.agy_mcp_001.mcp_remote_endpoint_must_be_an_http").as_ref(),
                    t!("rules.agy_mcp_001.use_a_streamable_http_server_url_or").as_ref(),
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
                t!("rules.agy_hk_001.an_antigravity_named_hook_must_map_events").as_ref(),
                t!("rules.agy_hk_001.use_a_named_hook_object_with_native").as_ref(),
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
                        t!("rules.agy_hk_001.hook_enabled_must_be_a_boolean").as_ref(),
                        t!("rules.agy_hk_001.use_true_or_false").as_ref(),
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
                    t!("rules.agy_hk_002.unrecognized_antigravity_lifecycle_event").as_ref(),
                    t!("rules.agy_hk_002.use_sessionstart_pretooluse_posttooluse_preinvocation_postinvocation_or").as_ref(),
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
                    t!("rules.agy_hk_001.hook_event_handlers_must_be_an_array").as_ref(),
                    t!("rules.agy_hk_001.use_matcher_groups_for_tool_events_and").as_ref(),
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
                            t!("rules.agy_hk_001.a_hook_matcher_must_be_a_string").as_ref(),
                            t!("rules.agy_hk_001.use_a_tool_name_expression_or_omit").as_ref(),
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
                            t!("rules.agy_hk_001.tool_hook_matcher_group_needs_a_hooks").as_ref(),
                            t!("rules.agy_hk_001.put_command_handlers_inside_the_matcher_s")
                                .as_ref(),
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
            t!("rules.agy_hk_001.invalid_antigravity_command_hook_handler").as_ref(),
            t!("rules.agy_hk_001.set_a_non_empty_command_optional_type").as_ref(),
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
                t!("rules.agy_kb_001.invalid_or_protected_antigravity_keybinding_override")
                    .as_ref(),
                t!("rules.agy_kb_001.map_actions_to_arrays_of_hotkey_strings").as_ref(),
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
                t!("rules.agy_rl_004.invalid_antigravity_rule_registry_entry").as_ref(),
                t!("rules.agy_rl_004.use_arrays_of_objects_with_path_and").as_ref(),
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
            let key = syntax.to_ascii_lowercase().replace('-', "_");
            let message_key = format!("rules.{key}.frontmatter_error");
            let suggestion_key = format!("rules.{key}.frontmatter_suggestion");
            diagnostics.push(
                Diagnostic::error(path.to_path_buf(), 1, 1, syntax, t!(&message_key).as_ref())
                    .with_suggestion(t!(&suggestion_key).as_ref()),
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
                    t!("rules.agy_ag_002.antigravity_agents_need_a_non_empty_name").as_ref(),
                    t!("rules.agy_ag_002.set_both_fields_in_the_agent_frontmatter").as_ref(),
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
                    t!("rules.agy_ag_002.invalid_antigravity_agent_frontmatter_field").as_ref(),
                    t!("rules.agy_ag_002.use_documented_field_types_model_can_be").as_ref(),
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
                t!("rules.agy_rl_001.antigravity_ignores_a_rule_without_a_recognized").as_ref(),
                t!("rules.agy_rl_001.use_always_on_model_decision_glob_or").as_ref(),
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
                t!("rules.agy_rl_002.a_model_decision_rule_needs_a_description").as_ref(),
                t!("rules.agy_rl_002.describe_when_the_agent_should_load_the").as_ref(),
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
                t!("rules.agy_rl_002.a_glob_rule_needs_globs_or_glob").as_ref(),
                t!("rules.agy_rl_002.use_a_quoted_comma_separated_string_of").as_ref(),
            );
        }
        for key in ["description", "globs", "glob"] {
            if value.get(key).is_some_and(|v| !v.is_string()) {
                emit(
                    fields,
                    key,
                    t!("rules.agy_rl_002.rule_descriptions_and_glob_expressions_must_be").as_ref(),
                    t!("rules.agy_rl_002.use_the_native_antigravity_rule_frontmatter_types")
                        .as_ref(),
                );
            }
        }
        if expanded_rule_size(path, content, config).is_some_and(|size| size > 24_000) {
            emit(
                "AGY-RL-003",
                "trigger",
                t!("rules.agy_rl_003.antigravity_truncates_rule_files_after_24_000").as_ref(),
                t!("rules.agy_rl_003.split_the_rule_into_smaller_files_or").as_ref(),
            );
        }
    }
    diagnostics
}
