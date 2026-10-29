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
use rust_i18n::t;
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
        if path.extension().and_then(|s| s.to_str()) == Some("toml")
            && !matches!(
                resource,
                Some(
                    native::Resource::GrokHooks
                        | native::Resource::GrokMcp
                        | native::Resource::GrokLsp
                )
            )
        {
            let value = match toml::from_str::<toml::Value>(content) {
                Ok(value) => serde_json::to_value(value).unwrap_or(Value::Null),
                Err(error) => {
                    if cx.enabled("GRK-CFG-001") {
                        cx.push(
                            DiagnosticLevel::Error,
                            "GRK-CFG-001",
                            error.span().map_or((0, 0), |s| (s.start, s.end)),
                            t!("rules.grk_cfg_001.invalid_grok_toml_configuration").as_ref(),
                            t!("rules.grk_cfg_001.correct_the_toml_syntax_before_grok_loads")
                                .as_ref(),
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
                                t!(
                                    "rules.grk_sbx_001.grok_custom_profiles_cannot_redefine_a_built"
                                )
                                .as_ref(),
                                t!("rules.grk_sbx_001.choose_a_custom_name_and_extend_a").as_ref(),
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
    let key = rule.to_ascii_lowercase().replace('-', "_");
    let message_key = format!("rules.{key}.schema_message");
    let suggestion_key = format!("rules.{key}.schema_suggestion");
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
            t!(&message_key, keyword = issue.keyword.as_str()).as_ref(),
            t!(&suggestion_key).as_ref(),
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
    let default_home = native::home_dir().map(|home| home.join(".grok"));
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
                                t!("rules.grk_cfg_002.grok_ignores_this_mcp_option_in_project")
                                    .as_ref(),
                                t!("rules.grk_cfg_002.only_mcp_max_output_bytes_is_loaded")
                                    .as_ref(),
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
                        t!("rules.grk_cfg_002.grok_ignores_this_setting_in_project_config")
                            .as_ref(),
                        t!("rules.grk_cfg_002.put_user_settings_in_grok_home_config").as_ref(),
                    );
                }
            }
        }
    }
    for (pointer, minimum, message) in [
        (
            "/ui/simple_mode",
            "1.0.69",
            t!("rules.grk_cfg_004.grok_1_0_69_retired_ui_simple").as_ref(),
        ),
        (
            "/ui/vim_mode",
            "1.0.68",
            t!("rules.grk_cfg_004.grok_1_0_68_replaced_ui_vim").as_ref(),
        ),
        (
            "/memory_v2/compact_index_enabled",
            "1.0.63",
            t!("rules.grk_cfg_004.grok_1_0_63_retired_memory_v2").as_ref(),
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
                t!("rules.grk_cfg_004.remove_retired_settings_use_keys_vim_off").as_ref(),
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
            t!("rules.grk_cfg_005.grok_honors_the_bypass_permissions_lock_only").as_ref(),
            t!("rules.grk_cfg_005.set_the_lock_in_etc_grok_requirements").as_ref(),
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
                        t!("rules.grk_cfg_006.grok_version_override_bound_must_be_a").as_ref(),
                        t!("rules.grk_cfg_006.use_a_complete_major_minor_patch_version").as_ref(),
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
                t!("rules.grk_cfg_006.this_grok_version_range_can_never_apply").as_ref(),
                t!("rules.grk_cfg_006.put_minimum_version_at_or_below_maximum").as_ref(),
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
            t!("rules.grk_mcp_001.grok_mcp_server_declarations_must_be_a").as_ref(),
            t!("rules.grk_mcp_001.map_server_names_to_stdio_command_or").as_ref(),
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
            t!("rules.grk_mcp_001.grok_mcp_server_must_be_an_object").as_ref(),
            t!("rules.grk_mcp_001.configure_command_args_env_or_url_headers").as_ref(),
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
            t!("rules.grk_mcp_001.grok_mcp_server_needs_a_non_empty").as_ref(),
            t!("rules.grk_mcp_001.use_command_for_stdio_or_url_for").as_ref(),
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
                t!("rules.grk_mcp_001.invalid_grok_mcp_server_option_type").as_ref(),
                t!("rules.grk_mcp_001.use_the_types_documented_in_the_grok").as_ref(),
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
            t!("rules.grk_hk_001.grok_hook_files_need_a_hooks_object").as_ref(),
            t!("rules.grk_hk_001.put_lifecycle_events_inside_the_hooks_map").as_ref(),
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
            t!("rules.grk_hk_001.grok_hooks_must_map_event_names_to").as_ref(),
            t!("rules.grk_hk_001.use_the_documented_hooks_object").as_ref(),
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
                t!("rules.grk_hk_002.grok_skips_this_unknown_hook_event").as_ref(),
                t!("rules.grk_hk_002.use_a_documented_lifecycle_event_or_supported").as_ref(),
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
                t!("rules.grk_hk_001.hook_event_must_contain_a_matcher_group").as_ref(),
                t!("rules.grk_hk_001.put_handlers_in_each_group_s_hooks").as_ref(),
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
                        t!("rules.grk_hk_001.hook_matcher_must_be_a_string_or").as_ref(),
                        t!("rules.grk_hk_001.use_a_tool_matcher_expression_or_omit").as_ref(),
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
                            t!("rules.grk_hk_003.invalid_grok_hook_matcher_regular_expression")
                                .as_ref(),
                            t!("rules.grk_hk_003.use_a_valid_expression_in_grok_s").as_ref(),
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
                    t!("rules.grk_hk_001.hook_matcher_group_needs_a_hooks_array").as_ref(),
                    t!("rules.grk_hk_001.add_command_or_http_hook_handlers").as_ref(),
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
                        t!("rules.grk_hk_001.invalid_grok_hook_handler").as_ref(),
                        t!("rules.grk_hk_001.use_type_command_with_command_or_type").as_ref(),
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
                    t!("rules.grk_pl_002.grok_skips_plugin_component_paths_outside_the").as_ref(),
                    t!("rules.grk_pl_002.keep_component_paths_within_the_plugin_directory")
                        .as_ref(),
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
                t!("rules.grk_pl_001.invalid_grok_plugin_lsp_server_configuration").as_ref(),
                t!("rules.grk_pl_001.use_the_native_lsp_command_argument_transport").as_ref(),
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
                t!("rules.grk_ag_001.grok_agent_needs_valid_yaml_frontmatter").as_ref(),
                t!("rules.grk_ag_001.add_a_complete_agent_metadata_object_with").as_ref(),
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
                    t!("rules.grk_ag_002.invalid_grok_inline_mcp_server_reference").as_ref(),
                    t!("rules.grk_ag_002.use_a_server_name_a_single_name").as_ref(),
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
                    t!("rules.grk_ag_004.grok_plugin_agents_cannot_use_this_privileged").as_ref(),
                    t!("rules.grk_ag_004.configure_trusted_plugin_session_mcp_and_hooks").as_ref(),
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
            t!("rules.grk_sk_001.grok_skill_allowed_tools_does_not_grant").as_ref(),
            t!("rules.grk_sk_001.use_grok_permission_rules_or_an_agent").as_ref(),
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
                t!("rules.grk_sk_002.grok_treats_only_literal_true_as_enabled").as_ref(),
                t!("rules.grk_sk_002.use_true_or_false_not_yes_no").as_ref(),
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
