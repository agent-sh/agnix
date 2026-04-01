//! Codex CLI plugin manifest validation (CDX-PL-001 to CDX-PL-014).
//!
//! Validates `.codex-plugin/plugin.json` manifests for the Codex CLI
//! plugin system introduced in v0.117.0.

use crate::{
    config::LintConfig,
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
];

/// Max number of defaultPrompt entries
const MAX_DEFAULT_PROMPT_COUNT: usize = 3;
/// Max characters per defaultPrompt entry
const MAX_DEFAULT_PROMPT_LEN: usize = 128;

pub struct CodexPluginValidator;

impl Validator for CodexPluginValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if !config.rules().codex {
            return diagnostics;
        }

        // CDX-PL-001: Manifest must be in .codex-plugin/
        let is_in_codex_plugin = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|n| n.eq_ignore_ascii_case(".codex-plugin"))
            .unwrap_or(false);

        if config.is_rule_enabled("CDX-PL-001") && !is_in_codex_plugin {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CDX-PL-001",
                    t!("rules.cdx_pl_001.message"),
                )
                .with_suggestion(t!("rules.cdx_pl_001.suggestion")),
            );
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
                if !trimmed.is_empty() && !is_valid_plugin_name(trimmed) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CDX-PL-004",
                            t!("rules.cdx_pl_004.message", name = trimmed),
                        )
                        .with_suggestion(t!("rules.cdx_pl_004.suggestion")),
                    );
                }
            }
        }

        // CDX-PL-005/006/007: Component path validation (skills, mcpServers, apps)
        let path_rules_enabled = config.is_rule_enabled("CDX-PL-005")
            || config.is_rule_enabled("CDX-PL-006")
            || config.is_rule_enabled("CDX-PL-007");
        if path_rules_enabled {
            for field in &["skills", "mcpServers", "apps"] {
                if let Some(val) = raw_value.get(*field).and_then(|v| v.as_str()) {
                    validate_component_path(val, field, path, content, config, &mut diagnostics);
                }
            }
        }

        // CDX-PL-008/009/010: defaultPrompt validation
        if let Some(interface) = raw_value.get("interface") {
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
        }

        // CDX-PL-013: hooks field not supported
        if config.is_rule_enabled("CDX-PL-013") && raw_value.get("hooks").is_some() {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "CDX-PL-013",
                    t!("rules.cdx_pl_013.message"),
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

/// Validate a component path field (skills, mcpServers, apps).
fn validate_component_path(
    p: &str,
    field: &str,
    path: &Path,
    content: &str,
    config: &LintConfig,
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

/// Validate defaultPrompt field.
fn validate_default_prompt(
    value: &serde_json::Value,
    path: &Path,
    config: &LintConfig,
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
    fn test_cdx_pl_001_not_in_codex_plugin() {
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

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-001"));
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

    // ===== CDX-PL-013: hooks not supported =====

    #[test]
    fn test_cdx_pl_013_hooks_present() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","hooks":{"preStart":"echo hi"}}"#,
        );

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CDX-PL-013"));
    }

    #[test]
    fn test_cdx_pl_013_no_hooks() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".codex-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{"name":"test","description":"desc"}"#);

        let validator = CodexPluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CDX-PL-013"));
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
