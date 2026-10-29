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
                    "Pi models.json does not accept block comments.",
                    "Use // line comments or ordinary JSON.",
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
                                "Pi ignores this setting in project configuration.",
                                "Move this setting to the agent-directory settings.json.",
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
                        "Pi loads models.json from its agent directory, not the project .pi directory.",
                        "Put provider and model definitions in <agent-dir>/models.json.",
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
                        "Pi loads keybindings.json from its agent directory.",
                        "Put keybindings in <agent-dir>/keybindings.json and run /reload.",
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
            "autoEnableCodemode must be a boolean.",
            "Use true or false.",
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
            "mcpServers must be an object mapping server names to configurations.",
            "Use the documented mcpServers map.",
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
                "Invalid Pi MCP server name.",
                "Use ASCII letters, digits, underscores and hyphens.",
            );
        }
        if !namespaces.insert(name.replace('-', "_")) {
            issue(
                cx,
                root,
                "PI-MCP-004",
                &base,
                DiagnosticLevel::Error,
                "Pi MCP server names collide after hyphens become underscores.",
                "Give the servers distinct normalized names.",
            );
        }
        let Some(object) = server.as_object() else {
            issue(
                cx,
                root,
                "PI-MCP-002",
                &base,
                DiagnosticLevel::Error,
                "An MCP server configuration must be an object.",
                "Define a stdio server, HTTP server or project override.",
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
                "A project MCP override can only set enabled, exposure and toolExposure.",
                "Define a complete server, or keep only the inherited server's override fields.",
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
                    "Invalid Pi MCP server option.",
                    "Use the documented boolean, string, exposure or positive timeout value.",
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
                "Pi MCP transport type must be a string when present.",
                "Omit type for inference, or use a documented transport name.",
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
                    "MCP URL must use HTTP or HTTPS.",
                    "Use a Streamable HTTP MCP endpoint.",
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
                    "MCP headers must map names to strings.",
                    "Use string header values, environment references or command references.",
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
                        "Project MCP configuration cannot select global login credentials.",
                        "Configure auth.provider only in the global agent-directory mcp.json.",
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
                        "MCP provider authentication needs a provider name and HTTPS or a loopback URL.",
                        "Use auth.provider with an HTTPS or local-loopback endpoint in global configuration.",
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
                            "Invalid stdio MCP option.",
                            "Use a string array for args, a string map for env, and a string for cwd.",
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
                "Pi MCP server needs command (stdio) or url (Streamable HTTP).",
                "Legacy SSE is unsupported; select the matching type and transport fields.",
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
            "MCP OAuth settings must be an object.",
            "Use the documented OAuth client fields.",
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
            "Invalid Pi MCP OAuth callback or client registration settings.",
            "Use HTTP loopback callbacks, matching callback ports, and the documented dcr/cimd client options.",
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
                    "A Pi package manifest must be an object.",
                    "Declare resource paths under the package.json pi object.",
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
                "Pi package resources must be arrays of path strings.",
                "Use paths or glob/exclusion patterns relative to the package root.",
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
                "Pi gallery media must be a path string.",
                "Set a relative image/video path, or omit this optional field.",
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
                "A host-provided Pi package is installed as a runtime dependency.",
                "Move the host package to peerDependencies with a * range so extensions share Pi's runtime modules.",
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
            "Pi theme names cannot be system or contain a slash.",
            "Choose a unique theme name and use it as the filename.",
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
                            "Invalid Pi theme color literal.",
                            "Use #rgb/#rrggbb or a supported OKLCH/OKHSL value with finite channels and valid ranges.",
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
                        "Pi theme references a missing or cyclic color variable.",
                        "Define the variable without a cycle, or use a documented literal color.",
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
                "Invalid Pi prompt frontmatter.",
            )
            .with_suggestion(
                "Use valid YAML with optional string description and argument-hint fields.",
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
                    "Pi ignores non-string prompt description and argument-hint fields.",
                )
                .with_suggestion(
                    "Use strings, or omit the fields to use the body's description fallback.",
                ),
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
                "Pi OAuth provider configuration requires an explicit baseUrl.",
                "Set the provider's baseUrl when oauth is configured.",
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
                "Pi rejects an empty provider configuration with no model or connection override.",
                "Define models, baseUrl, headers, compat, modelOverrides or authentication overrides, or remove the provider entry.",
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
