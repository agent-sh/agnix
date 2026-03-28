//! GitHub Copilot validation rules (COP-001 to COP-027)
//!
//! Validates:
//! - COP-001: Empty instruction file (HIGH) - files must have content
//! - COP-002: Invalid frontmatter (HIGH) - scoped files require valid YAML with applyTo
//! - COP-003: Invalid glob pattern (HIGH) - applyTo must contain valid globs
//! - COP-004: Unknown frontmatter keys (MEDIUM) - warn about unrecognized keys
//! - COP-005: Invalid excludeAgent value (HIGH) - must be "code-review" or "coding-agent"
//! - COP-006: File length limit (MEDIUM) - global files should not exceed ~4000 characters
//! - COP-007 to COP-012: Custom agent validation
//! - COP-013 to COP-015: Reusable prompt validation
//! - COP-017: Hooks schema validation
//! - COP-018: Setup workflow validation
//! - COP-019: Plugin manifest missing required fields (HIGH)
//! - COP-020: Plugin manifest invalid field types (MEDIUM)
//! - COP-022: CLI SKILL.md missing required frontmatter (HIGH)
//! - COP-023: CLI SKILL.md name format (MEDIUM)
//! - COP-024: Unknown SKILL.md frontmatter field (MEDIUM)
//! - COP-025: CLI `.agent.md` in wrong location (LOW)
//! - COP-026: Deprecated SSE transport in MCP config (LOW)
//! - COP-027: Deprecated `infer` field in custom agent frontmatter (LOW)

use crate::{
    FileType,
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::{Validator, ValidatorMetadata},
    schemas::{
        copilot::{is_body_empty, is_content_empty, parse_frontmatter, validate_glob_pattern},
        copilot_agent::parse_agent_frontmatter,
        copilot_hooks::{
            has_copilot_setup_steps_job, parse_hooks_json, parse_setup_steps_yaml,
            validate_hooks_schema,
        },
        copilot_prompt::{
            VALID_AGENT_MODES, is_body_empty as is_prompt_body_empty, parse_prompt_frontmatter,
        },
    },
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &[
    "COP-001", "COP-002", "COP-003", "COP-004", "COP-005", "COP-006", "COP-007", "COP-008",
    "COP-009", "COP-010", "COP-011", "COP-012", "COP-013", "COP-014", "COP-015", "COP-017",
    "COP-018", "COP-019", "COP-020", "COP-022", "COP-023", "COP-024", "COP-025", "COP-026",
    "COP-027",
];

pub struct CopilotValidator;

fn line_byte_range(content: &str, line_number: usize) -> Option<(usize, usize)> {
    if line_number == 0 {
        return None;
    }

    let mut current_line = 1usize;
    let mut line_start = 0usize;

    for (idx, ch) in content.char_indices() {
        if current_line == line_number && ch == '\n' {
            return Some((line_start, idx + 1));
        }
        if ch == '\n' {
            current_line += 1;
            line_start = idx + 1;
        }
    }

    if current_line == line_number {
        Some((line_start, content.len()))
    } else {
        None
    }
}

fn frontmatter_key_line(raw: &str, start_line: usize, key: &str) -> usize {
    let key_name = key.trim_end_matches(':').trim();
    raw.lines()
        .enumerate()
        .find(|(_, line)| {
            let trimmed = line.trim_start();
            if let Some(after_key) = trimmed.strip_prefix(key_name) {
                after_key.trim_start().starts_with(':')
            } else {
                false
            }
        })
        .map(|(idx, _)| start_line + 1 + idx)
        .unwrap_or(start_line)
}

fn is_setup_steps_workflow(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("copilot-setup-steps.yml") | Some("copilot-setup-steps.yaml")
    )
}

fn validate_custom_agent(path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let parsed = parse_agent_frontmatter(content);
    let body = parsed.as_ref().map_or(content, |p| p.body.as_str());

    if config.is_rule_enabled("COP-011") {
        const MAX_AGENT_BODY_CHARS: usize = 30_000;
        let body_len = body.chars().count();
        if body_len > MAX_AGENT_BODY_CHARS {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "COP-011",
                    format!(
                        "Custom agent prompt body exceeds {} characters (found {})",
                        MAX_AGENT_BODY_CHARS, body_len
                    ),
                )
                .with_suggestion("Reduce agent prompt size to 30000 characters or fewer."),
            );
        }
    }

    if let Some(parsed) = &parsed {
        if let Some(err) = &parsed.parse_error {
            if config.is_rule_enabled("COP-008") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        parsed.start_line,
                        0,
                        "COP-008",
                        format!("Custom agent frontmatter contains invalid YAML: {err}"),
                    )
                    .with_suggestion("Fix YAML syntax in custom agent frontmatter."),
                );
            } else if config.is_rule_enabled("COP-007") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        parsed.start_line,
                        0,
                        "COP-007",
                        format!("Custom agent frontmatter contains invalid YAML: {err}"),
                    )
                    .with_suggestion("Fix YAML syntax in custom agent frontmatter."),
                );
            }
            return diagnostics;
        }
    }

    if config.is_rule_enabled("COP-007") {
        let has_frontmatter = parsed.is_some();
        let has_description = parsed
            .as_ref()
            .and_then(|p| p.schema.as_ref())
            .and_then(|s| s.description.as_ref())
            .is_some_and(|desc| !desc.trim().is_empty());
        if !has_description {
            let (message, suggestion) = if !has_frontmatter {
                (
                    "Custom agent file must start with YAML frontmatter containing a non-empty 'description' field",
                    "Add a YAML frontmatter block at the top of the file with a non-empty 'description' key.",
                )
            } else {
                (
                    "Custom agent frontmatter is missing required 'description' field",
                    "Add a non-empty 'description' key in YAML frontmatter.",
                )
            };
            diagnostics.push(
                Diagnostic::error(path.to_path_buf(), 1, 0, "COP-007", message)
                    .with_suggestion(suggestion),
            );
        }
    }

    if let Some(parsed) = &parsed {
        let cop_008_enabled = config.is_rule_enabled("COP-008");
        let cop_010_enabled = config.is_rule_enabled("COP-010");
        let cop_027_enabled = config.is_rule_enabled("COP-027");
        let raw_mapping = if cop_008_enabled || cop_010_enabled || cop_027_enabled {
            serde_yaml::from_str::<serde_yaml::Value>(&parsed.raw)
                .ok()
                .and_then(|raw| raw.as_mapping().cloned())
        } else {
            None
        };

        if cop_008_enabled {
            for unknown in &parsed.unknown_keys {
                let mut diagnostic = Diagnostic::warning(
                    path.to_path_buf(),
                    unknown.line,
                    unknown.column,
                    "COP-008",
                    format!(
                        "Custom agent has unsupported frontmatter field '{}'",
                        unknown.key
                    ),
                )
                .with_suggestion(format!(
                    "Remove unknown frontmatter field '{}'.",
                    unknown.key
                ));

                if let Some((start, end)) = line_byte_range(content, unknown.line) {
                    diagnostic = diagnostic.with_fix(Fix::delete(
                        start,
                        end,
                        format!("Remove unknown agent field '{}'", unknown.key),
                        true,
                    ));
                }

                diagnostics.push(diagnostic);
            }

            if let Some(schema) = &parsed.schema {
                let raw_value = |key: &str| {
                    raw_mapping
                        .as_ref()
                        .and_then(|mapping| mapping.get(serde_yaml::Value::String(key.to_string())))
                        .cloned()
                };

                let disable_model_invocation = schema
                    .disable_model_invocation
                    .clone()
                    .or_else(|| raw_value("disable-model-invocation"));
                let user_invocable = schema
                    .user_invocable
                    .clone()
                    .or_else(|| raw_value("user-invocable"));
                let metadata = schema.metadata.clone().or_else(|| raw_value("metadata"));

                let metadata_valid = metadata.as_ref().is_none_or(|value| {
                    value.as_mapping().is_some_and(|mapping| {
                        mapping
                            .iter()
                            .all(|(key, value)| key.as_str().is_some() && value.as_str().is_some())
                    })
                });

                let typed_field_checks = [
                    (
                        "disable-model-invocation:",
                        disable_model_invocation
                            .as_ref()
                            .is_some_and(|value| !value.is_bool()),
                        "disable-model-invocation",
                        "Set 'disable-model-invocation' to true or false.",
                    ),
                    (
                        "user-invocable:",
                        user_invocable
                            .as_ref()
                            .is_some_and(|value| !value.is_bool()),
                        "user-invocable",
                        "Set 'user-invocable' to true or false.",
                    ),
                    (
                        "metadata:",
                        !metadata_valid,
                        "metadata",
                        "Set 'metadata' to an object with string keys and string values.",
                    ),
                ];

                for (prefix, invalid, field_name, suggestion) in typed_field_checks {
                    if invalid {
                        let line = frontmatter_key_line(&parsed.raw, parsed.start_line, prefix);
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                line,
                                0,
                                "COP-008",
                                format!("Field '{}' has invalid value type", field_name),
                            )
                            .with_suggestion(suggestion),
                        );
                    }
                }
            }
        }

        if let Some(schema) = &parsed.schema {
            if config.is_rule_enabled("COP-009") {
                if let Some(target) = &schema.target {
                    const VALID_TARGETS: &[&str] = &["vscode", "github-copilot"];
                    if !VALID_TARGETS.contains(&target.as_str()) {
                        let line = frontmatter_key_line(&parsed.raw, parsed.start_line, "target:");
                        let mut diagnostic = Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            0,
                            "COP-009",
                            format!(
                                "Invalid custom agent target '{}'; expected 'vscode' or 'github-copilot'",
                                target
                            ),
                        )
                        .with_suggestion("Set target to 'vscode' or 'github-copilot'.");

                        if let Some(suggested) =
                            super::find_closest_value(target.as_str(), VALID_TARGETS)
                        {
                            if let Some((start, end)) =
                                crate::rules::find_yaml_value_range(content, parsed, "target", true)
                            {
                                let slice = content.get(start..end).unwrap_or("");
                                let replacement = if slice.starts_with('"') {
                                    format!("\"{}\"", suggested)
                                } else if slice.starts_with('\'') {
                                    format!("'{}'", suggested)
                                } else {
                                    suggested.to_string()
                                };
                                diagnostic = diagnostic.with_fix(Fix::replace(
                                    start,
                                    end,
                                    replacement,
                                    format!("Replace target with '{}'", suggested),
                                    false,
                                ));
                            }
                        }

                        diagnostics.push(diagnostic);
                    }
                }
            }

            if cop_010_enabled {
                let infer_is_non_boolean =
                    schema.infer.as_ref().is_some_and(|infer| !infer.is_bool());
                let infer_is_explicit_null = if schema.infer.is_none() {
                    raw_mapping
                        .as_ref()
                        .and_then(|map| map.get(serde_yaml::Value::String("infer".to_string())))
                        .cloned()
                        .is_some_and(|value| value.is_null())
                } else {
                    false
                };

                if infer_is_non_boolean || infer_is_explicit_null {
                    let line = frontmatter_key_line(&parsed.raw, parsed.start_line, "infer:");
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            line,
                            0,
                            "COP-010",
                            "Custom agent 'infer' field must be a boolean",
                        )
                        .with_suggestion("Set 'infer' to true or false."),
                    );
                }
            }

            // COP-027: Deprecated `infer` field in custom agent frontmatter (LOW)
            if config.is_rule_enabled("COP-027") {
                let has_infer = schema.infer.is_some() || {
                    raw_mapping
                        .as_ref()
                        .and_then(|map| map.get(serde_yaml::Value::String("infer".to_string())))
                        .is_some()
                };
                if has_infer {
                    let line = frontmatter_key_line(&parsed.raw, parsed.start_line, "infer:");
                    diagnostics.push(
                        Diagnostic::info(
                            path.to_path_buf(),
                            line,
                            0,
                            "COP-027",
                            "Field 'infer' is deprecated; use 'disable-model-invocation' and 'user-invocable' instead".to_string(),
                        )
                        .with_suggestion(
                            "Replace 'infer' with 'disable-model-invocation' and 'user-invocable'.",
                        ),
                    );
                }
            }

            let applies_to_github = !matches!(schema.target.as_deref(), Some("vscode"));
            if config.is_rule_enabled("COP-012") && applies_to_github {
                let unsupported = [
                    ("model:", schema.model.is_some(), "model"),
                    (
                        "argument-hint:",
                        schema.argument_hint.is_some(),
                        "argument-hint",
                    ),
                    ("handoffs:", schema.handoffs.is_some(), "handoffs"),
                ];

                for (prefix, present, field_name) in unsupported {
                    if present {
                        let line = frontmatter_key_line(&parsed.raw, parsed.start_line, prefix);
                        let mut diagnostic = Diagnostic::warning(
                            path.to_path_buf(),
                            line,
                            0,
                            "COP-012",
                            format!(
                                "Field '{}' is unsupported on GitHub.com custom agents",
                                field_name
                            ),
                        )
                        .with_suggestion(format!(
                            "Remove '{}' for GitHub.com compatibility.",
                            field_name
                        ));

                        if let Some((start, end)) = line_byte_range(content, line) {
                            diagnostic = diagnostic.with_fix(Fix::delete(
                                start,
                                end,
                                format!("Remove unsupported field '{}'", field_name),
                                true,
                            ));
                        }

                        diagnostics.push(diagnostic);
                    }
                }
            }
        }
    }

    diagnostics
}

fn validate_reusable_prompt(path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let parsed = parse_prompt_frontmatter(content);

    if let Some(parsed) = &parsed {
        if let Some(err) = &parsed.parse_error {
            if config.is_rule_enabled("COP-014") {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        parsed.start_line,
                        0,
                        "COP-014",
                        format!("Prompt frontmatter contains invalid YAML: {err}"),
                    )
                    .with_suggestion("Fix YAML syntax in prompt frontmatter."),
                );
            }
            return diagnostics;
        }
    }

    let body = parsed.as_ref().map_or(content, |p| p.body.as_str());

    if config.is_rule_enabled("COP-013") && is_prompt_body_empty(body) {
        let max_line = content.lines().count().max(1);
        let line = parsed
            .as_ref()
            .map_or(1, |p| (p.end_line + 1).min(max_line));
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                0,
                "COP-013",
                "Prompt file body is empty",
            )
            .with_suggestion("Add prompt text below the optional frontmatter."),
        );
    }

    if let Some(parsed) = &parsed {
        if config.is_rule_enabled("COP-014") {
            for unknown in &parsed.unknown_keys {
                let mut diagnostic = Diagnostic::warning(
                    path.to_path_buf(),
                    unknown.line,
                    unknown.column,
                    "COP-014",
                    format!(
                        "Prompt file has unsupported frontmatter field '{}'",
                        unknown.key
                    ),
                )
                .with_suggestion(format!(
                    "Remove unknown frontmatter field '{}'.",
                    unknown.key
                ));

                if let Some((start, end)) = line_byte_range(content, unknown.line) {
                    diagnostic = diagnostic.with_fix(Fix::delete(
                        start,
                        end,
                        format!("Remove unknown prompt field '{}'", unknown.key),
                        true,
                    ));
                }

                diagnostics.push(diagnostic);
            }
        }

        if config.is_rule_enabled("COP-015") {
            if let Some(schema) = &parsed.schema {
                if let Some(agent_mode) = &schema.agent {
                    if !VALID_AGENT_MODES.contains(&agent_mode.as_str()) {
                        let line = frontmatter_key_line(&parsed.raw, parsed.start_line, "agent:");
                        let mut diagnostic = Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            0,
                            "COP-015",
                            format!(
                                "Invalid prompt agent mode '{}'; expected one of: none, ask, always",
                                agent_mode
                            ),
                        )
                        .with_suggestion("Use agent mode 'none', 'ask', or 'always'.");

                        if let Some(suggested) =
                            super::find_closest_value(agent_mode.as_str(), VALID_AGENT_MODES)
                        {
                            if let Some((start, end)) =
                                crate::rules::find_yaml_value_range(content, parsed, "agent", true)
                            {
                                let slice = content.get(start..end).unwrap_or("");
                                let replacement = if slice.starts_with('"') {
                                    format!("\"{}\"", suggested)
                                } else if slice.starts_with('\'') {
                                    format!("'{}'", suggested)
                                } else {
                                    suggested.to_string()
                                };
                                diagnostic = diagnostic.with_fix(Fix::replace(
                                    start,
                                    end,
                                    replacement,
                                    format!("Replace agent mode with '{}'", suggested),
                                    false,
                                ));
                            }
                        }

                        diagnostics.push(diagnostic);
                    }
                }
            }
        }
    }

    diagnostics
}

fn validate_hooks_file(path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if is_setup_steps_workflow(path) {
        if !config.is_rule_enabled("COP-018") {
            return diagnostics;
        }

        match parse_setup_steps_yaml(content) {
            Ok(workflow) => {
                if !has_copilot_setup_steps_job(&workflow) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "COP-018",
                            "copilot-setup-steps workflow must define jobs.copilot-setup-steps with ubuntu runs-on and non-empty steps",
                        )
                        .with_suggestion(
                            "Define jobs.copilot-setup-steps with an Ubuntu runner and at least one step in .github/workflows/copilot-setup-steps.yml.",
                        ),
                    );
                }
            }
            Err(err) => diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "COP-018",
                    format!("Invalid copilot-setup-steps workflow YAML: {err}"),
                )
                .with_suggestion("Fix YAML syntax in copilot-setup-steps workflow."),
            ),
        }

        return diagnostics;
    }

    if !config.is_rule_enabled("COP-017") {
        return diagnostics;
    }

    match parse_hooks_json(content) {
        Ok(hooks) => {
            for error in validate_hooks_schema(&hooks) {
                diagnostics.push(
                    Diagnostic::error(path.to_path_buf(), 1, 0, "COP-017", error)
                        .with_suggestion("Fix hooks.json to match Copilot hooks schema."),
                );
            }
        }
        Err(err) => diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "COP-017",
                format!("Invalid hooks.json syntax: {err}"),
            )
            .with_suggestion("Fix JSON syntax in .github/hooks/hooks.json."),
        ),
    }

    diagnostics
}

/// Check if a path looks like it's under a Copilot plugin directory.
/// Returns true for paths like `.github/copilot-extensions/plugin.json`
/// or any `plugin.json` that contains Copilot-specific keys.
fn is_copilot_plugin_manifest(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()) == Some("plugin.json")
}

/// Check if content looks like a Copilot plugin manifest by checking for
/// Copilot-specific top-level keys.
fn looks_like_copilot_plugin(value: &serde_json::Value) -> bool {
    let obj = match value.as_object() {
        Some(o) => o,
        None => return false,
    };
    const COPILOT_KEYS: &[&str] = &["agents", "skills", "hooks", "mcpServers"];
    COPILOT_KEYS.iter().any(|key| obj.contains_key(*key))
}

fn validate_plugin_manifest(path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let value: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return diagnostics,
    };

    if !looks_like_copilot_plugin(&value) {
        return diagnostics;
    }

    let obj = match value.as_object() {
        Some(o) => o,
        None => return diagnostics,
    };

    // COP-019: Plugin manifest missing required fields (HIGH)
    if config.is_rule_enabled("COP-019") {
        let required_fields = ["name", "description", "version"];
        for field in required_fields {
            if !obj.contains_key(field) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "COP-019",
                        format!("Copilot plugin manifest missing required field '{}'", field),
                    )
                    .with_suggestion(format!("Add a '{}' field to the plugin manifest.", field)),
                );
            }
        }
    }

    // COP-020: Plugin manifest invalid field types (MEDIUM)
    if config.is_rule_enabled("COP-020") {
        #[allow(clippy::type_complexity)]
        let type_checks: &[(&str, fn(&serde_json::Value) -> bool, &str)] = &[
            ("name", |v| v.is_string(), "string"),
            ("version", |v| v.is_string(), "string"),
            ("description", |v| v.is_string(), "string"),
            ("keywords", |v| v.is_array(), "array"),
            ("agents", |v| v.is_object(), "object"),
            ("skills", |v| v.is_object(), "object"),
        ];

        for (field, check, expected_type) in type_checks {
            if let Some(val) = obj.get(*field) {
                if !check(val) {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            1,
                            0,
                            "COP-020",
                            format!(
                                "Copilot plugin manifest field '{}' should be {} but found {}",
                                field,
                                expected_type,
                                json_type_name(val)
                            ),
                        )
                        .with_suggestion(format!(
                            "Change '{}' to a {} value.",
                            field, expected_type
                        )),
                    );
                }
            }
        }
    }

    diagnostics
}

/// Quick content-based check for plugin manifest without full parse.
fn looks_like_copilot_plugin_content(content: &str) -> bool {
    match serde_json::from_str::<serde_json::Value>(content) {
        Ok(v) => looks_like_copilot_plugin(&v),
        Err(_) => false,
    }
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Check if a path is a SKILL.md under a Copilot CLI skills directory.
fn is_copilot_skill_md(path: &Path) -> bool {
    if path.file_name().and_then(|n| n.to_str()) != Some("SKILL.md") {
        return false;
    }
    let path_str = path.to_string_lossy();
    let normalized = path_str.replace('\\', "/");
    normalized.contains(".github/skills/") || normalized.contains(".agents/skills/")
}

fn validate_copilot_skill(path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let parsed = parse_agent_frontmatter(content);

    // COP-022: CLI SKILL.md missing required frontmatter (HIGH)
    if config.is_rule_enabled("COP-022") {
        match &parsed {
            None => {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "COP-022",
                        "Copilot CLI SKILL.md is missing required frontmatter with 'name' and 'description' fields".to_string(),
                    )
                    .with_suggestion(
                        "Add YAML frontmatter with 'name' and 'description' fields.",
                    ),
                );
            }
            Some(p) => {
                if let Some(ref err) = p.parse_error {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            p.start_line,
                            0,
                            "COP-022",
                            format!("Copilot CLI SKILL.md has invalid frontmatter: {}", err),
                        )
                        .with_suggestion("Fix YAML syntax in SKILL.md frontmatter."),
                    );
                } else {
                    let raw_yaml: serde_yaml::Value =
                        serde_yaml::from_str(&p.raw).unwrap_or(serde_yaml::Value::Null);
                    let mapping = raw_yaml.as_mapping();

                    let has_name = mapping
                        .and_then(|m| m.get(serde_yaml::Value::String("name".to_string())))
                        .is_some_and(|v| !v.is_null());
                    let has_description = mapping
                        .and_then(|m| m.get(serde_yaml::Value::String("description".to_string())))
                        .is_some_and(|v| !v.is_null());

                    if !has_name {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                p.start_line,
                                0,
                                "COP-022",
                                "Copilot CLI SKILL.md frontmatter is missing required field 'name'"
                                    .to_string(),
                            )
                            .with_suggestion("Add a 'name' field to the SKILL.md frontmatter."),
                        );
                    }

                    if !has_description {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                p.start_line,
                                0,
                                "COP-022",
                                "Copilot CLI SKILL.md frontmatter is missing required field 'description'".to_string(),
                            )
                            .with_suggestion(
                                "Add a 'description' field to the SKILL.md frontmatter.",
                            ),
                        );
                    }

                    // COP-023: CLI SKILL.md name format (MEDIUM)
                    if config.is_rule_enabled("COP-023") && has_name {
                        if let Some(name_val) = mapping
                            .and_then(|m| m.get(serde_yaml::Value::String("name".to_string())))
                        {
                            if let Some(name_str) = name_val.as_str() {
                                if !is_kebab_case(name_str) {
                                    let line = frontmatter_key_line(&p.raw, p.start_line, "name:");
                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            line,
                                            0,
                                            "COP-023",
                                            format!(
                                                "SKILL.md name '{}' must be lowercase kebab-case (e.g. 'my-skill')",
                                                name_str
                                            ),
                                        )
                                        .with_suggestion(
                                            "Use only lowercase letters and hyphens for the skill name.",
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    diagnostics
}

/// Check if a string is valid kebab-case: lowercase ASCII letters and hyphens only,
/// no leading/trailing hyphens, no consecutive hyphens, non-empty.
fn is_kebab_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    // Must be only lowercase ASCII letters, digits, and hyphens
    if !s
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return false;
    }
    // No leading or trailing hyphens
    if s.starts_with('-') || s.ends_with('-') {
        return false;
    }
    // No consecutive hyphens
    if s.contains("--") {
        return false;
    }
    true
}

/// Known SKILL.md frontmatter fields for Copilot CLI skills.
const KNOWN_SKILL_FRONTMATTER_KEYS: &[&str] = &["name", "description", "license"];

/// COP-024: Unknown SKILL.md frontmatter field.
/// Warn on frontmatter keys that aren't in the known set for SKILL.md files.
fn validate_copilot_skill_unknown_fields(
    path: &Path,
    content: &str,
    config: &LintConfig,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if !config.is_rule_enabled("COP-024") {
        return diagnostics;
    }

    let parsed = match parse_agent_frontmatter(content) {
        Some(p) => p,
        None => return diagnostics,
    };

    if parsed.parse_error.is_some() {
        return diagnostics;
    }

    let raw_yaml: serde_yaml::Value =
        serde_yaml::from_str(&parsed.raw).unwrap_or(serde_yaml::Value::Null);

    if let Some(mapping) = raw_yaml.as_mapping() {
        for key in mapping.keys() {
            if let Some(key_str) = key.as_str() {
                if !KNOWN_SKILL_FRONTMATTER_KEYS.contains(&key_str) {
                    let line = frontmatter_key_line(&parsed.raw, parsed.start_line, key_str);
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            line,
                            0,
                            "COP-024",
                            format!(
                                "Unknown SKILL.md frontmatter field '{}'; expected one of: {}",
                                key_str,
                                KNOWN_SKILL_FRONTMATTER_KEYS.join(", ")
                            ),
                        )
                        .with_suggestion(format!(
                            "Remove unknown field '{}' or use one of: {}.",
                            key_str,
                            KNOWN_SKILL_FRONTMATTER_KEYS.join(", ")
                        )),
                    );
                }
            }
        }
    }

    diagnostics
}

/// COP-025: CLI `.agent.md` in wrong location.
/// When a file ends with `.agent.md` but is not under `.github/agents/` or `~/.copilot/agents/`,
/// emit an info diagnostic suggesting the correct location.
fn validate_agent_md_location(path: &Path, config: &LintConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if !config.is_rule_enabled("COP-025") {
        return diagnostics;
    }

    let filename = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) if name.ends_with(".agent.md") => name,
        _ => return diagnostics,
    };

    let path_str = path.to_string_lossy();
    let normalized = path_str.replace('\\', "/");

    // Check if it's in a correct location
    let in_github_agents = normalized.contains(".github/agents/");
    let in_copilot_agents = normalized.contains(".copilot/agents/");

    if !in_github_agents && !in_copilot_agents {
        diagnostics.push(
            Diagnostic::info(
                path.to_path_buf(),
                1,
                0,
                "COP-025",
                format!(
                    "Agent file '{}' is not under '.github/agents/' or '~/.copilot/agents/'",
                    filename
                ),
            )
            .with_suggestion(
                "Move agent files to '.github/agents/' (project) or '~/.copilot/agents/' (global).",
            ),
        );
    }

    diagnostics
}

/// COP-026: Deprecated SSE transport in MCP config.
/// When an mcp-config.json server entry has `"type": "sse"`, warn that SSE transport
/// is deprecated in favor of HTTP/Streamable HTTP.
fn validate_mcp_config_sse(path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if !config.is_rule_enabled("COP-026") {
        return diagnostics;
    }

    // Only applies to mcp-config.json files
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if filename != "mcp-config.json" {
        return diagnostics;
    }

    let value: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return diagnostics,
    };

    // Check for servers with "type": "sse" - could be at top level or under "mcpServers"
    let check_servers = |servers: &serde_json::Value| -> Vec<String> {
        let mut sse_servers = Vec::new();
        if let Some(obj) = servers.as_object() {
            for (name, server) in obj {
                if let Some(transport_type) = server.get("type").and_then(|t| t.as_str()) {
                    if transport_type == "sse" {
                        sse_servers.push(name.clone());
                    }
                }
            }
        }
        sse_servers
    };

    let mut sse_servers = Vec::new();

    // Check "mcpServers" key
    if let Some(servers) = value.get("mcpServers") {
        sse_servers.extend(check_servers(servers));
    }

    // Check top-level keys that look like server definitions
    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            if key != "mcpServers" && val.is_object() {
                if let Some(t) = val.get("type").and_then(|t| t.as_str()) {
                    if t == "sse" {
                        sse_servers.push(key.clone());
                    }
                }
            }
        }
    }

    for server_name in sse_servers {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                1,
                0,
                "COP-026",
                format!(
                    "MCP server '{}' uses deprecated SSE transport type",
                    server_name
                ),
            )
            .with_suggestion("Replace 'sse' with 'http' or 'streamable-http' transport."),
        );
    }

    diagnostics
}

impl Validator for CopilotValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // COP-026: Deprecated SSE transport in MCP config
        if path.file_name().and_then(|n| n.to_str()) == Some("mcp-config.json") {
            return validate_mcp_config_sse(path, content, config);
        }

        // Path-based checks that work regardless of FileType dispatch
        if is_copilot_plugin_manifest(path) && looks_like_copilot_plugin_content(content) {
            return validate_plugin_manifest(path, content, config);
        }
        if is_copilot_skill_md(path) {
            let mut skill_diagnostics = validate_copilot_skill(path, content, config);
            // COP-024: Unknown SKILL.md frontmatter fields
            skill_diagnostics.extend(validate_copilot_skill_unknown_fields(path, content, config));
            return skill_diagnostics;
        }

        // COP-025: .agent.md in wrong location
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with(".agent.md") {
                let file_type = crate::detect_file_type(path);
                if file_type != FileType::CopilotAgent {
                    return validate_agent_md_location(path, config);
                }
            }
        }

        let file_type = crate::detect_file_type(path);
        match file_type {
            FileType::CopilotAgent => return validate_custom_agent(path, content, config),
            FileType::CopilotPrompt => return validate_reusable_prompt(path, content, config),
            FileType::CopilotHooks => return validate_hooks_file(path, content, config),
            FileType::Copilot | FileType::CopilotScoped => {}
            _ => return diagnostics,
        }

        // Determine if this is global or scoped instruction file
        let is_scoped = file_type == FileType::CopilotScoped;
        let scoped_frontmatter = if is_scoped {
            parse_frontmatter(content)
        } else {
            None
        };

        // COP-001: Empty instruction file (ERROR)
        if config.is_rule_enabled("COP-001") {
            if is_scoped {
                // For scoped files, check body after frontmatter
                if let Some(parsed) = scoped_frontmatter.as_ref() {
                    if is_body_empty(&parsed.body) {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                parsed.end_line + 1,
                                0,
                                "COP-001",
                                t!("rules.cop_001.message_no_content"),
                            )
                            .with_suggestion(t!("rules.cop_001.suggestion_empty")),
                        );
                    }
                } else if is_content_empty(content) {
                    // Scoped file with no frontmatter and no content
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "COP-001",
                            t!("rules.cop_001.message_empty"),
                        )
                        .with_suggestion(t!("rules.cop_001.suggestion_scoped_empty")),
                    );
                }
            } else {
                // For global files, check entire content
                if is_content_empty(content) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "COP-001",
                            t!("rules.cop_001.message_empty"),
                        )
                        .with_suggestion(t!("rules.cop_001.suggestion_empty")),
                    );
                }
            }
        }

        // COP-006: File length limit for global files (WARNING)
        const COPILOT_GLOBAL_LENGTH_LIMIT: usize = 4000;
        if config.is_rule_enabled("COP-006") && !is_scoped {
            let char_count = content.chars().count();
            if char_count > COPILOT_GLOBAL_LENGTH_LIMIT {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "COP-006",
                        t!("rules.cop_006.message", len = char_count),
                    )
                    .with_suggestion(t!("rules.cop_006.suggestion")),
                );
            }
        }

        // Rules COP-002, COP-003, COP-004, COP-005 only apply to scoped instruction files
        if !is_scoped {
            return diagnostics;
        }

        // Parse frontmatter for scoped files
        let parsed = match scoped_frontmatter {
            Some(p) => p,
            None => {
                // COP-002: Missing frontmatter in scoped file
                if config.is_rule_enabled("COP-002") && !is_content_empty(content) {
                    let mut diagnostic = Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "COP-002",
                        t!("rules.cop_002.message_missing"),
                    )
                    .with_suggestion(t!("rules.cop_002.suggestion_add_frontmatter"));

                    // Unsafe auto-fix: insert template frontmatter at start of file.
                    diagnostic = diagnostic.with_fix(Fix::insert(
                        0,
                        "---\napplyTo: \"**/*\"\n---\n",
                        t!("rules.cop_002.fix"),
                        false,
                    ));

                    diagnostics.push(diagnostic);
                }
                return diagnostics;
            }
        };

        // COP-002: Invalid frontmatter (YAML parse error)
        if config.is_rule_enabled("COP-002") {
            if let Some(ref error) = parsed.parse_error {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        parsed.start_line,
                        0,
                        "COP-002",
                        t!("rules.cop_002.message_invalid_yaml", error = error.as_str()),
                    )
                    .with_suggestion(t!("rules.cop_002.suggestion_fix_yaml")),
                );
                // Can't continue validating if YAML is broken
                return diagnostics;
            }

            // Check for missing applyTo field
            if let Some(ref schema) = parsed.schema {
                if schema.apply_to.is_none() {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            parsed.start_line,
                            0,
                            "COP-002",
                            t!("rules.cop_002.message_missing_apply_to"),
                        )
                        .with_suggestion(t!("rules.cop_002.suggestion_add_apply_to")),
                    );
                }
            }
        }

        // COP-003: Invalid glob pattern
        if config.is_rule_enabled("COP-003") {
            if let Some(ref schema) = parsed.schema {
                if let Some(ref apply_to) = schema.apply_to {
                    let validation = validate_glob_pattern(apply_to);
                    if !validation.valid {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                parsed.start_line + 1, // applyTo is typically on line 2
                                0,
                                "COP-003",
                                t!(
                                    "rules.cop_003.message",
                                    pattern = apply_to.as_str(),
                                    error = validation.error.unwrap_or_default()
                                ),
                            )
                            .with_suggestion(t!("rules.cop_003.suggestion")),
                        );
                    }
                }
            }
        }

        // COP-004: Unknown frontmatter keys (WARNING)
        if config.is_rule_enabled("COP-004") {
            for unknown in &parsed.unknown_keys {
                let mut diagnostic = Diagnostic::warning(
                    path.to_path_buf(),
                    unknown.line,
                    unknown.column,
                    "COP-004",
                    t!("rules.cop_004.message", key = unknown.key.as_str()),
                )
                .with_suggestion(t!("rules.cop_004.suggestion", key = unknown.key.as_str()));

                // Safe auto-fix: remove unknown top-level frontmatter key line.
                if let Some((start, end)) = line_byte_range(content, unknown.line) {
                    diagnostic = diagnostic.with_fix(Fix::delete(
                        start,
                        end,
                        format!("Remove unknown frontmatter key '{}'", unknown.key),
                        true,
                    ));
                }

                diagnostics.push(diagnostic);
            }
        }

        // COP-005: Invalid excludeAgent value (ERROR)
        if config.is_rule_enabled("COP-005") {
            if let Some(ref schema) = parsed.schema {
                if let Some(ref agent_value) = schema.exclude_agent {
                    const VALID_AGENTS: &[&str] = &["code-review", "coding-agent"];
                    if !VALID_AGENTS.contains(&agent_value.as_str()) {
                        // Find the line number of excludeAgent in raw frontmatter
                        let line = parsed
                            .raw
                            .lines()
                            .enumerate()
                            .find(|(_, l)| l.trim_start().starts_with("excludeAgent:"))
                            .map(|(i, _)| parsed.start_line + 1 + i)
                            .unwrap_or(parsed.start_line + 1);

                        let mut diagnostic = Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            0,
                            "COP-005",
                            t!("rules.cop_005.message", value = agent_value.as_str()),
                        )
                        .with_suggestion(t!("rules.cop_005.suggestion"));

                        // Unsafe auto-fix: replace with closest valid agent value
                        if let Some(closest) =
                            super::find_closest_value(agent_value.as_str(), VALID_AGENTS)
                        {
                            if let Some((start, end)) = crate::rules::find_yaml_value_range(
                                content,
                                &parsed,
                                "excludeAgent",
                                true,
                            ) {
                                let slice = content.get(start..end).unwrap_or("");
                                let replacement = if slice.starts_with('"') {
                                    format!("\"{}\"", closest)
                                } else if slice.starts_with('\'') {
                                    format!("'{}'", closest)
                                } else {
                                    closest.to_string()
                                };
                                diagnostic = diagnostic.with_fix(Fix::replace(
                                    start,
                                    end,
                                    replacement,
                                    t!("rules.cop_005.fix", fixed = closest),
                                    false,
                                ));
                            }
                        }

                        diagnostics.push(diagnostic);
                    }
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate_global(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/copilot-instructions.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_scoped(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/instructions/typescript.instructions.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_scoped_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/instructions/typescript.instructions.md"),
            content,
            config,
        )
    }

    fn validate_agent(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/agents/reviewer.agent.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_agent_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/agents/reviewer.agent.md"),
            content,
            config,
        )
    }

    fn validate_prompt(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/prompts/refactor.prompt.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_hooks(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/hooks/hooks.json"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_setup_steps(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/workflows/copilot-setup-steps.yml"),
            content,
            &LintConfig::default(),
        )
    }

    // ===== COP-001: Empty Instruction File =====

    #[test]
    fn test_cop_001_empty_global_file() {
        let diagnostics = validate_global("");
        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert_eq!(cop_001.len(), 1);
        assert_eq!(cop_001[0].level, DiagnosticLevel::Error);
        assert!(cop_001[0].message.contains("empty"));
    }

    #[test]
    fn test_cop_001_whitespace_only_global() {
        let diagnostics = validate_global("   \n\n\t  ");
        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert_eq!(cop_001.len(), 1);
    }

    #[test]
    fn test_cop_001_valid_global_file() {
        let content = "# Copilot Instructions\n\nFollow the coding style guide.";
        let diagnostics = validate_global(content);
        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert!(cop_001.is_empty());
    }

    #[test]
    fn test_cop_001_empty_scoped_body() {
        let content = r#"---
applyTo: "**/*.ts"
---
"#;
        let diagnostics = validate_scoped(content);
        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert_eq!(cop_001.len(), 1);
        assert!(cop_001[0].message.contains("no content after frontmatter"));
    }

    #[test]
    fn test_cop_001_valid_scoped_file() {
        let content = r#"---
applyTo: "**/*.ts"
---
# TypeScript Instructions

Use strict mode.
"#;
        let diagnostics = validate_scoped(content);
        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert!(cop_001.is_empty());
    }

    // ===== COP-002: Invalid Frontmatter =====

    #[test]
    fn test_cop_002_missing_frontmatter() {
        let content = "# Instructions without frontmatter";
        let diagnostics = validate_scoped(content);
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert_eq!(cop_002.len(), 1);
        assert!(cop_002[0].message.contains("missing required frontmatter"));
    }

    #[test]
    fn test_cop_002_has_autofix_for_missing_frontmatter() {
        let content = "# Instructions without frontmatter";
        let diagnostics = validate_scoped(content);
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert_eq!(cop_002.len(), 1);
        assert!(
            cop_002[0].has_fixes(),
            "COP-002 should have auto-fix for missing frontmatter"
        );
        let fix = &cop_002[0].fixes[0];
        assert!(!fix.safe, "COP-002 fix should be unsafe");
        assert_eq!(fix.start_byte, 0, "Fix should insert at start of file");
        assert_eq!(fix.end_byte, 0, "Fix should be an insert (start == end)");
        assert!(
            fix.replacement.contains("applyTo:"),
            "Fix should contain applyTo field"
        );
    }

    #[test]
    fn test_cop_002_no_autofix_for_yaml_error() {
        // YAML parse error should not get an insert-frontmatter fix
        let content = "---\napplyTo: [unclosed\n---\n# Body\n";
        let diagnostics = validate_scoped(content);
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert_eq!(cop_002.len(), 1);
        // This diagnostic is about invalid YAML, not missing frontmatter, so no insert fix
        assert!(
            !cop_002[0].has_fixes(),
            "COP-002 should not have auto-fix for YAML parse errors"
        );
    }

    #[test]
    fn test_cop_002_invalid_yaml() {
        let content = r#"---
applyTo: [unclosed
---
# Body
"#;
        let diagnostics = validate_scoped(content);
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert_eq!(cop_002.len(), 1);
        assert!(cop_002[0].message.contains("Invalid YAML"));
    }

    #[test]
    fn test_cop_002_missing_apply_to() {
        let content = r#"---
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert_eq!(cop_002.len(), 1);
        assert!(cop_002[0].message.contains("missing required 'applyTo'"));
    }

    #[test]
    fn test_cop_002_valid_frontmatter() {
        let content = r#"---
applyTo: "**/*.ts"
---
# TypeScript Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert!(cop_002.is_empty());
    }

    // ===== COP-003: Invalid Glob Pattern =====

    #[test]
    fn test_cop_003_invalid_glob() {
        let content = r#"---
applyTo: "[unclosed"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-003").collect();
        assert_eq!(cop_003.len(), 1);
        assert!(cop_003[0].message.contains("Invalid glob pattern"));
    }

    #[test]
    fn test_cop_003_valid_glob_patterns() {
        let patterns = vec!["**/*.ts", "*.rs", "src/**/*.js", "tests/**/*.test.ts"];

        for pattern in patterns {
            let content = format!(
                r#"---
applyTo: "{}"
---
# Instructions
"#,
                pattern
            );
            let diagnostics = validate_scoped(&content);
            let cop_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-003").collect();
            assert!(cop_003.is_empty(), "Pattern '{}' should be valid", pattern);
        }
    }

    // ===== COP-004: Unknown Frontmatter Keys =====

    #[test]
    fn test_cop_004_unknown_keys() {
        let content = r#"---
applyTo: "**/*.ts"
unknownKey: value
anotherBadKey: 123
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-004").collect();
        assert_eq!(cop_004.len(), 2);
        assert_eq!(cop_004[0].level, DiagnosticLevel::Warning);
        assert!(cop_004.iter().any(|d| d.message.contains("unknownKey")));
        assert!(cop_004.iter().any(|d| d.message.contains("anotherBadKey")));
        assert!(
            cop_004.iter().all(|d| d.has_fixes()),
            "All unknown key diagnostics should include safe deletion fixes"
        );
        assert!(cop_004.iter().all(|d| d.fixes[0].safe));
    }

    #[test]
    fn test_cop_004_no_unknown_keys() {
        let content = r#"---
applyTo: "**/*.rs"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-004").collect();
        assert!(cop_004.is_empty());
    }

    // ===== Global vs Scoped Behavior =====

    #[test]
    fn test_global_file_no_frontmatter_rules() {
        // Global files should not trigger COP-002/003/004
        let content = "# Instructions without frontmatter";
        let diagnostics = validate_global(content);

        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        let cop_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-003").collect();
        let cop_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-004").collect();

        assert!(cop_002.is_empty());
        assert!(cop_003.is_empty());
        assert!(cop_004.is_empty());
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_copilot_category() {
        let mut config = LintConfig::default();
        config.rules_mut().copilot = false;

        let content = "";
        let diagnostics = validate_scoped_with_config(content, &config);

        let cop_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("COP-"))
            .collect();
        assert!(cop_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["COP-001".to_string()];

        let content = "";
        let diagnostics = validate_scoped_with_config(content, &config);

        let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
        assert!(cop_001.is_empty());
    }

    // ===== Combined Issues =====

    #[test]
    fn test_multiple_issues() {
        let content = r#"---
unknownKey: value
---
"#;
        let diagnostics = validate_scoped(content);

        // Should have:
        // - COP-001 for empty body
        // - COP-002 for missing applyTo
        // - COP-004 for unknown key
        assert!(
            diagnostics.iter().any(|d| d.rule == "COP-001"),
            "Expected COP-001"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "COP-002"),
            "Expected COP-002"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "COP-004"),
            "Expected COP-004"
        );
    }

    #[test]
    fn test_valid_scoped_no_issues() {
        let content = r#"---
applyTo: "**/*.ts"
---
# TypeScript Guidelines

Always use strict mode and explicit types.
"#;
        let diagnostics = validate_scoped(content);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    // ===== Additional COP rule tests =====

    #[test]
    fn test_cop_001_newlines_only() {
        let content = "\n\n\n";
        let diagnostics = validate_global(content);
        assert!(diagnostics.iter().any(|d| d.rule == "COP-001"));
    }

    #[test]
    fn test_cop_001_spaces_and_tabs() {
        let content = "   \t\t   ";
        let diagnostics = validate_global(content);
        assert!(diagnostics.iter().any(|d| d.rule == "COP-001"));
    }

    #[test]
    fn test_cop_002_yaml_with_tabs() {
        // YAML doesn't allow tabs for indentation
        let content = "---\n\tapplyTo: \"**/*.ts\"\n---\nBody";
        let diagnostics = validate_scoped(content);
        assert!(diagnostics.iter().any(|d| d.rule == "COP-002"));
    }

    #[test]
    fn test_cop_002_valid_frontmatter_no_error() {
        // Test that valid frontmatter doesn't trigger COP-002
        let content = r#"---
applyTo: "**/*.ts"
---
Body content"#;
        let diagnostics = validate_scoped(content);
        // Valid frontmatter should not trigger COP-002
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert!(
            cop_002.is_empty(),
            "Valid frontmatter should not trigger COP-002"
        );
    }

    #[test]
    fn test_cop_003_all_valid_patterns() {
        let valid_patterns = [
            "**/*.ts",
            "*.rs",
            "src/**/*.py",
            "tests/*.test.js",
            "{src,lib}/**/*.ts",
        ];

        for pattern in valid_patterns {
            let content = format!("---\napplyTo: \"{}\"\n---\nBody", pattern);
            let diagnostics = validate_scoped(&content);
            let cop_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-003").collect();
            assert!(cop_003.is_empty(), "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_cop_003_invalid_patterns() {
        let invalid_patterns = ["[invalid", "***", "**["];

        for pattern in invalid_patterns {
            let content = format!("---\napplyTo: \"{}\"\n---\nBody", pattern);
            let diagnostics = validate_scoped(&content);
            let cop_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-003").collect();
            assert!(
                !cop_003.is_empty(),
                "Pattern '{}' should be invalid",
                pattern
            );
        }
    }

    #[test]
    fn test_cop_004_all_known_keys() {
        let content = r#"---
applyTo: "**/*.ts"
---
Body"#;
        let diagnostics = validate_scoped(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "COP-004"));
    }

    #[test]
    fn test_cop_004_multiple_unknown_keys() {
        let content = r#"---
applyTo: "**/*.ts"
unknownKey1: value1
unknownKey2: value2
---
Body"#;
        let diagnostics = validate_scoped(content);
        let cop_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-004").collect();
        // Should report at least one unknown key warning
        assert!(!cop_004.is_empty());
    }

    #[test]
    fn test_all_cop_rules_can_be_disabled() {
        let rules = [
            "COP-001", "COP-002", "COP-003", "COP-004", "COP-005", "COP-006",
        ];
        let long_content = make_long_content();

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];

            // Content and path that could trigger each rule
            let (content, path): (&str, &str) = match rule {
                "COP-001" => ("", ".github/copilot-instructions.md"),
                "COP-002" => (
                    "Content without frontmatter",
                    ".github/instructions/test.instructions.md",
                ),
                "COP-003" => (
                    "---\napplyTo: \"[invalid\"\n---\nBody",
                    ".github/instructions/test.instructions.md",
                ),
                "COP-004" => (
                    "---\nunknown: value\n---\nBody",
                    ".github/instructions/test.instructions.md",
                ),
                "COP-005" => (
                    "---\napplyTo: \"**/*.ts\"\nexcludeAgent: \"invalid\"\n---\nBody",
                    ".github/instructions/test.instructions.md",
                ),
                "COP-006" => (&long_content, ".github/copilot-instructions.md"),
                _ => unreachable!("Unknown rule: {rule}"),
            };

            let validator = CopilotValidator;
            let diagnostics = validator.validate(Path::new(path), content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    /// Generate long content for COP-006 tests (>4000 chars)
    fn make_long_content() -> String {
        let mut s = String::from("# Copilot Instructions\n\n");
        while s.len() <= 4001 {
            s.push_str("Follow consistent naming conventions for variables and functions.\n");
        }
        s
    }

    fn validate_global_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/copilot-instructions.md"),
            content,
            config,
        )
    }

    // ===== COP-005: Invalid excludeAgent Value =====

    #[test]
    fn test_cop_005_invalid_exclude_agent() {
        let content = r#"---
applyTo: "**/*.ts"
excludeAgent: "invalid-agent"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert_eq!(cop_005.len(), 1);
        assert_eq!(cop_005[0].level, DiagnosticLevel::Error);
        assert!(cop_005[0].message.contains("invalid-agent"));
    }

    #[test]
    fn test_cop_005_valid_code_review() {
        let content = r#"---
applyTo: "**/*.ts"
excludeAgent: "code-review"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert!(cop_005.is_empty());
    }

    #[test]
    fn test_cop_005_valid_coding_agent() {
        let content = r#"---
applyTo: "**/*.ts"
excludeAgent: "coding-agent"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert!(cop_005.is_empty());
    }

    #[test]
    fn test_cop_005_absent_exclude_agent() {
        let content = r#"---
applyTo: "**/*.ts"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert!(cop_005.is_empty());
    }

    #[test]
    fn test_cop_005_global_file_no_trigger() {
        let content = r#"---
applyTo: "**/*.ts"
excludeAgent: "invalid-agent"
---
# Instructions
"#;
        // Global files should not trigger COP-005 (scoped-only rule)
        let diagnostics = validate_global(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert!(cop_005.is_empty());
    }

    #[test]
    fn test_cop_005_case_sensitive() {
        let content =
            "---\napplyTo: \"**/*.ts\"\nexcludeAgent: \"Code-Review\"\n---\n# Instructions\n";
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert_eq!(cop_005.len(), 1, "Mixed-case value should trigger COP-005");
        // Case-insensitive match should produce an auto-fix
        assert!(
            cop_005[0].has_fixes(),
            "COP-005 should have auto-fix for case mismatch"
        );
        let fix = &cop_005[0].fixes[0];
        assert!(!fix.safe, "COP-005 fix should be unsafe");
        assert!(
            fix.replacement.contains("code-review"),
            "Fix should suggest 'code-review', got: {}",
            fix.replacement
        );
    }

    #[test]
    fn test_cop_005_empty_string() {
        let content = "---\napplyTo: \"**/*.ts\"\nexcludeAgent: \"\"\n---\n# Instructions\n";
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert_eq!(
            cop_005.len(),
            1,
            "Empty excludeAgent should trigger COP-005"
        );
        // Empty string should NOT get a fix (no close match)
        assert!(
            !cop_005[0].has_fixes(),
            "COP-005 should not auto-fix empty string"
        );
    }

    #[test]
    fn test_cop_005_autofix_nonsense() {
        let content =
            "---\napplyTo: \"**/*.ts\"\nexcludeAgent: \"nonsense\"\n---\n# Instructions\n";
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert_eq!(cop_005.len(), 1);
        // "nonsense" has no close match - should NOT get a fix
        assert!(
            !cop_005[0].has_fixes(),
            "COP-005 should not auto-fix nonsense values"
        );
    }

    // ===== COP-006: File Length Limit =====

    #[test]
    fn test_cop_006_short_file() {
        let content = "# Short copilot instructions\n\nFollow the coding standards.";
        let diagnostics = validate_global(content);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert!(cop_006.is_empty());
    }

    #[test]
    fn test_cop_006_long_file() {
        let long_content = make_long_content();
        let expected_len = long_content.len().to_string();
        let diagnostics = validate_global(&long_content);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert_eq!(cop_006.len(), 1);
        assert_eq!(cop_006[0].level, DiagnosticLevel::Warning);
        assert!(
            cop_006[0].message.contains(&expected_len),
            "Diagnostic message should contain the file length"
        );
    }

    #[test]
    fn test_cop_006_exact_boundary() {
        // 4000 chars should pass
        let content_4000 = "x".repeat(4000);
        let diagnostics = validate_global(&content_4000);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert!(cop_006.is_empty(), "4000 chars should not trigger COP-006");

        // 4001 chars should warn
        let content_4001 = "x".repeat(4001);
        let diagnostics = validate_global(&content_4001);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert_eq!(cop_006.len(), 1, "4001 chars should trigger COP-006");
    }

    #[test]
    fn test_cop_006_scoped_file_no_trigger() {
        // Scoped files should not trigger COP-006
        let mut content = String::from("---\napplyTo: \"**/*.ts\"\n---\n# Instructions\n\n");
        while content.len() <= 5000 {
            content.push_str("Follow consistent naming conventions for all variables.\n");
        }
        let diagnostics = validate_scoped(&content);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert!(cop_006.is_empty());
    }

    #[test]
    fn test_cop_006_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["COP-006".to_string()];

        let diagnostics = validate_global_with_config(&make_long_content(), &config);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert!(cop_006.is_empty());
    }

    // ===== COP-007..COP-015, COP-017, COP-018 =====

    #[test]
    fn test_cop_007_missing_description() {
        let diagnostics = validate_agent(
            r#"---
target: vscode
---
Review pull requests.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-007"));
    }

    #[test]
    fn test_cop_007_missing_frontmatter_message() {
        let diagnostics = validate_agent("Review pull requests.");
        let cop_007 = diagnostics
            .iter()
            .find(|d| d.rule == "COP-007")
            .expect("expected COP-007");
        assert!(cop_007.message.contains("must start with YAML frontmatter"));
    }

    #[test]
    fn test_cop_008_unknown_agent_field() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
unknown-field: true
---
Review pull requests.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-008"));
    }

    #[test]
    fn test_cop_008_allows_current_agent_invocation_keys() {
        let diagnostics = validate_agent(
            r#"---
name: reviewer
description: Review pull requests
disable-model-invocation: true
user-invocable: true
metadata:
  owner: security
---
Review pull requests.
"#,
        );
        assert!(
            diagnostics.iter().all(|d| d.rule != "COP-008"),
            "Documented keys should not trigger COP-008"
        );
    }

    #[test]
    fn test_cop_008_rejects_null_current_agent_invocation_keys() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
disable-model-invocation: null
user-invocable: null
metadata: null
---
Review pull requests.
"#,
        );
        let cop_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-008").collect();
        assert_eq!(
            cop_008.len(),
            3,
            "Expected one COP-008 per null typed field"
        );
    }

    #[test]
    fn test_cop_008_invalid_agent_frontmatter_yaml() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
target: [vscode
---
Review pull requests.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-008"));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("invalid YAML"))
        );
        let cop_008 = diagnostics
            .iter()
            .find(|d| d.rule == "COP-008")
            .expect("expected COP-008");
        assert_eq!(cop_008.level, DiagnosticLevel::Error);
        assert!(
            diagnostics.iter().all(|d| d.rule != "COP-007"),
            "Invalid YAML should not be reported as missing description"
        );
    }

    #[test]
    fn test_cop_008_disabled_still_reports_parse_error() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["COP-008".to_string()];
        let diagnostics = validate_agent_with_config(
            r#"---
description: Review pull requests
target: [vscode
---
Review pull requests.
"#,
            &config,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "COP-008"));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule == "COP-007" && d.message.contains("invalid YAML")),
            "Parse errors should still be surfaced when COP-008 is disabled"
        );
    }

    #[test]
    fn test_cop_009_invalid_target() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
target: desktop
---
Review pull requests.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-009"));
    }

    #[test]
    fn test_cop_009_line_detection_handles_space_before_colon() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
target : desktop
---
Review pull requests.
"#,
        );
        let cop_009 = diagnostics
            .iter()
            .find(|d| d.rule == "COP-009")
            .expect("expected COP-009");
        assert_eq!(cop_009.line, 3);
    }

    #[test]
    fn test_cop_010_invalid_infer_type() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
infer: "auto"
---
Review pull requests.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-010"));
    }

    #[test]
    fn test_cop_010_invalid_numeric_infer_type() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
infer: 1
---
Review pull requests.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-010"));
    }

    #[test]
    fn test_cop_010_invalid_null_infer_type() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
infer: null
---
Review pull requests.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-010"));
    }

    #[test]
    fn test_cop_010_accepts_boolean_infer() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
infer: true
---
Review pull requests.
"#,
        );
        assert!(
            diagnostics.iter().all(|d| d.rule != "COP-010"),
            "Boolean infer should not trigger COP-010"
        );
    }

    #[test]
    fn test_cop_010_accepts_boolean_false_infer() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
infer: false
---
Review pull requests.
"#,
        );
        assert!(
            diagnostics.iter().all(|d| d.rule != "COP-010"),
            "Boolean infer=false should not trigger COP-010"
        );
    }

    #[test]
    fn test_cop_011_agent_body_length_limit() {
        let long_body = "x".repeat(30_001);
        let diagnostics =
            validate_agent(&format!("---\ndescription: Long agent\n---\n{}", long_body));
        assert!(diagnostics.iter().any(|d| d.rule == "COP-011"));
    }

    #[test]
    fn test_cop_012_github_unsupported_fields() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
model: gpt-4
---
Review pull requests.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-012"));
    }

    #[test]
    fn test_cop_012_skips_vscode_target() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
target: vscode
model: gpt-4
---
Review pull requests.
"#,
        );
        assert!(
            diagnostics.iter().all(|d| d.rule != "COP-012"),
            "COP-012 should not fire for VS Code-targeted agents"
        );
    }

    #[test]
    fn test_cop_013_empty_prompt_body() {
        let diagnostics = validate_prompt(
            r#"---
description: Refactor selected code
---
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-013"));
    }

    #[test]
    fn test_cop_013_line_is_clamped_to_file_length() {
        let content = r#"---
description: Refactor selected code
---"#;
        let diagnostics = validate_prompt(content);
        let cop_013 = diagnostics
            .iter()
            .find(|d| d.rule == "COP-013")
            .expect("expected COP-013");
        assert_eq!(cop_013.line, 3);
    }

    #[test]
    fn test_cop_014_unknown_prompt_field() {
        let diagnostics = validate_prompt(
            r#"---
description: Refactor selected code
mystery: true
---
Refactor the selected code.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-014"));
    }

    #[test]
    fn test_cop_014_invalid_prompt_frontmatter_yaml() {
        let diagnostics = validate_prompt(
            r#"---
description: Refactor selected code
agent: [ask
---
Refactor the selected code.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-014"));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("invalid YAML"))
        );
    }

    #[test]
    fn test_cop_014_invalid_prompt_frontmatter_yaml_does_not_emit_cop_013() {
        let diagnostics = validate_prompt(
            r#"---
description: Refactor selected code
agent: [ask
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-014"));
        assert!(
            diagnostics.iter().all(|d| d.rule != "COP-013"),
            "Invalid frontmatter should not also report empty prompt body"
        );
    }

    #[test]
    fn test_cop_015_invalid_prompt_agent_mode() {
        let diagnostics = validate_prompt(
            r#"---
description: Refactor selected code
agent: maybe
---
Refactor the selected code.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-015"));
    }

    #[test]
    fn test_cop_017_hooks_schema_validation() {
        let diagnostics = validate_hooks(
            r#"{
  "version": 1,
  "hooks": [
    { "type": "command", "events": ["notReal"], "command": { "bash": "echo hi" } }
  ]
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-017"));
    }

    #[test]
    fn test_cop_017_invalid_hooks_json_syntax() {
        let diagnostics = validate_hooks("{");
        assert!(diagnostics.iter().any(|d| d.rule == "COP-017"));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("Invalid hooks.json syntax"))
        );
    }

    #[test]
    fn test_cop_018_setup_steps_requires_copilot_setup_steps_job() {
        let diagnostics = validate_setup_steps(
            r#"
name: Copilot Setup Steps
jobs:
  build:
    runs-on: ubuntu-latest
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-018"));
    }

    #[test]
    fn test_cop_018_invalid_setup_workflow_yaml() {
        let diagnostics = validate_setup_steps("name: Copilot Setup Steps\njobs: [");
        assert!(diagnostics.iter().any(|d| d.rule == "COP-018"));
        assert!(diagnostics.iter().any(|d| {
            d.message
                .contains("Invalid copilot-setup-steps workflow YAML")
        }));
    }

    // ===== Autofix Tests for New Fixes =====

    #[test]
    fn test_cop_008_has_fix() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
unknown-field: true
---
Review pull requests.
"#,
        );
        let cop_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-008").collect();
        assert_eq!(cop_008.len(), 1);
        assert!(cop_008[0].has_fixes(), "COP-008 should have auto-fix");
        assert!(cop_008[0].fixes[0].safe, "COP-008 fix should be safe");
        assert!(
            cop_008[0].fixes[0].is_deletion(),
            "COP-008 fix should be a deletion"
        );
    }

    #[test]
    fn test_cop_010_has_no_fix() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
infer: "auto"
---
Review pull requests.
"#,
        );
        let cop_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-010").collect();
        assert_eq!(cop_010.len(), 1);
        assert!(
            !cop_010[0].has_fixes(),
            "COP-010 should not offer auto-fix for infer type errors"
        );
    }

    #[test]
    fn test_cop_012_has_fix() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
model: gpt-4
---
Review pull requests.
"#,
        );
        let cop_012: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-012").collect();
        assert_eq!(cop_012.len(), 1);
        assert!(cop_012[0].has_fixes(), "COP-012 should have auto-fix");
        assert!(cop_012[0].fixes[0].safe, "COP-012 fix should be safe");
    }

    #[test]
    fn test_cop_014_has_fix() {
        let diagnostics = validate_prompt(
            r#"---
description: Refactor selected code
mystery: true
---
Refactor the selected code.
"#,
        );
        let cop_014: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-014").collect();
        assert_eq!(cop_014.len(), 1);
        assert!(cop_014[0].has_fixes(), "COP-014 should have auto-fix");
        assert!(cop_014[0].fixes[0].safe, "COP-014 fix should be safe");
    }

    #[test]
    fn test_cop_009_has_fix_for_case_mismatch() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
target: VSCode
---
Review pull requests.
"#,
        );
        let cop_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-009").collect();
        assert_eq!(cop_009.len(), 1);
        assert!(
            cop_009[0].has_fixes(),
            "COP-009 should have auto-fix for case mismatch"
        );
        assert!(!cop_009[0].fixes[0].safe, "COP-009 fix should be unsafe");
        assert!(cop_009[0].fixes[0].replacement.contains("vscode"));
    }

    #[test]
    fn test_cop_015_has_fix_for_case_mismatch() {
        let diagnostics = validate_prompt(
            r#"---
description: Refactor selected code
agent: Always
---
Refactor the selected code.
"#,
        );
        let cop_015: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-015").collect();
        assert_eq!(cop_015.len(), 1);
        assert!(
            cop_015[0].has_fixes(),
            "COP-015 should have auto-fix for case mismatch"
        );
        assert!(!cop_015[0].fixes[0].safe, "COP-015 fix should be unsafe");
        assert!(cop_015[0].fixes[0].replacement.contains("always"));
    }

    // ===== Helper functions for new rules =====

    fn validate_plugin(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/copilot-extensions/plugin.json"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_copilot_skill_md(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/skills/my-skill/SKILL.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_copilot_skill_md_agents(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".agents/skills/my-skill/SKILL.md"),
            content,
            &LintConfig::default(),
        )
    }

    // ===== COP-019: Plugin Manifest Missing Required Fields =====

    #[test]
    fn test_cop_019_missing_name() {
        let diagnostics = validate_plugin(
            r#"{
  "description": "A Copilot plugin",
  "version": "1.0.0",
  "agents": {}
}"#,
        );
        let cop_019: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-019").collect();
        assert_eq!(cop_019.len(), 1);
        assert_eq!(cop_019[0].level, DiagnosticLevel::Error);
        assert!(cop_019[0].message.contains("name"));
    }

    #[test]
    fn test_cop_019_missing_all_required() {
        let diagnostics = validate_plugin(
            r#"{
  "agents": {}
}"#,
        );
        let cop_019: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-019").collect();
        assert_eq!(cop_019.len(), 3);
    }

    #[test]
    fn test_cop_019_all_present() {
        let diagnostics = validate_plugin(
            r#"{
  "name": "my-plugin",
  "description": "A Copilot plugin",
  "version": "1.0.0",
  "skills": {}
}"#,
        );
        let cop_019: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-019").collect();
        assert!(cop_019.is_empty());
    }

    #[test]
    fn test_cop_019_not_copilot_plugin() {
        // Without Copilot-specific keys, should not trigger
        let diagnostics = validate_plugin(
            r#"{
  "foo": "bar"
}"#,
        );
        let cop_019: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-019").collect();
        assert!(cop_019.is_empty());
    }

    // ===== COP-020: Plugin Manifest Invalid Field Types =====

    #[test]
    fn test_cop_020_name_not_string() {
        let diagnostics = validate_plugin(
            r#"{
  "name": 123,
  "description": "A Copilot plugin",
  "version": "1.0.0",
  "agents": {}
}"#,
        );
        let cop_020: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-020").collect();
        assert_eq!(cop_020.len(), 1);
        assert_eq!(cop_020[0].level, DiagnosticLevel::Warning);
        assert!(cop_020[0].message.contains("name"));
        assert!(cop_020[0].message.contains("string"));
    }

    #[test]
    fn test_cop_020_keywords_not_array() {
        let diagnostics = validate_plugin(
            r#"{
  "name": "my-plugin",
  "description": "A Copilot plugin",
  "version": "1.0.0",
  "keywords": "not-an-array",
  "hooks": {}
}"#,
        );
        let cop_020: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-020").collect();
        assert_eq!(cop_020.len(), 1);
        assert!(cop_020[0].message.contains("keywords"));
        assert!(cop_020[0].message.contains("array"));
    }

    #[test]
    fn test_cop_020_valid_types() {
        let diagnostics = validate_plugin(
            r#"{
  "name": "my-plugin",
  "description": "A Copilot plugin",
  "version": "1.0.0",
  "keywords": ["copilot"],
  "agents": {},
  "skills": {}
}"#,
        );
        let cop_020: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-020").collect();
        assert!(cop_020.is_empty());
    }

    // ===== COP-022: CLI SKILL.md Missing Required Frontmatter =====

    #[test]
    fn test_cop_022_missing_frontmatter() {
        let diagnostics = validate_copilot_skill_md("# My Skill\n\nDo something useful.");
        let cop_022: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-022").collect();
        assert_eq!(cop_022.len(), 1);
        assert_eq!(cop_022[0].level, DiagnosticLevel::Error);
        assert!(cop_022[0].message.contains("missing required frontmatter"));
    }

    #[test]
    fn test_cop_022_missing_name() {
        let diagnostics = validate_copilot_skill_md(
            r#"---
description: A useful skill
---
# My Skill
"#,
        );
        let cop_022: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-022").collect();
        assert_eq!(cop_022.len(), 1);
        assert!(cop_022[0].message.contains("name"));
    }

    #[test]
    fn test_cop_022_missing_description() {
        let diagnostics = validate_copilot_skill_md(
            r#"---
name: my-skill
---
# My Skill
"#,
        );
        let cop_022: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-022").collect();
        assert_eq!(cop_022.len(), 1);
        assert!(cop_022[0].message.contains("description"));
    }

    #[test]
    fn test_cop_022_valid_frontmatter() {
        let diagnostics = validate_copilot_skill_md(
            r#"---
name: my-skill
description: A useful skill
---
# My Skill
"#,
        );
        let cop_022: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-022").collect();
        assert!(cop_022.is_empty());
    }

    #[test]
    fn test_cop_022_agents_skills_path() {
        let diagnostics = validate_copilot_skill_md_agents("# My Skill\n\nDo something useful.");
        let cop_022: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-022").collect();
        assert_eq!(cop_022.len(), 1);
    }

    // ===== COP-023: CLI SKILL.md Name Format =====

    #[test]
    fn test_cop_023_uppercase_name() {
        let diagnostics = validate_copilot_skill_md(
            r#"---
name: MySkill
description: A useful skill
---
# My Skill
"#,
        );
        let cop_023: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-023").collect();
        assert_eq!(cop_023.len(), 1);
        assert_eq!(cop_023[0].level, DiagnosticLevel::Warning);
        assert!(cop_023[0].message.contains("kebab-case"));
    }

    #[test]
    fn test_cop_023_underscores() {
        let diagnostics = validate_copilot_skill_md(
            r#"---
name: my_skill
description: A useful skill
---
# My Skill
"#,
        );
        let cop_023: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-023").collect();
        assert_eq!(cop_023.len(), 1);
    }

    #[test]
    fn test_cop_023_spaces() {
        let diagnostics = validate_copilot_skill_md(
            r#"---
name: my skill
description: A useful skill
---
# My Skill
"#,
        );
        let cop_023: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-023").collect();
        assert_eq!(cop_023.len(), 1);
    }

    #[test]
    fn test_cop_023_valid_kebab_case() {
        let diagnostics = validate_copilot_skill_md(
            r#"---
name: my-useful-skill
description: A useful skill
---
# My Skill
"#,
        );
        let cop_023: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-023").collect();
        assert!(cop_023.is_empty());
    }

    // ===== COP-027: Deprecated `infer` Field =====

    #[test]
    fn test_cop_027_deprecated_infer_true() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
infer: true
---
Review pull requests.
"#,
        );
        let cop_027: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-027").collect();
        assert_eq!(cop_027.len(), 1);
        assert_eq!(cop_027[0].level, DiagnosticLevel::Info);
        assert!(cop_027[0].message.contains("deprecated"));
        assert!(cop_027[0].message.contains("disable-model-invocation"));
    }

    #[test]
    fn test_cop_027_deprecated_infer_false() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
infer: false
---
Review pull requests.
"#,
        );
        let cop_027: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-027").collect();
        assert_eq!(cop_027.len(), 1);
    }

    #[test]
    fn test_cop_027_no_infer_field() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
---
Review pull requests.
"#,
        );
        let cop_027: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-027").collect();
        assert!(cop_027.is_empty());
    }

    // ===== COP-024: Unknown SKILL.md frontmatter field =====

    fn validate_skill(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/skills/my-skill/SKILL.md"),
            content,
            &LintConfig::default(),
        )
    }

    #[test]
    fn test_cop_024_unknown_skill_frontmatter_field() {
        let content = r#"---
name: my-skill
description: A test skill
unknown_field: some value
---
This is the skill body.
"#;
        let diagnostics = validate_skill(content);
        let cop_024: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-024").collect();
        assert_eq!(cop_024.len(), 1);
        assert!(cop_024[0].message.contains("unknown_field"));
        assert_eq!(cop_024[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_cop_024_known_skill_fields_no_warning() {
        let content = r#"---
name: my-skill
description: A test skill
license: MIT
---
This is the skill body.
"#;
        let diagnostics = validate_skill(content);
        let cop_024: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-024").collect();
        assert!(cop_024.is_empty());
    }

    #[test]
    fn test_cop_024_multiple_unknown_fields() {
        let content = r#"---
name: my-skill
description: A test skill
author: someone
version: 1.0
---
Skill body.
"#;
        let diagnostics = validate_skill(content);
        let cop_024: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-024").collect();
        assert_eq!(cop_024.len(), 2);
        assert!(cop_024.iter().any(|d| d.message.contains("author")));
        assert!(cop_024.iter().any(|d| d.message.contains("version")));
    }

    // ===== COP-025: CLI .agent.md in wrong location =====

    #[test]
    fn test_cop_025_agent_md_in_wrong_location() {
        let validator = CopilotValidator;
        let diagnostics = validator.validate(
            Path::new("reviewer.agent.md"),
            "---\ndescription: Review PRs\n---\nReview pull requests.",
            &LintConfig::default(),
        );
        let cop_025: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-025").collect();
        assert_eq!(cop_025.len(), 1);
        assert!(cop_025[0].message.contains("reviewer.agent.md"));
        assert_eq!(cop_025[0].level, DiagnosticLevel::Info);
    }

    #[test]
    fn test_cop_025_agent_md_in_correct_location() {
        let diagnostics =
            validate_agent("---\ndescription: Review PRs\n---\nReview pull requests.");
        let cop_025: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-025").collect();
        assert!(cop_025.is_empty());
    }

    #[test]
    fn test_cop_025_non_agent_md_file() {
        let validator = CopilotValidator;
        let diagnostics = validator.validate(
            Path::new("README.md"),
            "Some content",
            &LintConfig::default(),
        );
        let cop_025: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-025").collect();
        assert!(cop_025.is_empty());
    }

    // ===== COP-026: Deprecated SSE transport in MCP config =====

    fn validate_mcp_config(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new("mcp-config.json"),
            content,
            &LintConfig::default(),
        )
    }

    #[test]
    fn test_cop_026_sse_transport_deprecated() {
        let content = r#"{
    "mcpServers": {
        "my-server": {
            "type": "sse",
            "url": "http://localhost:3000"
        }
    }
}"#;
        let diagnostics = validate_mcp_config(content);
        let cop_026: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-026").collect();
        assert_eq!(cop_026.len(), 1);
        assert!(cop_026[0].message.contains("my-server"));
        assert!(cop_026[0].message.contains("SSE"));
        assert_eq!(cop_026[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_cop_026_http_transport_no_warning() {
        let content = r#"{
    "mcpServers": {
        "my-server": {
            "type": "http",
            "url": "http://localhost:3000"
        }
    }
}"#;
        let diagnostics = validate_mcp_config(content);
        let cop_026: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-026").collect();
        assert!(cop_026.is_empty());
    }

    #[test]
    fn test_cop_026_non_mcp_config_file() {
        // Should not trigger for files named differently
        let validator = CopilotValidator;
        let content = r#"{
    "mcpServers": {
        "my-server": {
            "type": "sse",
            "url": "http://localhost:3000"
        }
    }
}"#;
        let diagnostics = validator.validate(
            Path::new("other-config.json"),
            content,
            &LintConfig::default(),
        );
        let cop_026: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-026").collect();
        assert!(cop_026.is_empty());
    }

    #[test]
    fn test_cop_026_multiple_sse_servers() {
        let content = r#"{
    "mcpServers": {
        "server-a": {
            "type": "sse",
            "url": "http://localhost:3000"
        },
        "server-b": {
            "type": "sse",
            "url": "http://localhost:4000"
        },
        "server-c": {
            "type": "http",
            "url": "http://localhost:5000"
        }
    }
}"#;
        let diagnostics = validate_mcp_config(content);
        let cop_026: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-026").collect();
        assert_eq!(cop_026.len(), 2);
    }
}
