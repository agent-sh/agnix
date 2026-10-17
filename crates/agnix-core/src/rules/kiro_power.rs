//! Kiro power validation rules.
//!
//! Legacy `POWER.md` powers (KR-PW-001 to KR-PW-008, KR-PW-017):
//! - KR-PW-001: POWER.md frontmatter Kiro cannot read, or without the
//!   description and keywords Kiro activates a power by
//! - KR-PW-002: Empty POWER.md keywords array
//! - KR-PW-003: Empty POWER.md body
//! - KR-PW-004: Adjacent power mcp.json Kiro cannot read
//! - KR-PW-005: Step heading without content
//! - KR-PW-006: Duplicate keywords
//! - KR-PW-007: Name invalid characters
//! - KR-PW-008: Secrets in power body
//! - KR-PW-017: POWER.md ignored because the power has a plugin.json
//!
//! Agent Plugins `plugin.json` powers (KR-PW-009 to KR-PW-016, KR-PW-018),
//! validated by [`KiroPowerPluginValidator`] for any `plugin.json` path it is
//! given (installed under `~/.kiro/powers/installed/<name>/` or published):
//! - KR-PW-009: plugin.json is not a JSON object
//! - KR-PW-010: Missing or unsupported `$schema`
//! - KR-PW-011: Missing or invalid `name`
//! - KR-PW-012: Field outside the Agent Plugins manifest
//! - KR-PW-013: Field value Kiro ignores (wrong type)
//! - KR-PW-014: Plugin skill Kiro skips
//! - KR-PW-015: No activation description or keywords
//! - KR-PW-016: Field the powers docs list as required is missing
//! - KR-PW-018: Legacy `steering/` directory in a plugin power
//!
//! Ground truth: Kiro CLI 2.32 V3 engine (`PowersManager`, `AgentPluginLoader`)
//! and kiro.dev/docs/powers. Powers are a V3 (and IDE) feature.

use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata, kiro_steering::find_secret, line_col_at_offset},
    schemas::kiro_power::{
        AGENT_PLUGINS_SCHEMA_PATTERN, JsonMember, PLUGIN_AUTHOR_FIELDS, PLUGIN_MANIFEST_FIELDS,
        is_valid_plugin_name, is_valid_skill_name, json_children, parse_kiro_power,
        parse_plugin_manifest,
    },
};
use regex::Regex;
use rust_i18n::t;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;

const RULE_IDS: &[&str] = &[
    "KR-PW-001",
    "KR-PW-002",
    "KR-PW-003",
    "KR-PW-004",
    "KR-PW-005",
    "KR-PW-006",
    "KR-PW-007",
    "KR-PW-008",
    "KR-PW-017",
];

// Used once the integrator routes plugin.json powers to KiroPowerPluginValidator.
#[allow(dead_code)]
const PLUGIN_RULE_IDS: &[&str] = &[
    "KR-PW-009",
    "KR-PW-010",
    "KR-PW-011",
    "KR-PW-012",
    "KR-PW-013",
    "KR-PW-014",
    "KR-PW-015",
    "KR-PW-016",
    "KR-PW-018",
];

/// Legacy POWER.md names: lowercase letters, digits and hyphens (kebab-case).
fn power_name_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[a-z0-9][a-z0-9-]*$").expect("power name pattern must compile"))
}

// Used once the integrator routes plugin.json powers to KiroPowerPluginValidator.
#[allow(dead_code)]
fn plugin_schema_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(AGENT_PLUGINS_SCHEMA_PATTERN).expect("schema pattern must compile")
    })
}

/// `## Step ...` headings (any level from 2 down), the documented step form.
fn step_heading_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^ {0,3}(#{2,6})[ \t]+(step\b.*)$").expect("step heading pattern")
    })
}

/// Line and column of `key: ...` in YAML frontmatter text, as absolute 1-based
/// coordinates; falls back to the first frontmatter line.
fn frontmatter_key_location(content: &str, frontmatter_start: usize, key: &str) -> (usize, usize) {
    let parts = crate::parsers::frontmatter::split_frontmatter(content);
    let entries = crate::rules::kiro_steering::yaml_top_level_entries(&parts.frontmatter);
    let offset = entries
        .iter()
        .find(|entry| entry.key == key)
        .map_or(0, |entry| entry.start);
    line_col_at_offset(content, frontmatter_start + offset)
}

pub struct KiroPowerValidator;

impl Validator for KiroPowerValidator {
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
        let fs = config.fs();
        let power_dir = path.parent();

        // KR-PW-017: Kiro loads a power from plugin.json when it exists and
        // never reads POWER.md (or its steering/ and mcp.json) then.
        if let Some(dir) = power_dir
            && fs.is_file(&dir.join("plugin.json"))
        {
            if config.is_rule_enabled("KR-PW-017") {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        1,
                        "KR-PW-017",
                        t!("rules.kr_pw_017.message"),
                    )
                    .with_suggestion(t!("rules.kr_pw_017.suggestion")),
                );
            }
            return diagnostics;
        }

        let parsed = parse_kiro_power(content);

        if config.is_rule_enabled("KR-PW-001") {
            if let Some(parse_error) = parsed.parse_error.as_ref() {
                // A YAML error makes Kiro skip the whole power.
                let key = if parsed.has_closing_frontmatter {
                    "kr_pw_001_parse"
                } else {
                    "kr_pw_001_unclosed"
                };
                let (message_key, suggestion_key) = (
                    format!("rules.{key}.message"),
                    format!("rules.{key}.suggestion"),
                );
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        parse_error.line,
                        parse_error.column,
                        "KR-PW-001",
                        t!(&message_key, error = parse_error.message.as_str()),
                    )
                    .with_suggestion(t!(&suggestion_key)),
                );
            } else if !parsed.has_frontmatter || parsed.frontmatter_not_first {
                let key = if parsed.frontmatter_not_first {
                    "kr_pw_001_not_first"
                } else {
                    "kr_pw_001_none"
                };
                let (message_key, suggestion_key) = (
                    format!("rules.{key}.message"),
                    format!("rules.{key}.suggestion"),
                );
                diagnostics.push(
                    Diagnostic::warning(path.to_path_buf(), 1, 1, "KR-PW-001", t!(&message_key))
                        .with_suggestion(t!(&suggestion_key)),
                );
            } else if let Some(frontmatter) = parsed.frontmatter.as_ref() {
                // Kiro reads `description` and an array `keywords`; `name`
                // comes from the power directory, so it is not required.
                let mut missing = Vec::new();
                if frontmatter
                    .description
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                {
                    missing.push("description");
                }
                if frontmatter.keywords.is_none() {
                    missing.push("keywords");
                }
                if !missing.is_empty() {
                    let (line, col) = line_col_at_offset(content, parsed.frontmatter_start);
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            line,
                            col,
                            "KR-PW-001",
                            t!("rules.kr_pw_001.message", fields = missing.join(", ")),
                        )
                        .with_suggestion(t!("rules.kr_pw_001.suggestion")),
                    );
                }
            }
        }

        if config.is_rule_enabled("KR-PW-002")
            && let Some(frontmatter) = parsed.frontmatter.as_ref()
            && let Some(keywords) = frontmatter.keywords.as_ref()
            && keywords.is_empty()
        {
            let (line, col) =
                frontmatter_key_location(content, parsed.frontmatter_start, "keywords");
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "KR-PW-002",
                    t!("rules.kr_pw_002.message"),
                )
                .with_suggestion(t!("rules.kr_pw_002.suggestion")),
            );
        }

        if config.is_rule_enabled("KR-PW-003")
            && parsed.frontmatter.is_some()
            && parsed.body.trim().is_empty()
        {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    1,
                    "KR-PW-003",
                    t!("rules.kr_pw_003.message"),
                )
                .with_suggestion(t!("rules.kr_pw_003.suggestion")),
            );
        }

        // KR-PW-004: Kiro reads the adjacent mcp.json with JSON.parse; invalid
        // JSON makes the whole power unreadable, a non-object mcpServers just
        // yields no servers.
        if config.is_rule_enabled("KR-PW-004")
            && let Some(dir) = power_dir
        {
            let mcp_path = dir.join("mcp.json");
            if fs.exists(&mcp_path) {
                let parsed_mcp = fs
                    .read_to_string(&mcp_path)
                    .ok()
                    .map(|text| serde_json::from_str::<Value>(&text));
                match parsed_mcp {
                    Some(Ok(value)) => {
                        if !value.get("mcpServers").is_some_and(Value::is_object) {
                            diagnostics.push(
                                Diagnostic::warning(
                                    path.to_path_buf(),
                                    1,
                                    1,
                                    "KR-PW-004",
                                    t!("rules.kr_pw_004.message"),
                                )
                                .with_suggestion(t!("rules.kr_pw_004.suggestion")),
                            );
                        }
                    }
                    Some(Err(error)) => diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            1,
                            "KR-PW-004",
                            t!("rules.kr_pw_004_parse.message", error = error.to_string()),
                        )
                        .with_suggestion(t!("rules.kr_pw_004_parse.suggestion")),
                    ),
                    None => diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            1,
                            "KR-PW-004",
                            t!("rules.kr_pw_004_parse.message", error = "unreadable file"),
                        )
                        .with_suggestion(t!("rules.kr_pw_004_parse.suggestion")),
                    ),
                }
            }
        }

        let body_line_offset = line_col_at_offset(content, parsed.body_start).0 - 1;

        // KR-PW-005: a `## Step ...` heading followed by no content before the
        // next heading of the same or a higher level (code fences skipped).
        if config.is_rule_enabled("KR-PW-005") && parsed.frontmatter.is_some() {
            let lines: Vec<&str> = parsed.body.lines().collect();
            let mut in_fence = false;
            let mut fenced = vec![false; lines.len()];
            for (i, line) in lines.iter().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                    in_fence = !in_fence;
                    fenced[i] = true;
                } else {
                    fenced[i] = in_fence;
                }
            }
            let heading_level = |line: &str| {
                let trimmed = line.trim_start();
                let hashes = trimmed.bytes().take_while(|b| *b == b'#').count();
                ((1..=6).contains(&hashes) && trimmed[hashes..].starts_with([' ', '\t']))
                    .then_some(hashes)
            };
            for (i, line) in lines.iter().enumerate() {
                if fenced[i] {
                    continue;
                }
                let Some(captures) = step_heading_pattern().captures(line) else {
                    continue;
                };
                let level = captures[1].len();
                let has_content = lines[i + 1..]
                    .iter()
                    .enumerate()
                    .take_while(|(j, next)| {
                        fenced[i + 1 + j] || heading_level(next).is_none_or(|l| l > level)
                    })
                    .any(|(_, next)| !next.trim().is_empty());
                if !has_content {
                    let step = captures[2].trim();
                    diagnostics.push(
                        Diagnostic::info(
                            path.to_path_buf(),
                            body_line_offset + i + 1,
                            1,
                            "KR-PW-005",
                            t!("rules.kr_pw_005.message", step = step),
                        )
                        .with_suggestion(t!("rules.kr_pw_005.suggestion")),
                    );
                }
            }
        }

        // KR-PW-006: Duplicate keywords
        if config.is_rule_enabled("KR-PW-006")
            && let Some(frontmatter) = parsed.frontmatter.as_ref()
            && let Some(keywords) = frontmatter.keywords.as_ref()
        {
            let mut seen = HashSet::new();
            let mut duplicates = Vec::new();
            for keyword in keywords {
                let normalized = keyword.trim().to_ascii_lowercase();
                if !normalized.is_empty() && !seen.insert(normalized) {
                    duplicates.push(keyword.trim());
                }
            }
            if !duplicates.is_empty() {
                let (line, col) =
                    frontmatter_key_location(content, parsed.frontmatter_start, "keywords");
                diagnostics.push(
                    Diagnostic::info(
                        path.to_path_buf(),
                        line,
                        col,
                        "KR-PW-006",
                        t!(
                            "rules.kr_pw_006.message",
                            duplicates = duplicates.join(", ")
                        ),
                    )
                    .with_suggestion(t!("rules.kr_pw_006.suggestion")),
                );
            }
        }

        // KR-PW-007: Name invalid characters
        if config.is_rule_enabled("KR-PW-007")
            && let Some(frontmatter) = parsed.frontmatter.as_ref()
            && let Some(name) = frontmatter.name.as_deref()
            && !name.trim().is_empty()
            && !power_name_pattern().is_match(name.trim())
        {
            let (line, col) = frontmatter_key_location(content, parsed.frontmatter_start, "name");
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "KR-PW-007",
                    t!("rules.kr_pw_007.message", name = name.trim()),
                )
                .with_suggestion(t!("rules.kr_pw_007.suggestion")),
            );
        }

        // KR-PW-008: Secrets in power body (literals only, not code identifiers)
        if config.is_rule_enabled("KR-PW-008")
            && parsed.frontmatter.is_some()
            && let Some((offset, _marker)) = find_secret(&parsed.body)
        {
            let (line, col) = line_col_at_offset(content, parsed.body_start + offset);
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "KR-PW-008",
                    t!("rules.kr_pw_008.message"),
                )
                .with_suggestion(t!("rules.kr_pw_008.suggestion")),
            );
        }

        diagnostics
    }
}

// ---------------------------------------------------------------------------
// plugin.json
// ---------------------------------------------------------------------------

/// Validates an Agent Plugins `plugin.json` power manifest. Works for any
/// path: the power root is the manifest's directory.
// Used once the integrator routes plugin.json powers to KiroPowerPluginValidator.
#[allow(dead_code)]
pub struct KiroPowerPluginValidator;

// Used once the integrator routes plugin.json powers to KiroPowerPluginValidator.
#[allow(dead_code)]
fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

// Used once the integrator routes plugin.json powers to KiroPowerPluginValidator.
#[allow(dead_code)]
struct Manifest<'a> {
    path: &'a Path,
    content: &'a str,
    members: Vec<JsonMember>,
}

// Used once the integrator routes plugin.json powers to KiroPowerPluginValidator.
#[allow(dead_code)]
impl Manifest<'_> {
    /// The last member with `key` (JSON.parse keeps the last duplicate).
    fn member(&self, key: &str) -> Option<&JsonMember> {
        self.members.iter().rev().find(|member| member.key == key)
    }

    fn key_location(&self, key: &str) -> (usize, usize) {
        let offset = self.member(key).map_or(0, |member| member.key_start);
        line_col_at_offset(self.content, offset)
    }

    fn value_location(&self, key: &str) -> (usize, usize) {
        match self.member(key) {
            Some(member) => line_col_at_offset(self.content, member.value_start),
            None => (1, 1),
        }
    }

    fn diagnostic(
        &self,
        level: crate::diagnostics::DiagnosticLevel,
        (line, col): (usize, usize),
        rule: &str,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Diagnostic {
        use crate::diagnostics::DiagnosticLevel;
        let diagnostic = match level {
            DiagnosticLevel::Error => {
                Diagnostic::error(self.path.to_path_buf(), line, col, rule, message)
            }
            DiagnosticLevel::Warning => {
                Diagnostic::warning(self.path.to_path_buf(), line, col, rule, message)
            }
            DiagnosticLevel::Info => {
                Diagnostic::info(self.path.to_path_buf(), line, col, rule, message)
            }
        };
        diagnostic.with_suggestion(suggestion)
    }
}

impl Validator for KiroPowerPluginValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: PLUGIN_RULE_IDS,
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        use crate::diagnostics::DiagnosticLevel::{Error, Info, Warning};
        let mut diagnostics = Vec::new();

        // KR-PW-009: Kiro JSON.parses the manifest and needs an object
        let parsed = match parse_plugin_manifest(content) {
            Ok(parsed) => parsed,
            Err(error) => {
                if config.is_rule_enabled("KR-PW-009") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            error.line,
                            error.column,
                            "KR-PW-009",
                            t!("rules.kr_pw_009.message", error = error.message.as_str()),
                        )
                        .with_suggestion(t!("rules.kr_pw_009.suggestion")),
                    );
                }
                return diagnostics;
            }
        };
        let Some(object) = parsed.value.as_object() else {
            if config.is_rule_enabled("KR-PW-009") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        1,
                        "KR-PW-009",
                        t!(
                            "rules.kr_pw_009_object.message",
                            kind = json_type_name(&parsed.value)
                        ),
                    )
                    .with_suggestion(t!("rules.kr_pw_009_object.suggestion")),
                );
            }
            return diagnostics;
        };
        let manifest = Manifest {
            path,
            content,
            members: parsed.members,
        };

        // KR-PW-010: $schema must name Agent Plugins 1.x, or Kiro rejects the power
        if config.is_rule_enabled("KR-PW-010") {
            match object.get("$schema") {
                None => diagnostics.push(manifest.diagnostic(
                    Error,
                    (1, 1),
                    "KR-PW-010",
                    t!("rules.kr_pw_010_missing.message"),
                    t!("rules.kr_pw_010_missing.suggestion"),
                )),
                Some(schema)
                    if !schema
                        .as_str()
                        .is_some_and(|s| plugin_schema_pattern().is_match(s)) =>
                {
                    let shown = schema
                        .as_str()
                        .map_or_else(|| schema.to_string(), str::to_string);
                    diagnostics.push(manifest.diagnostic(
                        Error,
                        manifest.value_location("$schema"),
                        "KR-PW-010",
                        t!("rules.kr_pw_010.message", schema = shown),
                        t!("rules.kr_pw_010.suggestion"),
                    ));
                }
                Some(_) => {}
            }
        }

        // KR-PW-011: name must be a valid Agent Plugins name, or Kiro rejects the power
        if config.is_rule_enabled("KR-PW-011") {
            match object.get("name") {
                None => diagnostics.push(manifest.diagnostic(
                    Error,
                    (1, 1),
                    "KR-PW-011",
                    t!("rules.kr_pw_011_missing.message"),
                    t!("rules.kr_pw_011.suggestion"),
                )),
                Some(name) if !name.as_str().is_some_and(is_valid_plugin_name) => {
                    let shown = name
                        .as_str()
                        .map_or_else(|| name.to_string(), str::to_string);
                    diagnostics.push(manifest.diagnostic(
                        Error,
                        manifest.value_location("name"),
                        "KR-PW-011",
                        t!("rules.kr_pw_011.message", name = shown),
                        t!("rules.kr_pw_011.suggestion"),
                    ));
                }
                Some(_) => {}
            }
        }

        // KR-PW-012: keys outside the manifest are ignored by Kiro and rejected
        // by the Agent Plugins schema (additionalProperties: false)
        if config.is_rule_enabled("KR-PW-012") {
            for key in object.keys() {
                if PLUGIN_MANIFEST_FIELDS.contains(&key.as_str()) {
                    continue;
                }
                let suggestion_key = match key.as_str() {
                    "mcpServers" => "kr_pw_012_mcp",
                    "skills" | "steering" | "hooks" | "agents" | "commands" => {
                        "kr_pw_012_component"
                    }
                    _ => "kr_pw_012",
                };
                let suggestion_key = format!("rules.{suggestion_key}.suggestion");
                diagnostics.push(manifest.diagnostic(
                    Warning,
                    manifest.key_location(key),
                    "KR-PW-012",
                    t!("rules.kr_pw_012.message", field = key.as_str()),
                    t!(&suggestion_key, field = key.as_str()),
                ));
            }
        }

        // KR-PW-013: values Kiro ignores because of their type
        if config.is_rule_enabled("KR-PW-013") {
            let mut ignored = |location: (usize, usize), field: String, problem: String| {
                diagnostics.push(manifest.diagnostic(
                    Warning,
                    location,
                    "KR-PW-013",
                    t!(
                        "rules.kr_pw_013.message",
                        field = field.as_str(),
                        problem = problem.as_str()
                    ),
                    t!("rules.kr_pw_013.suggestion", field = field.as_str()),
                ));
            };
            for field in [
                "version",
                "description",
                "displayName",
                "homepage",
                "repository",
                "license",
            ] {
                if let Some(value) = object.get(field)
                    && !value.is_string()
                {
                    ignored(
                        manifest.value_location(field),
                        field.to_string(),
                        format!("it is {}, not a string", json_type_name(value)),
                    );
                }
            }
            match object.get("author") {
                Some(Value::Object(author)) => {
                    if let Some(extra) = author
                        .keys()
                        .find(|k| !PLUGIN_AUTHOR_FIELDS.contains(&k.as_str()))
                    {
                        ignored(
                            manifest.value_location("author"),
                            "author".to_string(),
                            format!(
                                "it has the unsupported key '{extra}', so the whole author is dropped"
                            ),
                        );
                    } else {
                        for (key, value) in author {
                            if !value.is_string() {
                                ignored(
                                    manifest.value_location("author"),
                                    format!("author.{key}"),
                                    format!("it is {}, not a string", json_type_name(value)),
                                );
                            }
                        }
                    }
                }
                Some(other) => ignored(
                    manifest.value_location("author"),
                    "author".to_string(),
                    format!("it is {}, not an object", json_type_name(other)),
                ),
                None => {}
            }
            match object.get("keywords") {
                Some(Value::Array(items)) => {
                    let spans = manifest.member("keywords").and_then(|member| {
                        json_children(content, member.value_start, member.value_end)
                    });
                    for (index, item) in items.iter().enumerate() {
                        if item.is_string() {
                            continue;
                        }
                        let location = spans
                            .as_ref()
                            .and_then(|spans| spans.get(index))
                            .map_or_else(
                                || manifest.value_location("keywords"),
                                |span| line_col_at_offset(content, span.value_start),
                            );
                        ignored(
                            location,
                            format!("keywords[{index}]"),
                            format!("it is {}, not a string", json_type_name(item)),
                        );
                    }
                }
                Some(other) => ignored(
                    manifest.value_location("keywords"),
                    "keywords".to_string(),
                    format!("it is {}, not an array of strings", json_type_name(other)),
                ),
                None => {}
            }
            if let Some(extensions) = object.get("extensions")
                && !extensions.is_object()
            {
                ignored(
                    manifest.value_location("extensions"),
                    "extensions".to_string(),
                    format!("it is {}, not an object", json_type_name(extensions)),
                );
            }
        }

        // KR-PW-015: what Kiro activates a power by
        if config.is_rule_enabled("KR-PW-015") {
            let description = object.get("description").and_then(Value::as_str);
            if description.is_none_or(|d| d.trim().is_empty()) {
                let location = if object.contains_key("description") {
                    manifest.value_location("description")
                } else {
                    (1, 1)
                };
                diagnostics.push(manifest.diagnostic(
                    Warning,
                    location,
                    "KR-PW-015",
                    t!("rules.kr_pw_015_description.message"),
                    t!("rules.kr_pw_015_description.suggestion"),
                ));
            }
            let keyword_count = object
                .get("keywords")
                .and_then(Value::as_array)
                .map_or(0, |items| {
                    items.iter().filter(|item| item.is_string()).count()
                });
            if keyword_count == 0 {
                let location = if object.contains_key("keywords") {
                    manifest.value_location("keywords")
                } else {
                    (1, 1)
                };
                diagnostics.push(manifest.diagnostic(
                    Warning,
                    location,
                    "KR-PW-015",
                    t!("rules.kr_pw_015_keywords.message"),
                    t!("rules.kr_pw_015_keywords.suggestion"),
                ));
            }
        }

        // KR-PW-016: fields the powers docs list as required (Kiro loads without them)
        if config.is_rule_enabled("KR-PW-016") {
            let author_named = object
                .get("author")
                .and_then(|author| author.get("name"))
                .is_some_and(Value::is_string);
            let mut missing = Vec::new();
            if !object.contains_key("version") {
                missing.push("version");
            }
            if !author_named {
                missing.push("author.name");
            }
            if !missing.is_empty() {
                diagnostics.push(manifest.diagnostic(
                    Info,
                    (1, 1),
                    "KR-PW-016",
                    t!("rules.kr_pw_016.message", fields = missing.join(", ")),
                    t!("rules.kr_pw_016.suggestion"),
                ));
            }
        }

        let fs = config.fs();
        let Some(root) = path.parent() else {
            return diagnostics;
        };

        // KR-PW-014: skills Kiro skips when loading the plugin
        if config.is_rule_enabled("KR-PW-014") {
            let skills_dir = root.join("skills");
            if fs.exists(&skills_dir) && !fs.is_dir(&skills_dir) {
                diagnostics.push(manifest.diagnostic(
                    Error,
                    (1, 1),
                    "KR-PW-014",
                    t!("rules.kr_pw_014_not_dir.message"),
                    t!("rules.kr_pw_014_not_dir.suggestion"),
                ));
            } else if let Ok(entries) = fs.read_dir(&skills_dir) {
                // Declared skill name -> directories, in Kiro's processing order
                let mut declared: BTreeMap<String, Vec<String>> = BTreeMap::new();
                let mut dirs: Vec<_> = entries
                    .into_iter()
                    .filter(|entry| entry.metadata.is_dir)
                    .filter_map(|entry| {
                        let name = entry.path.file_name()?.to_str()?.to_string();
                        Some((name, entry.path))
                    })
                    .collect();
                dirs.sort();
                for (dir_name, dir_path) in dirs {
                    let skill_md = dir_path.join("SKILL.md");
                    if !fs.is_file(&skill_md) {
                        continue;
                    }
                    if !is_valid_skill_name(&dir_name) {
                        diagnostics.push(manifest.diagnostic(
                            Error,
                            (1, 1),
                            "KR-PW-014",
                            t!("rules.kr_pw_014_dir_name.message", dir = dir_name.as_str()),
                            t!("rules.kr_pw_014_dir_name.suggestion"),
                        ));
                        continue;
                    }
                    let declared_name = fs.read_to_string(&skill_md).ok().and_then(|text| {
                        let parts = crate::parsers::frontmatter::split_frontmatter(&text);
                        let value: serde_yaml::Value =
                            serde_yaml::from_str(&parts.frontmatter).ok()?;
                        value.get("name")?.as_str().map(str::to_string)
                    });
                    if let Some(declared_name) = declared_name {
                        declared.entry(declared_name).or_default().push(dir_name);
                    }
                }
                for (name, dirs) in declared {
                    if dirs.len() > 1 {
                        diagnostics.push(manifest.diagnostic(
                            Error,
                            (1, 1),
                            "KR-PW-014",
                            t!(
                                "rules.kr_pw_014_duplicate.message",
                                name = name.as_str(),
                                kept = dirs[0].as_str(),
                                skipped = dirs[1..].join(", ")
                            ),
                            t!("rules.kr_pw_014_duplicate.suggestion"),
                        ));
                    }
                }
            }
        }

        // KR-PW-018: a plugin power's steering comes only from dev.kiro/steering/
        if config.is_rule_enabled("KR-PW-018") && fs.is_dir(&root.join("steering")) {
            diagnostics.push(manifest.diagnostic(
                Warning,
                (1, 1),
                "KR-PW-018",
                t!("rules.kr_pw_018.message"),
                t!("rules.kr_pw_018.suggestion"),
            ));
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;
    use crate::fs::MockFileSystem;
    use std::fs;
    use std::sync::Arc;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = KiroPowerValidator;
        validator.validate(
            Path::new(".kiro/powers/test-power/POWER.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn rule<'a>(diagnostics: &'a [Diagnostic], id: &str) -> Vec<&'a Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == id).collect()
    }

    fn assert_rendered(diagnostics: &[Diagnostic]) {
        for d in diagnostics {
            assert!(
                !d.message.contains("%{") && !d.message.starts_with("rules."),
                "{}",
                d.message
            );
            let suggestion = d.suggestion.as_deref().unwrap_or_default();
            assert!(
                !suggestion.contains("%{") && !suggestion.starts_with("rules."),
                "{suggestion}"
            );
        }
    }

    const VALID: &str =
        "---\nname: valid\ndescription: test\nkeywords:\n  - kiro\n---\n# Body\nValid content.\n";

    #[test]
    fn test_kr_pw_001_missing_frontmatter() {
        let diagnostics = validate("# Missing frontmatter");
        let kr = rule(&diagnostics, "KR-PW-001");
        assert_eq!(kr.len(), 1);
        // Changed expectation: Kiro still loads the power (no description or
        // keywords), so this is a warning, not an error.
        assert_eq!(kr[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_kr_pw_001_missing_activation_fields() {
        let diagnostics = validate("---\nname: sample\n---\n# Sample\n");
        let kr = rule(&diagnostics, "KR-PW-001");
        assert_eq!(kr.len(), 1);
        assert!(
            kr[0].message.contains("description, keywords"),
            "{}",
            kr[0].message
        );
        assert_eq!(kr[0].line, 2);
    }

    #[test]
    fn test_kr_pw_001_name_not_required() {
        // Kiro takes the power name from its directory, not from POWER.md.
        let diagnostics = validate("---\ndescription: d\nkeywords: [a]\n---\n# Body\nText\n");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn test_kr_pw_001_non_array_keywords_count_as_missing() {
        let diagnostics =
            validate("---\nname: x\ndescription: d\nkeywords: solo\n---\n# Body\nText\n");
        let kr = rule(&diagnostics, "KR-PW-001");
        assert_eq!(kr.len(), 1);
        assert_eq!(kr[0].level, DiagnosticLevel::Warning);
        assert!(kr[0].message.contains("keywords"));
    }

    #[test]
    fn test_kr_pw_001_yaml_errors_are_fatal() {
        for content in [
            "---\nname: x\nkeywords: [a\n---\n# Body\n",
            "---\nname: x\ndescription: d\n# Body\n",
        ] {
            let diagnostics = validate(content);
            let kr = rule(&diagnostics, "KR-PW-001");
            assert_eq!(kr.len(), 1, "{content}");
            assert_eq!(kr[0].level, DiagnosticLevel::Error, "{content}");
        }
    }

    #[test]
    fn test_kr_pw_001_frontmatter_not_on_first_line() {
        let diagnostics = validate("\n---\nname: x\ndescription: d\nkeywords: [a]\n---\n# Body\n");
        let kr = rule(&diagnostics, "KR-PW-001");
        assert_eq!(kr.len(), 1);
        assert!(kr[0].message.contains("first line"), "{}", kr[0].message);
    }

    #[test]
    fn test_kr_pw_002_empty_keywords() {
        let diagnostics =
            validate("---\nname: empty-keywords\ndescription: test\nkeywords: []\n---\n# Body\n");
        let kr = rule(&diagnostics, "KR-PW-002");
        assert_eq!(kr.len(), 1);
        assert_eq!(kr[0].line, 4);
    }

    #[test]
    fn test_kr_pw_003_empty_body() {
        let diagnostics =
            validate("---\nname: empty-body\ndescription: test\nkeywords:\n  - one\n---\n");
        assert!(diagnostics.iter().any(|d| d.rule == "KR-PW-003"));
    }

    fn power_dir(
        name: &str,
        mcp: Option<&str>,
        plugin: bool,
    ) -> (tempfile::TempDir, std::path::PathBuf) {
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path().join(".kiro").join("powers").join(name);
        fs::create_dir_all(&dir).unwrap();
        let power_path = dir.join("POWER.md");
        fs::write(&power_path, VALID).unwrap();
        if let Some(mcp) = mcp {
            fs::write(dir.join("mcp.json"), mcp).unwrap();
        }
        if plugin {
            fs::write(dir.join("plugin.json"), "{}").unwrap();
        }
        (temp, power_path)
    }

    #[test]
    fn test_kr_pw_004_adjacent_mcp() {
        for (mcp, expected) in [
            (r#"{"mcpServers":[]}"#, Some(DiagnosticLevel::Warning)),
            (r#"{"servers":{}}"#, Some(DiagnosticLevel::Warning)),
            (r#"{"mcpServers":{}}"#, None),
            (r#"{"mcpServers":{"a":{"command":1}}}"#, None),
            (
                "{\n  // comment\n  \"mcpServers\": {}\n}",
                Some(DiagnosticLevel::Error),
            ),
            (r#"{"mcpServers":{},}"#, Some(DiagnosticLevel::Error)),
        ] {
            let (_temp, power_path) = power_dir("p", Some(mcp), false);
            let diagnostics =
                KiroPowerValidator.validate(&power_path, VALID, &LintConfig::default());
            let kr = rule(&diagnostics, "KR-PW-004");
            assert_eq!(kr.first().map(|d| d.level), expected, "{mcp}");
            assert_rendered(&diagnostics);
        }
    }

    #[test]
    fn test_kr_pw_017_plugin_json_makes_power_md_ignored() {
        let (_temp, power_path) = power_dir("p", Some("{}"), true);
        let diagnostics =
            KiroPowerValidator.validate(&power_path, "# no frontmatter\n", &LintConfig::default());
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].rule, "KR-PW-017");
        assert_rendered(&diagnostics);
    }

    #[test]
    fn test_valid_power_has_no_pw_diagnostics() {
        assert!(validate(VALID).is_empty());
    }

    #[test]
    fn test_kr_pw_005_step_missing_description() {
        let diagnostics = validate(
            "---\nname: empty-step\ndescription: test\nkeywords:\n  - one\n---\n## Step 1\n## Step 2\nSome content here.\n",
        );
        let kr = rule(&diagnostics, "KR-PW-005");
        assert_eq!(kr.len(), 1);
        assert_eq!(kr[0].line, 7);
        // Changed expectation: an empty doc section does not affect loading.
        assert_eq!(kr[0].level, DiagnosticLevel::Info);
        assert!(kr[0].message.contains("'Step 1'"), "{}", kr[0].message);
        assert_rendered(&diagnostics);
    }

    #[test]
    fn test_kr_pw_005_only_step_headings() {
        // Changed expectation: other `##` headings are not steps.
        let diagnostics = validate(
            "---\nname: x\ndescription: d\nkeywords: [a]\n---\n## Overview\n## Usage\ntext\n",
        );
        assert!(rule(&diagnostics, "KR-PW-005").is_empty());
    }

    #[test]
    fn test_kr_pw_005_subsections_and_code_count_as_content() {
        for body in [
            "## Step 1: Setup\n### Details\ntext\n## Step 2\ntext\n",
            "## Step 1\n```bash\n## not a heading\n```\n",
            "```\n## Step 1\n```\ntext\n",
        ] {
            let content = format!("---\nname: x\ndescription: d\nkeywords: [a]\n---\n{body}");
            assert!(rule(&validate(&content), "KR-PW-005").is_empty(), "{body}");
        }
    }

    #[test]
    fn test_kr_pw_005_step_with_description_no_diagnostic() {
        let diagnostics = validate(
            "---\nname: good-steps\ndescription: test\nkeywords:\n  - one\n---\n## Step 1\nThis step does something.\n## Step 2\nThis step does another thing.\n",
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-PW-005"));
    }

    // M14: KR-PW-005 step at end of body with no content after
    #[test]
    fn test_kr_pw_005_step_at_end_of_body() {
        let diagnostics = validate(
            "---\nname: end-step\ndescription: test\nkeywords:\n  - one\n---\n## Step 1\n",
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-PW-005"));
    }

    #[test]
    fn test_kr_pw_006_duplicate_keywords() {
        let diagnostics = validate(
            "---\nname: dupes\ndescription: test\nkeywords:\n  - foo\n  - bar\n  - Foo\n  - bar\n---\n# Body\n",
        );
        let kr = rule(&diagnostics, "KR-PW-006");
        assert_eq!(kr.len(), 1);
        assert!(kr[0].message.ends_with("Foo, bar"), "{}", kr[0].message);
        assert_eq!(kr[0].line, 4);
        assert_rendered(&diagnostics);
    }

    #[test]
    fn test_kr_pw_006_unique_keywords_no_diagnostic() {
        let diagnostics = validate(
            "---\nname: unique\ndescription: test\nkeywords:\n  - foo\n  - bar\n---\n# Body\n",
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-PW-006"));
    }

    #[test]
    fn test_kr_pw_007_invalid_name_characters() {
        for name in ["My Power!", "my_power"] {
            let diagnostics = validate(&format!(
                "---\nname: {name}\ndescription: test\nkeywords:\n  - one\n---\n# Body\n"
            ));
            let kr = rule(&diagnostics, "KR-PW-007");
            assert_eq!(kr.len(), 1, "{name}");
            assert_eq!(kr[0].line, 2);
        }
    }

    #[test]
    fn test_kr_pw_007_valid_names_no_diagnostic() {
        for name in ["my-power-1", "my-power-test-123"] {
            let diagnostics = validate(&format!(
                "---\nname: {name}\ndescription: test\nkeywords:\n  - one\n---\n# Body\n"
            ));
            assert!(diagnostics.iter().all(|d| d.rule != "KR-PW-007"), "{name}");
        }
    }

    #[test]
    fn test_kr_pw_008_secrets_in_body() {
        let diagnostics = validate(
            "---\nname: secrets\ndescription: test\nkeywords:\n  - one\n---\n# Setup\nConfigure with api_key= hardcodedsecret123value\n",
        );
        let kr = rule(&diagnostics, "KR-PW-008");
        assert_eq!(kr.len(), 1);
        assert_eq!((kr[0].line, kr[0].column), (8, 16));
    }

    #[test]
    fn test_kr_pw_008_code_identifiers_not_flagged() {
        // Corpus false positive (RustConn POWER.md): a Rust type annotation.
        let diagnostics = validate(
            "---\nname: x\ndescription: d\nkeywords: [a]\n---\n# Code\n```rust\nlet password: SecretString = SecretString::new(value.into());\n```\n",
        );
        assert!(rule(&diagnostics, "KR-PW-008").is_empty());
    }

    #[test]
    fn test_kr_pw_008_no_secrets_no_diagnostic() {
        assert!(validate(VALID).iter().all(|d| d.rule != "KR-PW-008"));
        let diagnostics = validate(
            "---\nname: template\ndescription: test\nkeywords:\n  - one\n---\nConfigure with api_key= ${API_KEY}\n",
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-PW-008"));
    }

    #[test]
    fn test_metadata() {
        let metadata = KiroPowerValidator.metadata();
        assert_eq!(metadata.name, "KiroPowerValidator");
        assert_eq!(metadata.rule_ids, RULE_IDS);
        let metadata = KiroPowerPluginValidator.metadata();
        assert_eq!(metadata.name, "KiroPowerPluginValidator");
        assert_eq!(metadata.rule_ids, PLUGIN_RULE_IDS);
    }

    // ===== plugin.json =====

    const PLUGIN_PATH: &str = "/powers/supabase/plugin.json";

    /// power-builder's manifest from kirodotdev/powers (valid).
    const POWER_BUILDER: &str = r#"{
  "$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
  "name": "power-builder",
  "version": "1.0.0",
  "description": "Create new Kiro Powers using the Agent Plugins specification.",
  "author": {
    "name": "Kiro Team",
    "url":"https://kiro.dev"
  },
  "keywords": ["kiro power", "power builder"],
  "license": "MIT",
  "repository": "https://github.com/kirodotdev/powers"
}"#;

    fn plugin_config(fs: Arc<MockFileSystem>) -> LintConfig {
        let mut config = LintConfig::default();
        config.set_fs(fs);
        config
    }

    fn validate_plugin(content: &str) -> Vec<Diagnostic> {
        let fs = Arc::new(MockFileSystem::new());
        fs.add_file(PLUGIN_PATH, content);
        KiroPowerPluginValidator.validate(Path::new(PLUGIN_PATH), content, &plugin_config(fs))
    }

    #[test]
    fn test_plugin_valid_manifest_has_no_diagnostics() {
        assert!(validate_plugin(POWER_BUILDER).is_empty());
        // displayName is accepted by Kiro (not in the 1.0.0 schema).
        let with_display = POWER_BUILDER.replacen(
            "\"name\": \"power-builder\",",
            "\"name\": \"power-builder\",\n  \"displayName\": \"Power Builder\",",
            1,
        );
        assert!(validate_plugin(&with_display).is_empty());
    }

    #[test]
    fn test_kr_pw_009_invalid_json_or_not_object() {
        for content in ["{\"name\": \"x\",}", "// c\n{}", "[1]", "\"x\""] {
            let diagnostics = validate_plugin(content);
            assert_eq!(diagnostics.len(), 1, "{content}: {diagnostics:?}");
            assert_eq!(diagnostics[0].rule, "KR-PW-009");
            assert_eq!(diagnostics[0].level, DiagnosticLevel::Error);
            assert_rendered(&diagnostics);
        }
    }

    #[test]
    fn test_kr_pw_010_schema() {
        let missing = POWER_BUILDER.replacen(
            "\"$schema\": \"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json\",\n",
            "",
            1,
        );
        let diagnostics = validate_plugin(&missing);
        assert_eq!(rule(&diagnostics, "KR-PW-010").len(), 1);
        for schema in [
            "https://agent-plugins.org/schemas/2.0.0/plugin.schema.json",
            "https://json.schemastore.org/claude-code-plugin.json",
            "http://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
        ] {
            let content = POWER_BUILDER.replacen(
                "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
                schema,
                1,
            );
            let diagnostics = validate_plugin(&content);
            let kr = rule(&diagnostics, "KR-PW-010");
            assert_eq!(kr.len(), 1, "{schema}");
            assert_eq!((kr[0].line, kr[0].column), (2, 14), "{schema}");
            assert_rendered(&diagnostics);
        }
        let minor = POWER_BUILDER.replacen("1.0.0/plugin", "1.2.10/plugin", 1);
        assert!(validate_plugin(&minor).is_empty());
    }

    #[test]
    fn test_kr_pw_011_name() {
        for (name, line) in [("\"Power_Builder\"", 3), ("7", 3), ("\"a--b\"", 3)] {
            let content = POWER_BUILDER.replacen("\"power-builder\"", name, 1);
            let diagnostics = validate_plugin(&content);
            let kr = rule(&diagnostics, "KR-PW-011");
            assert_eq!(kr.len(), 1, "{name}");
            assert_eq!(kr[0].line, line, "{name}");
            assert_eq!(kr[0].level, DiagnosticLevel::Error);
        }
        let missing = POWER_BUILDER.replacen("\"name\": \"power-builder\",\n", "", 1);
        assert_eq!(rule(&validate_plugin(&missing), "KR-PW-011").len(), 1);
    }

    #[test]
    fn test_kr_pw_012_unknown_fields() {
        let content = POWER_BUILDER.replacen(
            "\"license\": \"MIT\",",
            "\"license\": \"MIT\",\n  \"mcpServers\": {},\n  \"skills\": [\"x\"],\n  \"notes\": \"x\",",
            1,
        );
        let diagnostics = validate_plugin(&content);
        let kr = rule(&diagnostics, "KR-PW-012");
        assert_eq!(kr.len(), 3);
        assert_eq!(kr[0].level, DiagnosticLevel::Warning);
        assert_eq!((kr[0].line, kr[0].column), (12, 3));
        assert!(kr[0].suggestion.as_deref().unwrap().contains("mcp.json"));
        assert_rendered(&diagnostics);
    }

    #[test]
    fn test_kr_pw_013_ignored_values() {
        let content = r#"{
  "$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
  "name": "x",
  "version": 1,
  "description": "d",
  "author": {"name": "a", "company": "c"},
  "keywords": ["ok", 3],
  "extensions": []
}"#;
        let diagnostics = validate_plugin(content);
        let kr = rule(&diagnostics, "KR-PW-013");
        let lines: Vec<_> = kr.iter().map(|d| (d.line, d.column)).collect();
        assert_eq!(lines, [(4, 14), (6, 13), (7, 22), (8, 17)], "{kr:?}");
        assert!(kr[1].message.contains("company"), "{}", kr[1].message);
        assert_rendered(&diagnostics);
        let author_string = POWER_BUILDER.replacen(
            "{\n    \"name\": \"Kiro Team\",\n    \"url\":\"https://kiro.dev\"\n  }",
            "\"Kiro Team\"",
            1,
        );
        assert_eq!(rule(&validate_plugin(&author_string), "KR-PW-013").len(), 1);
    }

    #[test]
    fn test_kr_pw_015_and_016_activation_and_docs_fields() {
        let minimal = r#"{"$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json", "name": "x"}"#;
        let diagnostics = validate_plugin(minimal);
        assert_eq!(rule(&diagnostics, "KR-PW-015").len(), 2);
        let info = rule(&diagnostics, "KR-PW-016");
        assert_eq!(info.len(), 1);
        assert_eq!(info[0].level, DiagnosticLevel::Info);
        assert!(
            info[0].message.contains("version, author.name"),
            "{}",
            info[0].message
        );
        assert_rendered(&diagnostics);
        let empty_keywords = POWER_BUILDER.replacen("[\"kiro power\", \"power builder\"]", "[]", 1);
        assert_eq!(
            rule(&validate_plugin(&empty_keywords), "KR-PW-015").len(),
            1
        );
    }

    #[test]
    fn test_kr_pw_014_and_018_power_layout() {
        let fs = Arc::new(MockFileSystem::new());
        fs.add_file(PLUGIN_PATH, POWER_BUILDER);
        // MockFileSystem lists only explicitly added directories.
        for dir in [
            "skills",
            "skills/setup",
            "skills/setup-copy",
            "skills/Bad_Name",
            "steering",
        ] {
            fs.add_dir(format!("/powers/supabase/{dir}"));
        }
        fs.add_file(
            "/powers/supabase/skills/setup/SKILL.md",
            "---\nname: setup\ndescription: d\n---\nx\n",
        );
        fs.add_file(
            "/powers/supabase/skills/setup-copy/SKILL.md",
            "---\nname: setup\ndescription: d\n---\nx\n",
        );
        fs.add_file(
            "/powers/supabase/skills/Bad_Name/SKILL.md",
            "---\nname: bad\ndescription: d\n---\nx\n",
        );
        fs.add_dir("/powers/supabase/skills/no-skill-md");
        fs.add_file("/powers/supabase/steering/guide.md", "# Guide\n");
        let diagnostics = KiroPowerPluginValidator.validate(
            Path::new(PLUGIN_PATH),
            POWER_BUILDER,
            &plugin_config(fs),
        );
        let kr = rule(&diagnostics, "KR-PW-014");
        assert_eq!(kr.len(), 2, "{diagnostics:?}");
        assert!(kr.iter().any(|d| d.message.contains("skills/Bad_Name")));
        assert!(kr.iter().any(|d| d.message.contains("setup-copy")));
        assert_eq!(rule(&diagnostics, "KR-PW-018").len(), 1);
        assert_rendered(&diagnostics);

        let fs = Arc::new(MockFileSystem::new());
        fs.add_file(PLUGIN_PATH, POWER_BUILDER);
        fs.add_file("/powers/supabase/skills", "not a directory");
        let diagnostics = KiroPowerPluginValidator.validate(
            Path::new(PLUGIN_PATH),
            POWER_BUILDER,
            &plugin_config(fs),
        );
        assert_eq!(rule(&diagnostics, "KR-PW-014").len(), 1);
    }
}
