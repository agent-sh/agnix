//! Pi configuration contracts, pinned to the documented v1.4.0 formats.
use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    file_types::{FileType, native},
    rules::{
        Validator, ValidatorMetadata,
        config_json::JsonCx,
        tool_contract::{Schema, check_schema, json_document, pointer_span},
    },
    schemas::kiro_json::JsonNode,
};
use rust_i18n::t;
use serde_json::Value;
use std::{collections::HashSet, path::Path, sync::LazyLock};

const RULES: &[&str] = &[
    "PI-CFG-001",
    "PI-CFG-002",
    "PI-CFG-003",
    "PI-MDL-001",
    "PI-MDL-002",
    "PI-KB-001",
    "PI-MCP-001",
    "PI-MCP-002",
    "PI-MCP-003",
    "PI-MCP-004",
    "PI-MCP-005",
    "PI-PKG-001",
    "PI-PKG-002",
    "PI-TH-001",
    "PI-TH-002",
    "PI-PR-001",
    "PI-PR-002",
];

pub struct PiValidator;

impl Validator for PiValidator {
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
        if crate::pipeline::resolve_file_type(path, config) == FileType::PiPrompt {
            return prompt(path, content, config);
        }
        let actual_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let filename = if native::detect(path) == Some(FileType::PiConfig) {
            actual_name
        } else {
            native::hinted_basename(content).unwrap_or(actual_name)
        };
        let mut cx = JsonCx::new(path, content, config);
        let theme_resource = native::pi_resource(path, "themes")
            || native::resource_owner(
                path,
                config.fs().as_ref(),
                native::selected(config),
                config.root_dir().map(|root| root.as_path()),
            ) == Some(native::Resource::PiTheme);
        let models = filename == "models.json" && !theme_resource;
        // ModelConfig strips // comments and trailing commas, but not /* */.
        if models && block_comment(content).is_some() {
            if cx.enabled("PI-CFG-001") {
                let offset = block_comment(content).unwrap_or(0);
                cx.push(
                    DiagnosticLevel::Error,
                    "PI-CFG-001",
                    (offset, offset + 2),
                    t!("rules.pi_cfg_001.pi_models_json_does_not_accept_block").as_ref(),
                    t!("rules.pi_cfg_001.use_line_comments_or_ordinary_json").as_ref(),
                );
            }
            return cx.diagnostics;
        }
        let Some((root, value)) = json_document(&mut cx, content, "PI-CFG-001", models) else {
            return cx.diagnostics;
        };
        // A resource filename does not change the format of its directory.
        if theme_resource || filename == "theme.json" {
            check_schema(&mut cx, &root, &value, Schema::PiTheme, "PI-TH-001");
            theme(&mut cx, &root, &value);
            return cx.diagnostics;
        }
        match filename {
            "settings.json" => {
                check_schema(&mut cx, &root, &value, Schema::PiSettings, "PI-CFG-002");
                if native::pi_project(path) {
                    for key in ["cacheWarming", "defaultProjectTrust", "httpProxy"] {
                        if value.get(key).is_some() {
                            issue(
                                &mut cx,
                                &root,
                                "PI-CFG-003",
                                &format!("/{key}"),
                                DiagnosticLevel::Warning,
                                t!("rules.pi_cfg_003.pi_ignores_this_setting_in_project_configuration").as_ref(),
                                t!("rules.pi_cfg_003.move_this_setting_to_the_agent_directory").as_ref(),
                            );
                        }
                    }
                }
            }
            "models.json" => {
                if native::pi_project(path) {
                    issue(
                        &mut cx,
                        &root,
                        "PI-CFG-003",
                        "",
                        DiagnosticLevel::Warning,
                        t!("rules.pi_cfg_003.pi_loads_models_json_from_its_agent").as_ref(),
                        t!("rules.pi_cfg_003.put_provider_and_model_definitions_in_agent").as_ref(),
                    );
                }
                check_schema(&mut cx, &root, &value, Schema::PiModels, "PI-MDL-001");
                model_runtime(&mut cx, &root, &value);
            }
            "keybindings.json" => {
                if native::pi_project(path) {
                    issue(
                        &mut cx,
                        &root,
                        "PI-CFG-003",
                        "",
                        DiagnosticLevel::Warning,
                        t!("rules.pi_cfg_003.pi_loads_keybindings_json_from_its_agent").as_ref(),
                        t!("rules.pi_cfg_003.put_keybindings_in_agent_dir_keybindings_json")
                            .as_ref(),
                    );
                }
                check_schema(&mut cx, &root, &value, Schema::PiKeybindings, "PI-KB-001");
            }
            "mcp.json" => mcp(&mut cx, &root, &value, native::pi_project(path)),
            "package.json" => package(&mut cx, &root, &value),
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
fn exposure(value: &Value) -> bool {
    value.as_str().is_some_and(|s| {
        [
            "codemode",
            "codemode-deferred",
            "direct",
            "deferred",
            "hidden",
        ]
        .contains(&s)
    })
}
fn loopback(url: &url::Url) -> bool {
    matches!(
        url.host_str(),
        Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
    )
}

fn mcp(cx: &mut JsonCx<'_>, root: &JsonNode, value: &Value, project: bool) {
    if value
        .get("autoEnableCodemode")
        .is_some_and(|v| !v.is_boolean())
    {
        issue(
            cx,
            root,
            "PI-MCP-001",
            "/autoEnableCodemode",
            DiagnosticLevel::Error,
            t!("rules.pi_mcp_001.autoenablecodemode_must_be_a_boolean").as_ref(),
            t!("rules.pi_mcp_001.use_true_or_false").as_ref(),
        );
    }
    let Some(servers) = value.get("mcpServers") else {
        return;
    };
    let Some(servers) = servers.as_object() else {
        issue(
            cx,
            root,
            "PI-MCP-001",
            "/mcpServers",
            DiagnosticLevel::Error,
            t!("rules.pi_mcp_001.mcpservers_must_be_an_object_mapping_server").as_ref(),
            t!("rules.pi_mcp_001.use_the_documented_mcpservers_map").as_ref(),
        );
        return;
    };
    let mut namespaces = HashSet::new();
    for (name, server) in servers {
        let base = format!("/mcpServers/{}", escaped(name));
        let valid_name = !name.is_empty()
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'));
        if !valid_name {
            issue(
                cx,
                root,
                "PI-MCP-002",
                &base,
                DiagnosticLevel::Error,
                t!("rules.pi_mcp_002.invalid_pi_mcp_server_name").as_ref(),
                t!("rules.pi_mcp_002.use_ascii_letters_digits_underscores_and_hyphens").as_ref(),
            );
        }
        if !namespaces.insert(name.replace('-', "_")) {
            issue(
                cx,
                root,
                "PI-MCP-004",
                &base,
                DiagnosticLevel::Error,
                t!("rules.pi_mcp_004.pi_mcp_server_names_collide_after_hyphens").as_ref(),
                t!("rules.pi_mcp_004.give_the_servers_distinct_normalized_names").as_ref(),
            );
        }
        let Some(object) = server.as_object() else {
            issue(
                cx,
                root,
                "PI-MCP-002",
                &base,
                DiagnosticLevel::Error,
                t!("rules.pi_mcp_002.an_mcp_server_configuration_must_be_an").as_ref(),
                t!("rules.pi_mcp_002.define_a_stdio_server_http_server_or").as_ref(),
            );
            continue;
        };
        let partial = project
            && !["command", "url", "type"]
                .iter()
                .any(|key| object.contains_key(*key));
        if partial
            && object
                .keys()
                .any(|key| !["enabled", "exposure", "toolExposure"].contains(&key.as_str()))
        {
            issue(
                cx,
                root,
                "PI-MCP-002",
                &base,
                DiagnosticLevel::Error,
                t!("rules.pi_mcp_002.a_project_mcp_override_can_only_set").as_ref(),
                t!("rules.pi_mcp_002.define_a_complete_server_or_keep_only").as_ref(),
            );
        }
        for (key, entry) in object {
            let valid = match key.as_str() {
                "enabled" => entry.is_boolean(),
                "description" => entry.is_string(),
                "exposure" => exposure(entry),
                "toolExposure" => entry
                    .as_object()
                    .is_some_and(|map| map.values().all(exposure)),
                "timeout" => entry.as_f64().is_some_and(|number| number > 0.0),
                _ => true,
            };
            if !valid {
                issue(
                    cx,
                    root,
                    "PI-MCP-002",
                    &format!("{base}/{}", escaped(key)),
                    DiagnosticLevel::Error,
                    t!("rules.pi_mcp_002.invalid_pi_mcp_server_option").as_ref(),
                    t!("rules.pi_mcp_002.use_the_documented_boolean_string_exposure_or").as_ref(),
                );
            }
        }
        if partial {
            continue;
        }
        if object.get("type").is_some_and(|value| !value.is_string()) {
            issue(
                cx,
                root,
                "PI-MCP-002",
                &format!("{base}/type"),
                DiagnosticLevel::Error,
                t!("rules.pi_mcp_002.pi_mcp_transport_type_must_be_a").as_ref(),
                t!("rules.pi_mcp_002.omit_type_for_inference_or_use_a").as_ref(),
            );
            continue;
        }
        let kind = object.get("type").and_then(Value::as_str);
        let remote = object
            .get("url")
            .and_then(Value::as_str)
            .filter(|_| kind.is_none() || matches!(kind, Some("http" | "streamable-http")));
        let local = object
            .get("command")
            .and_then(Value::as_str)
            .filter(|_| kind.is_none() || kind == Some("stdio"));
        if let Some(endpoint) = remote {
            if !url::Url::parse(endpoint).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
            {
                issue(
                    cx,
                    root,
                    "PI-MCP-002",
                    &format!("{base}/url"),
                    DiagnosticLevel::Error,
                    t!("rules.pi_mcp_002.mcp_url_must_use_http_or_https").as_ref(),
                    t!("rules.pi_mcp_002.use_a_streamable_http_mcp_endpoint").as_ref(),
                );
            }
            if object
                .get("headers")
                .is_some_and(|entry| !string_map(entry))
            {
                issue(
                    cx,
                    root,
                    "PI-MCP-002",
                    &format!("{base}/headers"),
                    DiagnosticLevel::Error,
                    t!("rules.pi_mcp_002.mcp_headers_must_map_names_to_strings").as_ref(),
                    t!("rules.pi_mcp_002.use_string_header_values_environment_references_or")
                        .as_ref(),
                );
            }
            if let Some(auth) = object.get("auth") {
                if project {
                    issue(
                        cx,
                        root,
                        "PI-MCP-003",
                        &format!("{base}/auth"),
                        DiagnosticLevel::Error,
                        t!("rules.pi_mcp_003.project_mcp_configuration_cannot_select_global_login")
                            .as_ref(),
                        t!("rules.pi_mcp_003.configure_auth_provider_only_in_the_global").as_ref(),
                    );
                }
                if !auth
                    .get("provider")
                    .and_then(Value::as_str)
                    .is_some_and(|s| !s.is_empty())
                    || !url::Url::parse(endpoint)
                        .is_ok_and(|url| url.scheme() == "https" || loopback(&url))
                {
                    issue(
                        cx,
                        root,
                        "PI-MCP-003",
                        &format!("{base}/auth"),
                        DiagnosticLevel::Error,
                        t!("rules.pi_mcp_003.mcp_provider_authentication_needs_a_provider_name")
                            .as_ref(),
                        t!("rules.pi_mcp_003.use_auth_provider_with_an_https_or").as_ref(),
                    );
                }
            }
            if let Some(oauth) = object.get("oauth") {
                oauth_config(cx, root, &format!("{base}/oauth"), oauth);
            }
        } else if local.is_some() {
            for key in ["args", "env", "cwd"] {
                if let Some(entry) = object.get(key) {
                    let valid = match key {
                        "args" => strings(entry),
                        "env" => string_map(entry),
                        _ => entry.is_string(),
                    };
                    if !valid {
                        issue(
                            cx,
                            root,
                            "PI-MCP-002",
                            &format!("{base}/{key}"),
                            DiagnosticLevel::Error,
                            t!("rules.pi_mcp_002.invalid_stdio_mcp_option").as_ref(),
                            t!("rules.pi_mcp_002.use_a_string_array_for_args_a").as_ref(),
                        );
                    }
                }
            }
        } else {
            issue(
                cx,
                root,
                "PI-MCP-002",
                &base,
                DiagnosticLevel::Error,
                t!("rules.pi_mcp_002.pi_mcp_server_needs_command_stdio_or").as_ref(),
                t!("rules.pi_mcp_002.legacy_sse_is_unsupported_select_the_matching").as_ref(),
            );
        }
    }
}

fn oauth_config(cx: &mut JsonCx<'_>, root: &JsonNode, base: &str, value: &Value) {
    let Some(object) = value.as_object() else {
        issue(
            cx,
            root,
            "PI-MCP-005",
            base,
            DiagnosticLevel::Error,
            t!("rules.pi_mcp_005.mcp_oauth_settings_must_be_an_object").as_ref(),
            t!("rules.pi_mcp_005.use_the_documented_oauth_client_fields").as_ref(),
        );
        return;
    };
    let port = object.get("callbackPort");
    let callback = object.get("callbackUrl");
    let callback_url = callback
        .and_then(Value::as_str)
        .and_then(|s| url::Url::parse(s).ok());
    let mut valid = true;
    for key in [
        "clientId",
        "clientSecret",
        "scope",
        "clientName",
        "authServerMetadataUrl",
    ] {
        if object.get(key).is_some_and(|v| !v.is_string()) {
            valid = false;
        }
    }
    if object
        .get("clientName")
        .is_some_and(|v| v.as_str().is_some_and(|s| s.trim().is_empty()))
    {
        valid = false;
    }
    if port.is_some_and(|v| {
        !v.as_f64()
            .is_some_and(|p| p.fract() == 0.0 && (1.0..=65535.0).contains(&p))
    }) {
        valid = false;
    }
    if callback.is_some()
        && !callback_url.as_ref().is_some_and(|u| {
            u.scheme() == "http" && loopback(u) && u.query().is_none() && u.fragment().is_none()
        })
    {
        valid = false;
    }
    if let (Some(url), Some(port)) = (callback_url.as_ref(), port.and_then(Value::as_f64)) {
        if url.port().is_some_and(|p| f64::from(p) != port) {
            valid = false;
        }
    }
    if let Some(registration) = object.get("clientRegistration") {
        match registration.as_str() {
            Some("dcr") => {}
            Some("cimd") => {
                if object.contains_key("clientId") || object.contains_key("clientName") {
                    valid = false;
                }
                if callback_url.as_ref().is_some_and(|u| {
                    matches!(u.host_str(), Some("[::1]" | "::1")) || u.path() != "/callback"
                }) {
                    valid = false;
                }
            }
            _ => valid = false,
        }
    }
    if object.get("authServerMetadataUrl").is_some_and(|v| {
        !v.as_str()
            .and_then(|s| url::Url::parse(s).ok())
            .is_some_and(|u| u.scheme() == "https" || (u.scheme() == "http" && loopback(&u)))
    }) {
        valid = false;
    }
    if !valid {
        issue(
            cx,
            root,
            "PI-MCP-005",
            base,
            DiagnosticLevel::Error,
            t!("rules.pi_mcp_005.invalid_pi_mcp_oauth_callback_or_client").as_ref(),
            t!("rules.pi_mcp_005.use_http_loopback_callbacks_matching_callback_ports").as_ref(),
        );
    }
}

fn package(cx: &mut JsonCx<'_>, root: &JsonNode, value: &Value) {
    let empty = serde_json::Map::new();
    let object = match value.get("pi") {
        None => &empty,
        Some(manifest) => match manifest.as_object() {
            Some(object) => object,
            None => {
                issue(
                    cx,
                    root,
                    "PI-PKG-001",
                    "/pi",
                    DiagnosticLevel::Error,
                    t!("rules.pi_pkg_001.a_pi_package_manifest_must_be_an").as_ref(),
                    t!("rules.pi_pkg_001.declare_resource_paths_under_the_package_json").as_ref(),
                );
                return;
            }
        },
    };
    for key in ["extensions", "skills", "prompts", "themes"] {
        if object.get(key).is_some_and(|v| !strings(v)) {
            issue(
                cx,
                root,
                "PI-PKG-001",
                &format!("/pi/{key}"),
                DiagnosticLevel::Error,
                t!("rules.pi_pkg_001.pi_package_resources_must_be_arrays_of").as_ref(),
                t!("rules.pi_pkg_001.use_paths_or_glob_exclusion_patterns_relative").as_ref(),
            );
        }
    }
    for key in ["image", "video"] {
        if object.get(key).is_some_and(|v| !v.is_string()) {
            issue(
                cx,
                root,
                "PI-PKG-001",
                &format!("/pi/{key}"),
                DiagnosticLevel::Error,
                t!("rules.pi_pkg_001.pi_gallery_media_must_be_a_path").as_ref(),
                t!("rules.pi_pkg_001.set_a_relative_image_video_path_or").as_ref(),
            );
        }
    }
    for host in [
        "@earendil-works/pi-ai",
        "@earendil-works/pi-agent-core",
        "@earendil-works/pi-coding-agent",
        "@earendil-works/pi-tui",
        "typebox",
    ] {
        if value
            .get("dependencies")
            .and_then(|v| v.get(host))
            .is_some()
        {
            issue(
                cx,
                root,
                "PI-PKG-002",
                &format!("/dependencies/{}", escaped(host)),
                DiagnosticLevel::Warning,
                t!("rules.pi_pkg_002.a_host_provided_pi_package_is_installed").as_ref(),
                t!("rules.pi_pkg_002.move_the_host_package_to_peerdependencies_with").as_ref(),
            );
        }
    }
}

fn theme(cx: &mut JsonCx<'_>, root: &JsonNode, value: &Value) {
    if value
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|s| s == "system" || s.contains('/'))
    {
        issue(
            cx,
            root,
            "PI-TH-002",
            "/name",
            DiagnosticLevel::Error,
            t!("rules.pi_th_002.pi_theme_names_cannot_be_system_or").as_ref(),
            t!("rules.pi_th_002.choose_a_unique_theme_name_and_use").as_ref(),
        );
    }
    let empty = serde_json::Map::new();
    let vars = value
        .get("vars")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    for group in ["vars", "colors", "export"] {
        let Some(entries) = value.get(group).and_then(Value::as_object) else {
            continue;
        };
        for (name, entry) in entries {
            let mut current = entry;
            let mut seen = HashSet::new();
            while let Some(reference) = current.as_str() {
                let lower = reference.to_ascii_lowercase();
                if reference.is_empty()
                    || reference.starts_with('#')
                    || lower.starts_with("oklch(")
                    || lower.starts_with("okhsl(")
                {
                    if !reference.is_empty() && !color_literal(reference) {
                        issue(
                            cx,
                            root,
                            "PI-TH-002",
                            &format!("/{group}/{}", escaped(name)),
                            DiagnosticLevel::Error,
                            t!("rules.pi_th_002.invalid_pi_theme_color_literal").as_ref(),
                            t!("rules.pi_th_002.use_rgb_rrggbb_or_a_supported_oklch").as_ref(),
                        );
                    }
                    break;
                }
                if !seen.insert(reference) || !vars.contains_key(reference) {
                    issue(
                        cx,
                        root,
                        "PI-TH-002",
                        &format!("/{group}/{}", escaped(name)),
                        DiagnosticLevel::Error,
                        t!("rules.pi_th_002.pi_theme_references_a_missing_or_cyclic").as_ref(),
                        t!("rules.pi_th_002.define_the_variable_without_a_cycle_or").as_ref(),
                    );
                    break;
                }
                current = &vars[reference];
            }
        }
    }
}

fn color_literal(value: &str) -> bool {
    if let Some(hex) = value.strip_prefix('#') {
        return matches!(hex.len(), 3 | 6) && hex.bytes().all(|byte| byte.is_ascii_hexdigit());
    }
    const NUMBER: &str = r"[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?";
    static LCH: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(&format!(
            r"(?i)^oklch\(\s*({NUMBER})(%)?\s+({NUMBER})\s+({NUMBER})(?:deg)?\s*\)$"
        ))
        .expect("fixed OKLCH pattern")
    });
    static HSL: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(&format!(
            r"(?i)^okhsl\(\s*({NUMBER})(?:deg)?\s+({NUMBER})(%)?\s+({NUMBER})(%)?\s*\)$"
        ))
        .expect("fixed OKHSL pattern")
    });
    if let Some(caps) = LCH.captures(value) {
        let number = |index| {
            caps.get(index)
                .and_then(|part| part.as_str().parse::<f64>().ok())
                .filter(|number| number.is_finite())
        };
        return number(1)
            .zip(number(3))
            .zip(number(4))
            .is_some_and(|((l, c), _)| {
                let l = if caps.get(2).is_some() { l / 100.0 } else { l };
                (0.0..=1.0).contains(&l) && c >= 0.0
            });
    }
    if let Some(caps) = HSL.captures(value) {
        let number = |index| {
            caps.get(index)
                .and_then(|part| part.as_str().parse::<f64>().ok())
                .filter(|number| number.is_finite())
        };
        return number(1)
            .zip(number(2))
            .zip(number(4))
            .is_some_and(|((_, s), l)| {
                let s = if caps.get(3).is_some() { s / 100.0 } else { s };
                let l = if caps.get(5).is_some() { l / 100.0 } else { l };
                (0.0..=1.0).contains(&s) && (0.0..=1.0).contains(&l)
            });
    }
    false
}

fn prompt(path: &Path, content: &str, config: &PerFileLintConfig<'_>) -> Vec<Diagnostic> {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, "SYSTEM.md" | "APPEND_SYSTEM.md"))
        && native::detect(path) == Some(FileType::PiPrompt)
        && !native::pi_resource(path, "prompts")
    {
        return Vec::new();
    }
    let parts = crate::parsers::frontmatter::split_frontmatter(content);
    if !parts.has_frontmatter || !parts.has_closing {
        return Vec::new();
    }
    let value = if parts.has_closing
        && crate::parsers::frontmatter::check_yaml_depth(&parts.frontmatter).is_ok()
    {
        serde_yaml::from_str::<Value>(&parts.frontmatter).ok()
    } else {
        None
    };
    if value.is_none() && config.is_rule_enabled("PI-PR-001") {
        return vec![
            Diagnostic::error(
                path.to_path_buf(),
                1,
                1,
                "PI-PR-001",
                t!("rules.pi_pr_001.invalid_pi_prompt_frontmatter").as_ref(),
            )
            .with_suggestion(
                t!("rules.pi_pr_001.use_valid_yaml_with_optional_string_description").as_ref(),
            ),
        ];
    }
    if let Some(object) = value.as_ref().and_then(Value::as_object) {
        if ["description", "argument-hint"]
            .iter()
            .any(|key| object.get(*key).is_some_and(|value| !value.is_string()))
            && config.is_rule_enabled("PI-PR-002")
        {
            return vec![
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    1,
                    "PI-PR-002",
                    t!("rules.pi_pr_002.pi_ignores_non_string_prompt_description_and").as_ref(),
                )
                .with_suggestion(t!("rules.pi_pr_002.use_strings_or_omit_the_fields_to").as_ref()),
            ];
        }
    }
    Vec::new()
}

fn model_runtime(cx: &mut JsonCx<'_>, root: &JsonNode, value: &Value) {
    let Some(providers) = value.get("providers").and_then(Value::as_object) else {
        return;
    };
    for (name, provider) in providers {
        let Some(provider) = provider.as_object() else {
            continue;
        };
        let base = format!("/providers/{}", escaped(name));
        let text = |key| {
            provider
                .get(key)
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty())
        };
        let has_models = provider
            .get("models")
            .and_then(Value::as_array)
            .is_some_and(|v| !v.is_empty());
        let has_overrides = provider
            .get("modelOverrides")
            .and_then(Value::as_object)
            .is_some_and(|v| !v.is_empty());
        if text("oauth") && !text("baseUrl") {
            issue(
                cx,
                root,
                "PI-MDL-002",
                &base,
                DiagnosticLevel::Error,
                t!("rules.pi_mdl_002.pi_oauth_provider_configuration_requires_an_explicit")
                    .as_ref(),
                t!("rules.pi_mdl_002.set_the_provider_s_baseurl_when_oauth").as_ref(),
            );
        }
        if !has_models
            && !text("baseUrl")
            && !provider.contains_key("headers")
            && !provider.contains_key("compat")
            && !has_overrides
            && !text("apiKey")
            && !text("oauth")
            && !provider.contains_key("authHeader")
        {
            issue(
                cx,
                root,
                "PI-MDL-002",
                &base,
                DiagnosticLevel::Error,
                t!("rules.pi_mdl_002.pi_rejects_an_empty_provider_configuration_with").as_ref(),
                t!("rules.pi_mdl_002.define_models_baseurl_headers_compat_modeloverrides_or")
                    .as_ref(),
            );
        }
    }
}

/// Locate a /* token outside JSON strings and // comments without changing offsets.
fn block_comment(content: &str) -> Option<usize> {
    let bytes = content.as_bytes();
    let (mut index, mut quoted, mut escaped, mut line_comment) = (0, false, false, false);
    while index < bytes.len() {
        let ch = bytes[index];
        if line_comment {
            if matches!(ch, b'\n' | b'\r') {
                line_comment = false;
            }
        } else if quoted {
            if escaped {
                escaped = false;
            } else if ch == b'\\' {
                escaped = true;
            } else if ch == b'"' {
                quoted = false;
            }
        } else if ch == b'"' {
            quoted = true;
        } else if ch == b'/' && bytes.get(index + 1) == Some(&b'/') {
            line_comment = true;
            index += 1;
        } else if ch == b'/' && bytes.get(index + 1) == Some(&b'*') {
            return Some(index);
        }
        index += 1;
    }
    None
}
