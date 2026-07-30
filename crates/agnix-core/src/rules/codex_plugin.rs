//! Codex CLI plugin manifest validation (CDX-PL-001 to CDX-PL-016).
//!
//! Validates `.codex-plugin/plugin.json` manifests for the Codex CLI
//! plugin system introduced in v0.117.0.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::{Validator, ValidatorMetadata},
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &[
    "CDX-PL-001",
    "CDX-PL-002",
    "CDX-PL-003",
    "CDX-PL-004",
    "CDX-PL-005",
    "CDX-PL-006",
    "CDX-PL-007",
    "CDX-PL-008",
    "CDX-PL-009",
    "CDX-PL-010",
    "CDX-PL-011",
    "CDX-PL-012",
    "CDX-PL-013",
    "CDX-PL-014",
    "CDX-PL-015",
    "CDX-PL-016",
];

/// Max number of defaultPrompt entries
const MAX_DEFAULT_PROMPT_COUNT: usize = 3;
/// Max characters per defaultPrompt entry
const MAX_DEFAULT_PROMPT_LEN: usize = 128;
const AGENT_PLUGIN_SCHEMA_URI: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";
pub(crate) const AGENT_PLUGIN_SCHEMA_PREFIX: &str = "https://agent-plugins.org/schemas/";

pub struct CodexPluginValidator;

impl Validator for CodexPluginValidator {
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
        let mut diagnostics = Vec::new();

        if !config.rules().codex {
            return diagnostics;
        }

        let schema = serde_json::from_str::<serde_json::Value>(content)
            .ok()
            .and_then(|value| {
                value
                    .get("$schema")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned)
            });
        let manifest_directory = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str());
        let is_in_codex_plugin =
            manifest_directory.is_some_and(|name| name.eq_ignore_ascii_case(".codex-plugin"));
        let is_in_other_legacy_plugin_directory = manifest_directory.is_some_and(|name| {
            name.eq_ignore_ascii_case(".claude-plugin")
                || name.eq_ignore_ascii_case(".cursor-plugin")
        });
        let is_agent_plugin = !is_in_codex_plugin
            && !is_in_other_legacy_plugin_directory
            && schema
                .as_deref()
                .is_some_and(|schema| schema.starts_with(AGENT_PLUGIN_SCHEMA_PREFIX));

        if !is_in_codex_plugin && !is_agent_plugin {
            return diagnostics;
        }

        if config.is_rule_enabled("CDX-PL-001")
            && is_agent_plugin
            && schema.as_deref() != Some(AGENT_PLUGIN_SCHEMA_URI)
        {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CDX-PL-001",
                    t!(
                        "rules.cdx_pl_001.unsupported_schema",
                        schema = schema.as_deref().unwrap_or("")
                    ),
                )
                .with_suggestion(t!("rules.cdx_pl_001.schema_suggestion")),
            );
        }

        // Codex rejects unsupported Agent Plugins schemas before interpreting
        // any other manifest fields. Do the same even when CDX-PL-001 is
        // disabled so fields from a future schema do not produce cascades.
        if is_agent_plugin && schema.as_deref() != Some(AGENT_PLUGIN_SCHEMA_URI) {
            return diagnostics;
        }

        // CDX-PL-002: Parse JSON
        let raw_value: serde_json::Value = match serde_json::from_str(content) {
            Ok(v) => v,
            Err(e) => {
                if config.is_rule_enabled("CDX-PL-002") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CDX-PL-002",
                            t!("rules.cdx_pl_002.message", error = e.to_string()),
                        )
                        .with_suggestion(t!("rules.cdx_pl_002.suggestion")),
                    );
                }
                return diagnostics;
            }
        };

        if is_agent_plugin
            && !validate_agent_plugin_types(&raw_value, path, config, &mut diagnostics)
        {
            return diagnostics;
        }

        // Agent Plugins keeps Codex-specific fields under
        // extensions["com.openai"]. The portable root fixes skills and MCP
        // paths and does not expose legacy apps/hooks/interface fields.
        let codex_fields = if is_agent_plugin {
            raw_value
                .get("extensions")
                .and_then(serde_json::Value::as_object)
                .and_then(|extensions| extensions.get("com.openai"))
                .filter(|extension| extension.is_object())
        } else {
            Some(&raw_value)
        };

        // CDX-PL-003: Missing or empty name
        if config.is_rule_enabled("CDX-PL-003") {
            let name_missing = match raw_value.get("name") {
                Some(v) => {
                    !v.is_string() || v.as_str().map(|s| s.trim().is_empty()).unwrap_or(true)
                }
                None => true,
            };
            if name_missing {
                let mut diagnostic = Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CDX-PL-003",
                    t!("rules.cdx_pl_003.message"),
                )
                .with_suggestion(t!("rules.cdx_pl_003.suggestion"));

                if let Some((start, end, _)) =
                    crate::span_utils::find_unique_json_string_value_range(content, "name")
                {
                    diagnostic = diagnostic.with_fix(Fix::replace(
                        start,
                        end,
                        "my-codex-plugin",
                        "Set plugin name to 'my-codex-plugin'",
                        false,
                    ));
                }

                diagnostics.push(diagnostic);
            }
        }

        // CDX-PL-004: Invalid name characters
        if config.is_rule_enabled("CDX-PL-004") {
            if let Some(name) = raw_value.get("name").and_then(|v| v.as_str()) {
                let trimmed = name.trim();
                let valid = if is_agent_plugin {
                    is_valid_agent_plugin_name(trimmed)
                } else {
                    is_valid_plugin_name(trimmed)
                };
                if !trimmed.is_empty() && !valid {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CDX-PL-004",
                            if is_agent_plugin {
                                t!("rules.cdx_pl_004.agent_message", name = trimmed)
                            } else {
                                t!("rules.cdx_pl_004.message", name = trimmed)
                            },
                        )
                        .with_suggestion(if is_agent_plugin {
                            t!("rules.cdx_pl_004.agent_suggestion")
                        } else {
                            t!("rules.cdx_pl_004.suggestion")
                        }),
                    );
                }
            }
        }

        // CDX-PL-005/006/007: Component path validation.
        let path_rules_enabled = config.is_rule_enabled("CDX-PL-005")
            || config.is_rule_enabled("CDX-PL-006")
            || config.is_rule_enabled("CDX-PL-007");
        if path_rules_enabled && let Some(fields) = codex_fields {
            let component_fields: &[&str] = if is_agent_plugin {
                &["apps"]
            } else {
                &["skills", "mcpServers", "apps"]
            };
            for field in component_fields {
                if let Some(val) = fields.get(*field).and_then(|v| v.as_str()) {
                    validate_component_path(val, field, path, content, config, &mut diagnostics);
                }
            }
            if let Some(hooks) = fields.get("hooks") {
                validate_hooks(hooks, path, content, config, &mut diagnostics);
            }
        }

        // CDX-PL-015: skills must be a string path. Codex ignores malformed
        // values with a warning, which can make bundled skills disappear.
        if !is_agent_plugin
            && config.is_rule_enabled("CDX-PL-015")
            && let Some(skills) = raw_value.get("skills")
            && !skills.is_string()
        {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "CDX-PL-015",
                    t!(
                        "rules.cdx_pl_015.message",
                        actual = json_value_type_name(skills)
                    ),
                )
                .with_suggestion(t!("rules.cdx_pl_015.suggestion")),
            );
        }

        // CDX-PL-008/009/010: defaultPrompt validation
        if let Some(interface) = codex_fields.and_then(|fields| fields.get("interface")) {
            if let Some(dp) = interface.get("defaultPrompt") {
                validate_default_prompt(dp, path, config, &mut diagnostics);
            }

            // CDX-PL-011: URL validation
            if config.is_rule_enabled("CDX-PL-011") {
                for field in &[
                    "websiteUrl",
                    "websiteURL",
                    "privacyPolicyUrl",
                    "privacyPolicyURL",
                    "termsOfServiceUrl",
                    "termsOfServiceURL",
                ] {
                    if let Some(url_val) = interface.get(*field) {
                        validate_interface_url(url_val, field, path, &mut diagnostics);
                    }
                }
            }

            // CDX-PL-012: Asset path validation (composerIcon, logo, screenshots)
            if config.is_rule_enabled("CDX-PL-012") {
                for field in &["composerIcon", "logo"] {
                    if let Some(val) = interface.get(*field).and_then(|v| v.as_str()) {
                        validate_asset_path(val, field, path, &mut diagnostics);
                    }
                }
                if let Some(screenshots) = interface.get("screenshots").and_then(|v| v.as_array()) {
                    for (i, entry) in screenshots.iter().enumerate() {
                        if let Some(val) = entry.as_str() {
                            let field_name = format!("screenshots[{}]", i);
                            validate_asset_path(val, &field_name, path, &mut diagnostics);
                        }
                    }
                }
            }

            // CDX-PL-016: dark-mode logo asset path validation. Codex
            // rust-v0.142.2 added interface.logoDark as a separate local
            // manifest field from remote catalog logoUrlDark values.
            if config.is_rule_enabled("CDX-PL-016")
                && let Some(val) = interface.get("logoDark").and_then(|v| v.as_str())
            {
                validate_logo_dark_path(val, path, content, &mut diagnostics);
            }
        }

        // CDX-PL-013: invalid hooks shape. Valid path forms are checked above
        // with the component path rules; inline hook objects are accepted.
        if let Some(hooks) = codex_fields.and_then(|fields| fields.get("hooks"))
            && config.is_rule_enabled("CDX-PL-013")
            && !is_valid_hooks_shape(hooks)
        {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "CDX-PL-013",
                    t!(
                        "rules.cdx_pl_013.message",
                        actual = json_value_type_name(hooks)
                    ),
                )
                .with_suggestion(t!("rules.cdx_pl_013.suggestion")),
            );
        }

        // CDX-PL-014: Missing description (recommendation)
        if config.is_rule_enabled("CDX-PL-014") {
            let desc_missing = match raw_value.get("description") {
                Some(v) => {
                    !v.is_string() || v.as_str().map(|s| s.trim().is_empty()).unwrap_or(true)
                }
                None => true,
            };
            if desc_missing {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "CDX-PL-014",
                        t!("rules.cdx_pl_014.message"),
                    )
                    .with_suggestion(t!("rules.cdx_pl_014.suggestion")),
                );
            }
        }

        diagnostics
    }
}

/// Validate plugin name: ASCII alphanumeric, hyphens, and underscores only.
fn is_valid_plugin_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn is_valid_agent_plugin_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.contains("--")
        && !name.contains("..")
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-".contains(&byte))
        && name
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && name
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
}

fn validate_agent_plugin_types(
    value: &serde_json::Value,
    path: &Path,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    let mut valid = true;

    for field in [
        "version",
        "description",
        "homepage",
        "repository",
        "license",
    ] {
        if let Some(field_value) = object.get(field)
            && !field_value.is_string()
        {
            valid = false;
            push_agent_plugin_type_diagnostic(
                path,
                field,
                "string",
                field_value,
                config,
                diagnostics,
            );
        }
    }

    if let Some(author) = object.get("author") {
        if let Some(author) = author.as_object() {
            for (field, field_value) in author {
                if !["name", "email", "url"].contains(&field.as_str()) {
                    valid = false;
                    push_agent_plugin_type_diagnostic(
                        path,
                        &format!("author.{field}"),
                        "name, email, or url",
                        field_value,
                        config,
                        diagnostics,
                    );
                } else if !field_value.is_string() {
                    valid = false;
                    push_agent_plugin_type_diagnostic(
                        path,
                        &format!("author.{field}"),
                        "string",
                        field_value,
                        config,
                        diagnostics,
                    );
                }
            }
        } else {
            valid = false;
            push_agent_plugin_type_diagnostic(
                path,
                "author",
                "object",
                author,
                config,
                diagnostics,
            );
        }
    }

    if let Some(keywords) = object.get("keywords")
        && (!keywords.is_array()
            || keywords
                .as_array()
                .is_some_and(|items| items.iter().any(|item| !item.is_string())))
    {
        valid = false;
        push_agent_plugin_type_diagnostic(
            path,
            "keywords",
            "array of strings",
            keywords,
            config,
            diagnostics,
        );
    }

    if let Some(extension) = object
        .get("extensions")
        .and_then(serde_json::Value::as_object)
        .and_then(|extensions| extensions.get("com.openai"))
        .and_then(serde_json::Value::as_object)
    {
        for (field, expected, is_valid) in [
            (
                "apps",
                "string",
                extension
                    .get("apps")
                    .is_none_or(|value| value.is_null() || value.is_string()),
            ),
            (
                "interface",
                "object",
                extension
                    .get("interface")
                    .is_none_or(|value| value.is_null() || value.is_object()),
            ),
        ] {
            if !is_valid {
                valid = false;
                let field_value = extension.get(field).expect("checked field exists");
                push_agent_plugin_type_diagnostic(
                    path,
                    &format!("extensions.com.openai.{field}"),
                    expected,
                    field_value,
                    config,
                    diagnostics,
                );
            }
        }
    }

    valid
}

fn push_agent_plugin_type_diagnostic(
    path: &Path,
    field: &str,
    expected: &str,
    value: &serde_json::Value,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if config.is_rule_enabled("CDX-PL-002") {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CDX-PL-002",
                t!(
                    "rules.cdx_pl_002.agent_type",
                    field = field,
                    expected = expected,
                    actual = json_value_type_name(value)
                ),
            )
            .with_suggestion(t!("rules.cdx_pl_002.agent_type_suggestion")),
        );
    }
}

fn is_valid_hooks_shape(value: &serde_json::Value) -> bool {
    value.is_null()
        || value.is_string()
        || value.is_object()
        || value.as_array().is_some_and(|entries| {
            entries.is_empty()
                || entries.iter().all(serde_json::Value::is_string)
                || entries.iter().all(serde_json::Value::is_object)
        })
}

fn validate_hooks(
    value: &serde_json::Value,
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(hook_path) = value.as_str() {
        validate_component_path(hook_path, "hooks", path, content, config, diagnostics);
    } else if let Some(hooks) = value.as_array()
        && hooks.iter().all(serde_json::Value::is_string)
    {
        for hook_path in hooks.iter().filter_map(serde_json::Value::as_str) {
            validate_component_path(hook_path, "hooks", path, content, config, diagnostics);
        }
    }
}

/// Validate a component path field (skills, mcpServers, apps).
fn validate_component_path(
    p: &str,
    field: &str,
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let trimmed = p.trim();
    if trimmed.is_empty() {
        return;
    }

    // CDX-PL-006: Check for .. traversal
    if config.is_rule_enabled("CDX-PL-006") && has_traversal(trimmed) {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CDX-PL-006",
                t!("rules.cdx_pl_006.message", path = trimmed, field = field),
            )
            .with_suggestion(t!("rules.cdx_pl_006.suggestion")),
        );
        return;
    }

    // CDX-PL-007: Check for bare ./
    if config.is_rule_enabled("CDX-PL-007") && (trimmed == "./" || trimmed == ".\\") {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CDX-PL-007",
                t!("rules.cdx_pl_007.message", path = trimmed, field = field),
            )
            .with_suggestion(t!("rules.cdx_pl_007.suggestion")),
        );
        return;
    }

    // CDX-PL-005: Must start with ./
    if config.is_rule_enabled("CDX-PL-005")
        && !trimmed.starts_with("./")
        && !trimmed.starts_with(".\\")
    {
        let mut diagnostic = Diagnostic::error(
            path.to_path_buf(),
            1,
            0,
            "CDX-PL-005",
            t!("rules.cdx_pl_005.message", path = trimmed, field = field),
        )
        .with_suggestion(t!("rules.cdx_pl_005.suggestion"));

        // Safe autofix: prepend ./
        if !is_absolute_path(trimmed) {
            if let Some((start, end)) =
                crate::rules::find_unique_json_string_value_span(content, field, trimmed)
            {
                let fixed = format!("./{}", trimmed);
                diagnostic = diagnostic.with_fix(Fix::replace(
                    start,
                    end,
                    &fixed,
                    format!("Prepend './' to path: '{}'", trimmed),
                    true,
                ));
            }
        }

        diagnostics.push(diagnostic);
    }
}

/// Check if path has .. traversal in any component.
fn has_traversal(p: &str) -> bool {
    p.split(['/', '\\']).any(|part| part == "..")
}

/// Check if path is absolute.
fn is_absolute_path(p: &str) -> bool {
    p.starts_with('/')
        || p.starts_with('\\')
        || (p.len() >= 2 && p.as_bytes()[0].is_ascii_alphabetic() && p.as_bytes()[1] == b':')
}

fn json_value_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Validate defaultPrompt field.
fn validate_default_prompt(
    value: &serde_json::Value,
    path: &Path,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let entries: Vec<&str> = match value {
        serde_json::Value::String(s) => vec![s.as_str()],
        serde_json::Value::Array(arr) => arr.iter().filter_map(|v| v.as_str()).collect(),
        _ => return,
    };

    // CDX-PL-008: Max count
    if config.is_rule_enabled("CDX-PL-008") && entries.len() > MAX_DEFAULT_PROMPT_COUNT {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                1,
                0,
                "CDX-PL-008",
                t!("rules.cdx_pl_008.message", count = entries.len()),
            )
            .with_suggestion(t!("rules.cdx_pl_008.suggestion")),
        );
    }

    for entry in &entries {
        // Normalize whitespace like Codex does
        let normalized: String = entry.split_whitespace().collect::<Vec<_>>().join(" ");

        // CDX-PL-010: Empty after normalization
        if config.is_rule_enabled("CDX-PL-010") && normalized.is_empty() {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "CDX-PL-010",
                    t!("rules.cdx_pl_010.message"),
                )
                .with_suggestion(t!("rules.cdx_pl_010.suggestion")),
            );
            continue;
        }

        // CDX-PL-009: Max length
        if config.is_rule_enabled("CDX-PL-009")
            && normalized.chars().count() > MAX_DEFAULT_PROMPT_LEN
        {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "CDX-PL-009",
                    t!(
                        "rules.cdx_pl_009.message",
                        length = normalized.chars().count()
                    ),
                )
                .with_suggestion(t!("rules.cdx_pl_009.suggestion")),
            );
        }
    }
}

/// Validate an interface URL field.
fn validate_interface_url(
    value: &serde_json::Value,
    field: &str,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match value.as_str() {
        Some(url) => {
            if !url.is_empty() && !url.starts_with("http://") && !url.starts_with("https://") {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "CDX-PL-011",
                        t!("rules.cdx_pl_011.message", url = url, field = field),
                    )
                    .with_suggestion(t!("rules.cdx_pl_011.suggestion")),
                );
            }
        }
        None => {
            let val_str = value.to_string();
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "CDX-PL-011",
                    t!(
                        "rules.cdx_pl_011.message",
                        url = val_str.as_str(),
                        field = field
                    ),
                )
                .with_suggestion(t!("rules.cdx_pl_011.suggestion")),
            );
        }
    }
}

/// Validate an asset path in the interface section.
fn validate_asset_path(p: &str, field: &str, path: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let trimmed = p.trim();
    if trimmed.is_empty() {
        return;
    }

    if has_traversal(trimmed) {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                1,
                0,
                "CDX-PL-012",
                t!("rules.cdx_pl_012.message", path = trimmed, field = field),
            )
            .with_suggestion(t!("rules.cdx_pl_012.suggestion")),
        );
        return;
    }

    if !trimmed.starts_with("./") && !trimmed.starts_with(".\\") {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                1,
                0,
                "CDX-PL-012",
                t!("rules.cdx_pl_012.message", path = trimmed, field = field),
            )
            .with_suggestion(t!("rules.cdx_pl_012.suggestion")),
        );
    }
}

/// Validate the dark-mode logo asset path in the interface section.
fn validate_logo_dark_path(p: &str, path: &Path, content: &str, diagnostics: &mut Vec<Diagnostic>) {
    let trimmed = p.trim();
    if trimmed.is_empty() {
        return;
    }

    if has_traversal(trimmed) || (!trimmed.starts_with("./") && !trimmed.starts_with(".\\")) {
        let line = find_json_key_line(content, "logoDark").unwrap_or(1);
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                0,
                "CDX-PL-016",
                t!("rules.cdx_pl_016.message", path = trimmed),
            )
            .with_suggestion(t!("rules.cdx_pl_016.suggestion")),
        );
    }
}

fn find_json_key_line(content: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{}\"", key);
    content
        .lines()
        .position(|line| line.contains(&needle))
        .map(|idx| idx + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use std::fs;
    use tempfile::TempDir;

    fn write_plugin(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    // ===== CDX-PL-001: Location check =====

    #[test]
    fn test_unrelated_root_plugin_is_not_treated_as_codex_plugin() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test-plugin","description":"desc"}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_cdx_pl_001_valid_location() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test-plugin","description":"desc"}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CDX-PL-001"));
    }

    #[test]
    fn test_cdx_pl_001_agent_plugin_root_manifest() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        let content = r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"example.plugin"}"#;
        write_plugin(&plugin_path, content);

        let diagnostics =
            CodexPluginValidator.validate(&plugin_path, content, &LintConfig::default());

        assert!(
            !diagnostics
                .iter()
                .any(|d| d.rule == "CDX-PL-001" || d.rule == "CDX-PL-004")
        );
    }

    #[test]
    fn test_cdx_pl_001_unsupported_agent_plugin_schema() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        let content = r#"{"$schema":"https://agent-plugins.org/schemas/2.0.0/plugin.schema.json","name":42,"extensions":{"com.openai":{"apps":"bad","hooks":42,"interface":{"websiteUrl":"bad"}}}}"#;
        write_plugin(&plugin_path, content);

        let diagnostics =
            CodexPluginValidator.validate(&plugin_path, content, &LintConfig::default());

        assert_eq!(
            diagnostics
                .iter()
                .map(|d| d.rule.as_str())
                .collect::<Vec<_>>(),
            vec!["CDX-PL-001"],
            "unsupported schemas must not produce cascade diagnostics"
        );
    }

    #[test]
    fn test_agent_plugin_requires_string_name() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        for name in [None, Some("42")] {
            let name = name.map_or_else(String::new, |name| format!(r#","name":{name}"#));
            let content =
                format!(r#"{{"$schema":"{AGENT_PLUGIN_SCHEMA_URI}"{name},"description":"desc"}}"#);
            write_plugin(&plugin_path, &content);

            let diagnostics =
                CodexPluginValidator.validate(&plugin_path, &content, &LintConfig::default());

            assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-003"));
        }
    }

    #[test]
    fn test_agent_plugin_rejects_wrong_metadata_types() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        for field in [
            r#""version":null"#,
            r#""homepage":42"#,
            r#""author":{"name":null}"#,
            r#""keywords":["valid",42]"#,
        ] {
            let content = format!(
                r#"{{"$schema":"{AGENT_PLUGIN_SCHEMA_URI}","name":"example","description":"desc",{field}}}"#
            );
            write_plugin(&plugin_path, &content);

            let diagnostics =
                CodexPluginValidator.validate(&plugin_path, &content, &LintConfig::default());
            assert!(
                diagnostics.iter().any(|d| d.rule == "CDX-PL-002"),
                "invalid Agent Plugins metadata should be rejected: {field}"
            );
        }
    }

    #[test]
    fn test_agent_plugin_valid_metadata_types() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        let content = format!(
            r#"{{"$schema":"{AGENT_PLUGIN_SCHEMA_URI}","name":"example","version":"1","description":"desc","author":{{"name":"A","email":"a@example.com","url":"https://example.com"}},"homepage":"https://example.com","repository":"repo","license":"MIT","keywords":["tool"]}}"#
        );
        write_plugin(&plugin_path, &content);

        let diagnostics =
            CodexPluginValidator.validate(&plugin_path, &content, &LintConfig::default());

        assert!(!diagnostics.iter().any(|d| d.rule == "CDX-PL-002"));
    }

    #[test]
    fn test_agent_plugin_tolerates_ignored_extensions_shapes_like_codex() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        for extensions in ["false", r#"{"com.openai":"ignored"}"#] {
            let content = format!(
                r#"{{"$schema":"{AGENT_PLUGIN_SCHEMA_URI}","name":"example","description":"desc","extensions":{extensions}}}"#
            );
            write_plugin(&plugin_path, &content);

            let diagnostics =
                CodexPluginValidator.validate(&plugin_path, &content, &LintConfig::default());
            assert!(
                !diagnostics.iter().any(|d| d.rule == "CDX-PL-002"),
                "Codex ignores unsupported extension containers: {extensions}"
            );
        }
    }

    #[test]
    fn test_cdx_pl_001_disabled() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        write_plugin(&plugin_path, r#"{"name":"test","description":"desc"}"#);

        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CDX-PL-001".to_string()];

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CDX-PL-001"));
    }

    // ===== CDX-PL-002: Invalid JSON =====

    #[test]
    fn test_cdx_pl_002_invalid_json() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{ invalid json }"#);

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-002"));
    }

    #[test]
    fn test_cdx_pl_002_empty_file() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(&plugin_path, "");

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-002"));
    }

    // ===== CDX-PL-003: Missing/empty name =====

    #[test]
    fn test_cdx_pl_003_missing_name() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{"description":"desc"}"#);

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-003"));
    }

    #[test]
    fn test_cdx_pl_003_empty_name() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        let content = r#"{"name":"  ","description":"desc"}"#;
        write_plugin(&plugin_path, content);

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(&plugin_path, content, &LintConfig::default());

        let cdx_pl_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CDX-PL-003")
            .collect();
        assert_eq!(cdx_pl_003.len(), 1);
        assert!(cdx_pl_003[0].has_fixes());
        assert!(!cdx_pl_003[0].fixes[0].safe);
    }

    #[test]
    fn test_cdx_pl_003_valid_name() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{"name":"my-plugin","description":"desc"}"#);

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CDX-PL-003"));
    }

    // ===== CDX-PL-004: Invalid name characters =====

    #[test]
    fn test_cdx_pl_004_invalid_chars() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"my plugin!","description":"desc"}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-004"));
    }

    #[test]
    fn test_cdx_pl_004_dots_in_name() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{"name":"my.plugin","description":"desc"}"#);

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-004"));
    }

    #[test]
    fn test_cdx_pl_004_valid_kebab_case() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"my-cool_plugin123","description":"desc"}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CDX-PL-004"));
    }

    // ===== CDX-PL-005: Path must start with ./ =====

    #[test]
    fn test_cdx_pl_005_missing_dot_slash() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","skills":"skills/"}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CDX-PL-005")
            .collect();
        assert_eq!(pl_005.len(), 1);
    }

    #[test]
    fn test_cdx_pl_005_absolute_path() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","skills":"/usr/local/skills"}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-005"));
    }

    #[test]
    fn test_cdx_pl_005_valid_path() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","skills":"./skills/"}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CDX-PL-005"));
    }

    // ===== CDX-PL-015: skills field type =====

    #[test]
    fn test_cdx_pl_015_non_string_skills_warns() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","skills":["./skills"]}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-015"));
    }

    #[test]
    fn test_cdx_pl_015_string_skills_no_diagnostic() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","skills":"./skills"}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CDX-PL-015"));
    }

    // ===== CDX-PL-006: Path traversal =====

    #[test]
    fn test_cdx_pl_006_traversal() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","mcpServers":"../outside"}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-006"));
    }

    #[test]
    fn test_cdx_pl_006_embedded_traversal() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","apps":"./foo/../bar"}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-006"));
    }

    #[test]
    fn test_cdx_pl_006_skills_path_traversal() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","skills":"./foo/../bar"}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-006"));
    }

    // ===== CDX-PL-007: Bare ./ path =====

    #[test]
    fn test_cdx_pl_007_bare_dot_slash() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","skills":"./"}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-007"));
    }

    // ===== CDX-PL-008: Too many defaultPrompt entries =====

    #[test]
    fn test_cdx_pl_008_too_many_prompts() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","interface":{"defaultPrompt":["a","b","c","d"]}}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-008"));
    }

    #[test]
    fn test_cdx_pl_008_three_prompts_ok() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","interface":{"defaultPrompt":["a","b","c"]}}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CDX-PL-008"));
    }

    // ===== CDX-PL-009: defaultPrompt entry too long =====

    #[test]
    fn test_cdx_pl_009_prompt_too_long() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        let long_prompt = "x".repeat(129);
        let content = format!(
            r#"{{"name":"test","description":"desc","interface":{{"defaultPrompt":["{}"]}}}}"#,
            long_prompt
        );
        write_plugin(&plugin_path, &content);

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(&plugin_path, &content, &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-009"));
    }

    #[test]
    fn test_cdx_pl_009_prompt_128_chars_ok() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        let prompt = "x".repeat(128);
        let content = format!(
            r#"{{"name":"test","description":"desc","interface":{{"defaultPrompt":["{}"]}}}}"#,
            prompt
        );
        write_plugin(&plugin_path, &content);

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(&plugin_path, &content, &LintConfig::default());

        assert!(!diagnostics.iter().any(|d| d.rule == "CDX-PL-009"));
    }

    // ===== CDX-PL-010: Empty defaultPrompt entry =====

    #[test]
    fn test_cdx_pl_010_empty_prompt() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","interface":{"defaultPrompt":["  "]}}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-010"));
    }

    // ===== CDX-PL-011: Invalid URL =====

    #[test]
    fn test_cdx_pl_011_invalid_url() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","interface":{"websiteUrl":"not-a-url"}}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-011"));
    }

    #[test]
    fn test_cdx_pl_011_valid_https() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","interface":{"websiteUrl":"https://example.com"}}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CDX-PL-011"));
    }

    // ===== CDX-PL-012: Asset path =====

    #[test]
    fn test_cdx_pl_012_logo_missing_dot_slash() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","interface":{"logo":"assets/logo.png"}}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-012"));
    }

    #[test]
    fn test_cdx_pl_012_valid_logo() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","interface":{"logo":"./assets/logo.png"}}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CDX-PL-012"));
    }

    #[test]
    fn test_cdx_pl_012_screenshots_traversal() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","interface":{"screenshots":["./valid.png","../escape.png"]}}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-012"));
    }

    // ===== CDX-PL-016: Dark-mode logo asset path =====

    #[test]
    fn test_cdx_pl_016_logo_dark_missing_dot_slash() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            "{\n  \"name\":\"test\",\n  \"description\":\"desc\",\n  \"interface\":{\n    \"logoDark\":\"assets/logo-dark.png\"\n  }\n}",
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CDX-PL-016")
            .expect("CDX-PL-016 diagnostic");
        assert_eq!(hit.line, 5);
        assert!(!diagnostics.iter().any(|d| d.rule == "CDX-PL-012"));
    }

    #[test]
    fn test_cdx_pl_016_logo_dark_valid_path() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","interface":{"logoDark":"./assets/logo-dark.png"}}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CDX-PL-016"));
    }

    #[test]
    fn test_cdx_pl_016_logo_dark_traversal() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","interface":{"logoDark":"./assets/../secret.png"}}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-016"));
    }

    #[test]
    fn test_cdx_pl_016_can_be_disabled() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","interface":{"logoDark":"assets/logo-dark.png"}}"#,
        );

        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CDX-PL-016".to_string()];

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CDX-PL-016"));
    }

    // ===== CDX-PL-013: hooks shape =====

    #[test]
    fn test_cdx_pl_013_accepts_supported_hooks_forms() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        for hooks in [
            r#""./hooks.json""#,
            r#"["./one.json","./two.json"]"#,
            r#"{"hooks":{}}"#,
            r#"[{"hooks":{}},{"hooks":{}}]"#,
        ] {
            let content = format!(r#"{{"name":"test","description":"desc","hooks":{hooks}}}"#);
            write_plugin(&plugin_path, &content);

            let diagnostics =
                CodexPluginValidator.validate(&plugin_path, &content, &LintConfig::default());
            assert!(
                !diagnostics.iter().any(|d| d.rule == "CDX-PL-013"),
                "supported hooks form should be accepted: {hooks}"
            );
        }
    }

    #[test]
    fn test_cdx_pl_013_rejects_invalid_hooks_shapes() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        for hooks in ["42", r#"["./hooks.json",{}]"#] {
            let content = format!(r#"{{"name":"test","description":"desc","hooks":{hooks}}}"#);
            write_plugin(&plugin_path, &content);

            let diagnostics =
                CodexPluginValidator.validate(&plugin_path, &content, &LintConfig::default());
            assert!(
                diagnostics.iter().any(|d| d.rule == "CDX-PL-013"),
                "invalid hooks shape should be reported: {hooks}"
            );
        }
    }

    #[test]
    fn test_agent_plugin_inspects_openai_extension() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        let content = format!(
            r#"{{"$schema":"{AGENT_PLUGIN_SCHEMA_URI}","name":"example","description":"desc","extensions":{{"com.openai":{{"apps":"apps","hooks":["hooks.json"],"interface":{{"websiteUrl":"invalid"}}}}}}}}"#
        );
        write_plugin(&plugin_path, &content);

        let diagnostics =
            CodexPluginValidator.validate(&plugin_path, &content, &LintConfig::default());

        assert_eq!(
            diagnostics
                .iter()
                .filter(|d| d.rule == "CDX-PL-005")
                .count(),
            2,
            "apps and hooks paths in extensions.com.openai should be checked"
        );
        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-011"));
    }

    #[test]
    fn test_agent_plugin_rejects_invalid_openai_extension_types() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        for field in [r#""apps":[]"#, r#""interface":"invalid""#] {
            let content = format!(
                r#"{{"$schema":"{AGENT_PLUGIN_SCHEMA_URI}","name":"example","description":"desc","extensions":{{"com.openai":{{{field}}}}}}}"#
            );
            write_plugin(&plugin_path, &content);

            let diagnostics =
                CodexPluginValidator.validate(&plugin_path, &content, &LintConfig::default());
            assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-002"));
        }
    }

    // ===== CDX-PL-014: Missing description =====

    #[test]
    fn test_cdx_pl_014_missing_description() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{"name":"test"}"#);

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let cdx_pl_014: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CDX-PL-014")
            .collect();
        assert_eq!(cdx_pl_014.len(), 1);
        assert_eq!(
            cdx_pl_014[0].level,
            crate::diagnostics::DiagnosticLevel::Warning
        );
    }

    #[test]
    fn test_cdx_pl_014_has_description() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"A great plugin"}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CDX-PL-014"));
    }

    // ===== Category disable =====

    #[test]
    fn test_codex_category_disabled() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        write_plugin(&plugin_path, r#"{ invalid json }"#);

        let mut config = LintConfig::default();
        config.rules_mut().codex = false;

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(diagnostics.is_empty());
    }

    // ===== String defaultPrompt =====

    #[test]
    fn test_default_prompt_string_form() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","interface":{"defaultPrompt":"Summarize inbox"}}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        // Single valid string - no defaultPrompt errors
        assert!(!diagnostics.iter().any(|d| d.rule.starts_with("CDX-PL-008")
            || d.rule.starts_with("CDX-PL-009")
            || d.rule.starts_with("CDX-PL-010")));
    }

    // ===== Complete valid manifest =====

    #[test]
    fn test_complete_valid_manifest() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{
                "name": "my-codex-plugin",
                "description": "A test Codex plugin",
                "skills": "./skills",
                "mcpServers": "./mcp-servers",
                "apps": "./apps",
                "interface": {
                    "displayName": "My Plugin",
                    "shortDescription": "Short desc",
                    "websiteUrl": "https://example.com",
                    "defaultPrompt": ["Prompt one", "Prompt two"],
                    "logo": "./assets/logo.png",
                    "screenshots": ["./assets/s1.png"]
                }
            }"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(
            diagnostics.is_empty(),
            "Complete valid manifest should have no diagnostics, got: {:?}",
            diagnostics.iter().map(|d| &d.rule).collect::<Vec<_>>()
        );
    }
}
