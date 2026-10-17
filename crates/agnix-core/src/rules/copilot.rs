//! GitHub Copilot validation rules (COP-001 to COP-039, CP-SK-002)
//!
//! Validates:
//! - COP-001: Empty instruction file (HIGH)
//! - COP-002: Scoped instruction frontmatter (HIGH for invalid YAML; a file
//!   without `applyTo` is a warning, or a note when it has a `description`)
//! - COP-003: Invalid glob pattern in `applyTo` (HIGH)
//! - COP-004: Unknown frontmatter keys (MEDIUM)
//! - COP-005: Invalid `excludeAgent` value (HIGH) - "code-review" or "cloud-agent"
//! - COP-006: Instruction file over about 1,000 lines (MEDIUM)
//! - COP-007 to COP-012, COP-027, COP-028, COP-032 to COP-034: custom agents
//! - COP-013 to COP-015: prompt files
//! - COP-017, COP-031, COP-036, COP-037: hook files (`rules::copilot_hooks`)
//! - COP-018, COP-029: `copilot-setup-steps` workflow
//! - COP-019, COP-020, COP-039: CLI plugin manifests
//! - COP-025: `.agent.md` outside the agent folders (LOW)
//! - COP-030: `@` file references Copilot CLI does not load (MEDIUM)
//! - COP-035: portable and VS Code MCP files (MEDIUM)
//! - COP-038: instruction pointing code review at an external link (LOW)
//! - CP-SK-002: skill pre-approves `shell` or `bash` (MEDIUM)

use crate::{
    FileType,
    config::{LintConfig, PerFileLintConfig},
    diagnostics::{Diagnostic, DiagnosticLevel, Fix},
    parsers::frontmatter::split_frontmatter,
    rules::{
        Validator, ValidatorMetadata,
        config_json::{JsonCx, key_span, node_span},
        kiro_steering::{frontmatter_edit_yields, yaml_top_level_entries},
    },
    schemas::{
        copilot::{
            is_body_empty, is_content_empty, parse_frontmatter, split_comma_separated_globs,
            validate_glob_pattern,
        },
        copilot_agent::{ParsedAgentFrontmatter, REASONING_EFFORT_VALUES, parse_agent_frontmatter},
        copilot_hooks::{
            SETUP_STEPS_JOB_KEYS, SETUP_STEPS_MAX_TIMEOUT_MINUTES, is_macos_runner_label,
            parse_setup_steps_yaml, runs_on_labels, setup_steps_job,
        },
        copilot_prompt::{
            BUILT_IN_AGENTS, is_body_empty as is_prompt_body_empty, parse_prompt_frontmatter,
        },
        kiro_json::{JsonNode, JsonValue, parse_json_document},
    },
};
use regex::Regex;
use rust_i18n::t;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

const RULE_IDS: &[&str] = &[
    "COP-001",
    "COP-002",
    "COP-003",
    "COP-004",
    "COP-005",
    "COP-006",
    "COP-007",
    "COP-008",
    "COP-009",
    "COP-010",
    "COP-011",
    "COP-012",
    "COP-013",
    "COP-014",
    "COP-015",
    "COP-017",
    "COP-018",
    "COP-019",
    "COP-020",
    "COP-025",
    "COP-027",
    "COP-028",
    "COP-029",
    "COP-030",
    "COP-031",
    "COP-032",
    "COP-033",
    "COP-034",
    "COP-035",
    "COP-036",
    "COP-037",
    "COP-038",
    "COP-039",
    "CP-SK-002",
];

/// "Limit any single instruction file to a maximum of about 1,000 lines."
const INSTRUCTION_LINE_LIMIT: usize = 1000;

/// Copilot cloud agent's custom agent prompt limit ("The prompt can be a
/// maximum of 30,000 characters").
const MAX_AGENT_BODY_CHARS: usize = 30_000;

/// Legacy plugin description limit ("Brief description. Max 1024 chars.").
const PLUGIN_DESCRIPTION_LIMIT: usize = 1024;

/// The `$schema` values that opt a plugin into Agent Plugins semantics.
const AGENT_PLUGINS_SCHEMAS: &[&str] = &[
    "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
    "https://agent-plugins.org/schemas/1.1.0/plugin.schema.json",
];

/// The closed Agent Plugins 1.0 manifest schema.
const AGENT_PLUGINS_FIELDS: &[&str] = &[
    "$schema",
    "name",
    "version",
    "description",
    "author",
    "homepage",
    "repository",
    "license",
    "keywords",
    "extensions",
];

/// Component path fields of the legacy manifest, which Agent Plugins 1.0
/// replaces with fixed locations.
const LEGACY_COMPONENT_FIELDS: &[&str] = &[
    "agents",
    "skills",
    "commands",
    "hooks",
    "mcpServers",
    "lspServers",
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

/// 1-based line of a byte offset.
fn line_of_offset(content: &str, offset: usize) -> usize {
    content[..offset.min(content.len())]
        .bytes()
        .filter(|b| *b == b'\n')
        .count()
        + 1
}

/// Byte range that removes the top-level frontmatter entry `key` (key line,
/// continuation lines and line break). The edit is applied to a copy and
/// re-parsed: it is returned only when the frontmatter then parses to the
/// original mapping without `key` and the body is unchanged, so multi-line
/// values never leave orphaned lines behind. `None` for a repeated key or an
/// entry that cannot be removed exactly.
pub(crate) fn frontmatter_key_removal(content: &str, key: &str) -> Option<(usize, usize)> {
    let parts = split_frontmatter(content);
    if !parts.has_frontmatter || !parts.has_closing {
        return None;
    }
    let yaml: serde_yaml::Value = serde_yaml::from_str(&parts.frontmatter).ok()?;
    let mut expected = yaml.clone();
    expected
        .as_mapping_mut()?
        .shift_remove(serde_yaml::Value::String(key.to_string()))?;
    // Removing the only key leaves an empty block, which parses as null.
    if expected
        .as_mapping()
        .is_some_and(serde_yaml::Mapping::is_empty)
    {
        expected = serde_yaml::Value::Null;
    }
    let entries = yaml_top_level_entries(&parts.frontmatter);
    let mut matching = entries.iter().filter(|entry| entry.key == key);
    let entry = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    let start = parts.frontmatter_start + entry.start;
    let mut end = parts.frontmatter_start + entry.value_end;
    let rest = content.get(end..)?;
    if rest.starts_with("\r\n") {
        end += 2;
    } else if rest.starts_with('\n') {
        end += 1;
    }
    frontmatter_edit_yields(content, start, end, "", &expected).then_some((start, end))
}

/// True when the run explicitly targets Copilot (`tools = ["copilot"]` or
/// `"github-copilot"`). Checks on files that other tools read too (AGENTS.md,
/// CLAUDE.md, GEMINI.md, `.mcp.json`, `.agents/skills`) run only then.
fn copilot_selected(config: &LintConfig) -> bool {
    config.tools().iter().any(|tool| {
        tool.eq_ignore_ascii_case("copilot") || tool.eq_ignore_ascii_case("github-copilot")
    })
}

fn ends_with_ignore_case(value: &str, suffix: &str) -> bool {
    value.len() >= suffix.len()
        && value
            .get(value.len() - suffix.len()..)
            .is_some_and(|tail| tail.eq_ignore_ascii_case(suffix))
}

fn path_has_components(path: &Path, first: &str, second: &str) -> bool {
    crate::file_types::path_contains_consecutive_components(path, first, second)
}

fn is_setup_steps_workflow(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("copilot-setup-steps.yml") | Some("copilot-setup-steps.yaml")
    )
}

fn is_copilot_settings_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("settings.json") | Some("settings.local.json")
    )
}

// ---------------------------------------------------------------------------
// Custom agents
// ---------------------------------------------------------------------------

/// True when the agent file sits in a folder Copilot reads agents from:
/// `.github/agents/`, `~/.copilot/agents/`, a root `agents/` folder
/// (organization and enterprise agents in the `.github` or `.github-private`
/// repository, and CLI plugin agents), or VS Code's profile `prompts/` folder.
fn is_in_agent_folder(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(component, Component::Normal(name) if name.to_str().is_some_and(|n| n.eq_ignore_ascii_case("agents")))
    }) || path_has_components(path, "User", "prompts")
}

/// The agent ID part of a file name: the name without `.agent.md` or `.md`.
fn agent_stem(filename: &str) -> &str {
    if ends_with_ignore_case(filename, ".agent.md") {
        &filename[..filename.len() - ".agent.md".len()]
    } else if ends_with_ignore_case(filename, ".md") {
        &filename[..filename.len() - ".md".len()]
    } else {
        filename
    }
}

fn validate_custom_agent(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // COP-032: custom chat modes were renamed to custom agents.
    if ends_with_ignore_case(filename, ".chatmode.md") {
        if config.is_rule_enabled("COP-032") {
            let stem = &filename[..filename.len() - ".chatmode.md".len()];
            diagnostics.push(
                Diagnostic::info(
                    path.to_path_buf(),
                    1,
                    0,
                    "COP-032",
                    t!("rules.cop_032.message", file = filename),
                )
                .with_suggestion(t!("rules.cop_032.suggestion", stem = stem)),
            );
        }
        return diagnostics;
    }

    // COP-025: `.agent.md` outside every folder Copilot reads agents from.
    if !is_in_agent_folder(path) {
        if config.is_rule_enabled("COP-025") {
            diagnostics.push(
                Diagnostic::info(
                    path.to_path_buf(),
                    1,
                    0,
                    "COP-025",
                    t!("rules.cop_025.message", file = filename),
                )
                .with_suggestion(t!("rules.cop_025.suggestion")),
            );
        }
        return diagnostics;
    }

    // COP-033: "the filename may only contain the following characters:
    // `.`, `-`, `_`, `a-z`, `A-Z`, `0-9`."
    let stem = agent_stem(filename);
    if config.is_rule_enabled("COP-033")
        && !stem.is_empty()
        && !stem
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                1,
                0,
                "COP-033",
                t!("rules.cop_033.message", file = filename),
            )
            .with_suggestion(t!("rules.cop_033.suggestion")),
        );
    }

    let parsed = parse_agent_frontmatter(content);
    let body = parsed.as_ref().map_or(content, |p| p.body.as_str());

    if config.is_rule_enabled("COP-011") {
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

    let target = parsed
        .as_ref()
        .and_then(|p| p.schema.as_ref())
        .and_then(|s| s.target.clone());
    // An agent with `target: vscode` is for VS Code and other IDEs only, where
    // "All fields in this table are optional" and `mcp-servers` and
    // `metadata` are "Not used".
    let vscode_only = target.as_deref() == Some("vscode");

    if config.is_rule_enabled("COP-007") && !vscode_only {
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

    let Some(parsed) = &parsed else {
        return diagnostics;
    };

    let raw_mapping = serde_yaml::from_str::<serde_yaml::Value>(&parsed.raw)
        .ok()
        .and_then(|raw| raw.as_mapping().cloned());
    let raw_value = |key: &str| {
        raw_mapping
            .as_ref()
            .and_then(|mapping| mapping.get(serde_yaml::Value::String(key.to_string())))
            .cloned()
    };

    if config.is_rule_enabled("COP-008") {
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

            // Copilot ignores fields it does not know, so removing one is safe
            // when the whole entry can be removed exactly.
            if let Some((start, end)) = frontmatter_key_removal(content, &unknown.key) {
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
            let string_or_strings = |value: &serde_yaml::Value| {
                value.is_string()
                    || value
                        .as_sequence()
                        .is_some_and(|items| items.iter().all(serde_yaml::Value::is_string))
            };
            let model_invalid = schema
                .model
                .as_ref()
                .is_some_and(|value| !string_or_strings(value));
            let tools_invalid = schema
                .tools
                .as_ref()
                .is_some_and(|value| !string_or_strings(value));
            let agents_invalid = raw_value("agents").is_some_and(|value| {
                !value
                    .as_sequence()
                    .is_some_and(|items| items.iter().all(serde_yaml::Value::is_string))
            });
            let hooks_invalid = raw_value("hooks").is_some_and(|value| !value.is_mapping());
            let effort = raw_value("reasoning-effort");
            let effort_invalid = effort.as_ref().is_some_and(|value| {
                !value
                    .as_str()
                    .is_some_and(|effort| REASONING_EFFORT_VALUES.contains(&effort))
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
                (
                    "model:",
                    model_invalid,
                    "model",
                    "Set 'model' to a model name or a list of model names in preference order.",
                ),
                (
                    "tools:",
                    tools_invalid,
                    "tools",
                    "Set 'tools' to a list of tool names or a comma-separated string.",
                ),
                (
                    "agents:",
                    agents_invalid,
                    "agents",
                    "Set 'agents' to a list of custom agent names (['*'] allows all).",
                ),
                (
                    "hooks:",
                    hooks_invalid,
                    "hooks",
                    "Set 'hooks' to a map of event names to hook commands.",
                ),
                (
                    "reasoning-effort:",
                    effort_invalid,
                    "reasoning-effort",
                    "Set 'reasoning-effort' to low, medium, high, xhigh or max.",
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

    let Some(schema) = &parsed.schema else {
        return diagnostics;
    };

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

                if let Some(suggested) = super::find_closest_value(target.as_str(), VALID_TARGETS) {
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

    if config.is_rule_enabled("COP-010") {
        let infer_is_non_boolean = schema.infer.as_ref().is_some_and(|infer| !infer.is_bool());
        let infer_is_explicit_null =
            schema.infer.is_none() && raw_value("infer").is_some_and(|value| value.is_null());

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

    // COP-027: GitHub retired `infer` ("**Retired**. Use
    // `disable-model-invocation` and `user-invocable` instead."), and VS Code
    // deprecated it.
    if config.is_rule_enabled("COP-027") {
        let has_infer = schema.infer.is_some() || raw_value("infer").is_some();
        if has_infer {
            let line = frontmatter_key_line(&parsed.raw, parsed.start_line, "infer:");
            // A LOW note: VS Code still reads the deprecated field.
            let mut diagnostic = Diagnostic::info(
                path.to_path_buf(),
                line,
                0,
                "COP-027",
                t!("rules.cop_027.message"),
            )
            .with_suggestion(t!("rules.cop_027.suggestion"));
            // GitHub documents disable-model-invocation = !infer, with
            // user-invocable defaulting to true independently of infer.
            // Explicit replacement fields take precedence and are never overwritten.
            if let Some(map) = raw_mapping.as_ref()
                && let Some(infer) = schema.infer.as_ref().and_then(|value| value.as_bool())
                && let Some((start, end)) =
                    crate::rules::find_yaml_value_range(content, parsed, "infer", true)
                && matches!(&content[start..end], "true" | "false")
                && let Some((line_start, _)) = line_byte_range(content, line)
                && content[line_start..start].trim_end() == "infer:"
            {
                let newline = if content.contains("\r\n") {
                    "\r\n"
                } else {
                    "\n"
                };
                let mut replacements = Vec::new();
                let mut valid = true;
                for (key, value) in [
                    ("disable-model-invocation", !infer),
                    ("user-invocable", true),
                ] {
                    if let Some(explicit) = map.get(serde_yaml::Value::String(key.to_string())) {
                        valid &= explicit.is_bool();
                    } else {
                        replacements.push(format!("{key}: {value}"));
                    }
                }
                if valid {
                    diagnostic = diagnostic.with_fix(Fix::replace(
                        line_start,
                        end,
                        replacements.join(newline),
                        t!("rules.cop_027.fix"),
                        false,
                    ));
                }
            }
            diagnostics.push(diagnostic);
        }
    }

    // COP-012: "The `argument-hint` and `handoffs` properties from VS Code
    // and other IDE custom agents are currently not supported for Copilot
    // cloud agent on GitHub.com. They are ignored to ensure compatibility."
    // VS Code uses both, so the fields are reported as a note and never
    // removed. `model` is a documented GitHub.com property and not reported.
    if config.is_rule_enabled("COP-012") && !vscode_only {
        let unsupported = [
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
                diagnostics.push(
                    Diagnostic::info(
                        path.to_path_buf(),
                        line,
                        0,
                        "COP-012",
                        t!("rules.cop_012.message", field = field_name),
                    )
                    .with_suggestion(t!("rules.cop_012.suggestion", field = field_name)),
                );
            }
        }
    }

    if !vscode_only {
        validate_agent_mcp_servers(
            path,
            content,
            parsed,
            raw_value("mcp-servers"),
            config,
            &mut diagnostics,
        );
    }

    diagnostics
}

/// The line of `key:` nested under the top-level `parent:` key.
fn nested_key_line(parsed: &ParsedAgentFrontmatter, parent: &str, key: &str) -> usize {
    let mut in_parent = false;
    let mut parent_line = parsed.start_line;
    for (idx, line) in parsed.raw.lines().enumerate() {
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();
        if indent == 0 && !trimmed.is_empty() && !trimmed.starts_with('#') {
            in_parent = trimmed
                .strip_prefix(parent)
                .is_some_and(|rest| rest.trim_start().starts_with(':'));
            if in_parent {
                parent_line = parsed.start_line + 1 + idx;
            }
            continue;
        }
        if in_parent
            && let Some(rest) = trimmed
                .strip_prefix(key)
                .or_else(|| trimmed.strip_prefix(&format!("\"{key}\"")))
                .or_else(|| trimmed.strip_prefix(&format!("'{key}'")))
            && rest.trim_start().starts_with(':')
        {
            return parsed.start_line + 1 + idx;
        }
    }
    parent_line
}

/// `${{ secrets.NAME }}` / `${{ vars.NAME }}` references.
fn actions_reference_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\$\{\{\s*(?:secrets|vars)\.([A-Za-z_][A-Za-z0-9_]*)\s*\}\}")
            .expect("actions reference pattern")
    })
}

/// `$NAME`, `${NAME}` and `${NAME:-default}` references.
fn shell_reference_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)(?::-[^}]*)?\}|\$([A-Za-z_][A-Za-z0-9_]*)")
            .expect("shell reference pattern")
    })
}

/// COP-028 and COP-034 for a custom agent's `mcp-servers` map, which Copilot
/// cloud agent and the CLI use ("Not used in VS Code and other IDE custom
/// agents").
fn validate_agent_mcp_servers(
    path: &Path,
    content: &str,
    parsed: &ParsedAgentFrontmatter,
    servers: Option<serde_yaml::Value>,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(servers) = servers.as_ref().and_then(serde_yaml::Value::as_mapping) else {
        return;
    };
    for (name, server) in servers {
        let Some(name) = name.as_str() else {
            continue;
        };
        let Some(server) = server.as_mapping() else {
            continue;
        };
        let get = |key: &str| server.get(serde_yaml::Value::String(key.to_string()));

        if config.is_rule_enabled("COP-034") {
            let line = nested_key_line(parsed, "mcp-servers", name);
            match get("tools") {
                None => diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        0,
                        "COP-034",
                        t!("rules.cop_034.missing_tools", server = name),
                    )
                    .with_suggestion(t!("rules.cop_034.missing_tools_suggestion")),
                ),
                Some(tools)
                    if tools
                        .as_sequence()
                        .is_some_and(|items| items.iter().any(|t| t.as_str() == Some("*"))) =>
                {
                    diagnostics.push(
                        Diagnostic::info(
                            path.to_path_buf(),
                            line,
                            0,
                            "COP-034",
                            t!("rules.cop_034.all_tools", server = name),
                        )
                        .with_suggestion(t!("rules.cop_034.all_tools_suggestion")),
                    );
                }
                Some(_) => {}
            }
        }

        if !config.is_rule_enabled("COP-028") {
            continue;
        }
        for field in ["env", "headers"] {
            let Some(values) = get(field).and_then(serde_yaml::Value::as_mapping) else {
                continue;
            };
            for value in values.values().filter_map(serde_yaml::Value::as_str) {
                let mut reported = Vec::new();
                for capture in actions_reference_regex().captures_iter(value) {
                    let var = &capture[1];
                    if !var.starts_with("COPILOT_MCP_") && !reported.contains(&var.to_string()) {
                        reported.push(var.to_string());
                        let line = content
                            .find(&capture[0])
                            .map_or(parsed.start_line, |offset| line_of_offset(content, offset));
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                line,
                                0,
                                "COP-028",
                                t!("rules.cop_028.message", name = var, server = name),
                            )
                            .with_suggestion(t!("rules.cop_028.suggestion", name = var)),
                        );
                    }
                }
                if actions_reference_regex().is_match(value) {
                    continue;
                }
                for capture in shell_reference_regex().captures_iter(value) {
                    let Some(var) = capture.get(1).or_else(|| capture.get(2)) else {
                        continue;
                    };
                    let var = var.as_str();
                    if var.starts_with("COPILOT_MCP_") || reported.contains(&var.to_string()) {
                        continue;
                    }
                    reported.push(var.to_string());
                    let line = content
                        .find(&capture[0])
                        .map_or(parsed.start_line, |offset| line_of_offset(content, offset));
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            line,
                            0,
                            "COP-028",
                            t!("rules.cop_028.shell_message", name = var, server = name),
                        )
                        .with_suggestion(t!("rules.cop_028.suggestion", name = var)),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Prompt files
// ---------------------------------------------------------------------------

/// Custom agent IDs and names defined in the repository: `.github/agents/`
/// files (ID = file name without `.agent.md` or `.md`, plus the `name` field)
/// and `.claude/agents/`, which VS Code also reads. `None` when the
/// repository has no `.github/agents/` folder, since the agent may then be a
/// user or organization agent agnix cannot see.
fn repository_agent_names(prompt_path: &Path, config: &LintConfig) -> Option<Vec<String>> {
    let prompts_dir = prompt_path.parent()?;
    let github = prompts_dir.parent()?;
    let root = github.parent()?;
    let fs = config.fs();
    let agents_dir = github.join("agents");
    if !fs.is_dir(&agents_dir) {
        return None;
    }
    let mut names = Vec::new();
    let mut pending = vec![agents_dir, root.join(".claude").join("agents")];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = fs.read_dir(&dir) else {
            continue;
        };
        for entry in entries {
            if entry.metadata.is_dir {
                pending.push(entry.path);
                continue;
            }
            let Some(file) = entry.path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !ends_with_ignore_case(file, ".md") {
                continue;
            }
            names.push(agent_stem(file).to_string());
            if let Ok(text) = fs.read_to_string(&entry.path)
                && let Some(parsed) = parse_agent_frontmatter(&text)
                && let Some(name) = parsed.schema.and_then(|s| s.name)
            {
                names.push(name);
            }
        }
    }
    Some(names)
}

fn validate_reusable_prompt(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
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

    let Some(parsed) = &parsed else {
        return diagnostics;
    };

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

            if let Some((start, end)) = frontmatter_key_removal(content, &unknown.key) {
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

    // COP-015: "`ask`, `agent`, `plan`, or the name of a custom agent".
    if config.is_rule_enabled("COP-015")
        && let Some(schema) = &parsed.schema
        && let Some(agent) = &schema.agent
    {
        let agent = agent.trim();
        if !agent.is_empty()
            && !BUILT_IN_AGENTS.contains(&agent)
            && let Some(names) = repository_agent_names(path, config)
            && !names.iter().any(|name| name == agent)
        {
            let line = frontmatter_key_line(&parsed.raw, parsed.start_line, "agent:");
            let close = names
                .iter()
                .map(String::as_str)
                .chain(BUILT_IN_AGENTS.iter().copied())
                .find(|name| name.eq_ignore_ascii_case(agent));
            let suggestion = match close {
                Some(close) => t!("rules.cop_015.case_suggestion", name = close),
                None => t!("rules.cop_015.suggestion"),
            };
            diagnostics.push(
                Diagnostic::info(
                    path.to_path_buf(),
                    line,
                    0,
                    "COP-015",
                    t!("rules.cop_015.message", agent = agent),
                )
                .with_suggestion(suggestion),
            );
        }
    }

    diagnostics
}

// ---------------------------------------------------------------------------
// copilot-setup-steps workflow
// ---------------------------------------------------------------------------

/// Line numbers of the `copilot-setup-steps` job and of its direct keys.
fn setup_job_lines(content: &str) -> (Option<usize>, Vec<(String, usize)>) {
    let mut job_line = None;
    let mut job_indent = 0;
    let mut child_indent = None;
    let mut keys = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.len() - trimmed.len();
        if job_line.is_none() {
            if indent > 0
                && (trimmed.starts_with("copilot-setup-steps:")
                    || trimmed.starts_with("\"copilot-setup-steps\":")
                    || trimmed.starts_with("'copilot-setup-steps':"))
            {
                job_line = Some(idx + 1);
                job_indent = indent;
            }
            continue;
        }
        if indent <= job_indent {
            break;
        }
        let child = *child_indent.get_or_insert(indent);
        if indent == child
            && let Some((key, _)) = trimmed.split_once(':')
        {
            keys.push((key.trim().trim_matches(['"', '\'']).to_string(), idx + 1));
        }
    }
    (job_line, keys)
}

fn validate_setup_steps(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let cop_018 = config.is_rule_enabled("COP-018");
    let cop_029 = config.is_rule_enabled("COP-029");
    if !cop_018 && !cop_029 {
        return diagnostics;
    }

    let workflow = match parse_setup_steps_yaml(content) {
        Ok(workflow) => workflow,
        Err(err) => {
            if cop_018 {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "COP-018",
                        format!("Invalid copilot-setup-steps workflow YAML: {err}"),
                    )
                    .with_suggestion("Fix YAML syntax in copilot-setup-steps workflow."),
                );
            }
            return diagnostics;
        }
    };

    // "The job MUST be called `copilot-setup-steps` or it will not be picked
    // up by Copilot."
    let Some(job) = setup_steps_job(&workflow) else {
        if cop_018 {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "COP-018",
                    t!("rules.cop_018.missing_job"),
                )
                .with_suggestion(t!("rules.cop_018.missing_job_suggestion")),
            );
        }
        return diagnostics;
    };
    let (job_line, key_lines) = setup_job_lines(content);
    let job_line = job_line.unwrap_or(1);
    let line_of = |key: &str| {
        key_lines
            .iter()
            .find(|(k, _)| k == key)
            .map_or(job_line, |(_, line)| *line)
    };

    if cop_018
        && let Some(runs_on) = job.get(serde_yaml::Value::String("runs-on".into()))
        && let Some(label) = runs_on_labels(runs_on)
            .into_iter()
            .find(|label| is_macos_runner_label(label))
    {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line_of("runs-on"),
                0,
                "COP-018",
                t!("rules.cop_018.macos_runner", label = label),
            )
            .with_suggestion(t!("rules.cop_018.macos_runner_suggestion")),
        );
    }

    if cop_029 {
        for key in job.keys().filter_map(serde_yaml::Value::as_str) {
            // A display `name` is cosmetic; every other setting changes how
            // the job runs, so silently dropping it matters.
            if SETUP_STEPS_JOB_KEYS.contains(&key) || key == "name" {
                continue;
            }
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line_of(key),
                    0,
                    "COP-029",
                    t!("rules.cop_029.ignored_key", key = key),
                )
                .with_suggestion(t!("rules.cop_029.ignored_key_suggestion")),
            );
        }
        if let Some(minutes) = job
            .get(serde_yaml::Value::String("timeout-minutes".into()))
            .and_then(serde_yaml::Value::as_f64)
            && minutes > SETUP_STEPS_MAX_TIMEOUT_MINUTES
        {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line_of("timeout-minutes"),
                    0,
                    "COP-029",
                    t!("rules.cop_029.timeout", minutes = minutes.to_string()),
                )
                .with_suggestion(t!("rules.cop_029.timeout_suggestion")),
            );
        }
    }

    diagnostics
}

// ---------------------------------------------------------------------------
// Plugin manifests
// ---------------------------------------------------------------------------

/// Agent Plugins 1.0 name rule: 1-64 characters of lowercase ASCII letters,
/// digits, hyphens and periods, alphanumeric at both ends, no `--` or `..`.
fn is_agent_plugins_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '.'))
        && bytes[0].is_ascii_alphanumeric()
        && bytes[bytes.len() - 1].is_ascii_alphanumeric()
        && !name.contains("--")
        && !name.contains("..")
}

/// Legacy name rule: "Kebab-case plugin name (letters, numbers, and hyphens
/// only). Maximum 64 characters."
fn is_legacy_plugin_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn validate_plugin_manifest(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let mut cx = JsonCx::new(path, content, config);
    let document = match parse_json_document(content, false) {
        Ok(document) => document,
        Err(error) => {
            if cx.enabled("COP-019") {
                cx.push_at(
                    DiagnosticLevel::Error,
                    "COP-019",
                    (error.line, error.column),
                    t!("rules.cop_019.parse_error", error = error.message.as_str()),
                    t!("rules.cop_019.parse_error_suggestion"),
                );
            }
            return cx.diagnostics;
        }
    };
    let root = &document.root;
    let Some(members) = root.value.as_object() else {
        if cx.enabled("COP-019") {
            cx.push(
                DiagnosticLevel::Error,
                "COP-019",
                node_span(root),
                t!("rules.cop_019.not_object"),
                t!("rules.cop_019.parse_error_suggestion"),
            );
        }
        return cx.diagnostics;
    };

    let schema_value = root.value.get("$schema");
    let schema_url = schema_value.and_then(|m| m.value.value.as_str());
    let agent_plugins = schema_url.is_some_and(|url| AGENT_PLUGINS_SCHEMAS.contains(&url));

    // COP-019: required fields. A `$schema` that names an Agent Plugins
    // version Copilot CLI does not support makes it reject the plugin.
    if cx.enabled("COP-019") {
        if let (Some(member), Some(url)) = (schema_value, schema_url)
            && !agent_plugins
            && url.contains("agent-plugins.org")
        {
            cx.push(
                DiagnosticLevel::Error,
                "COP-019",
                node_span(&member.value),
                t!("rules.cop_019.unsupported_schema", url = url),
                t!("rules.cop_019.unsupported_schema_suggestion"),
            );
        }
        if root.value.get("name").is_none() {
            cx.push(
                DiagnosticLevel::Error,
                "COP-019",
                members.first().map_or(node_span(root), key_span),
                t!("rules.cop_019.missing_name"),
                t!("rules.cop_019.missing_name_suggestion"),
            );
        }
    }

    // COP-020: field types.
    if cx.enabled("COP-020") {
        for member in members {
            let value = &member.value.value;
            let strings = |node: &JsonValue| {
                node.as_array()
                    .is_some_and(|items| items.iter().all(|i| i.value.as_str().is_some()))
            };
            let expected = match member.key.as_str() {
                "$schema" | "name" | "version" | "description" | "homepage" | "repository"
                | "license" | "category" => (!value.as_str().is_some()).then_some("string"),
                "keywords" | "tags" => (!strings(value)).then_some("array of strings"),
                "author" => (!value.as_object().is_some()).then_some("object"),
                "agents" | "skills" | "commands" if !agent_plugins => (!(value.as_str().is_some()
                    || strings(value)))
                .then_some("string or array of strings"),
                "hooks" | "mcpServers" | "lspServers" if !agent_plugins => {
                    (!(value.as_str().is_some() || value.as_object().is_some()))
                        .then_some("string or object")
                }
                "extensions" if agent_plugins => (!value.as_object().is_some()).then_some("object"),
                "extensions" => {
                    (!(value.as_str().is_some() || strings(value) || value.as_object().is_some()))
                        .then_some("string, array of strings or object")
                }
                _ => None,
            };
            if let Some(expected) = expected {
                cx.push(
                    DiagnosticLevel::Warning,
                    "COP-020",
                    node_span(&member.value),
                    t!(
                        "rules.cop_020.message",
                        field = member.key.as_str(),
                        expected = expected,
                        got = value.type_name()
                    ),
                    t!(
                        "rules.cop_020.suggestion",
                        field = member.key.as_str(),
                        expected = expected
                    ),
                );
            }
        }
    }

    // COP-039: name and description constraints, and the closed Agent
    // Plugins schema ("Unknown top-level fields are reported and ignored").
    if cx.enabled("COP-039") {
        if let Some(member) = root.value.get("name")
            && let Some(name) = member.value.value.as_str()
        {
            let valid = if agent_plugins {
                is_agent_plugins_name(name)
            } else {
                is_legacy_plugin_name(name)
            };
            if !valid {
                let message = if agent_plugins {
                    t!("rules.cop_039.agent_plugins_name", name = name)
                } else {
                    t!("rules.cop_039.legacy_name", name = name)
                };
                cx.push(
                    DiagnosticLevel::Warning,
                    "COP-039",
                    node_span(&member.value),
                    message,
                    t!("rules.cop_039.name_suggestion"),
                );
            }
        }
        if let Some(member) = root.value.get("description")
            && let Some(description) = member.value.value.as_str()
            && description.chars().count() > PLUGIN_DESCRIPTION_LIMIT
        {
            cx.push(
                DiagnosticLevel::Warning,
                "COP-039",
                node_span(&member.value),
                t!(
                    "rules.cop_039.description",
                    len = description.chars().count().to_string()
                ),
                t!("rules.cop_039.description_suggestion"),
            );
        }
        if agent_plugins {
            for member in members {
                if AGENT_PLUGINS_FIELDS.contains(&member.key.as_str()) {
                    continue;
                }
                let message = if LEGACY_COMPONENT_FIELDS.contains(&member.key.as_str()) {
                    t!("rules.cop_039.component_field", field = member.key.as_str())
                } else {
                    t!("rules.cop_039.unknown_field", field = member.key.as_str())
                };
                cx.push(
                    DiagnosticLevel::Info,
                    "COP-039",
                    key_span(member),
                    message,
                    t!("rules.cop_039.unknown_field_suggestion"),
                );
            }
        }
    }

    cx.diagnostics
}

// ---------------------------------------------------------------------------
// Skills (CP-SK-002)
// ---------------------------------------------------------------------------

/// SKILL.md files Copilot loads: `.github/skills/` and `~/.copilot/skills/`
/// always; `.agents/skills/` and `.claude/skills/`, which Copilot also reads,
/// when the run targets Copilot.
fn is_copilot_skill(path: &Path, config: &LintConfig) -> bool {
    path_has_components(path, ".github", "skills")
        || path_has_components(path, ".copilot", "skills")
        || (copilot_selected(config)
            && (path_has_components(path, ".agents", "skills")
                || path_has_components(path, ".claude", "skills")))
}

fn validate_copilot_skill(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if !config.is_rule_enabled("CP-SK-002") || !is_copilot_skill(path, config) {
        return diagnostics;
    }
    let parts = split_frontmatter(content);
    if !parts.has_frontmatter || !parts.has_closing {
        return diagnostics;
    }
    let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&parts.frontmatter) else {
        return diagnostics;
    };
    let Some(allowed) = yaml.get("allowed-tools") else {
        return diagnostics;
    };
    let tokens: Vec<String> = match allowed {
        serde_yaml::Value::String(text) => text
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect(),
        serde_yaml::Value::Sequence(items) => items
            .iter()
            .filter_map(serde_yaml::Value::as_str)
            .map(|t| t.trim().to_string())
            .collect(),
        _ => Vec::new(),
    };
    let Some(tool) = tokens
        .iter()
        .find(|t| t.eq_ignore_ascii_case("shell") || t.eq_ignore_ascii_case("bash"))
    else {
        return diagnostics;
    };
    let line = yaml_top_level_entries(&parts.frontmatter)
        .iter()
        .find(|entry| entry.key == "allowed-tools")
        .map_or(1, |entry| {
            line_of_offset(content, parts.frontmatter_start + entry.start)
        });
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            0,
            "CP-SK-002",
            t!("rules.cp_sk_002.message", tool = tool.as_str()),
        )
        .with_suggestion(t!("rules.cp_sk_002.suggestion")),
    );
    diagnostics
}

// ---------------------------------------------------------------------------
// MCP files (COP-035)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum McpFile {
    /// `.vscode/mcp.json`: VS Code format, top-level `servers`.
    VsCode,
    /// Workspace-root `.mcp.json`: portable format, no SSE, `cwd`, env files
    /// or input variables.
    WorkspacePortable,
    /// `~/.copilot/mcp-config.json`: portable format, no env files or input
    /// variables.
    CopilotUser,
}

fn mcp_file_kind(path: &Path, config: &LintConfig) -> Option<McpFile> {
    let filename = path.file_name()?.to_str()?;
    let parent = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("");
    match filename {
        "mcp.json" if parent.eq_ignore_ascii_case(".vscode") => Some(McpFile::VsCode),
        "mcp-config.json" if parent.eq_ignore_ascii_case(".copilot") => Some(McpFile::CopilotUser),
        // `.mcp.json` is Claude Code's project MCP file as well, so the
        // portable-format limits apply only when the run targets Copilot.
        ".mcp.json" if copilot_selected(config) => Some(McpFile::WorkspacePortable),
        _ => None,
    }
}

fn validate_mcp_file(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let mut cx = JsonCx::new(path, content, config);
    if !cx.enabled("COP-035") {
        return cx.diagnostics;
    }
    let Some(kind) = mcp_file_kind(path, config) else {
        return cx.diagnostics;
    };
    let Ok(document) = parse_json_document(content, kind == McpFile::VsCode) else {
        return cx.diagnostics;
    };
    let root = &document.root;
    let servers = root.value.get("servers");
    let mcp_servers = root.value.get("mcpServers");

    match kind {
        McpFile::VsCode => {
            if let (Some(member), None) = (mcp_servers, servers) {
                cx.push(
                    DiagnosticLevel::Warning,
                    "COP-035",
                    key_span(member),
                    t!("rules.cop_035.vscode_key"),
                    t!("rules.cop_035.vscode_key_suggestion"),
                );
            }
            return cx.diagnostics;
        }
        McpFile::WorkspacePortable | McpFile::CopilotUser => {
            if let (Some(member), None) = (servers, mcp_servers) {
                cx.push(
                    DiagnosticLevel::Warning,
                    "COP-035",
                    key_span(member),
                    t!("rules.cop_035.portable_key"),
                    t!("rules.cop_035.portable_key_suggestion"),
                );
            }
        }
    }

    let Some(entries) = mcp_servers.and_then(|m| m.value.value.as_object()) else {
        return cx.diagnostics;
    };
    for server in entries {
        let name = server.key.as_str();
        let Some(fields) = server.value.value.as_object() else {
            continue;
        };
        for field in fields {
            let unsupported = match field.key.as_str() {
                "envFile" => true,
                "cwd" => kind == McpFile::WorkspacePortable,
                "type" => {
                    kind == McpFile::WorkspacePortable && field.value.value.as_str() == Some("sse")
                }
                _ => false,
            };
            if unsupported {
                cx.push(
                    DiagnosticLevel::Warning,
                    "COP-035",
                    key_span(field),
                    t!(
                        "rules.cop_035.unsupported_field",
                        server = name,
                        field = field.key.as_str()
                    ),
                    t!("rules.cop_035.unsupported_field_suggestion"),
                );
            }
        }
        if let Some(offset) = find_input_variable(content, &server.value) {
            cx.push(
                DiagnosticLevel::Warning,
                "COP-035",
                (offset, offset + "${input:".len()),
                t!("rules.cop_035.input_variable", server = name),
                t!("rules.cop_035.input_variable_suggestion"),
            );
        }
    }
    cx.diagnostics
}

/// Offset of the first `${input:` reference inside a server entry.
fn find_input_variable(content: &str, server: &JsonNode) -> Option<usize> {
    content
        .get(server.start..server.end)?
        .find("${input:")
        .map(|offset| server.start + offset)
}

// ---------------------------------------------------------------------------
// Instruction files: @ references (COP-030) and external links (COP-038)
// ---------------------------------------------------------------------------

/// Which Copilot CLI rule applies to the `@` references of a file.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RefScope {
    /// `.github/copilot-instructions.md`, AGENTS.md, CLAUDE.md: references
    /// load when they stay inside the repository.
    Loaded,
    /// GEMINI.md and `*.instructions.md`: "File references are not expanded".
    NotExpanded,
}

/// The directory a path resolves to after lexical `..` handling, or `None`
/// when it climbs above the starting directory's root.
fn resolve_lexically(base: &Path, relative: &str) -> Option<PathBuf> {
    let mut resolved = base.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::ParentDir => {
                if !resolved.pop() {
                    return None;
                }
            }
            Component::Normal(part) => resolved.push(part),
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(resolved)
}

fn validate_file_references(
    path: &Path,
    content: &str,
    scope: RefScope,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !config.is_rule_enabled("COP-030") {
        return;
    }
    let base = path.parent().unwrap_or(Path::new(""));
    for import in crate::parsers::markdown::extract_imports(content) {
        let reference = import.path.as_str();
        let message = match scope {
            RefScope::NotExpanded => t!("rules.cop_030.not_expanded", reference = reference),
            RefScope::Loaded => {
                let bytes = reference.as_bytes();
                let absolute = reference.starts_with('/')
                    || reference.starts_with('\\')
                    || (bytes.len() > 2
                        && bytes[0].is_ascii_alphabetic()
                        && bytes[1] == b':'
                        && matches!(bytes[2], b'\\' | b'/'));
                if reference.starts_with('~') || absolute {
                    t!("rules.cop_030.not_loaded", reference = reference)
                } else if let Some(root) = config.root_dir()
                    && base.starts_with(root)
                    && resolve_lexically(base, reference)
                        .is_none_or(|target| !target.starts_with(root))
                {
                    t!("rules.cop_030.outside_repo", reference = reference)
                } else {
                    continue;
                }
            }
        };
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                import.line,
                import.column,
                "COP-030",
                message,
            )
            .with_suggestion(t!("rules.cop_030.suggestion")),
        );
    }
}

/// A sentence that tells Copilot to follow guidance at a URL.
fn external_link_directive_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)\b(?:follow(?:ing)?|according to|refer to|adhere to|comply with|conform to|as (?:described|defined|documented|outlined) (?:in|at|on)|(?:standards|guidelines|conventions|rules|style guide) (?:at|in|on|from))\b[^\n]*?https?://|https?://[^\s)>\]]+[^\n]*?\b(?:for (?:the |our )?(?:standards|guidelines|conventions|rules))\b",
        )
        .expect("external link directive pattern")
    })
}

/// COP-038: code review instructions that point at an external link
/// ("**Follow external links**" is listed under "Unsupported instruction
/// types").
fn validate_external_links(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !config.is_rule_enabled("COP-038") {
        return;
    }
    let mut in_fence = false;
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || !line.contains("://") {
            continue;
        }
        if let Some(found) = external_link_directive_regex().find(line) {
            diagnostics.push(
                Diagnostic::info(
                    path.to_path_buf(),
                    idx + 1,
                    found.start(),
                    "COP-038",
                    t!("rules.cop_038.message"),
                )
                .with_suggestion(t!("rules.cop_038.suggestion")),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Validator
// ---------------------------------------------------------------------------

impl Validator for CopilotValidator {
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

        let file_type = crate::detect_file_type(path);
        match file_type {
            FileType::CopilotAgent => return validate_custom_agent(path, content, config),
            FileType::CopilotPrompt => return validate_reusable_prompt(path, content, config),
            FileType::CopilotHooks if is_setup_steps_workflow(path) => {
                return validate_setup_steps(path, content, config);
            }
            FileType::CopilotHooks if is_copilot_settings_file(path) => {
                return super::copilot_hooks::validate_settings_file(path, content, config);
            }
            FileType::CopilotHooks => {
                return super::copilot_hooks::validate_hook_file(path, content, config);
            }
            FileType::CopilotPlugin => return validate_plugin_manifest(path, content, config),
            FileType::Skill => return validate_copilot_skill(path, content, config),
            FileType::Mcp => return validate_mcp_file(path, content, config),
            FileType::ClaudeMd | FileType::GeminiMd => {
                // Claude Code and Gemini CLI expand these files' imports
                // themselves; Copilot's limits apply only when the run
                // targets Copilot.
                if copilot_selected(config) {
                    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let scope = match filename {
                        "AGENTS.md" | "CLAUDE.md" => Some(RefScope::Loaded),
                        "GEMINI.md" => Some(RefScope::NotExpanded),
                        _ => None,
                    };
                    if let Some(scope) = scope {
                        validate_file_references(path, content, scope, config, &mut diagnostics);
                    }
                }
                return diagnostics;
            }
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

        // COP-006: "Limit any single instruction file to a maximum of about
        // 1,000 lines." Applies to repository-wide and path-specific files.
        if config.is_rule_enabled("COP-006") {
            let lines = content.lines().count();
            if lines > INSTRUCTION_LINE_LIMIT {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        INSTRUCTION_LINE_LIMIT + 1,
                        0,
                        "COP-006",
                        t!(
                            "rules.cop_006.message",
                            lines = lines,
                            limit = INSTRUCTION_LINE_LIMIT
                        ),
                    )
                    .with_suggestion(t!("rules.cop_006.suggestion")),
                );
            }
        }

        // COP-030: Copilot CLI expands `@path` references in
        // copilot-instructions.md, but not in *.instructions.md.
        let ref_scope = if is_scoped {
            RefScope::NotExpanded
        } else {
            RefScope::Loaded
        };
        validate_file_references(path, content, ref_scope, config, &mut diagnostics);

        // COP-038 unless the file opts out of code review.
        let excludes_code_review = scoped_frontmatter
            .as_ref()
            .and_then(|parsed| parsed.schema.as_ref())
            .and_then(|schema| schema.exclude_agent.as_deref())
            == Some("code-review");
        if !excludes_code_review {
            validate_external_links(path, content, config, &mut diagnostics);
        }

        // Rules COP-002, COP-003, COP-004, COP-005 only apply to scoped instruction files
        if !is_scoped {
            return diagnostics;
        }

        // Parse frontmatter for scoped files
        let parsed = match scoped_frontmatter {
            Some(p) => p,
            None => {
                // COP-002: without frontmatter the file has no `applyTo`.
                // GitHub.com and Copilot CLI never apply it; VS Code
                // attaches it only on request.
                if config.is_rule_enabled("COP-002") && !is_content_empty(content) {
                    let mut diagnostic = Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "COP-002",
                        t!("rules.cop_002.message_missing"),
                    )
                    .with_suggestion(t!("rules.cop_002.suggestion_add_frontmatter"));

                    // Unsafe auto-fix: insert frontmatter that applies the
                    // file to every file, which changes when it is used.
                    diagnostic = diagnostic.with_fix(Fix::insert(
                        0,
                        "---\napplyTo: \"**\"\n---\n",
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

            // Missing applyTo: GitHub.com ("create a frontmatter block
            // containing the `applyTo` keyword") never applies the file;
            // VS Code documents `applyTo` as optional and loads a file with a
            // `description` on demand.
            if let Some(ref schema) = parsed.schema {
                if schema.apply_to.is_none() {
                    let has_description = schema
                        .description
                        .as_deref()
                        .is_some_and(|d| !d.trim().is_empty());
                    let diagnostic = if has_description {
                        Diagnostic::info(
                            path.to_path_buf(),
                            parsed.start_line,
                            0,
                            "COP-002",
                            t!("rules.cop_002.message_on_demand"),
                        )
                    } else {
                        Diagnostic::warning(
                            path.to_path_buf(),
                            parsed.start_line,
                            0,
                            "COP-002",
                            t!("rules.cop_002.message_missing_apply_to"),
                        )
                    };
                    diagnostics.push(
                        diagnostic.with_suggestion(t!("rules.cop_002.suggestion_add_apply_to")),
                    );
                }
            }
        }

        // COP-003: Invalid glob pattern
        if config.is_rule_enabled("COP-003") {
            if let Some(ref schema) = parsed.schema {
                if let Some(ref apply_to) = schema.apply_to {
                    // `applyTo` takes multiple patterns in one string: "You can
                    // specify multiple patterns by separating them with commas",
                    // with `applyTo: "**/*.ts,**/*.tsx"` as the doc's own
                    // example. Passing the whole value to `glob::Pattern` failed
                    // on every such config. Each pattern is validated on its own;
                    // commas inside a brace or bracket group are not separators,
                    // which is what `split_comma_separated_globs` handles.
                    for pattern in split_comma_separated_globs(apply_to) {
                        let validation = validate_glob_pattern(pattern);
                        if !validation.valid {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    frontmatter_key_line(
                                        &parsed.raw,
                                        parsed.start_line,
                                        "applyTo:",
                                    ),
                                    0,
                                    "COP-003",
                                    t!(
                                        "rules.cop_003.message",
                                        pattern = pattern,
                                        error = validation.error.unwrap_or_default()
                                    ),
                                )
                                .with_suggestion(t!("rules.cop_003.suggestion")),
                            );
                        }
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

                // Safe auto-fix: Copilot ignores the key, and the whole entry
                // is removed exactly (verified by re-parsing).
                if let Some((start, end)) = frontmatter_key_removal(content, &unknown.key) {
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
                    // The documented values are `code-review` and `cloud-agent`
                    // ("Use either `\"code-review\"` or `\"cloud-agent\"`").
                    // Two lists, deliberately: `find_closest_value` also does
                    // substring matching, so a single list containing the legacy
                    // spelling let the autofix write `coding-agent` for an input
                    // like `coding` - reintroducing the exact bug this rule was
                    // fixed for, through a different input. Fix candidates must
                    // contain only documented values.
                    const DOCUMENTED_AGENTS: &[&str] = &["code-review", "cloud-agent"];
                    // `coding-agent` was never valid upstream. Accepted without a
                    // fix candidate so configs written against agnix's own
                    // previous wrong advice keep validating.
                    const ACCEPTED_AGENTS: &[&str] =
                        &["code-review", "cloud-agent", "coding-agent"];
                    // Accepted, but say so: a user who followed agnix's old
                    // advice otherwise ships a file Copilot will not honor and
                    // never hears about it.
                    if agent_value.as_str() == "coding-agent" {
                        diagnostics.push(
                            Diagnostic::info(
                                path.to_path_buf(),
                                parsed.start_line + 1,
                                0,
                                "COP-005",
                                t!("rules.cop_005.legacy_coding_agent"),
                            )
                            .with_suggestion(t!("rules.cop_005.legacy_suggestion")),
                        );
                    }

                    if !ACCEPTED_AGENTS.contains(&agent_value.as_str()) {
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
                            super::find_closest_value(agent_value.as_str(), DOCUMENTED_AGENTS)
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
        // VS Code documents applyTo as optional (the file is attached
        // manually), so a missing applyTo is a warning, not an error.
        assert_eq!(cop_002[0].level, DiagnosticLevel::Warning);
        assert!(cop_002[0].message.contains("no frontmatter"));
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
        assert_eq!(cop_002[0].level, DiagnosticLevel::Warning);
        assert!(cop_002[0].message.contains("no 'applyTo'"));
    }

    // "If you omit both `description` and `applyTo`, attach the file manually":
    // with a description VS Code loads the file on demand, so the missing
    // applyTo is only a note about GitHub.com and Copilot CLI.
    #[test]
    fn test_cop_002_description_without_apply_to_is_a_note() {
        let content = "---\ndescription: React component conventions\n---\n# React\n";
        let diagnostics = validate_scoped(content);
        let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
        assert_eq!(cop_002.len(), 1);
        assert_eq!(cop_002[0].level, DiagnosticLevel::Info);
    }

    // VS Code documents a `name` field ("Display name shown in the UI").
    #[test]
    fn test_cop_004_accepts_name() {
        let content = "---\nname: React\napplyTo: \"**/*.tsx\"\n---\n# React\n";
        let diagnostics = validate_scoped(content);
        assert!(diagnostics.iter().all(|d| d.rule != "COP-004"));
    }

    // The deletion fix removes the whole entry, continuation lines included,
    // and the result still parses.
    #[test]
    fn test_cop_004_fix_removes_multiline_entry_exactly() {
        let content = "---\napplyTo: \"**/*.ts\"\nlabels:\n  - a\n  - b\n---\n# Body\n";
        let diagnostics = validate_scoped(content);
        let cop_004 = diagnostics.iter().find(|d| d.rule == "COP-004").unwrap();
        let fix = &cop_004.fixes[0];
        assert!(fix.safe);
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        assert_eq!(fixed, "---\napplyTo: \"**/*.ts\"\n---\n# Body\n");
        assert!(validate_scoped(&fixed).iter().all(|d| d.rule != "COP-004"));
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

    /// Generate long content for COP-006 tests (1001 lines)
    fn make_long_content() -> String {
        let mut s = String::from("# Copilot Instructions\n");
        s.push_str(
            &"Follow consistent naming conventions for variables and functions.\n".repeat(1000),
        );
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

    /// `coding-agent` was never an upstream value - the documented pair is
    /// `code-review` / `cloud-agent`. It stays accepted so existing configs keep
    /// validating, but is now reported at info level rather than treated as
    /// valid, so the user learns Copilot will not honor the file.
    #[test]
    fn test_cop_005_legacy_coding_agent_is_reported_not_valid() {
        let content = r#"---
applyTo: "**/*.ts"
excludeAgent: "coding-agent"
---
# Instructions
"#;
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert_eq!(cop_005.len(), 1, "got: {diagnostics:?}");
        assert_eq!(
            cop_005[0].level,
            crate::diagnostics::DiagnosticLevel::Info,
            "the config still works; it is just not the documented spelling"
        );
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
        let diagnostics = validate_global(&long_content);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert_eq!(cop_006.len(), 1);
        assert_eq!(cop_006[0].level, DiagnosticLevel::Warning);
        assert_eq!(
            cop_006[0].line, 1001,
            "points at the first line past the limit"
        );
        assert!(cop_006[0].message.contains("1001 lines"));
    }

    // The rule used to count 4,000 characters, a figure that appears only in a
    // sample prompt of a 2025 blog post. The docs say "Limit any single
    // instruction file to a maximum of about 1,000 lines".
    #[test]
    fn test_cop_006_exact_boundary() {
        let content_1000 = "Use strict mode.\n".repeat(1000);
        let diagnostics = validate_global(&content_1000);
        assert!(
            diagnostics.iter().all(|d| d.rule != "COP-006"),
            "1000 lines should not trigger COP-006"
        );

        let content_1001 = "Use strict mode.\n".repeat(1001);
        let diagnostics = validate_global(&content_1001);
        let cop_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-006").collect();
        assert_eq!(cop_006.len(), 1, "1001 lines should trigger COP-006");

        // Long lines do not count: 40,000 characters on 10 lines pass.
        let wide = "x".repeat(4000) + "\n";
        let diagnostics = validate_global(&wide.repeat(10));
        assert!(diagnostics.iter().all(|d| d.rule != "COP-006"));
    }

    // The limit applies to "any single instruction file", path-specific ones
    // included (the rule used to check only copilot-instructions.md).
    #[test]
    fn test_cop_006_scoped_file_triggers() {
        let mut content = String::from("---\napplyTo: \"**/*.ts\"\n---\n# Instructions\n\n");
        content.push_str(&"Follow consistent naming conventions.\n".repeat(1000));
        let diagnostics = validate_scoped(&content);
        assert_eq!(
            diagnostics.iter().filter(|d| d.rule == "COP-006").count(),
            1
        );
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
name: reviewer
---
Review pull requests.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "COP-007"));
    }

    // VS Code treats every header field as optional ("All fields in this table
    // are optional"), so an agent that targets only VS Code needs no
    // description. This test used `target: vscode` to expect COP-007.
    #[test]
    fn test_cop_007_skips_vscode_only_agent() {
        let diagnostics = validate_agent(
            r#"---
target: vscode
---
Review pull requests.
"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "COP-007"));
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
argument-hint: "[file]"
handoffs:
  - label: Implement
    agent: agent
    prompt: Implement the plan.
---
Review pull requests.
"#,
        );
        let cop_012: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-012").collect();
        assert_eq!(cop_012.len(), 2);
        assert!(cop_012.iter().all(|d| d.level == DiagnosticLevel::Info));
    }

    // `model` is a documented GitHub.com property ("Model to use when this
    // custom agent executes"); the rule used to report and remove it.
    #[test]
    fn test_cop_012_does_not_report_model() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
model: gpt-4
---
Review pull requests.
"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "COP-012"));
    }

    #[test]
    fn test_cop_012_skips_vscode_target() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
target: vscode
argument-hint: "[file]"
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

    // `agent` takes "`ask`, `agent`, `plan`, or the name of a custom agent";
    // the rule used to accept none/ask/always and errored on the doc's own
    // `agent: 'agent'` example.
    #[test]
    fn test_cop_015_accepts_documented_values() {
        for value in ["ask", "agent", "plan", "'agent'", "my-reviewer"] {
            let content = format!(
                "---\ndescription: Refactor selected code\nagent: {value}\n---\nRefactor the selected code.\n"
            );
            let diagnostics = validate_prompt(&content);
            assert!(
                diagnostics.iter().all(|d| d.rule != "COP-015"),
                "{value}: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn test_cop_015_unknown_agent_in_repository_with_agents() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join(".github/agents")).unwrap();
        std::fs::create_dir_all(root.join(".github/prompts")).unwrap();
        std::fs::write(
            root.join(".github/agents/reviewer.agent.md"),
            "---\nname: Code Reviewer\ndescription: Reviews\n---\nReview.\n",
        )
        .unwrap();
        let prompt = root.join(".github/prompts/review.prompt.md");
        let validate = |agent: &str| {
            let content = format!("---\nagent: {agent}\n---\nReview the change.\n");
            CopilotValidator.validate(&prompt, &content, &LintConfig::default())
        };
        // By file ID and by `name`.
        assert!(validate("reviewer").iter().all(|d| d.rule != "COP-015"));
        assert!(
            validate("Code Reviewer")
                .iter()
                .all(|d| d.rule != "COP-015")
        );
        let diagnostics = validate("Reviewer");
        let cop_015: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-015").collect();
        assert_eq!(cop_015.len(), 1);
        assert_eq!(cop_015[0].level, DiagnosticLevel::Info);
        assert!(!cop_015[0].has_fixes());
        assert!(
            cop_015[0]
                .suggestion
                .as_deref()
                .unwrap()
                .contains("reviewer")
        );
    }

    #[test]
    fn test_cop_017_hooks_schema_validation() {
        let diagnostics = validate_hooks(
            r#"{
  "version": 1,
  "hooks": { "notReal": [{ "bash": "echo hi" }] }
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
                .any(|d| d.message.contains("not valid JSON"))
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

    // Deleting `argument-hint` or `handoffs` changes VS Code's behavior, so
    // COP-012 offers no fix (it used to delete them as a safe fix).
    #[test]
    fn test_cop_012_has_no_fix() {
        let diagnostics = validate_agent(
            r#"---
description: Review pull requests
argument-hint: "[file]"
---
Review pull requests.
"#,
        );
        let cop_012: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-012").collect();
        assert_eq!(cop_012.len(), 1);
        assert!(!cop_012[0].has_fixes());
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

    // ===== Helper functions for new rules =====

    fn validate_plugin(content: &str) -> Vec<Diagnostic> {
        let validator = CopilotValidator;
        validator.validate(
            Path::new(".github/plugin/plugin.json"),
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
  "agents": "agents/"
}"#,
        );
        let cop_019: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-019").collect();
        assert_eq!(cop_019.len(), 1);
        assert_eq!(cop_019[0].level, DiagnosticLevel::Error);
        assert!(cop_019[0].message.contains("name"));
    }

    // The legacy manifest's only required field is `name`; `description` and
    // `version` are "Optional metadata fields". The rule used to require all
    // three.
    #[test]
    fn test_cop_019_only_name_is_required() {
        let diagnostics = validate_plugin(r#"{ "name": "my-plugin" }"#);
        assert!(diagnostics.iter().all(|d| d.rule != "COP-019"));
        let diagnostics = validate_plugin(
            r#"{
  "$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
  "name": "my-dev-tools"
}"#,
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn test_cop_019_unsupported_agent_plugins_version() {
        let diagnostics = validate_plugin(
            r#"{
  "$schema": "https://agent-plugins.org/schemas/9.0.0/plugin.schema.json",
  "name": "my-dev-tools"
}"#,
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule == "COP-019" && d.line == 2)
        );
    }

    #[test]
    fn test_cop_019_invalid_json() {
        let diagnostics = validate_plugin("{ \"name\": ");
        assert!(diagnostics.iter().any(|d| d.rule == "COP-019"));
    }

    // ===== COP-020: Plugin Manifest Invalid Field Types =====

    #[test]
    fn test_cop_020_name_not_string() {
        let diagnostics = validate_plugin(
            r#"{
  "name": 123,
  "description": "A Copilot plugin",
  "version": "1.0.0",
  "agents": "agents/"
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
  "hooks": "hooks.json"
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
  "agents": "agents/",
  "skills": ["skills/", "more-skills/"],
  "hooks": { "preToolUse": [] },
  "mcpServers": ".mcp.json"
}"#,
        );
        let cop_020: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-020").collect();
        assert!(cop_020.is_empty(), "{cop_020:?}");
    }

    // Component paths are "string | string[]" in the legacy manifest; the rule
    // used to require objects for `agents` and `skills`.
    #[test]
    fn test_cop_020_component_paths_reject_objects() {
        let diagnostics = validate_plugin(r#"{ "name": "x", "agents": {} }"#);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule == "COP-020" && d.message.contains("agents"))
        );
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
        assert!(cop_027[0].message.contains("retired"));
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

    #[test]
    fn test_cop_027_migration_roundtrip_and_precedence() {
        for newline in ["\n", "\r\n"] {
            for infer in [true, false] {
                for explicit in [
                    "",
                    "disable-model-invocation: false\n",
                    "user-invocable: false\n",
                    "disable-model-invocation: true\nuser-invocable: false\n",
                ] {
                    let content = format!("---\ndescription: Réview\ninfer: {infer} # keep this note\n{explicit}---\nBody unchanged.\n").replace('\n', newline);
                    let diagnostics = validate_agent(&content);
                    let diagnostic = diagnostics.iter().find(|d| d.rule == "COP-027").unwrap();
                    assert_eq!(diagnostic.fixes.len(), 1, "{content}");
                    let fix = &diagnostic.fixes[0];
                    assert!(!fix.safe);
                    let mut fixed = content.clone();
                    fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
                    let yaml = fixed.split("---").nth(1).unwrap();
                    let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
                    assert!(value.get("infer").is_none());
                    let expected_disable = if explicit.contains("disable-model-invocation: true") {
                        true
                    } else if explicit.contains("disable-model-invocation: false") {
                        false
                    } else {
                        !infer
                    };
                    assert_eq!(
                        value["disable-model-invocation"].as_bool(),
                        Some(expected_disable)
                    );
                    assert_eq!(
                        value["user-invocable"].as_bool(),
                        Some(!explicit.contains("user-invocable: false"))
                    );
                    assert!(fixed.contains("# keep this note"));
                    assert!(fixed.ends_with(&format!("Body unchanged.{newline}")));
                    assert!(!validate_agent(&fixed).iter().any(|d| d.rule == "COP-027"));
                }
            }
        }
    }

    #[test]
    fn test_cop_027_withholds_ambiguous_or_invalid_fix() {
        for fields in [
            "infer: 'false'",
            "infer: null",
            "infer: [false]",
            "infer: &setting false\nuser-invocable: *setting",
            "infer: true\ninfer: false",
            "infer: false\nuser-invocable: null",
            "infer: false\ndisable-model-invocation: 'true'",
        ] {
            let content = format!("---\ndescription: Review\n{fields}\n---\nBody\n");
            assert!(
                validate_agent(&content)
                    .iter()
                    .filter(|d| d.rule == "COP-027")
                    .all(|d| !d.has_fixes()),
                "{content}"
            );
        }
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

    /// `excludeAgent` takes `code-review` or `cloud-agent` ("Use either
    /// `"code-review"` or `"cloud-agent"`"). agnix listed `coding-agent`, which
    /// was never an upstream value, so the doc's own value was rejected AND the
    /// unsafe autofix rewrote a correct value into the wrong one.
    #[test]
    fn test_cop_005_accepts_documented_cloud_agent() {
        for value in ["code-review", "cloud-agent"] {
            let content = format!(
                r#"---
applyTo: "**"
excludeAgent: "{value}"
---

Use strict mode.
"#
            );
            let diagnostics = validate_scoped(&content);
            let hits: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
            assert!(
                hits.is_empty(),
                "documented excludeAgent value '{value}' must be accepted, got: {hits:?}"
            );
        }
    }

    /// The autofix must never write an undocumented value. `find_closest_value`
    /// also does substring matching, so keeping the legacy spelling in the
    /// fix-candidate list let `coding` be "fixed" to `coding-agent` - the very
    /// bug this rule was corrected for, reached through a different input. The
    /// earlier tests missed it by asserting only that a diagnostic fired, never
    /// its replacement text.
    #[test]
    fn test_cop_005_autofix_never_writes_an_undocumented_value() {
        let content = r#"---
applyTo: "**"
excludeAgent: "coding"
---

Use strict mode.
"#;
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert_eq!(cop_005.len(), 1, "got: {diagnostics:?}");

        for fix in &cop_005[0].fixes {
            assert!(
                !fix.replacement.contains("coding-agent"),
                "the fix must not write the undocumented spelling, got: {:?}",
                fix.replacement
            );
        }
    }

    /// `coding-agent` is kept as a deprecated alias so configs written against
    /// agnix's own previous wrong advice keep validating - but it is reported at
    /// info level rather than accepted silently, so the user learns the file will
    /// not be honored.
    #[test]
    fn test_cop_005_tolerates_legacy_coding_agent() {
        let content = r#"---
applyTo: "**"
excludeAgent: "coding-agent"
---

Use strict mode.
"#;
        let diagnostics = validate_scoped(content);
        let cop_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-005").collect();
        assert_eq!(
            cop_005.len(),
            1,
            "the legacy spelling should be reported once, got: {diagnostics:?}"
        );
        assert_eq!(
            cop_005[0].level,
            crate::diagnostics::DiagnosticLevel::Info,
            "info, not error: the config still works, it is just not documented"
        );
    }

    /// A genuinely unknown agent is still rejected.
    #[test]
    fn test_cop_005_still_rejects_unknown_agent() {
        let content = r#"---
applyTo: "**"
excludeAgent: "bogus-agent"
---

Use strict mode.
"#;
        let diagnostics = validate_scoped(content);
        assert!(
            diagnostics.iter().any(|d| d.rule == "COP-005"),
            "an unknown value must still be flagged, got: {diagnostics:?}"
        );
    }

    /// `applyTo` takes several patterns in one string: "You can specify multiple
    /// patterns by separating them with commas", with `applyTo: "**/*.ts,**/*.tsx"`
    /// as the doc's own example. The whole string was passed to `glob::Pattern`,
    /// so every such config errored.
    #[test]
    fn test_cop_003_accepts_comma_separated_apply_to() {
        let content = r#"---
applyTo: "**/*.ts,**/*.tsx"
---

Use strict mode.
"#;
        let diagnostics = validate_scoped(content);
        let hits: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-003").collect();
        assert!(
            hits.is_empty(),
            "the doc's own comma-separated example must validate, got: {hits:?}"
        );
    }

    /// Commas inside a brace group are not separators.
    #[test]
    fn test_cop_003_brace_group_commas_are_not_separators() {
        let content = r#"---
applyTo: "src/**/*.{ts,tsx},lib/**/*.js"
---

Use strict mode.
"#;
        let diagnostics = validate_scoped(content);
        let hits: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-003").collect();
        assert!(
            hits.is_empty(),
            "brace-group commas must not split the pattern, got: {hits:?}"
        );
    }

    /// An invalid segment inside a comma list is still reported.
    #[test]
    fn test_cop_003_still_flags_invalid_segment() {
        let content = r#"---
applyTo: "**/*.ts,[unclosed"
---

Use strict mode.
"#;
        let diagnostics = validate_scoped(content);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule == "COP-003" && d.message.contains("[unclosed")),
            "the bad segment must be named, got: {diagnostics:?}"
        );
    }
}

/// Rules and corrections from the 2026-10-17 vendor-docs survey.
#[cfg(test)]
mod survey_tests {
    use super::*;
    use crate::config::LintConfig;

    fn validate_at(path: &str, content: &str) -> Vec<Diagnostic> {
        CopilotValidator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn validate_with_tools(path: &str, content: &str, tools: &[&str]) -> Vec<Diagnostic> {
        let mut config = LintConfig::default();
        config.set_tools(tools.iter().map(|t| t.to_string()).collect());
        CopilotValidator.validate(Path::new(path), content, &config)
    }

    fn rule<'a>(diagnostics: &'a [Diagnostic], id: &str) -> Vec<&'a Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == id).collect()
    }

    const AGENT: &str = ".github/agents/reviewer.agent.md";

    // VS Code documents `agents`, `reasoning-effort` and `hooks` (Preview),
    // and `model` as a string or an array; COP-008 reported them as unknown
    // and deleted them with a safe fix.
    #[test]
    fn cop_008_accepts_vs_code_header_fields() {
        let content = "---\ndescription: Coordinates work\nagents: ['Researcher', 'Implementer']\nreasoning-effort: xhigh\nmodel: ['model-a', 'model-b']\nhooks:\n  PostToolUse:\n    - type: command\n      command: ./scripts/format.sh\n---\nCoordinate.\n";
        let diagnostics = validate_at(AGENT, content);
        assert!(rule(&diagnostics, "COP-008").is_empty(), "{diagnostics:#?}");
    }

    #[test]
    fn cop_008_checks_new_field_values() {
        let content = "---\ndescription: d\nreasoning-effort: extreme\nagents: Researcher\nhooks: [x]\nmodel: 5\n---\nBody\n";
        let diagnostics = validate_at(AGENT, content);
        let cop_008 = rule(&diagnostics, "COP-008");
        assert_eq!(cop_008.len(), 4, "{cop_008:#?}");
        assert!(cop_008.iter().any(|d| d.line == 3));
    }

    #[test]
    fn cop_018_flags_only_macos_runners() {
        let path = ".github/workflows/copilot-setup-steps.yml";
        for runs_on in [
            "ubuntu-latest",
            "windows-latest",
            "ubuntu-4-core",
            "arc-scale-set-name",
            "[self-hosted, linux]",
        ] {
            let content = format!("jobs:\n  copilot-setup-steps:\n    runs-on: {runs_on}\n");
            let diagnostics = validate_at(path, &content);
            assert!(
                rule(&diagnostics, "COP-018").is_empty(),
                "{runs_on}: {diagnostics:#?}"
            );
        }
        let content = "jobs:\n  copilot-setup-steps:\n    runs-on: macos-latest\n    steps:\n      - run: echo\n";
        let diagnostics = validate_at(path, content);
        let cop_018 = rule(&diagnostics, "COP-018");
        assert_eq!(cop_018.len(), 1);
        assert_eq!(cop_018[0].line, 3);
        assert_eq!(cop_018[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn cop_029_ignored_keys_and_timeout() {
        let path = ".github/workflows/copilot-setup-steps.yml";
        let content = "on: workflow_dispatch\njobs:\n  copilot-setup-steps:\n    name: Setup\n    runs-on: ubuntu-latest\n    timeout-minutes: 90\n    env:\n      NODE_ENV: test\n    permissions:\n      contents: read\n    steps:\n      - run: npm ci\n";
        let diagnostics = validate_at(path, content);
        let cop_029 = rule(&diagnostics, "COP-029");
        assert_eq!(cop_029.len(), 2, "{cop_029:#?}");
        assert!(
            cop_029
                .iter()
                .any(|d| d.line == 7 && d.message.contains("'env'"))
        );
        assert!(
            cop_029
                .iter()
                .any(|d| d.line == 6 && d.message.contains("90"))
        );
    }

    #[test]
    fn cop_028_agent_mcp_secret_prefix() {
        let content = "---\ndescription: d\nmcp-servers:\n  custom-mcp:\n    type: local\n    command: some-command\n    tools: [\"tool-1\"]\n    env:\n      OK: ${{ secrets.COPILOT_MCP_TOKEN }}\n      BAD: ${{ secrets.API_TOKEN }}\n      SHELL: $GITHUB_TOKEN\n      DEFAULT: ${COPILOT_MCP_X:-fallback}\n---\nBody\n";
        let diagnostics = validate_at(AGENT, content);
        let cop_028 = rule(&diagnostics, "COP-028");
        assert_eq!(cop_028.len(), 2, "{cop_028:#?}");
        let error = cop_028
            .iter()
            .find(|d| d.level == DiagnosticLevel::Error)
            .unwrap();
        assert_eq!(error.line, 10);
        let warning = cop_028
            .iter()
            .find(|d| d.level == DiagnosticLevel::Warning)
            .unwrap();
        assert_eq!(warning.line, 11);
        // `mcp-servers` is not used in VS Code, so VS Code-only agents skip it.
        let vscode = content.replace("description: d", "description: d\ntarget: vscode");
        assert!(rule(&validate_at(AGENT, &vscode), "COP-028").is_empty());
    }

    #[test]
    fn cop_034_mcp_server_tools() {
        let content = "---\ndescription: d\nmcp-servers:\n  all-tools:\n    command: a\n    tools: [\"*\"]\n  no-tools:\n    command: b\n  scoped:\n    command: c\n    tools: [\"read_file\"]\n---\nBody\n";
        let diagnostics = validate_at(AGENT, content);
        let cop_034 = rule(&diagnostics, "COP-034");
        assert_eq!(cop_034.len(), 2, "{cop_034:#?}");
        assert!(
            cop_034
                .iter()
                .any(|d| d.level == DiagnosticLevel::Info && d.line == 4)
        );
        assert!(
            cop_034
                .iter()
                .any(|d| d.level == DiagnosticLevel::Warning && d.line == 7)
        );
    }

    #[test]
    fn plain_md_and_nested_agents_are_copilot_agents() {
        for path in [
            ".github/agents/reviewer.md",
            ".github/agents/security/auditor.agent.md",
            "/home/u/.copilot/agents/helper.md",
            "docsite/agents/team.agent.md",
        ] {
            assert_eq!(
                crate::detect_file_type(Path::new(path)),
                FileType::CopilotAgent,
                "{path}"
            );
        }
        assert_eq!(
            crate::detect_file_type(Path::new(".github/agents/README.md")),
            FileType::Unknown
        );
        // Claude Code subagents keep their own type.
        assert_eq!(
            crate::detect_file_type(Path::new(".claude/agents/reviewer.md")),
            FileType::Agent
        );
    }

    #[test]
    fn cop_025_accepts_agents_folders() {
        let content = "---\ndescription: Review PRs\n---\nReview.\n";
        // Organization agents live in a root agents/ folder of the .github repo.
        assert!(rule(&validate_at("agents/reviewer.agent.md", content), "COP-025").is_empty());
        assert_eq!(
            rule(
                &validate_at("prompts/reviewer.agent.md", content),
                "COP-025"
            )
            .len(),
            1
        );
    }

    #[test]
    fn cop_032_chat_mode_files() {
        let path = ".github/chatmodes/planner.chatmode.md";
        assert_eq!(
            crate::detect_file_type(Path::new(path)),
            FileType::CopilotAgent
        );
        let diagnostics = validate_at(path, "---\ndescription: Plan\n---\nPlan.\n");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
        assert_eq!(diagnostics[0].rule, "COP-032");
        assert!(
            diagnostics[0]
                .suggestion
                .as_deref()
                .unwrap()
                .contains("planner.agent.md")
        );
    }

    #[test]
    fn cop_033_agent_filename_characters() {
        let content = "---\ndescription: Review\n---\nReview.\n";
        assert_eq!(
            rule(
                &validate_at(".github/agents/code reviewer.agent.md", content),
                "COP-033"
            )
            .len(),
            1
        );
        for ok in [
            "code-reviewer.agent.md",
            "Code_Reviewer.v2.agent.md",
            "reviewer.md",
        ] {
            let path = format!(".github/agents/{ok}");
            assert!(
                rule(&validate_at(&path, content), "COP-033").is_empty(),
                "{ok}"
            );
        }
    }

    #[test]
    fn cop_030_file_references() {
        let content =
            "# Repo\n\nSee @~/.claude/personal.md and @/etc/shared.md and @docs/style.md.\n";
        let diagnostics = validate_at(".github/copilot-instructions.md", content);
        let cop_030 = rule(&diagnostics, "COP-030");
        assert_eq!(cop_030.len(), 2, "{cop_030:#?}");
        assert!(cop_030.iter().all(|d| d.line == 3));

        // `@` is not expanded in *.instructions.md.
        let scoped = "---\napplyTo: \"**\"\n---\nFollow @docs/style.md.\n";
        let diagnostics = validate_at(".github/instructions/style.instructions.md", scoped);
        assert_eq!(rule(&diagnostics, "COP-030").len(), 1);

        // CLAUDE.md and GEMINI.md are checked only when the run targets Copilot.
        let claude = "Use @~/.claude/me.md\n";
        assert!(validate_at("CLAUDE.md", claude).is_empty());
        assert_eq!(
            rule(
                &validate_with_tools("CLAUDE.md", claude, &["copilot"]),
                "COP-030"
            )
            .len(),
            1
        );
        let gemini = "Import @./docs/a.md\n";
        assert!(validate_at("GEMINI.md", gemini).is_empty());
        assert_eq!(
            rule(
                &validate_with_tools("GEMINI.md", gemini, &["github-copilot"]),
                "COP-030"
            )
            .len(),
            1
        );
    }

    #[test]
    fn cop_035_mcp_formats() {
        let vscode = validate_at(
            ".vscode/mcp.json",
            r#"{ "mcpServers": { "a": { "command": "x" } } }"#,
        );
        assert_eq!(rule(&vscode, "COP-035").len(), 1);
        assert!(
            validate_at(
                ".vscode/mcp.json",
                r#"{ "servers": { "a": { "command": "x" } } }"#
            )
            .is_empty()
        );

        let user = validate_at(
            "/home/u/.copilot/mcp-config.json",
            r#"{ "mcpServers": { "a": { "command": "x", "envFile": ".env", "env": { "K": "${input:key}" } } } }"#,
        );
        assert_eq!(rule(&user, "COP-035").len(), 2, "{user:#?}");

        // `.mcp.json` is also Claude Code's file: checked only for Copilot runs.
        let portable =
            r#"{ "mcpServers": { "a": { "type": "sse", "url": "https://x", "cwd": "tools" } } }"#;
        assert!(validate_at(".mcp.json", portable).is_empty());
        assert_eq!(
            rule(
                &validate_with_tools(".mcp.json", portable, &["copilot"]),
                "COP-035"
            )
            .len(),
            2
        );
    }

    #[test]
    fn cop_038_external_links() {
        let content = "# Review\n\n- Follow the standards at https://example.com/standards\n- Docs: https://example.com\n\n```\nfollow https://example.com/code\n```\n";
        let diagnostics = validate_at(".github/copilot-instructions.md", content);
        let cop_038 = rule(&diagnostics, "COP-038");
        assert_eq!(cop_038.len(), 1, "{cop_038:#?}");
        assert_eq!(cop_038[0].line, 3);
        // Files that opt out of code review are not reported.
        let scoped = "---\napplyTo: \"**\"\nexcludeAgent: \"code-review\"\n---\nFollow the guide at https://example.com\n";
        assert!(
            rule(
                &validate_at(".github/instructions/a.instructions.md", scoped),
                "COP-038"
            )
            .is_empty()
        );
    }

    #[test]
    fn cop_039_plugin_names() {
        let path = ".github/plugin/plugin.json";
        assert_eq!(
            crate::detect_file_type(Path::new(path)),
            FileType::CopilotPlugin
        );
        assert_eq!(
            crate::detect_file_type(Path::new("my-plugin/.plugin/plugin.json")),
            FileType::CopilotPlugin
        );
        let legacy = validate_at(path, r#"{ "name": "my plugin!" }"#);
        assert_eq!(rule(&legacy, "COP-039").len(), 1);
        let agent_plugins = validate_at(
            path,
            r#"{ "$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json", "name": "My--Tools", "skills": "skills/", "category": "x" }"#,
        );
        let cop_039 = rule(&agent_plugins, "COP-039");
        assert_eq!(cop_039.len(), 3, "{cop_039:#?}");
        assert_eq!(
            cop_039
                .iter()
                .filter(|d| d.level == DiagnosticLevel::Info)
                .count(),
            2
        );
        let long = format!(
            r#"{{ "name": "x", "description": "{}" }}"#,
            "d".repeat(1025)
        );
        assert_eq!(rule(&validate_at(path, &long), "COP-039").len(), 1);
    }

    #[test]
    fn cp_sk_002_shell_pre_approval() {
        let skill = "---\nname: image-convert\ndescription: Convert images. Use when asked to convert.\nallowed-tools: shell\n---\nRun the script.\n";
        let diagnostics = validate_at(".github/skills/image-convert/SKILL.md", skill);
        let cp_sk_002 = rule(&diagnostics, "CP-SK-002");
        assert_eq!(cp_sk_002.len(), 1);
        assert_eq!(cp_sk_002[0].line, 4);
        let list = skill.replace("allowed-tools: shell", "allowed-tools:\n  - view\n  - Bash");
        assert_eq!(
            rule(
                &validate_at(".github/skills/x/SKILL.md", &list),
                "CP-SK-002"
            )
            .len(),
            1
        );
        let safe = skill.replace("allowed-tools: shell", "allowed-tools: view, edit");
        assert!(validate_at(".github/skills/x/SKILL.md", &safe).is_empty());
        // .agents/skills is shared with other tools: checked only for Copilot runs.
        assert!(validate_at(".agents/skills/x/SKILL.md", skill).is_empty());
        assert_eq!(
            rule(
                &validate_with_tools(".agents/skills/x/SKILL.md", skill, &["copilot"]),
                "CP-SK-002"
            )
            .len(),
            1
        );
    }

    #[test]
    fn frontmatter_key_removal_is_exact_or_absent() {
        let content = "---\na: 1\nb:\n  - x\n  - y\nc: 3\n---\nBody\n";
        let (start, end) = frontmatter_key_removal(content, "b").unwrap();
        let mut fixed = content.to_string();
        fixed.replace_range(start..end, "");
        assert_eq!(fixed, "---\na: 1\nc: 3\n---\nBody\n");
        // Duplicate keys are not removed; the only key leaves an empty block.
        assert!(frontmatter_key_removal("---\na: 1\na: 2\n---\nB\n", "a").is_none());
        let only = "---\na: 1\n---\nB\n";
        let (start, end) = frontmatter_key_removal(only, "a").unwrap();
        assert_eq!(&only[start..end], "a: 1\n");
        let crlf = "---\r\na: 1\r\nb: 2\r\n---\r\nBody\r\n";
        let (start, end) = frontmatter_key_removal(crlf, "b").unwrap();
        assert_eq!(&crlf[start..end], "b: 2\r\n");
    }
}
