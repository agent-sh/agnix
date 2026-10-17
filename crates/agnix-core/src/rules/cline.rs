//! Cline validation rules (CLN-001 to CLN-016, CL-SK-004)
//!
//! Validates:
//! - CLN-001: Empty Cline rule file (HIGH) - files must have content
//! - CLN-002: Invalid paths glob in clinerules (HIGH) - glob patterns must be valid
//! - CLN-003: Unknown or wrongly typed rule frontmatter key (MEDIUM) - Cline
//!   reads `paths`, `name`, `disabled` and `enabled`
//! - CLN-004: Scalar paths in clinerules (HIGH) - must be array, not scalar
//! - CLN-005: Empty Cline workflow file (HIGH) - workflow files must have content
//! - CLN-006: Unknown or wrongly typed workflow frontmatter key (MEDIUM)
//! - CLN-009: Cline hook uses unknown event name (MEDIUM) - hook filenames must match valid events
//! - CLN-010: Frontmatter Cline cannot parse (HIGH)
//! - CLN-011: `paths` entries that make Cline drop the condition (HIGH)
//! - CLN-012: Hook file the VS Code extension cannot run (MEDIUM)
//! - CLN-013: `.clineignore`, which Cline is removing (LOW)
//! - CLN-014: Literal secret in a rule or workflow (MEDIUM)
//! - CLN-015: `paths` pattern that can never match (MEDIUM)
//! - CLN-016: Remote MCP server without `type` in cline_mcp_settings.json (MEDIUM)
//! - CL-SK-004: Cline skill without instructions (MEDIUM)
//!
//! Cline reads rules from `.clinerules/` and `.cline/rules/`, workflows from
//! `.clinerules/workflows/` and `.cline/workflows/`, hooks from
//! `.clinerules/hooks/` (VS Code extension) and `.cline/hooks/` (CLI), and
//! skills from `.cline/skills/`, `.clinerules/skills/`, `.claude/skills/`
//! and `.agents/skills/`.

use crate::{
    FileType,
    config::{LintConfig, PerFileLintConfig},
    diagnostics::{Diagnostic, DiagnosticLevel, Fix},
    parsers::frontmatter::split_frontmatter,
    rules::{
        Validator, ValidatorMetadata,
        config_json::{JsonCx, key_span},
        kiro_steering::{find_secret, frontmatter_edit_yields, yaml_top_level_entries},
    },
    schemas::{
        cline::{is_content_empty, parse_frontmatter, validate_glob_pattern},
        kiro_json::parse_json_document,
    },
};
use regex::Regex;
use rust_i18n::t;
use serde_yaml::Value as YamlValue;
use std::path::Path;
use std::sync::OnceLock;

const RULE_IDS: &[&str] = &[
    "CLN-001", "CLN-002", "CLN-003", "CLN-004", "CLN-005", "CLN-006", "CLN-009", "CLN-010",
    "CLN-011", "CLN-012", "CLN-013", "CLN-014", "CLN-015", "CLN-016",
];

/// File hook names Cline runs (sdk/examples/hooks/README.md, "File Hooks vs
/// Runtime Hooks").
const VALID_HOOK_EVENTS: &[&str] = &[
    "TaskStart",
    "TaskResume",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "TaskComplete",
    "TaskError",
    "TaskCancel",
    "SessionShutdown",
];

/// Hook names in the same table that file hooks do not get: "`PreCompact` |
/// not wired for file hooks today".
const UNWIRED_HOOK_EVENTS: &[&str] = &["PreCompact"];

/// Rule frontmatter keys Cline reads: `paths` ("the supported conditional")
/// and `name`, `disabled` and `enabled` (SDK `parseRuleConfigFromMarkdown`).
const RULE_KEYS: &[&str] = &["paths", "name", "disabled", "enabled"];

/// Workflow frontmatter keys Cline reads (SDK
/// `parseWorkflowConfigFromMarkdown`).
const WORKFLOW_KEYS: &[&str] = &["name", "disabled", "enabled"];

/// Check whether two consecutive path components match a predicate, without allocating.
fn has_consecutive_components(path: &Path, predicate: impl Fn(&str, &str) -> bool) -> bool {
    let mut prev: Option<&str> = None;
    for component in path.components() {
        if let Some(s) = component.as_os_str().to_str() {
            if let Some(p) = prev {
                if predicate(p, s) {
                    return true;
                }
            }
            prev = Some(s);
        }
    }
    false
}

/// Returns true if the path is under `.clinerules/workflows/` or `.cline/workflows/`.
fn is_workflow_path(path: &Path) -> bool {
    has_consecutive_components(path, |a, b| {
        (a == ".clinerules" || a == ".cline") && b == "workflows"
    })
}

/// Returns true if the path is under `.clinerules/hooks/` or `.cline/hooks/`.
fn is_hook_path(path: &Path) -> bool {
    has_consecutive_components(path, |a, b| {
        (a == ".clinerules" || a == ".cline") && b == "hooks"
    })
}

/// The folder whose `hooks/` directory directly holds this file:
/// `.clinerules` (VS Code extension) or `.cline` (CLI). Files in subfolders
/// of `hooks/` are not hooks.
fn hook_folder(path: &Path) -> Option<&str> {
    let hooks = path.parent()?;
    if hooks.file_name()?.to_str()? != "hooks" {
        return None;
    }
    hooks
        .parent()?
        .file_name()?
        .to_str()
        .filter(|dir| *dir == ".clinerules" || *dir == ".cline")
}

/// True when the run explicitly targets Cline.
fn cline_selected(config: &LintConfig) -> bool {
    config
        .tools()
        .iter()
        .any(|tool| tool.eq_ignore_ascii_case("cline"))
}

/// Returns true for a SKILL.md Cline loads: `.cline/skills/` (recommended) and
/// `.clinerules/skills/`, plus `.agents/skills/` and `.claude/skills/` when
/// the run targets Cline (other tools read those folders too).
fn is_cline_skill_path(path: &Path, config: &LintConfig) -> bool {
    if path.file_name().and_then(|n| n.to_str()) != Some("SKILL.md") {
        return false;
    }
    has_consecutive_components(path, |a, b| {
        (a == ".cline" || a == ".clinerules") && b == "skills"
    }) || (cline_selected(config)
        && has_consecutive_components(path, |a, b| {
            (a == ".agents" || a == ".claude") && b == "skills"
        }))
}

pub struct ClineValidator;

/// Line of a top-level frontmatter key (1-based file line), or `fallback`.
/// `raw` is the frontmatter text, which starts on line 2.
fn key_line(raw: &str, key: &str, fallback: usize) -> usize {
    raw.lines()
        .position(|line| {
            !line.starts_with([' ', '\t'])
                && line
                    .strip_prefix(key)
                    .is_some_and(|rest| rest.trim_start().starts_with(':'))
        })
        .map_or(fallback, |idx| idx + 2)
}

/// Provider tokens that are credentials wherever they appear.
fn token_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"\b(?:sk-(?:proj-|ant-)?[A-Za-z0-9_-]{20,}|ghp_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{40,}|xox[abprs]-[A-Za-z0-9-]{10,}|AKIA[0-9A-Z]{16}|glpat-[A-Za-z0-9_-]{20})",
        )
        .expect("token pattern")
    })
}

/// CLN-014: "Commit `.cline/` files you want to share with your team. Keep
/// secrets out of the repo." Reports the first literal secret of the file.
fn check_secrets(
    path: &Path,
    content: &str,
    kind: &str,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !config.is_rule_enabled("CLN-014") {
        return;
    }
    let assignment = find_secret(content).map(|(offset, _)| offset);
    let token = token_regex().find(content).map(|m| m.start());
    let Some(offset) = assignment.into_iter().chain(token).min() else {
        return;
    };
    let line = content[..offset].matches('\n').count() + 1;
    let column = offset - content[..offset].rfind('\n').map_or(0, |idx| idx + 1);
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            column,
            "CLN-014",
            t!("rules.cln_014.message", kind = kind),
        )
        .with_suggestion(t!("rules.cln_014.suggestion")),
    );
}

/// Parsed frontmatter of a rule or workflow.
enum Frontmatter {
    /// No frontmatter.
    None,
    /// Frontmatter Cline cannot read (CLN-010 reported).
    Invalid,
    /// Frontmatter keys (empty for an empty or non-mapping block), the raw
    /// text and whether the body after it is empty.
    Valid {
        mapping: serde_yaml::Mapping,
        raw: String,
        end_line: usize,
        empty_body: bool,
    },
}

/// Parse frontmatter the way Cline does and report CLN-010 when it cannot
/// be read: "If frontmatter can't be parsed, Cline fails open. The rule
/// activates with raw content visible", and the SDK loader rejects the file
/// ("Failed to parse YAML frontmatter").
fn read_frontmatter(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Frontmatter {
    let Some(parsed) = parse_frontmatter(content) else {
        return Frontmatter::None;
    };
    let enabled = config.is_rule_enabled("CLN-010");
    if !parsed.closed {
        if enabled {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CLN-010",
                    t!("rules.cln_010.unclosed"),
                )
                .with_suggestion(t!("rules.cln_010.unclosed_suggestion")),
            );
        }
        return Frontmatter::Invalid;
    }
    match serde_yaml::from_str::<YamlValue>(&parsed.raw) {
        Ok(value) => Frontmatter::Valid {
            // Cline treats frontmatter that is not a mapping as empty.
            mapping: match value {
                YamlValue::Mapping(mapping) => mapping,
                _ => serde_yaml::Mapping::new(),
            },
            raw: parsed.raw,
            end_line: parsed.end_line,
            empty_body: parsed.body.trim().is_empty(),
        },
        Err(error) => {
            if enabled {
                // An unquoted glob that starts with `*` is a YAML alias, the
                // usual cause: `- *.md`.
                let alias = parsed.raw.lines().position(|line| {
                    let trimmed = line.trim_start();
                    trimmed.starts_with("- *") || trimmed.contains(": *")
                });
                let (line, suggestion) = match alias {
                    Some(idx) => (idx + 2, t!("rules.cln_010.alias_suggestion")),
                    None => (
                        error.location().map_or(1, |loc| loc.line() + 1),
                        t!("rules.cln_010.suggestion"),
                    ),
                };
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        0,
                        "CLN-010",
                        t!("rules.cln_010.message", error = error.to_string()),
                    )
                    .with_suggestion(suggestion),
                );
            }
            Frontmatter::Invalid
        }
    }
}

/// CLN-003 / CLN-006: unknown keys, and `name`, `disabled` or `enabled` of
/// the wrong type (the SDK loader throws "Frontmatter field '...' must be a
/// boolean." and drops the file).
fn check_keys(
    path: &Path,
    content: &str,
    frontmatter: (&serde_yaml::Mapping, &str),
    rule: &str,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !config.is_rule_enabled(rule) {
        return;
    }
    let (mapping, raw) = frontmatter;
    let workflow = rule == "CLN-006";
    let known = if workflow { WORKFLOW_KEYS } else { RULE_KEYS };
    for (key, value) in mapping {
        let Some(key) = key.as_str() else {
            continue;
        };
        let line = key_line(raw, key, 1);
        if !known.contains(&key) {
            let diagnostic = if workflow {
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    rule,
                    t!("rules.cln_006.message", key = key),
                )
                .with_suggestion(t!("rules.cln_006.suggestion", key = key))
            } else {
                let mut diagnostic = Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    rule,
                    t!("rules.cln_003.message", key = key),
                )
                .with_suggestion(t!("rules.cln_003.suggestion", key = key));
                // Unsafe: Cline ignores the key, but another tool reading
                // the same file may not.
                if let Some((start, end)) = super::copilot::frontmatter_key_removal(content, key) {
                    diagnostic = diagnostic.with_fix(Fix::delete(
                        start,
                        end,
                        t!("rules.cln_003.fix", key = key),
                        false,
                    ));
                }
                diagnostic
            };
            diagnostics.push(diagnostic);
            continue;
        }
        let expected = match key {
            "disabled" | "enabled" if !value.is_bool() && !value.is_null() => Some("a boolean"),
            "name" if !value.is_string() && !value.is_null() => Some("a string"),
            _ => None,
        };
        if let Some(expected) = expected {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    0,
                    rule,
                    t!("rules.cln_003.field_type", key = key, expected = expected),
                )
                .with_suggestion(t!("rules.cln_003.field_type_suggestion", key = key)),
            );
        }
    }
}

/// True for a pattern Cline can never match: "Candidate paths are expected
/// to be workspace-root-relative POSIX paths", so a leading `/` or `\`, a
/// drive letter or a `..` segment never matches.
fn is_unmatchable_pattern(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    pattern.starts_with('/')
        || pattern.starts_with('\\')
        || (bytes.len() > 1 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
        || pattern.split(['/', '\\']).any(|segment| segment == "..")
}

/// The exact CLN-004 edit: replace the `paths` entry with a one-item YAML
/// array, verified by re-parsing.
fn scalar_paths_fix(content: &str, mapping: &serde_yaml::Mapping, pattern: &str) -> Option<Fix> {
    let parts = split_frontmatter(content);
    let entries = yaml_top_level_entries(&parts.frontmatter);
    let mut matching = entries.iter().filter(|entry| entry.key == "paths");
    let entry = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let escaped = pattern.replace('\\', "\\\\").replace('"', "\\\"");
    let replacement = format!("paths:{newline}  - \"{escaped}\"");
    let start = parts.frontmatter_start + entry.start;
    let end = parts.frontmatter_start + entry.value_end;
    let mut expected = mapping.clone();
    expected.insert(
        YamlValue::String("paths".into()),
        YamlValue::Sequence(vec![YamlValue::String(pattern.to_string())]),
    );
    frontmatter_edit_yields(
        content,
        start,
        end,
        &replacement,
        &YamlValue::Mapping(expected),
    )
    .then(|| Fix::replace(start, end, replacement, t!("rules.cln_004.fix"), true))
}

impl Validator for ClineValidator {
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
            FileType::ClineIgnore => return validate_clineignore(path, config),
            FileType::ClineMcpSettings => return validate_mcp_settings(path, content, config),
            FileType::ClineRules | FileType::ClineRulesFolder => {}
            _ => return diagnostics,
        }

        // Workflow and hook files get their own specialized rules
        if file_type == FileType::ClineRulesFolder {
            if is_hook_path(path) {
                self.validate_hook(path, content, config, &mut diagnostics);
                return diagnostics;
            }
            if is_workflow_path(path) {
                self.validate_workflow(path, content, config, &mut diagnostics);
                return diagnostics;
            }
        }

        check_secrets(path, content, "rule", config, &mut diagnostics);

        let frontmatter = if file_type == FileType::ClineRulesFolder {
            read_frontmatter(path, content, config, &mut diagnostics)
        } else {
            // The single `.clinerules` file is plain text.
            Frontmatter::None
        };

        // CLN-001: "Missing instructions body in rule file." (SDK); the
        // extension skips files whose trimmed content is empty.
        if config.is_rule_enabled("CLN-001") {
            match &frontmatter {
                Frontmatter::None if is_content_empty(content) => diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CLN-001",
                        t!("rules.cln_001.message_empty"),
                    )
                    .with_suggestion(t!("rules.cln_001.suggestion_empty")),
                ),
                Frontmatter::Valid {
                    end_line,
                    empty_body: true,
                    ..
                } => {
                    let total_lines = content.lines().count().max(1);
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            (end_line + 1).min(total_lines),
                            0,
                            "CLN-001",
                            t!("rules.cln_001.message_no_content"),
                        )
                        .with_suggestion(t!("rules.cln_001.suggestion_no_content")),
                    );
                }
                _ => {}
            }
        }

        let Frontmatter::Valid { mapping, raw, .. } = frontmatter else {
            return diagnostics;
        };

        check_keys(
            path,
            content,
            (&mapping, &raw),
            "CLN-003",
            config,
            &mut diagnostics,
        );

        let Some(paths) = mapping.get(YamlValue::String("paths".into())) else {
            return diagnostics;
        };
        let paths_line = key_line(&raw, "paths", 2);
        match paths {
            // CLN-004: "Invalid type -> ignore conditional (fail-open)"
            YamlValue::String(pattern) => {
                if config.is_rule_enabled("CLN-004") {
                    let mut diagnostic = Diagnostic::error(
                        path.to_path_buf(),
                        paths_line,
                        0,
                        "CLN-004",
                        t!("rules.cln_004.message"),
                    )
                    .with_suggestion(t!("rules.cln_004.suggestion", pattern = pattern));
                    if let Some(fix) = scalar_paths_fix(content, &mapping, pattern) {
                        diagnostic = diagnostic.with_fix(fix);
                    }
                    diagnostics.push(diagnostic);
                }
                check_pattern(path, pattern, paths_line, config, &mut diagnostics);
            }
            YamlValue::Sequence(items) => {
                // CLN-011: `isNonEmptyStringArray` - one bad item drops the
                // whole condition.
                let invalid = items
                    .iter()
                    .any(|item| item.as_str().is_none_or(str::is_empty));
                if invalid && config.is_rule_enabled("CLN-011") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            paths_line,
                            0,
                            "CLN-011",
                            t!("rules.cln_011.items"),
                        )
                        .with_suggestion(t!("rules.cln_011.suggestion")),
                    );
                }
                for pattern in items.iter().filter_map(YamlValue::as_str) {
                    check_pattern(path, pattern, paths_line, config, &mut diagnostics);
                }
            }
            other => {
                if config.is_rule_enabled("CLN-011") {
                    let got = match other {
                        YamlValue::Null => "empty",
                        YamlValue::Bool(_) => "a boolean",
                        YamlValue::Number(_) => "a number",
                        YamlValue::Mapping(_) => "a mapping",
                        _ => "not an array",
                    };
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            paths_line,
                            0,
                            "CLN-011",
                            t!("rules.cln_011.not_array", got = got),
                        )
                        .with_suggestion(t!("rules.cln_011.suggestion")),
                    );
                }
            }
        }

        diagnostics
    }
}

/// CLN-002 and CLN-015 for one `paths` pattern.
fn check_pattern(
    path: &Path,
    pattern: &str,
    line: usize,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return;
    }
    if config.is_rule_enabled("CLN-002") {
        let validation = validate_glob_pattern(pattern);
        if !validation.valid {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    0,
                    "CLN-002",
                    t!(
                        "rules.cln_002.message",
                        pattern = pattern,
                        error = validation.error.unwrap_or_default()
                    ),
                )
                .with_suggestion(t!("rules.cln_002.suggestion")),
            );
        }
    }
    if config.is_rule_enabled("CLN-015") && is_unmatchable_pattern(pattern) {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                0,
                "CLN-015",
                t!("rules.cln_015.message", pattern = pattern),
            )
            .with_suggestion(t!("rules.cln_015.suggestion")),
        );
    }
}

/// CLN-013: "To avoid creating a false sense of security, we are removing
/// `.clineignore` as a supported feature." Not reported when a PreToolUse
/// hook sits next to it (the documented guard hook reads `.clineignore`).
fn validate_clineignore(path: &Path, config: &PerFileLintConfig<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if !config.is_rule_enabled("CLN-013") {
        return diagnostics;
    }
    if let Some(root) = path.parent().filter(|root| !root.as_os_str().is_empty()) {
        let fs = config.fs();
        let has_guard = [root.join(".clinerules/hooks"), root.join(".cline/hooks")]
            .iter()
            .filter_map(|dir| fs.read_dir(dir).ok())
            .flatten()
            .any(|entry| {
                entry
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|stem| stem == "PreToolUse")
            });
        if has_guard {
            return diagnostics;
        }
    }
    diagnostics.push(
        Diagnostic::info(
            path.to_path_buf(),
            1,
            0,
            "CLN-013",
            t!("rules.cln_013.message"),
        )
        .with_suggestion(t!("rules.cln_013.suggestion")),
    );
    diagnostics
}

/// CLN-016: "The `type` field selects the transport. Omitting it defaults to
/// the legacy `sse` transport for backward compatibility".
fn validate_mcp_settings(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let mut cx = JsonCx::new(path, content, config);
    if !cx.enabled("CLN-016") {
        return cx.diagnostics;
    }
    let Ok(document) = parse_json_document(content, true) else {
        return cx.diagnostics;
    };
    let Some(servers) = document
        .root
        .value
        .get("mcpServers")
        .and_then(|m| m.value.value.as_object())
    else {
        return cx.diagnostics;
    };
    for server in servers {
        let fields = &server.value.value;
        if fields.as_object().is_some()
            && fields.get("url").is_some()
            && fields.get("type").is_none()
        {
            cx.push(
                DiagnosticLevel::Warning,
                "CLN-016",
                key_span(server),
                t!("rules.cln_016.message", server = server.key.as_str()),
                t!("rules.cln_016.suggestion"),
            );
        }
    }
    cx.diagnostics
}

// =============================================================================
// Workflow rules (CLN-005, CLN-006) and hook rules (CLN-009, CLN-012)
// =============================================================================

impl ClineValidator {
    /// Workflow-specific rules (CLN-005, CLN-006, CLN-010, CLN-014).
    fn validate_workflow(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        check_secrets(path, content, "workflow", config, diagnostics);
        let frontmatter = read_frontmatter(path, content, config, diagnostics);

        // CLN-005: "Missing instructions body in workflow file." (SDK)
        if config.is_rule_enabled("CLN-005") {
            let message = match &frontmatter {
                Frontmatter::None if is_content_empty(content) => Some(t!("rules.cln_005.message")),
                Frontmatter::Valid {
                    empty_body: true, ..
                } => Some(t!("rules.cln_005.message_no_content")),
                _ => None,
            };
            if let Some(message) = message {
                diagnostics.push(
                    Diagnostic::error(path.to_path_buf(), 1, 0, "CLN-005", message)
                        .with_suggestion(t!("rules.cln_005.suggestion")),
                );
            }
        }

        if let Frontmatter::Valid { mapping, raw, .. } = &frontmatter {
            check_keys(
                path,
                content,
                (mapping, raw),
                "CLN-006",
                config,
                diagnostics,
            );
        }
    }

    /// Hook-specific rules (CLN-009, CLN-012).
    fn validate_hook(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let Some(folder) = hook_folder(path) else {
            return;
        };
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            return;
        };
        // Helper files kept next to the hooks.
        if name.starts_with('_') || name.starts_with('.') {
            return;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            return;
        };
        if config.is_rule_enabled("CLN-009") {
            if UNWIRED_HOOK_EVENTS.contains(&stem) {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "CLN-009",
                        t!("rules.cln_009.unwired", event = stem),
                    )
                    .with_suggestion(t!("rules.cln_009.unwired_suggestion")),
                );
            } else if !VALID_HOOK_EVENTS.contains(&stem) {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "CLN-009",
                        t!("rules.cln_009.message", event = stem),
                    )
                    .with_suggestion(t!("rules.cln_009.suggestion")),
                );
            }
        }
        // CLN-012: "Hooks in the extension live in `.clinerules/hooks/` ...,
        // must be named exactly after their event with no file extension,
        // and need a shebang and the executable bit". The CLI also runs
        // `.cline/hooks/PreToolUse.sh`, so `.cline/hooks/` is not checked.
        if folder != ".clinerules" || !config.is_rule_enabled("CLN-012") {
            return;
        }
        if name != stem && VALID_HOOK_EVENTS.contains(&stem) {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "CLN-012",
                    t!("rules.cln_012.extension", file = name, event = stem),
                )
                .with_suggestion(t!("rules.cln_012.extension_suggestion", event = stem)),
            );
        }
        if !content.trim_start_matches('\u{FEFF}').starts_with("#!") {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "CLN-012",
                    t!("rules.cln_012.shebang", file = name),
                )
                .with_suggestion(t!("rules.cln_012.shebang_suggestion")),
            );
        }
    }
}

// =============================================================================
// Cline skills (CL-SK-004). Secrets in skills are XP-SK-003's.
// =============================================================================

const CLINE_SKILL_RULE_IDS: &[&str] = &["CL-SK-004"];

pub struct ClineSkillValidator;

impl Validator for ClineSkillValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: CLINE_SKILL_RULE_IDS,
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if !is_cline_skill_path(path, config) {
            return diagnostics;
        }

        // CL-SK-004: "Missing instructions body in skill file." (SDK
        // `parseSkillConfigFromMarkdown`).
        let parts = split_frontmatter(content);
        let empty_body = if parts.has_frontmatter {
            parts.has_closing && parts.body.trim().is_empty()
        } else {
            content.trim().is_empty()
        };
        if empty_body && config.is_rule_enabled("CL-SK-004") {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    content.lines().count().max(1),
                    0,
                    "CL-SK-004",
                    t!("rules.cl_sk_004.message"),
                )
                .with_suggestion(t!("rules.cl_sk_004.suggestion")),
            );
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate_single(content: &str) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(Path::new(".clinerules"), content, &LintConfig::default())
    }

    fn validate_folder(content: &str) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(
            Path::new(".clinerules/typescript.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_folder_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(Path::new(".clinerules/typescript.md"), content, config)
    }

    fn validate_folder_txt(content: &str) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(
            Path::new(".clinerules/python.txt"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_workflow(content: &str) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(
            Path::new(".clinerules/workflows/deploy.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_workflow_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(
            Path::new(".clinerules/workflows/deploy.md"),
            content,
            config,
        )
    }

    fn validate_hook(path: &str, content: &str) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn validate_hook_with_config(
        path: &str,
        content: &str,
        config: &LintConfig,
    ) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(Path::new(path), content, config)
    }

    fn validate_cline_skill(path: &str, content: &str) -> Vec<Diagnostic> {
        let validator = ClineSkillValidator;
        validator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn validate_cline_skill_with_config(
        path: &str,
        content: &str,
        config: &LintConfig,
    ) -> Vec<Diagnostic> {
        let validator = ClineSkillValidator;
        validator.validate(Path::new(path), content, config)
    }

    // ===== CLN-001: Empty Clinerules File =====

    #[test]
    fn test_cln_001_empty_single_file() {
        let diagnostics = validate_single("");
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
        assert_eq!(cln_001[0].level, DiagnosticLevel::Error);
        assert!(cln_001[0].message.contains("empty"));
    }

    #[test]
    fn test_cln_001_whitespace_only_single() {
        let diagnostics = validate_single("   \n\n\t  ");
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
    }

    #[test]
    fn test_cln_001_valid_single_file() {
        let content = "# Project Rules\n\nAlways follow the coding style guide.";
        let diagnostics = validate_single(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert!(cln_001.is_empty());
    }

    #[test]
    fn test_cln_001_empty_folder_file() {
        let diagnostics = validate_folder("");
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
        assert_eq!(cln_001[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_cln_001_empty_body_after_frontmatter() {
        let content = "---\npaths:\n  - \"**/*.ts\"\n---\n";
        let diagnostics = validate_folder(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
        assert!(cln_001[0].message.contains("no content after frontmatter"));
    }

    #[test]
    fn test_cln_001_valid_folder_file() {
        let content = "---\npaths:\n  - \"**/*.ts\"\n---\n# TypeScript Rules\n\nUse strict mode.\n";
        let diagnostics = validate_folder(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert!(cln_001.is_empty());
    }

    #[test]
    fn test_cln_001_folder_no_frontmatter_with_content() {
        let content = "# Rules without frontmatter\n\nSome instructions.";
        let diagnostics = validate_folder(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert!(cln_001.is_empty());
    }

    #[test]
    fn test_cln_001_newlines_only() {
        let content = "\n\n\n";
        let diagnostics = validate_single(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CLN-001"));
    }

    // ===== CLN-002: Invalid Paths Glob =====

    #[test]
    fn test_cln_002_invalid_glob() {
        let content = "---\npaths:\n  - \"[unclosed\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
        assert_eq!(cln_002.len(), 1);
        assert_eq!(cln_002[0].level, DiagnosticLevel::Error);
        assert!(cln_002[0].message.contains("Invalid glob pattern"));
    }

    #[test]
    fn test_cln_002_validates_bom_prefixed_frontmatter() {
        let content = "\u{FEFF}---\npaths:\n  - \"[unclosed\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CLN-002"));
    }

    #[test]
    fn test_cln_002_valid_glob_patterns() {
        let patterns = vec!["**/*.ts", "*.rs", "src/**/*.js", "tests/**/*.test.ts"];

        for pattern in patterns {
            let content = format!("---\npaths:\n  - \"{}\"\n---\n# Instructions\n", pattern);
            let diagnostics = validate_folder(&content);
            let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
            assert!(cln_002.is_empty(), "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_cln_002_invalid_patterns() {
        let invalid_patterns = ["[invalid", "***", "**["];

        for pattern in invalid_patterns {
            let content = format!("---\npaths:\n  - \"{}\"\n---\nBody", pattern);
            let diagnostics = validate_folder(&content);
            let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
            assert!(
                !cln_002.is_empty(),
                "Pattern '{}' should be invalid",
                pattern
            );
        }
    }

    #[test]
    fn test_cln_002_multiple_patterns_mixed() {
        let content = "---\npaths:\n  - \"**/*.ts\"\n  - \"[invalid\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
        assert_eq!(
            cln_002.len(),
            1,
            "Only the invalid pattern should trigger CLN-002"
        );
        assert!(cln_002[0].message.contains("[invalid"));
    }

    #[test]
    fn test_cln_002_multiple_invalid_patterns() {
        let content = "---\npaths:\n  - \"[bad1\"\n  - \"**[bad2\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
        assert_eq!(
            cln_002.len(),
            2,
            "Both invalid patterns should trigger CLN-002"
        );
    }

    #[test]
    fn test_cln_002_no_paths_field() {
        // No paths field should not trigger CLN-002
        let content = r#"---
---
# Instructions
"#;
        let diagnostics = validate_folder(content);
        let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
        assert!(cln_002.is_empty());
    }

    #[test]
    fn test_cln_002_not_triggered_on_single_file() {
        // Single .clinerules file should not trigger CLN-002
        let content = "# Rules";
        let diagnostics = validate_single(content);
        let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
        assert!(cln_002.is_empty());
    }

    // ===== CLN-003: Unknown Frontmatter Keys =====

    #[test]
    fn test_cln_003_unknown_keys() {
        let content = "---\npaths:\n  - \"**/*.ts\"\nunknownKey: value\nanotherBadKey: 123\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-003").collect();
        assert_eq!(cln_003.len(), 2);
        assert_eq!(cln_003[0].level, DiagnosticLevel::Warning);
        assert!(cln_003.iter().any(|d| d.message.contains("unknownKey")));
        assert!(cln_003.iter().any(|d| d.message.contains("anotherBadKey")));
        assert!(
            cln_003.iter().all(|d| d.has_fixes()),
            "All unknown key diagnostics should include deletion fixes"
        );
        // Fix is marked unsafe because multi-line YAML values would leave orphaned lines
        assert!(cln_003.iter().all(|d| !d.fixes[0].safe));
    }

    #[test]
    fn test_cln_003_no_unknown_keys() {
        let content = "---\npaths:\n  - \"**/*.rs\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-003").collect();
        assert!(cln_003.is_empty());
    }

    #[test]
    fn test_cln_003_not_triggered_on_single_file() {
        let content = "# Rules";
        let diagnostics = validate_single(content);
        let cln_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-003").collect();
        assert!(cln_003.is_empty());
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_cline_category() {
        let mut config = LintConfig::default();
        config.rules_mut().cline = false;

        let content = "";
        let diagnostics = validate_folder_with_config(content, &config);

        let cln_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CLN-"))
            .collect();
        assert!(cln_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CLN-001".to_string()];

        let content = "";
        let diagnostics = validate_folder_with_config(content, &config);

        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert!(cln_001.is_empty());
    }

    // ===== Combined Issues =====

    #[test]
    fn test_multiple_issues() {
        let content = r#"---
unknownKey: value
---
"#;
        let diagnostics = validate_folder(content);

        // Should have CLN-001 (empty body) and CLN-003 (unknown key)
        assert!(
            diagnostics.iter().any(|d| d.rule == "CLN-001"),
            "Expected CLN-001"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "CLN-003"),
            "Expected CLN-003"
        );
    }

    #[test]
    fn test_valid_folder_no_issues() {
        let content = "---\npaths:\n  - \"**/*.ts\"\n---\n# TypeScript Guidelines\n\nAlways use strict mode and explicit types.\n";
        let diagnostics = validate_folder(content);
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got: {:?}",
            diagnostics
        );
    }

    // ===== All Rules Can Be Disabled =====

    #[test]
    fn test_all_cln_rules_can_be_disabled() {
        let rules = ["CLN-001", "CLN-002", "CLN-003", "CLN-004"];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];

            let (content, path): (&str, &str) = match rule {
                "CLN-001" => ("", ".clinerules"),
                "CLN-002" => (
                    "---\npaths:\n  - \"[invalid\"\n---\nBody",
                    ".clinerules/test.md",
                ),
                "CLN-003" => ("---\nunknown: value\n---\nBody", ".clinerules/test.md"),
                "CLN-004" => ("---\npaths: \"**/*.ts\"\n---\nBody", ".clinerules/test.md"),
                _ => unreachable!("Unknown rule: {rule}"),
            };

            let validator = ClineValidator;
            let diagnostics = validator.validate(Path::new(path), content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    // ===== CLN-004: Scalar Paths Error =====

    #[test]
    fn test_cln_004_scalar_paths_warns() {
        let content = "---\npaths: \"**/*.ts\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-004").collect();
        assert_eq!(cln_004.len(), 1);
        assert_eq!(cln_004[0].level, DiagnosticLevel::Error);
        assert!(cln_004[0].message.contains("non-array"));
    }

    #[test]
    fn test_cln_004_array_paths_no_warning() {
        let content = "---\npaths:\n  - \"**/*.ts\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-004").collect();
        assert!(cln_004.is_empty());
    }

    #[test]
    fn test_cln_004_has_autofix() {
        let content = "---\npaths: \"**/*.ts\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-004").collect();
        assert_eq!(cln_004.len(), 1);
        assert!(cln_004[0].has_fixes(), "CLN-004 should have an auto-fix");
        assert!(cln_004[0].fixes[0].safe, "CLN-004 fix should be safe");
        assert!(
            cln_004[0].fixes[0].replacement.contains("- \"**/*.ts\""),
            "Fix should convert scalar to array format, got: {}",
            cln_004[0].fixes[0].replacement
        );
    }

    #[test]
    fn test_cln_004_empty_array_no_warning() {
        let content = "---\npaths: []\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-004").collect();
        assert!(cln_004.is_empty(), "Empty array should not trigger CLN-004");
    }

    // ===== File Type Detection =====

    #[test]
    fn test_single_file_detection() {
        assert_eq!(
            crate::detect_file_type(Path::new(".clinerules")),
            FileType::ClineRules
        );
    }

    #[test]
    fn test_folder_file_detection() {
        assert_eq!(
            crate::detect_file_type(Path::new(".clinerules/typescript.md")),
            FileType::ClineRulesFolder
        );
    }

    #[test]
    fn test_folder_file_with_numeric_prefix() {
        assert_eq!(
            crate::detect_file_type(Path::new(".clinerules/01-coding.md")),
            FileType::ClineRulesFolder
        );
    }

    #[test]
    fn test_alternate_cline_rules_folder_uses_existing_rules() {
        let validator = ClineValidator;
        let empty = validator.validate(
            Path::new(".cline/rules/coding.md"),
            "",
            &LintConfig::default(),
        );
        assert!(empty.iter().any(|d| d.rule == "CLN-001"));

        let invalid_paths = validator.validate(
            Path::new(".cline/rules/coding.md"),
            "---\npaths: \"**/*.rs\"\n---\n# Coding rules\n",
            &LintConfig::default(),
        );
        assert!(invalid_paths.iter().any(|d| d.rule == "CLN-004"));
    }

    // ===== .txt file validation (mirrors .md tests) =====

    #[test]
    fn test_cln_001_empty_txt_file() {
        let diagnostics = validate_folder_txt("");
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
        assert_eq!(cln_001[0].level, DiagnosticLevel::Error);
        assert!(cln_001[0].message.contains("empty"));
    }

    #[test]
    fn test_cln_001_valid_txt_file() {
        let content = "---\npaths:\n  - \"**/*.py\"\n---\n# Python Rules\n\nFollow PEP 8.\n";
        let diagnostics = validate_folder_txt(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert!(cln_001.is_empty());
    }

    #[test]
    fn test_cln_002_bad_glob_in_txt() {
        let content = "---\npaths:\n  - \"[unclosed\"\n---\n# Instructions\n";
        let diagnostics = validate_folder_txt(content);
        let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
        assert_eq!(cln_002.len(), 1);
        assert_eq!(cln_002[0].level, DiagnosticLevel::Error);
        assert!(cln_002[0].message.contains("Invalid glob pattern"));
    }

    #[test]
    fn test_cln_003_unknown_keys_in_txt() {
        let content = "---\npaths:\n  - \"**/*.ts\"\nunknownKey: value\nanotherBadKey: 123\n---\n# Instructions\n";
        let diagnostics = validate_folder_txt(content);
        let cln_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-003").collect();
        assert_eq!(cln_003.len(), 2);
        assert_eq!(cln_003[0].level, DiagnosticLevel::Warning);
        assert!(cln_003.iter().any(|d| d.message.contains("unknownKey")));
        assert!(cln_003.iter().any(|d| d.message.contains("anotherBadKey")));
        assert!(
            cln_003.iter().all(|d| d.has_fixes()),
            "All unknown key diagnostics should include deletion fixes"
        );
        assert!(cln_003.iter().all(|d| !d.fixes[0].safe));
    }

    #[test]
    fn test_cln_004_scalar_paths_in_txt() {
        let content = "---\npaths: \"**/*.ts\"\n---\n# Instructions\n";
        let diagnostics = validate_folder_txt(content);
        let cln_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-004").collect();
        assert_eq!(cln_004.len(), 1);
        assert_eq!(cln_004[0].level, DiagnosticLevel::Error);
        assert!(cln_004[0].message.contains("non-array"));
        assert!(cln_004[0].has_fixes(), "CLN-004 should have an auto-fix");
        assert!(cln_004[0].fixes[0].safe, "CLN-004 fix should be safe");
        assert!(
            cln_004[0].fixes[0].replacement.contains("- \"**/*.ts\""),
            "Fix should convert scalar to array format, got: {}",
            cln_004[0].fixes[0].replacement
        );
    }

    #[test]
    fn test_cln_001_whitespace_only_txt() {
        let diagnostics = validate_folder_txt("   \n\n\t  ");
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
        assert_eq!(cln_001[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_cln_001_empty_body_after_frontmatter_txt() {
        let content = "---\npaths:\n  - \"**/*.py\"\n---\n";
        let diagnostics = validate_folder_txt(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
        assert!(cln_001[0].message.contains("no content after frontmatter"));
    }

    #[test]
    fn test_cln_001_folder_no_frontmatter_with_content_txt() {
        let content = "# Rules without frontmatter\n\nSome instructions.";
        let diagnostics = validate_folder_txt(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert!(cln_001.is_empty());
    }

    #[test]
    fn test_valid_txt_no_diagnostics() {
        let content =
            "---\npaths:\n  - \"**/*.py\"\n---\n# Python Guidelines\n\nAlways use type hints.\n";
        let diagnostics = validate_folder_txt(content);
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics for valid .txt file, got: {:?}",
            diagnostics
        );
    }

    // ===== CLN-005: Empty Workflow File =====

    #[test]
    fn test_cln_005_empty_workflow() {
        let diagnostics = validate_workflow("");
        let cln_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-005").collect();
        assert_eq!(cln_005.len(), 1);
        assert_eq!(cln_005[0].level, DiagnosticLevel::Error);
        assert!(cln_005[0].message.contains("empty"));
    }

    #[test]
    fn test_cln_005_whitespace_only_workflow() {
        let diagnostics = validate_workflow("   \n\n\t  ");
        let cln_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-005").collect();
        assert_eq!(cln_005.len(), 1);
    }

    #[test]
    fn test_cln_005_valid_workflow() {
        let content = "# Deploy Workflow\n\n1. Build the project\n2. Run tests\n";
        let diagnostics = validate_workflow(content);
        let cln_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-005").collect();
        assert!(cln_005.is_empty());
    }

    #[test]
    fn test_cln_005_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CLN-005".to_string()];
        let diagnostics = validate_workflow_with_config("", &config);
        let cln_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-005").collect();
        assert!(cln_005.is_empty());
    }

    // ===== CLN-006: Unknown Workflow Frontmatter Key =====

    #[test]
    fn test_cln_006_workflow_with_frontmatter() {
        let content = "---\ntitle: Deploy\n---\n# Deploy steps\n";
        let diagnostics = validate_workflow(content);
        let cln_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-006").collect();
        assert_eq!(cln_006.len(), 1);
        assert_eq!(cln_006[0].level, DiagnosticLevel::Warning);
        assert!(cln_006[0].message.contains("'title'"));
        assert!(cln_006[0].fixes.is_empty());
    }

    #[test]
    fn test_cln_006_workflow_plain_markdown() {
        let content = "# Deploy Workflow\n\nStep 1: build\n";
        let diagnostics = validate_workflow(content);
        let cln_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-006").collect();
        assert!(cln_006.is_empty());
    }

    #[test]
    fn test_cln_006_detects_bom_prefixed_frontmatter() {
        let content = "\u{FEFF}---\ntitle: Deploy\n---\n# Deploy steps\n";
        let diagnostics = validate_workflow(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CLN-006"));
    }

    #[test]
    fn test_cln_006_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CLN-006".to_string()];
        let content = "---\ntitle: Deploy\n---\n# Deploy steps\n";
        let diagnostics = validate_workflow_with_config(content, &config);
        let cln_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-006").collect();
        assert!(cln_006.is_empty());
    }

    #[test]
    fn test_cln_006_not_triggered_on_regular_folder_file() {
        // Regular .clinerules/*.md files can have frontmatter
        let content = "---\npaths:\n  - \"**/*.ts\"\n---\n# TypeScript Rules\n";
        let diagnostics = validate_folder(content);
        let cln_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-006").collect();
        assert!(cln_006.is_empty());
    }

    // ===== CLN-009: Unknown Hook Event Name =====

    #[test]
    fn test_cln_009_unknown_event() {
        let diagnostics = validate_hook(
            ".clinerules/hooks/InvalidEvent.sh",
            "#!/bin/bash\necho hello",
        );
        let cln_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-009").collect();
        assert_eq!(cln_009.len(), 1);
        assert_eq!(cln_009[0].level, DiagnosticLevel::Warning);
        assert!(cln_009[0].message.contains("InvalidEvent"));
    }

    #[test]
    fn test_cln_009_valid_events() {
        let events = [
            "TaskStart",
            "TaskResume",
            "TaskCancel",
            "TaskComplete",
            "PreToolUse",
            "PostToolUse",
            "UserPromptSubmit",
            "TaskError",
            "SessionShutdown",
        ];
        for event in events {
            let path = format!(".clinerules/hooks/{}.sh", event);
            let diagnostics = validate_hook(&path, "#!/bin/bash\necho hello");
            let cln_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-009").collect();
            assert!(
                cln_009.is_empty(),
                "Event '{}' should be valid but triggered CLN-009",
                event
            );
        }
    }

    #[test]
    fn test_cln_009_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CLN-009".to_string()];
        let diagnostics =
            validate_hook_with_config(".clinerules/hooks/BadEvent.sh", "#!/bin/bash", &config);
        let cln_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-009").collect();
        assert!(cln_009.is_empty());
    }

    #[test]
    fn test_cln_009_not_triggered_on_non_hook_path() {
        // A file not under hooks/ should not trigger CLN-009
        let diagnostics = validate_folder("#!/bin/bash\necho hello");
        let cln_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-009").collect();
        assert!(cln_009.is_empty());
    }

    // ===== Path detection tests =====

    #[test]
    fn test_is_workflow_path() {
        assert!(is_workflow_path(Path::new(
            ".clinerules/workflows/deploy.md"
        )));
        assert!(!is_workflow_path(Path::new(".clinerules/typescript.md")));
        assert!(!is_workflow_path(Path::new(
            ".clinerules/hooks/TaskStart.sh"
        )));
        assert!(is_workflow_path(Path::new(".cline/workflows/release.md")));
    }

    #[test]
    fn test_is_hook_path() {
        assert!(is_hook_path(Path::new(".clinerules/hooks/TaskStart.sh")));
        assert!(!is_hook_path(Path::new(".clinerules/typescript.md")));
        assert!(!is_hook_path(Path::new(".clinerules/workflows/deploy.md")));
        assert!(is_hook_path(Path::new(".cline/hooks/PreToolUse.sh")));
    }

    #[test]
    fn test_is_cline_skill_path() {
        let config = LintConfig::default();
        assert!(is_cline_skill_path(
            Path::new(".cline/skills/my-skill/SKILL.md"),
            &config
        ));
        assert!(is_cline_skill_path(
            Path::new(".clinerules/skills/my-skill/SKILL.md"),
            &config
        ));
        assert!(!is_cline_skill_path(
            Path::new(".claude/skills/my-skill/SKILL.md"),
            &config
        ));
        assert!(!is_cline_skill_path(
            Path::new(".cline/skills/my-skill/README.md"),
            &config
        ));
        let mut cline = LintConfig::default();
        cline.set_tools(vec!["cline".to_string()]);
        assert!(is_cline_skill_path(
            Path::new(".claude/skills/my-skill/SKILL.md"),
            &cline
        ));
        assert!(is_cline_skill_path(
            Path::new(".agents/skills/my-skill/SKILL.md"),
            &cline
        ));
    }

    // ===== All New Rules Can Be Disabled =====

    #[test]
    fn test_all_new_rules_can_be_disabled() {
        let cases: Vec<(&str, &str, &str)> = vec![
            ("CLN-005", ".clinerules/workflows/test.md", ""),
            (
                "CLN-006",
                ".clinerules/workflows/test.md",
                "---\ntitle: x\n---\n# Body",
            ),
            ("CLN-009", ".clinerules/hooks/BadEvent.sh", "#!/bin/bash"),
        ];

        for (rule, path, content) in cases {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];
            let validator = ClineValidator;
            let diagnostics = validator.validate(Path::new(path), content, &config);
            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    // ===== Workflow file type detection =====

    #[test]
    fn test_workflow_file_detected_as_cline_rules_folder() {
        assert_eq!(
            crate::detect_file_type(Path::new(".clinerules/workflows/deploy.md")),
            FileType::ClineRulesFolder
        );
    }

    #[test]
    fn test_hook_file_detected_as_cline_rules_folder() {
        for path in [
            ".clinerules/hooks/TaskStart",
            ".cline/hooks/PreToolUse.sh",
            "project/.cline/hooks/TaskError",
        ] {
            assert_eq!(
                crate::detect_file_type(Path::new(path)),
                FileType::ClineRulesFolder,
                "{path}"
            );
        }
        // Markdown notes next to the hook scripts are not hooks; this used
        // to be classified as a hook named "TaskStart" with a .md extension.
        assert_eq!(
            crate::detect_file_type(Path::new(".clinerules/hooks/TaskStart.md")),
            FileType::Unknown
        );
    }

    // ===== Survey corrections and new rules (CLN-010..016, CL-SK-004) =====

    fn rule<'a>(diagnostics: &'a [Diagnostic], id: &str) -> Vec<&'a Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == id).collect()
    }

    fn apply(content: &str, fix: &Fix) -> String {
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        fixed
    }

    fn frontmatter_of(content: &str) -> serde_yaml::Value {
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter && parts.has_closing, "{content}");
        serde_yaml::from_str(&parts.frontmatter).unwrap()
    }

    #[test]
    fn cln_003_accepts_the_keys_cline_reads() {
        let content = "---\nname: TypeScript\ndisabled: false\nenabled: true\npaths:\n  - \"src/**/*.ts\"\n---\n# TypeScript\n";
        assert!(validate_folder(content).is_empty());
    }

    #[test]
    fn cln_003_wrong_types_and_exact_removal() {
        let content = "---\npaths:\n  - \"src/**\"\ndisabled: \"yes\"\nname: 3\nglobs: \"*.ts\"\n---\n# Rules\n";
        let diagnostics = validate_folder(content);
        let cln_003 = rule(&diagnostics, "CLN-003");
        assert_eq!(cln_003.len(), 3, "{cln_003:#?}");
        let typed: Vec<_> = cln_003.iter().filter(|d| d.fixes.is_empty()).collect();
        assert_eq!(typed.len(), 2);
        assert!(
            typed
                .iter()
                .any(|d| d.line == 4 && d.message.contains("boolean"))
        );
        assert!(
            typed
                .iter()
                .any(|d| d.line == 5 && d.message.contains("string"))
        );
        let unknown = cln_003.iter().find(|d| !d.fixes.is_empty()).unwrap();
        assert_eq!(unknown.line, 6);
        assert!(!unknown.fixes[0].safe);
        let fixed = apply(content, &unknown.fixes[0]);
        let yaml = frontmatter_of(&fixed);
        assert!(yaml.get("globs").is_none());
        assert!(yaml.get("paths").is_some());
        assert!(fixed.ends_with("---\n# Rules\n"));
    }

    #[test]
    fn cln_004_fix_is_exact() {
        for content in [
            "---\npaths: \"**/*.ts\"\nname: ts\n---\n# Instructions\n",
            "---\r\npaths: '**/*.ts'\r\n---\r\n# Instructions\r\n",
        ] {
            let diagnostics = validate_folder(content);
            let cln_004 = rule(&diagnostics, "CLN-004");
            assert_eq!(cln_004.len(), 1);
            assert!(cln_004[0].message.contains("every request"));
            let fixed = apply(content, &cln_004[0].fixes[0]);
            let yaml = frontmatter_of(&fixed);
            assert_eq!(
                yaml.get("paths").unwrap(),
                &serde_yaml::Value::Sequence(vec!["**/*.ts".into()])
            );
            assert!(validate_folder(&fixed).is_empty(), "{fixed}");
        }
    }

    #[test]
    fn cln_005_and_cln_006_on_cline_workflows() {
        let validator = ClineValidator;
        let config = LintConfig::default();
        let path = Path::new(".cline/workflows/release.md");
        assert_eq!(crate::detect_file_type(path), FileType::ClineRulesFolder);

        let empty = validator.validate(path, "", &config);
        assert_eq!(rule(&empty, "CLN-005").len(), 1);
        assert!(rule(&empty, "CLN-001").is_empty());

        let no_steps = validator.validate(path, "---\nname: release\n---\n\n", &config);
        let cln_005 = rule(&no_steps, "CLN-005");
        assert_eq!(cln_005.len(), 1);
        assert!(cln_005[0].message.contains("after the frontmatter"));

        let documented = "---\nname: release\ndisabled: false\n---\n1. Tag the release\n";
        assert!(validator.validate(path, documented, &config).is_empty());

        let typed = validator.validate(
            path,
            "---\nenabled: \"no\"\ndescription: Ship it\n---\n1. Tag\n",
            &config,
        );
        let cln_006 = rule(&typed, "CLN-006");
        assert_eq!(cln_006.len(), 2, "{cln_006:#?}");
        assert!(cln_006.iter().all(|d| d.fixes.is_empty()));
        // Workflows are not rules: no CLN-001 or CLN-003.
        assert!(typed.iter().all(|d| d.rule == "CLN-006"));
    }

    #[test]
    fn cln_009_event_set_and_cli_folder() {
        let hook = |path: &str| validate_hook(path, "#!/bin/bash\necho '{}'\n");
        assert_eq!(
            rule(&hook(".cline/hooks/BogusEvent.sh"), "CLN-009").len(),
            1
        );
        assert!(hook(".cline/hooks/TaskError.sh").is_empty());
        assert!(hook(".clinerules/hooks/SessionShutdown").is_empty());
        let unwired = hook(".clinerules/hooks/PreCompact");
        let cln_009 = rule(&unwired, "CLN-009");
        assert_eq!(cln_009.len(), 1);
        assert!(cln_009[0].message.contains("not wired"));
        // Helper scripts and files in subfolders are not hooks.
        assert!(hook(".cline/hooks/_common.sh").is_empty());
        assert!(hook(".cline/hooks/lib/util.sh").is_empty());
    }

    #[test]
    fn cln_010_unparseable_frontmatter() {
        let alias = "---\npaths:\n  - *.md\n---\n# Docs rules\n";
        let diagnostics = validate_folder(alias);
        let cln_010 = rule(&diagnostics, "CLN-010");
        assert_eq!(cln_010.len(), 1);
        assert_eq!(cln_010[0].level, DiagnosticLevel::Error);
        assert_eq!(cln_010[0].line, 3);
        assert!(cln_010[0].suggestion.as_deref().unwrap().contains("Quote"));
        assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");

        let unclosed = validate_folder("---\npaths:\n  - \"src/**\"\n# Rules\n");
        let cln_010 = rule(&unclosed, "CLN-010");
        assert_eq!(cln_010.len(), 1);
        assert!(cln_010[0].message.contains("closing"));

        let broken = validate_workflow("---\nname: [deploy\n---\n1. Deploy\n");
        assert_eq!(rule(&broken, "CLN-010").len(), 1);

        // Type errors are not YAML errors.
        let typed = validate_folder("---\npaths: 5\n---\n# Rules\n");
        assert!(rule(&typed, "CLN-010").is_empty());
        assert_eq!(rule(&typed, "CLN-011").len(), 1);
    }

    #[test]
    fn cln_011_paths_shapes_that_drop_the_condition() {
        for (content, expected) in [
            ("---\npaths:\n  - \"src/**\"\n  - \"\"\n---\n# R\n", 1),
            ("---\npaths:\n  - 42\n---\n# R\n", 1),
            ("---\npaths:\n  - [nested]\n---\n# R\n", 1),
            ("---\npaths: 5\n---\n# R\n", 1),
            ("---\npaths:\n---\n# R\n", 1),
            ("---\npaths:\n  src: true\n---\n# R\n", 1),
            // `paths: []` disables the rule on purpose.
            ("---\npaths: []\n---\n# R\n", 0),
            ("---\npaths:\n  - \"src/**\"\n---\n# R\n", 0),
        ] {
            let diagnostics = validate_folder(content);
            let cln_011 = rule(&diagnostics, "CLN-011");
            assert_eq!(cln_011.len(), expected, "{content}: {diagnostics:#?}");
            assert!(cln_011.iter().all(|d| d.level == DiagnosticLevel::Error));
            assert!(cln_011.iter().all(|d| d.line == 2));
        }
    }

    #[test]
    fn cln_012_hooks_the_extension_cannot_run() {
        let with_extension = validate_hook(".clinerules/hooks/PreToolUse.sh", "#!/bin/bash\n");
        let cln_012 = rule(&with_extension, "CLN-012");
        assert_eq!(cln_012.len(), 1);
        assert!(
            cln_012[0]
                .suggestion
                .as_deref()
                .unwrap()
                .contains("PreToolUse")
        );

        let no_shebang = validate_hook(".clinerules/hooks/TaskStart", "echo '{}'\n");
        assert_eq!(rule(&no_shebang, "CLN-012").len(), 1);

        // The CLI runs `.cline/hooks/PreToolUse.sh`.
        assert!(validate_hook(".cline/hooks/PreToolUse.sh", "echo '{}'\n").is_empty());
        assert!(validate_hook(".clinerules/hooks/PreToolUse", "#!/usr/bin/env bash\n").is_empty());
    }

    #[test]
    fn cln_013_clineignore() {
        let validator = ClineValidator;
        let config = LintConfig::default();
        assert_eq!(
            crate::detect_file_type(Path::new(".clineignore")),
            FileType::ClineIgnore
        );
        let diagnostics = validator.validate(Path::new(".clineignore"), ".env\n", &config);
        let cln_013 = rule(&diagnostics, "CLN-013");
        assert_eq!(cln_013.len(), 1);
        assert_eq!(cln_013[0].level, DiagnosticLevel::Info);

        // The documented PreToolUse guard hook reads .clineignore.
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join(".cline/hooks")).unwrap();
        std::fs::write(
            temp.path().join(".cline/hooks/PreToolUse.sh"),
            "#!/bin/bash\n",
        )
        .unwrap();
        let guarded = validator.validate(&temp.path().join(".clineignore"), ".env\n", &config);
        assert!(guarded.is_empty(), "{guarded:#?}");
    }

    #[test]
    fn cln_014_literal_secrets() {
        let rule_file =
            "# Deploy\n\nUse api_key: \"sk-live-4f9a8b7c6d5e4f3a2b1c0d9e\" for staging.\n";
        let diagnostics = validate_folder(rule_file);
        let cln_014 = rule(&diagnostics, "CLN-014");
        assert_eq!(cln_014.len(), 1);
        assert_eq!(cln_014[0].line, 3);
        assert_eq!(cln_014[0].level, DiagnosticLevel::Warning);

        for clean in [
            "# Deploy\n\nSet API_KEY=your_api_key_here before running.\n",
            "# Auth\n\nRead the token: from `process.env.API_TOKEN`.\n",
            "# Auth\n\nThe password field is hashed with `hashedPassword`.\n",
        ] {
            assert!(
                rule(&validate_folder(clean), "CLN-014").is_empty(),
                "{clean}"
            );
        }

        let workflow = validate_workflow(
            "1. Run `gh auth login --with-token ghp_abcdefghijklmnopqrstuvwxyz0123456789`\n",
        );
        assert_eq!(rule(&workflow, "CLN-014").len(), 1);

        // Secrets in skills are reported by the cross-tool skill rules.
        let skill = validate_cline_skill(
            ".cline/skills/deploy/SKILL.md",
            "---\nname: deploy\ndescription: Deploys. Use when shipping.\n---\nexport AWS_ACCESS_KEY_ID=AKIAABCDEFGHIJKLMNOP\n",
        );
        assert!(rule(&skill, "CLN-014").is_empty());
    }

    #[test]
    fn cln_015_patterns_that_never_match() {
        let content = "---\npaths:\n  - \"/src/**\"\n  - \"../shared/**\"\n  - \"C:/repo/**\"\n  - \"src/**\"\n  - \"./docs/**\"\n---\n# R\n";
        let diagnostics = validate_folder(content);
        let cln_015 = rule(&diagnostics, "CLN-015");
        assert_eq!(cln_015.len(), 3, "{cln_015:#?}");
        assert!(cln_015.iter().all(|d| d.level == DiagnosticLevel::Warning));
    }

    #[test]
    fn cln_016_remote_server_without_type() {
        let validator = ClineValidator;
        let path = Path::new("/home/u/.cline/data/settings/cline_mcp_settings.json");
        assert_eq!(crate::detect_file_type(path), FileType::ClineMcpSettings);
        let content = r#"{
  "mcpServers": {
    "remote": { "url": "https://mcp.example.com/mcp", "headers": { "Authorization": "Bearer ${TOKEN}" } },
    "typed": { "url": "https://mcp.example.com/mcp", "type": "streamableHttp" },
    "local": { "command": "node", "args": ["server.js"] }
  }
}"#;
        let diagnostics = validator.validate(path, content, &LintConfig::default());
        let cln_016 = rule(&diagnostics, "CLN-016");
        assert_eq!(cln_016.len(), 1, "{diagnostics:#?}");
        assert_eq!(cln_016[0].line, 3);
        assert!(cln_016[0].message.contains("remote"));
        assert!(
            validator
                .validate(path, "{ \"mcpServers\": ", &LintConfig::default())
                .is_empty()
        );
    }

    #[test]
    fn cl_sk_004_empty_skill_body() {
        let empty = validate_cline_skill(
            ".cline/skills/my-skill/SKILL.md",
            "---\nname: my-skill\ndescription: Does things. Use when needed.\n---\n\n",
        );
        let cl_sk_004 = rule(&empty, "CL-SK-004");
        assert_eq!(cl_sk_004.len(), 1);
        assert_eq!(cl_sk_004[0].level, DiagnosticLevel::Warning);
        assert_eq!(
            rule(
                &validate_cline_skill(".clinerules/skills/my-skill/SKILL.md", ""),
                "CL-SK-004"
            )
            .len(),
            1
        );
        assert!(
            validate_cline_skill(
                ".cline/skills/my-skill/SKILL.md",
                "---\nname: my-skill\ndescription: Does things.\n---\n# Steps\n"
            )
            .is_empty()
        );
        // Shared skill folders only when the run targets Cline.
        let shared = "---\nname: my-skill\ndescription: Does things.\n---\n";
        assert!(validate_cline_skill(".claude/skills/my-skill/SKILL.md", shared).is_empty());
        let mut config = LintConfig::default();
        config.set_tools(vec!["cline".to_string()]);
        let selected =
            validate_cline_skill_with_config(".agents/skills/my-skill/SKILL.md", shared, &config);
        assert_eq!(rule(&selected, "CL-SK-004").len(), 1);
    }

    #[test]
    fn skill_resources_are_not_rules() {
        for path in [
            ".clinerules/skills/my-skill/docs/advanced.md",
            ".clinerules/skills/my-skill/templates/note.txt",
        ] {
            assert_ne!(
                crate::detect_file_type(Path::new(path)),
                FileType::ClineRulesFolder,
                "{path}"
            );
        }
        assert_eq!(
            crate::detect_file_type(Path::new(".clinerules/skills/my-skill/SKILL.md")),
            FileType::Skill
        );
    }

    #[test]
    fn survey_rules_can_be_disabled() {
        let cases: &[(&str, &str, &str)] = &[
            (
                "CLN-010",
                ".clinerules/r.md",
                "---\npaths:\n  - *.md\n---\nBody",
            ),
            ("CLN-011", ".clinerules/r.md", "---\npaths: 5\n---\nBody"),
            ("CLN-012", ".clinerules/hooks/TaskStart.sh", "echo"),
            ("CLN-013", ".clineignore", ".env"),
            (
                "CLN-014",
                ".clinerules/r.md",
                "token: ghp_abcdefghijklmnopqrstuvwxyz0123456789",
            ),
            (
                "CLN-015",
                ".clinerules/r.md",
                "---\npaths:\n  - \"/src/**\"\n---\nBody",
            ),
            (
                "CLN-016",
                "cline_mcp_settings.json",
                "{\"mcpServers\":{\"a\":{\"url\":\"https://x\"}}}",
            ),
        ];
        for (id, path, content) in cases {
            let enabled = ClineValidator.validate(Path::new(path), content, &LintConfig::default());
            assert!(enabled.iter().any(|d| d.rule == *id), "{id} should fire");
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![id.to_string()];
            let disabled = ClineValidator.validate(Path::new(path), content, &config);
            assert!(
                disabled.iter().all(|d| d.rule != *id),
                "{id} should be disabled"
            );
        }
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CL-SK-004".to_string()];
        assert!(
            validate_cline_skill_with_config(".cline/skills/x/SKILL.md", "", &config).is_empty()
        );
    }
}
