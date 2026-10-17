//! Gemini CLI agent definition validation (GM-AG-*, and GM-011 for the
//! names of an agent's MCP servers).
//!
//! Validates the YAML frontmatter of `.gemini/agents/*.md` files (and an
//! extension's `agents/*.md`) against the schema in
//! `packages/core/src/agents/agentLoader.ts` in the gemini-cli repo
//! (verified on main, 2026-10-17):
//!
//! - GM-AG-001: the `mcp_servers.*.auth` block added in gemini-cli v0.39.0
//!   (google-gemini/gemini-cli#24770).
//! - GM-AG-002: the file does not load: no frontmatter, YAML errors, a bad
//!   `name`, a missing `description` (local agents), a remote agent without
//!   exactly one valid agent card, wrongly typed fields.
//! - GM-AG-003: keys outside the strict local or remote schema.
//! - GM-AG-004: `max_turns` / `timeout_mins` (positive integers) and
//!   `temperature` (documented range 0.0 to 2.0).
//! - GM-AG-005: literal secrets in a remote agent's `auth`.
//! - GM-AG-006: a local agent that inherits every tool.
//!
//! `loadAgentsFromDirectory` skips files whose name starts with `_`, so
//! they get none of these rules.
//!
//! ## Auth block schema
//!
//! `auth` is `z.union([...])` of two `z.object`s, distinguished by `type`:
//!
//! 1. `type: "google-credentials"` - Application Default Credentials
//!    - `scopes: [string]` (optional)
//!
//! 2. `type: "oauth"` - Standard OAuth 2.0 flow, all optional:
//!    `client_id`, `client_secret`, `token_param_name` (strings),
//!    `scopes`, `audiences` (string arrays), `authorization_url`,
//!    `token_url`, `issuer`, `redirect_uri`, `registration_url` (URLs)
//!
//! A wrong `type`, a wrong value type or a malformed URL fails validation and
//! the agent is not loaded (error). The objects are not `.strict()`, so an
//! unknown field is dropped silently (warning).
//!

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    rules::{
        Validator, ValidatorMetadata,
        gemini_mcp::{gm_011_texts, underscore_free_name},
        seems_plaintext_secret,
    },
};
use rust_i18n::t;
use serde_yaml::Value as Yaml;
use std::ops::Range;
use std::path::Path;

/// Rules this validator owns. It also emits GM-011 for the names of an
/// agent's MCP servers; GeminiSettingsValidator lists that rule.
const RULE_IDS: &[&str] = &[
    "GM-AG-001",
    "GM-AG-002",
    "GM-AG-003",
    "GM-AG-004",
    "GM-AG-005",
    "GM-AG-006",
];

/// `localAgentSchema` keys (`z.object({...}).strict()`).
const LOCAL_KEYS: &[&str] = &[
    "kind",
    "name",
    "description",
    "display_name",
    "tools",
    "mcp_servers",
    "model",
    "temperature",
    "max_turns",
    "timeout_mins",
];

/// `remoteAgentUrlSchema` / `remoteAgentJsonSchema` keys (both `.strict()`).
const REMOTE_KEYS: &[&str] = &[
    "kind",
    "name",
    "description",
    "display_name",
    "auth",
    "agent_card_url",
    "agent_card_json",
];

/// Keys `guessIntendedKind` reads as a remote agent.
const REMOTE_ONLY_KEYS: &[&str] = &["agent_card_url", "agent_card_json", "auth"];

/// Secret values of a remote agent's `auth` that "support dynamic
/// resolution" (`$ENV_VAR`, `!command`), per auth type. `username` is not
/// a secret, and OAuth `client_secret` does not support `$` / `!` values.
const AUTH_SECRET_FIELDS: &[(&str, &[&str])] = &[
    ("apiKey", &["key"]),
    ("http", &["token", "password", "value"]),
];

/// "Model temperature (0.0 - 2.0)."
const TEMPERATURE_RANGE: (f64, f64) = (0.0, 2.0);

/// Allowed auth `type` discriminator values.
const VALID_AUTH_TYPES: &[&str] = &["google-credentials", "oauth"];

/// `type: oauth` fields of agentLoader.ts `mcpServerSchema.auth`.
const OAUTH_STRING_FIELDS: [&str; 3] = ["client_id", "client_secret", "token_param_name"];
const OAUTH_ARRAY_FIELDS: [&str; 2] = ["scopes", "audiences"];
const OAUTH_URL_FIELDS: [&str; 5] = [
    "authorization_url",
    "token_url",
    "issuer",
    "redirect_uri",
    "registration_url",
];

pub struct GeminiAgentValidator;

impl Validator for GeminiAgentValidator {
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
        // `loadAgentsFromDirectory` reads only `.md` files whose name does
        // not start with `_`: "Ignores files starting with _".
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('_'))
        {
            return Vec::new();
        }
        let mut diagnostics = check_definition(path, content, config);
        diagnostics.extend(check_auth_blocks(path, content, config));
        diagnostics
    }
}

/// GM-AG-001: `mcp_servers.*.auth` blocks.
fn check_auth_blocks(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if !config.is_rule_enabled("GM-AG-001") {
        return diagnostics;
    }

    // Gemini agent files have YAML frontmatter between `---` markers. A
    // missing frontmatter or malformed YAML is GM-AG-002.
    let Some(frontmatter_raw) = extract_frontmatter(content) else {
        return diagnostics;
    };

    let parsed: serde_yaml::Value = match serde_yaml::from_str(frontmatter_raw) {
        Ok(v) => v,
        Err(_) => return diagnostics,
    };

    // Walk mcp_servers.*.auth and validate each auth block.
    let Some(mcp_servers) = parsed
        .get("mcp_servers")
        .and_then(serde_yaml::Value::as_mapping)
    else {
        return diagnostics;
    };

    for (server_key, server_value) in mcp_servers {
        let server_name = server_key.as_str().unwrap_or("<unnamed>").to_string();
        let Some(server_obj) = server_value.as_mapping() else {
            continue;
        };
        let Some(auth) = server_obj.get("auth") else {
            continue;
        };

        validate_auth_block(path, content, &server_name, auth, &mut diagnostics);
    }

    diagnostics
}

fn validate_auth_block(
    path: &Path,
    content: &str,
    server: &str,
    auth: &serde_yaml::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let line = find_auth_line(content, server).unwrap_or(1);

    // Must be a mapping.
    let Some(auth_obj) = auth.as_mapping() else {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                0,
                "GM-AG-001",
                t!("rules.gm_ag_001.not_object", server = server),
            )
            .with_suggestion(t!("rules.gm_ag_001.suggestion")),
        );
        return;
    };

    // Discriminator check: `type` must be present, a string, and one of
    // the valid literals. Non-string and missing get different diagnostics
    // because "type is a number" is a different mistake from "type is
    // absent entirely".
    let Some(type_entry) = auth_obj.get("type") else {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                0,
                "GM-AG-001",
                t!("rules.gm_ag_001.missing_type", server = server),
            )
            .with_suggestion(t!("rules.gm_ag_001.suggestion")),
        );
        return;
    };
    let Some(type_str) = type_entry.as_str().map(str::to_string) else {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                0,
                "GM-AG-001",
                t!(
                    "rules.gm_ag_001.not_string",
                    server = server,
                    field = "type"
                ),
            )
            .with_suggestion(t!("rules.gm_ag_001.suggestion")),
        );
        return;
    };

    if !VALID_AUTH_TYPES.contains(&type_str.as_str()) {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                0,
                "GM-AG-001",
                t!(
                    "rules.gm_ag_001.invalid_type",
                    server = server,
                    value = type_str.as_str(),
                    valid = VALID_AUTH_TYPES.join(", ")
                ),
            )
            .with_suggestion(t!("rules.gm_ag_001.suggestion")),
        );
        return;
    }

    // Per-variant schema checks.
    match type_str.as_str() {
        "google-credentials" => {
            // Only `scopes` is read beyond `type`. Report other fields (for
            // example OAuth fields pasted under google-credentials): zod
            // drops them, so they have no effect.
            for (k, _) in auth_obj {
                let Some(key_str) = k.as_str() else { continue };
                if !matches!(key_str, "type" | "scopes") {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            line,
                            0,
                            "GM-AG-001",
                            t!(
                                "rules.gm_ag_001.unknown_field_google",
                                server = server,
                                field = key_str
                            ),
                        )
                        .with_suggestion(t!("rules.gm_ag_001.suggestion")),
                    );
                }
            }
            // `scopes`, if present, must be a list of strings.
            if let Some(scopes) = auth_obj.get("scopes") {
                validate_string_array(path, line, server, "scopes", scopes, diagnostics);
            }
        }
        "oauth" => {
            for (k, _) in auth_obj {
                let Some(key_str) = k.as_str() else { continue };
                if key_str != "type"
                    && !OAUTH_STRING_FIELDS.contains(&key_str)
                    && !OAUTH_ARRAY_FIELDS.contains(&key_str)
                    && !OAUTH_URL_FIELDS.contains(&key_str)
                {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            line,
                            0,
                            "GM-AG-001",
                            t!(
                                "rules.gm_ag_001.unknown_field_oauth",
                                server = server,
                                field = key_str
                            ),
                        )
                        .with_suggestion(t!("rules.gm_ag_001.suggestion")),
                    );
                }
            }
            // String fields, when present, must be strings.
            for string_key in OAUTH_STRING_FIELDS {
                if let Some(v) = auth_obj.get(string_key)
                    && v.as_str().is_none()
                {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            0,
                            "GM-AG-001",
                            t!(
                                "rules.gm_ag_001.not_string",
                                server = server,
                                field = string_key
                            ),
                        )
                        .with_suggestion(t!("rules.gm_ag_001.suggestion")),
                    );
                }
            }
            // URL fields, when present, must look like URLs (basic sanity -
            // upstream uses Zod's .url() check which rejects non-HTTPS/non-
            // HTTP and malformed strings).
            for url_key in OAUTH_URL_FIELDS {
                if let Some(v) = auth_obj.get(url_key) {
                    match v.as_str() {
                        None => diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                line,
                                0,
                                "GM-AG-001",
                                t!(
                                    "rules.gm_ag_001.not_string",
                                    server = server,
                                    field = url_key
                                ),
                            )
                            .with_suggestion(t!("rules.gm_ag_001.suggestion")),
                        ),
                        Some(s) if !looks_like_url(s) => diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                line,
                                0,
                                "GM-AG-001",
                                t!(
                                    "rules.gm_ag_001.invalid_url",
                                    server = server,
                                    field = url_key,
                                    value = s
                                ),
                            )
                            .with_suggestion(t!("rules.gm_ag_001.suggestion")),
                        ),
                        _ => {}
                    }
                }
            }
            for array_key in OAUTH_ARRAY_FIELDS {
                if let Some(value) = auth_obj.get(array_key) {
                    validate_string_array(path, line, server, array_key, value, diagnostics);
                }
            }
        }
        _ => unreachable!("type_str already validated against VALID_AUTH_TYPES"),
    }
}

fn validate_string_array(
    path: &Path,
    line: usize,
    server: &str,
    field: &str,
    value: &serde_yaml::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(seq) = value.as_sequence() else {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                0,
                "GM-AG-001",
                t!(
                    "rules.gm_ag_001.array_not_array",
                    server = server,
                    field = field
                ),
            )
            .with_suggestion(t!("rules.gm_ag_001.suggestion")),
        );
        return;
    };
    for (idx, scope) in seq.iter().enumerate() {
        if scope.as_str().is_none() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    0,
                    "GM-AG-001",
                    t!(
                        "rules.gm_ag_001.array_item_not_string",
                        server = server,
                        field = field,
                        index = idx.to_string().as_str()
                    ),
                )
                .with_suggestion(t!("rules.gm_ag_001.suggestion")),
            );
        }
    }
}

/// Basic URL sanity: must start with `http://` or `https://` and have a
/// host segment after the scheme. Deliberately lenient - upstream uses
/// Zod's URL validator which is also lenient about paths and query strings.
fn looks_like_url(s: &str) -> bool {
    if let Some(rest) = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
    {
        // There must be at least one character for the host.
        !rest.is_empty()
            && !rest.starts_with('/')
            && !rest.starts_with('?')
            && !rest.starts_with('#')
    } else {
        false
    }
}

/// Extract the YAML frontmatter block (between the first two `---` lines).
/// Returns the YAML body only; returns None if no frontmatter is present.
///
/// Walks lines once tracking a running byte offset, so locating the closing
/// marker is O(N) rather than O(N²) from re-splitting + summing on each hit.
fn extract_frontmatter(content: &str) -> Option<&str> {
    let stripped = content.strip_prefix('\u{FEFF}').unwrap_or(content);
    let rest = stripped
        .strip_prefix("---\n")
        .or_else(|| stripped.strip_prefix("---\r\n"))?;
    let mut offset = 0usize;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            return Some(&rest[..offset]);
        }
        offset += line.len();
    }
    None
}

/// 1-indexed line of the `auth:` key under `mcp_servers.<server>:` in a
/// YAML document. Falls back to 1 if the line cannot be located. This is a
/// best-effort scanner: when the server name has special YAML characters or
/// the auth block is on a single line (flow style), we accept a less-
/// precise line number.
fn find_auth_line(content: &str, server: &str) -> Option<usize> {
    let mut in_mcp_servers = false;
    let mut in_server = false;
    let server_prefix = format!("{server}:");
    let server_quoted_prefix = format!("\"{server}\":");

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        let lead_ws_len = line.len() - trimmed.len();

        if trimmed == "mcp_servers:" {
            in_mcp_servers = true;
            in_server = false;
            continue;
        }
        if in_mcp_servers {
            // Leave mcp_servers section when we hit a top-level key (zero indent).
            if lead_ws_len == 0 && !trimmed.starts_with('#') && !trimmed.is_empty() {
                in_mcp_servers = false;
                continue;
            }
            if trimmed.starts_with(&server_prefix) || trimmed.starts_with(&server_quoted_prefix) {
                in_server = true;
                continue;
            }
            if in_server {
                // Sibling server starts - stop looking.
                if lead_ws_len <= 2 && trimmed.ends_with(':') && !trimmed.starts_with("auth:") {
                    in_server = false;
                    continue;
                }
                if trimmed.starts_with("auth:") {
                    return Some(idx + 1);
                }
            }
        }
    }
    None
}

// ============================================================================
// Agent definition schema (GM-AG-002 .. GM-AG-006, GM-011)
// ============================================================================

/// The frontmatter as agentLoader.ts finds it, with `FRONTMATTER_REGEX`
/// (skillLoader.ts): `/^\uFEFF?---[ \t]*\r?\n([\s\S]*?)\r?\n---[ \t]*(?:\r?\n([\s\S]*))?$/`.
/// Returns the YAML text and its byte offset in `content`.
fn loader_frontmatter(content: &str) -> Option<(&str, usize)> {
    let bom = if content.starts_with('\u{FEFF}') {
        '\u{FEFF}'.len_utf8()
    } else {
        0
    };
    let rest = &content[bom..];
    let is_marker = |line: &str| {
        let line = line.strip_suffix('\r').unwrap_or(line);
        line.strip_prefix("---")
            .is_some_and(|tail| tail.chars().all(|c| c == ' ' || c == '\t'))
    };
    let first_end = rest.find('\n')?;
    if !is_marker(&rest[..first_end]) {
        return None;
    }
    let start = first_end + 1;
    let mut search = start;
    while let Some(found) = rest.get(search..)?.find('\n') {
        let newline = search + found;
        let line_start = newline + 1;
        let line_end = rest[line_start..]
            .find('\n')
            .map_or(rest.len(), |end| line_start + end);
        if is_marker(&rest[line_start..line_end]) {
            let yaml_end = if newline > start && rest.as_bytes()[newline - 1] == b'\r' {
                newline - 1
            } else {
                newline
            };
            return Some((&rest[start..yaml_end], bom + start));
        }
        search = line_start;
    }
    None
}

/// Lines of the frontmatter, to point diagnostics at keys (serde_yaml keeps
/// no positions).
struct FrontmatterLines<'a> {
    /// 1-based file line of the first YAML line.
    first_line: usize,
    lines: Vec<&'a str>,
}

impl<'a> FrontmatterLines<'a> {
    fn new(content: &'a str, yaml: &'a str, offset: usize) -> Self {
        Self {
            first_line: content[..offset].matches('\n').count() + 1,
            lines: yaml.lines().collect(),
        }
    }

    /// Indentation of a line, counting a `- ` list marker as indentation
    /// (so `- name: x` and `  kind: y` of one list item line up).
    fn indent(line: &str) -> usize {
        let spaces = line.len() - line.trim_start_matches(' ').len();
        let rest = &line[spaces..];
        if let Some(after) = rest.strip_prefix("- ").or_else(|| rest.strip_prefix("-\t")) {
            spaces + 2 + (after.len() - after.trim_start().len())
        } else {
            spaces
        }
    }

    /// The text of a line after its indentation and list marker.
    fn body(line: &str) -> &str {
        &line[Self::indent(line).min(line.len())..]
    }

    fn is_content(line: &str) -> bool {
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.starts_with('#')
    }

    /// Line ranges of the items of a top-level list (lines starting `-`).
    fn list_items(&self) -> Vec<Range<usize>> {
        let starts: Vec<usize> = (0..self.lines.len())
            .filter(|&i| self.lines[i].starts_with('-'))
            .collect();
        starts
            .iter()
            .enumerate()
            .map(|(n, &start)| start..starts.get(n + 1).copied().unwrap_or(self.lines.len()))
            .collect()
    }

    /// 1-based (line, column) of the key at `path` (`["auth", "token"]`)
    /// inside `range`, or of the range's first line when it is not found.
    fn position(&self, path: &[&str], range: Range<usize>) -> (usize, usize) {
        let fallback = (self.first_line + range.start, 1);
        let mut range = range;
        let mut found = None;
        for key in path {
            let Some(level) = range
                .clone()
                .find(|&i| Self::is_content(self.lines[i]))
                .map(|i| Self::indent(self.lines[i]))
            else {
                return found.unwrap_or(fallback);
            };
            let hit = range.clone().find(|&i| {
                let line = self.lines[i];
                Self::is_content(line)
                    && Self::indent(line) == level
                    && Self::starts_with_key(Self::body(line), key)
            });
            let Some(i) = hit else {
                return found.unwrap_or(fallback);
            };
            found = Some((self.first_line + i, Self::indent(self.lines[i]) + 1));
            let end = (i + 1..range.end)
                .find(|&j| Self::is_content(self.lines[j]) && Self::indent(self.lines[j]) <= level)
                .unwrap_or(range.end);
            range = i + 1..end;
        }
        found.unwrap_or(fallback)
    }

    /// 1-based (line, column) of the first occurrence of `needle`, or of the
    /// first YAML line.
    fn find_text(&self, needle: &str) -> (usize, usize) {
        self.lines
            .iter()
            .enumerate()
            .find_map(|(i, line)| {
                line.find(needle)
                    .map(|byte| (self.first_line + i, line[..byte].chars().count() + 1))
            })
            .unwrap_or((self.first_line, 1))
    }

    fn starts_with_key(body: &str, key: &str) -> bool {
        [key.to_string(), format!("\"{key}\""), format!("'{key}'")]
            .iter()
            .any(|form| {
                body.strip_prefix(form.as_str())
                    .is_some_and(|rest| rest.trim_start().starts_with(':'))
            })
    }
}

/// Local YAML tags (`!name`) anywhere in a value, which js-yaml's default
/// schema does not know. Core `!!` tags are left alone.
fn local_tags(value: &Yaml) -> Vec<String> {
    let mut tags = Vec::new();
    let mut stack = vec![value];
    while let Some(value) = stack.pop() {
        match value {
            Yaml::Tagged(tagged) => {
                let tag = tagged.tag.to_string();
                if !tag.starts_with("!!") && !tag.starts_with("tag:yaml.org,2002:") {
                    tags.push(tag);
                }
                stack.push(&tagged.value);
            }
            Yaml::Sequence(items) => stack.extend(items.iter()),
            Yaml::Mapping(map) => {
                for (key, item) in map {
                    stack.push(key);
                    stack.push(item);
                }
            }
            _ => {}
        }
    }
    tags.sort();
    tags.dedup();
    tags
}

fn yaml_type(value: &Yaml) -> &'static str {
    match value {
        Yaml::Null => "null",
        Yaml::Bool(_) => "boolean",
        Yaml::Number(_) => "number",
        Yaml::String(_) => "string",
        Yaml::Sequence(_) => "list",
        Yaml::Mapping(_) => "mapping",
        Yaml::Tagged(_) => "tagged value",
    }
}

fn yaml_text(value: &Yaml) -> String {
    match value {
        Yaml::String(s) => s.clone(),
        Yaml::Number(n) => n.to_string(),
        Yaml::Bool(b) => b.to_string(),
        other => yaml_type(other).to_string(),
    }
}

/// `z.number().int().positive()`: a whole number above zero (`10.0` is an
/// integer to JavaScript).
fn is_positive_integer(value: &Yaml) -> bool {
    match value {
        Yaml::Number(n) => {
            if let Some(i) = n.as_i64() {
                i > 0
            } else if let Some(u) = n.as_u64() {
                u > 0
            } else {
                n.as_f64()
                    .is_some_and(|f| f > 0.0 && f.fract() == 0.0 && f.is_finite())
            }
        }
        _ => false,
    }
}

/// `z.string().url()`, leniently: a scheme followed by `:` and more text.
fn is_url(text: &str) -> bool {
    text.split_once(':').is_some_and(|(scheme, rest)| {
        !rest.is_empty()
            && scheme
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AgentKind {
    Local,
    Remote,
}

impl AgentKind {
    fn label(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote => "remote",
        }
    }
}

struct DefinitionCx<'a, 'c> {
    path: &'a Path,
    config: &'a PerFileLintConfig<'c>,
    lines: FrontmatterLines<'a>,
    diagnostics: Vec<Diagnostic>,
}

impl DefinitionCx<'_, '_> {
    fn push(
        &mut self,
        level: DiagnosticLevel,
        rule: &str,
        (line, column): (usize, usize),
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) {
        if !self.config.is_rule_enabled(rule) {
            return;
        }
        let path = self.path.to_path_buf();
        let diagnostic = match level {
            DiagnosticLevel::Error => Diagnostic::error(path, line, column, rule, message),
            DiagnosticLevel::Warning => Diagnostic::warning(path, line, column, rule, message),
            DiagnosticLevel::Info => Diagnostic::info(path, line, column, rule, message),
        };
        self.diagnostics
            .push(diagnostic.with_suggestion(suggestion));
    }

    fn invalid(&mut self, at: (usize, usize), message: impl Into<String>) {
        self.push(
            DiagnosticLevel::Error,
            "GM-AG-002",
            at,
            message,
            t!("rules.gm_ag_002.suggestion"),
        );
    }

    fn at(&self, path: &[&str], range: &Range<usize>) -> (usize, usize) {
        self.lines.position(path, range.clone())
    }

    /// One agent definition: the whole frontmatter mapping, or an item of a
    /// list of remote agents (`range` is its lines).
    fn errors(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .count()
    }

    fn check_agent(&mut self, agent: &serde_yaml::Mapping, range: Range<usize>, in_list: bool) {
        let errors_before = self.errors();
        let get = |key: &str| agent.get(key);
        let has = |keys: &[&str]| keys.iter().any(|key| agent.contains_key(*key));

        // kind: `z.literal('local')` or `z.literal('remote')`, optional.
        let kind = match get("kind") {
            None => {
                // `guessIntendedKind`, and the list format is remote-only.
                if in_list
                    || (has(REMOTE_ONLY_KEYS)
                        && !has(&[
                            "tools",
                            "mcp_servers",
                            "model",
                            "temperature",
                            "max_turns",
                            "timeout_mins",
                        ]))
                {
                    AgentKind::Remote
                } else {
                    AgentKind::Local
                }
            }
            Some(Yaml::String(kind)) if kind == "local" => AgentKind::Local,
            Some(Yaml::String(kind)) if kind == "remote" => AgentKind::Remote,
            Some(other) => {
                let at = self.at(&["kind"], &range);
                self.invalid(
                    at,
                    t!(
                        "rules.gm_ag_002.bad_kind",
                        value = yaml_text(other).as_str()
                    ),
                );
                if has(REMOTE_ONLY_KEYS) {
                    AgentKind::Remote
                } else {
                    AgentKind::Local
                }
            }
        };
        if in_list && kind == AgentKind::Local {
            let at = self.at(&["kind"], &range);
            self.invalid(at, t!("rules.gm_ag_002.list_local"));
            return;
        }

        // name: `z.string().regex(/^[a-z0-9-_]+$/, 'Name must be a valid slug')`.
        let name = match get("name") {
            None | Some(Yaml::Null) => {
                let at = (self.lines.first_line + range.start, 1);
                self.invalid(at, t!("rules.gm_ag_002.missing", field = "name"));
                None
            }
            Some(Yaml::String(name)) => {
                let valid = !name.is_empty()
                    && name.chars().all(|c| {
                        c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_'
                    });
                if !valid {
                    let at = self.at(&["name"], &range);
                    self.invalid(at, t!("rules.gm_ag_002.bad_name", name = name.as_str()));
                }
                Some(name.clone())
            }
            Some(other) => {
                let at = self.at(&["name"], &range);
                self.invalid(
                    at,
                    t!(
                        "rules.gm_ag_002.not_string",
                        field = "name",
                        found = yaml_type(other)
                    ),
                );
                None
            }
        };
        let label = name.clone().unwrap_or_else(|| "<unnamed>".to_string());

        // description: `z.string().min(1)` (local), `z.string().optional()`
        // (remote).
        match (get("description"), kind) {
            (None | Some(Yaml::Null), AgentKind::Local) => {
                let at = match get("description") {
                    Some(_) => self.at(&["description"], &range),
                    None => (self.lines.first_line + range.start, 1),
                };
                self.invalid(at, t!("rules.gm_ag_002.missing", field = "description"));
            }
            (None, AgentKind::Remote) => {}
            (Some(Yaml::String(text)), AgentKind::Local) if text.is_empty() => {
                let at = self.at(&["description"], &range);
                self.invalid(at, t!("rules.gm_ag_002.missing", field = "description"));
            }
            (Some(Yaml::String(_)), _) => {}
            (Some(other), _) => {
                let at = self.at(&["description"], &range);
                self.invalid(
                    at,
                    t!(
                        "rules.gm_ag_002.not_string",
                        field = "description",
                        found = yaml_type(other)
                    ),
                );
            }
        }

        // Other string fields.
        let string_fields: &[&str] = match kind {
            AgentKind::Local => &["display_name", "model"],
            AgentKind::Remote => &["display_name"],
        };
        for field in string_fields {
            if let Some(value) = get(field)
                && !matches!(value, Yaml::String(_))
            {
                let at = self.at(&[field], &range);
                self.invalid(
                    at,
                    t!(
                        "rules.gm_ag_002.not_string",
                        field = *field,
                        found = yaml_type(value)
                    ),
                );
            }
        }

        // GM-AG-003: the schemas are `.strict()`.
        let allowed = match kind {
            AgentKind::Local => LOCAL_KEYS,
            AgentKind::Remote => REMOTE_KEYS,
        };
        for key in agent.keys() {
            let Some(key) = key.as_str() else { continue };
            if allowed.contains(&key) {
                continue;
            }
            let at = self.at(&[key], &range);
            if key == "mcpServers" && kind == AgentKind::Local {
                self.push(
                    DiagnosticLevel::Warning,
                    "GM-AG-003",
                    at,
                    t!("rules.gm_ag_003.mcp_servers_camel_case"),
                    t!("rules.gm_ag_003.mcp_servers_camel_case_suggestion"),
                );
                continue;
            }
            let suggestion = if key == "system_prompt" {
                t!("rules.gm_ag_003.system_prompt_suggestion")
            } else {
                t!(
                    "rules.gm_ag_003.suggestion",
                    kind = kind.label(),
                    valid = allowed.join(", ")
                )
            };
            self.push(
                DiagnosticLevel::Error,
                "GM-AG-003",
                at,
                t!("rules.gm_ag_003.message", key = key, kind = kind.label()),
                suggestion,
            );
        }

        match kind {
            AgentKind::Local => self.check_local(agent, &range, &label, errors_before),
            AgentKind::Remote => self.check_remote(agent, &range, &label),
        }
    }

    fn check_local(
        &mut self,
        agent: &serde_yaml::Mapping,
        range: &Range<usize>,
        label: &str,
        errors_before: usize,
    ) {
        // tools: `z.array(z.string()...)`. The GM-AG-006 advice waits until
        // the agent is known to load.
        let mut tool_advice = None;
        match agent.get("tools") {
            None => {
                let at = (self.lines.first_line.saturating_sub(1).max(1), 1);
                tool_advice = Some((at, t!("rules.gm_ag_006.no_tools", name = label)));
            }
            Some(Yaml::Sequence(tools)) => {
                if tools.iter().any(|tool| tool.as_str().is_none()) {
                    let at = self.at(&["tools"], range);
                    self.invalid(at, t!("rules.gm_ag_002.tools_not_strings"));
                } else if tools.iter().any(|tool| tool.as_str() == Some("*")) {
                    let at = self.at(&["tools"], range);
                    tool_advice = Some((at, t!("rules.gm_ag_006.wildcard", name = label)));
                }
            }
            Some(other) => {
                let at = self.at(&["tools"], range);
                self.invalid(
                    at,
                    t!(
                        "rules.gm_ag_002.not_list",
                        field = "tools",
                        found = yaml_type(other)
                    ),
                );
            }
        }

        // GM-AG-004: `max_turns` / `timeout_mins`:
        // `z.number().int().positive().optional()`; `temperature`:
        // `z.number().optional()`, documented as 0.0 - 2.0.
        for field in ["max_turns", "timeout_mins"] {
            if let Some(value) = agent.get(field)
                && !is_positive_integer(value)
            {
                let at = self.at(&[field], range);
                self.push(
                    DiagnosticLevel::Error,
                    "GM-AG-004",
                    at,
                    t!(
                        "rules.gm_ag_004.not_positive_integer",
                        field = field,
                        value = yaml_text(value).as_str()
                    ),
                    t!("rules.gm_ag_004.suggestion"),
                );
            }
        }
        if let Some(value) = agent.get("temperature") {
            let at = self.at(&["temperature"], range);
            match value.as_f64() {
                None => self.push(
                    DiagnosticLevel::Error,
                    "GM-AG-004",
                    at,
                    t!(
                        "rules.gm_ag_004.temperature_not_number",
                        found = yaml_type(value)
                    ),
                    t!("rules.gm_ag_004.suggestion"),
                ),
                Some(t) if !(TEMPERATURE_RANGE.0..=TEMPERATURE_RANGE.1).contains(&t) => self.push(
                    DiagnosticLevel::Warning,
                    "GM-AG-004",
                    at,
                    t!(
                        "rules.gm_ag_004.temperature_range",
                        value = yaml_text(value).as_str()
                    ),
                    t!("rules.gm_ag_004.suggestion"),
                ),
                Some(_) => {}
            }
        }

        // GM-011 for inline MCP servers (`mcp_servers`, and the `mcpServers`
        // spelling of the subagents page).
        for key in ["mcp_servers", "mcpServers"] {
            let Some(Yaml::Mapping(servers)) = agent.get(key) else {
                continue;
            };
            for server in servers.keys() {
                let Some(server) = server.as_str() else {
                    continue;
                };
                if let Some(suggested) = underscore_free_name(server) {
                    let at = self.at(&[key, server], range);
                    let (message, suggestion) = gm_011_texts(server, &suggested);
                    self.push(DiagnosticLevel::Warning, "GM-011", at, message, suggestion);
                }
            }
        }

        // GM-AG-006 only for an agent that loads: a rejected file has no
        // tools to scope.
        if let Some((at, message)) = tool_advice
            && self.errors() == errors_before
        {
            self.push(
                DiagnosticLevel::Info,
                "GM-AG-006",
                at,
                message,
                t!("rules.gm_ag_006.suggestion"),
            );
        }
    }

    fn check_remote(&mut self, agent: &serde_yaml::Mapping, range: &Range<usize>, label: &str) {
        // Exactly one of `agent_card_url` (`z.string().url()`) and
        // `agent_card_json` (a string that `JSON.parse` accepts).
        let url = agent.get("agent_card_url");
        let json = agent.get("agent_card_json");
        match (url, json) {
            (None, None) => {
                let at = self.at(&["name"], range);
                self.invalid(at, t!("rules.gm_ag_002.no_agent_card", name = label));
            }
            (Some(_), Some(_)) => {
                let at = self.at(&["agent_card_json"], range);
                self.invalid(at, t!("rules.gm_ag_002.both_agent_cards", name = label));
            }
            (Some(url), None) => {
                if !url.as_str().is_some_and(is_url) {
                    let at = self.at(&["agent_card_url"], range);
                    self.invalid(
                        at,
                        t!(
                            "rules.gm_ag_002.bad_agent_card_url",
                            value = yaml_text(url).as_str()
                        ),
                    );
                }
            }
            (None, Some(json)) => {
                let problem = match json.as_str() {
                    None => Some(yaml_type(json).to_string()),
                    Some(text) => serde_json::from_str::<serde_json::Value>(text)
                        .err()
                        .map(|error| error.to_string()),
                };
                if let Some(problem) = problem {
                    let at = self.at(&["agent_card_json"], range);
                    self.invalid(
                        at,
                        t!(
                            "rules.gm_ag_002.bad_agent_card_json",
                            error = problem.as_str()
                        ),
                    );
                }
            }
        }

        // GM-AG-005: "Prefer `$ENV_VAR` or `!command` over embedding secrets
        // directly in agent files".
        let Some(Yaml::Mapping(auth)) = agent.get("auth") else {
            return;
        };
        let Some(auth_type) = auth.get("type").and_then(Yaml::as_str) else {
            return;
        };
        let Some((_, fields)) = AUTH_SECRET_FIELDS.iter().find(|(t, _)| *t == auth_type) else {
            return;
        };
        for field in *fields {
            let Some(Yaml::String(value)) = auth.get(*field) else {
                continue;
            };
            // `$$` / `!!` escape a literal `$` / `!`; a single `$` or `!`
            // is resolved at load time.
            let literal = if let Some(rest) = value.strip_prefix("$$") {
                format!("${rest}")
            } else if let Some(rest) = value.strip_prefix("!!") {
                format!("!{rest}")
            } else if value.starts_with(['$', '!']) {
                continue;
            } else {
                value.to_string()
            };
            if seems_plaintext_secret(&literal) {
                let at = self.at(&["auth", field], range);
                self.push(
                    DiagnosticLevel::Warning,
                    "GM-AG-005",
                    at,
                    t!("rules.gm_ag_005.message", field = *field, name = label),
                    t!("rules.gm_ag_005.suggestion"),
                );
            }
        }
    }
}

/// GM-AG-002 .. GM-AG-006 and GM-011 for one agent file.
fn check_definition(path: &Path, content: &str, config: &PerFileLintConfig<'_>) -> Vec<Diagnostic> {
    const DEFINITION_RULES: &[&str] = &[
        "GM-AG-002",
        "GM-AG-003",
        "GM-AG-004",
        "GM-AG-005",
        "GM-AG-006",
        "GM-011",
    ];
    if !DEFINITION_RULES.iter().any(|id| config.is_rule_enabled(id)) {
        return Vec::new();
    }
    let Some((yaml, offset)) = loader_frontmatter(content) else {
        let mut cx = DefinitionCx {
            path,
            config,
            lines: FrontmatterLines {
                first_line: 1,
                lines: Vec::new(),
            },
            diagnostics: Vec::new(),
        };
        cx.invalid((1, 1), t!("rules.gm_ag_002.no_frontmatter"));
        return cx.diagnostics;
    };
    let mut cx = DefinitionCx {
        path,
        config,
        lines: FrontmatterLines::new(content, yaml, offset),
        diagnostics: Vec::new(),
    };
    let all = 0..cx.lines.lines.len();
    // Bound the parser's work on pathological input, as the frontmatter
    // parser does.
    if crate::parsers::frontmatter::check_yaml_depth(yaml).is_err() {
        return cx.diagnostics;
    }
    match serde_yaml::from_str::<Yaml>(yaml) {
        Err(error) => {
            let line = error
                .location()
                .map_or(cx.lines.first_line, |l| cx.lines.first_line + l.line() - 1);
            let message = error.to_string();
            let message = message
                .split(" at line ")
                .next()
                .unwrap_or(&message)
                .to_string();
            cx.invalid(
                (line, 1),
                t!("rules.gm_ag_002.yaml_error", error = message.as_str()),
            );
        }
        Ok(value) if !local_tags(&value).is_empty() => {
            // js-yaml's `load` throws "unknown tag" for a local tag such as
            // an unquoted `!command` value; serde_yaml accepts it.
            for tag in local_tags(&value) {
                let at = cx.lines.find_text(&tag);
                cx.invalid(at, t!("rules.gm_ag_002.yaml_tag", tag = tag.as_str()));
            }
        }
        Ok(Yaml::Mapping(agent)) => cx.check_agent(&agent, all, false),
        // "The loader explicitly supports multiple remote subagents defined
        // in a single Markdown file."
        Ok(Yaml::Sequence(items)) => {
            let ranges = cx.lines.list_items();
            for (index, item) in items.iter().enumerate() {
                let range = ranges.get(index).cloned().unwrap_or_else(|| all.clone());
                match item {
                    Yaml::Mapping(agent) => cx.check_agent(agent, range, true),
                    other => {
                        let at = (cx.lines.first_line + range.start, 1);
                        cx.invalid(
                            at,
                            t!("rules.gm_ag_002.not_mapping", found = yaml_type(other)),
                        );
                    }
                }
            }
        }
        Ok(other) => {
            let at = (cx.lines.first_line.saturating_sub(1).max(1), 1);
            cx.invalid(
                at,
                t!("rules.gm_ag_002.not_mapping", found = yaml_type(&other)),
            );
        }
    }
    cx.diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use std::path::PathBuf;

    /// GM-AG-001 diagnostics only: these tests are about the auth block.
    /// The fixtures carry no `tools` list, which GM-AG-006 reports; the
    /// definition rules are tested in `definition_tests` below.
    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = GeminiAgentValidator;
        validator
            .validate(
                &PathBuf::from(".gemini/agents/test.md"),
                content,
                &LintConfig::default(),
            )
            .into_iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect()
    }

    const AGENT_HEADER: &str = "---\nkind: local\nname: test-agent\ndescription: Test\n";
    // The system prompt is the Markdown body; `system_prompt` is not a
    // frontmatter key (the local agent schema is `.strict()`).
    const AGENT_FOOTER: &str = "---\n\nYou are a test.\n";

    fn wrap_mcp(mcp: &str) -> String {
        format!("{AGENT_HEADER}mcp_servers:\n{mcp}{AGENT_FOOTER}")
    }

    // ===== Positive cases =====

    #[test]
    fn test_google_credentials_with_scopes_is_valid() {
        let content = wrap_mcp(
            "  spanner:\n    url: https://spanner.googleapis.com/mcp\n    type: http\n    auth:\n      type: google-credentials\n      scopes:\n        - https://www.googleapis.com/auth/cloud-platform\n",
        );
        let diagnostics = validate(&content);
        assert!(
            diagnostics.is_empty(),
            "Valid google-credentials auth must not flag, got {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_oauth_full_shape_is_valid() {
        let content = wrap_mcp(
            "  myserver:\n    url: https://example.com/mcp\n    type: http\n    auth:\n      type: oauth\n      client_id: abc\n      client_secret: secret\n      scopes: [read, write]\n      authorization_url: https://accounts.example.com/authorize\n      token_url: https://accounts.example.com/token\n",
        );
        let diagnostics = validate(&content);
        assert!(
            diagnostics.is_empty(),
            "Valid oauth auth must not flag, got {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_google_credentials_no_scopes_is_valid() {
        // scopes is optional per the Zod schema.
        let content = wrap_mcp(
            "  myserver:\n    url: https://example.com/mcp\n    auth:\n      type: google-credentials\n",
        );
        let diagnostics = validate(&content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_oauth_minimal_shape_is_valid() {
        // All oauth fields except `type` are optional.
        let content = wrap_mcp(
            "  myserver:\n    url: https://example.com/mcp\n    auth:\n      type: oauth\n",
        );
        let diagnostics = validate(&content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_no_auth_block_is_fine() {
        let content = wrap_mcp("  myserver:\n    url: https://example.com/mcp\n");
        let diagnostics = validate(&content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_no_mcp_servers_is_fine() {
        let content = format!("{AGENT_HEADER}{AGENT_FOOTER}");
        let diagnostics = validate(&content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_missing_frontmatter_is_silent() {
        // Silent for GM-AG-001 only; GM-AG-002 reports the missing
        // frontmatter (see definition_tests).
        let diagnostics = validate("# Just markdown, no frontmatter\n");
        assert!(diagnostics.is_empty());
    }

    // ===== Type discriminator =====

    #[test]
    fn test_missing_type_flags() {
        let content = wrap_mcp("  myserver:\n    auth:\n      scopes:\n        - foo\n");
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.to_lowercase().contains("missing"));
    }

    #[test]
    fn test_non_string_type_flags_as_type_not_string() {
        // Regression: cursor caught that non-string type was being reported
        // as "missing" because and_then(as_str) collapses both cases. The
        // validator now emits a distinct "type must be a string" message.
        let content = wrap_mcp("  myserver:\n    auth:\n      type: 12345\n");
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
        let msg = hits[0].message.to_lowercase();
        assert!(
            msg.contains("auth.type") && msg.contains("string"),
            "expected 'auth.type must be a string' style message, got: {}",
            hits[0].message
        );
        assert!(!msg.contains("missing"));
    }

    #[test]
    fn test_invalid_type_flags() {
        let content = wrap_mcp("  myserver:\n    auth:\n      type: basic-auth\n");
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("basic-auth"));
        assert!(hits[0].message.contains("google-credentials"));
    }

    #[test]
    fn test_non_object_auth_flags() {
        let content = wrap_mcp("  myserver:\n    auth: \"google-credentials\"\n");
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    // ===== Unknown fields =====

    #[test]
    fn test_oauth_field_on_google_credentials_flags() {
        // client_id doesn't belong on google-credentials. The auth objects
        // are not strict, so zod drops the field: a warning, not an error.
        let content = wrap_mcp(
            "  myserver:\n    auth:\n      type: google-credentials\n      client_id: abc\n",
        );
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("client_id"));
        assert_eq!(hits[0].level, crate::diagnostics::DiagnosticLevel::Warning);
    }

    #[test]
    fn test_unknown_field_on_oauth_flags() {
        let content =
            wrap_mcp("  myserver:\n    auth:\n      type: oauth\n      random_extra: yes\n");
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("random_extra"));
        assert_eq!(hits[0].level, crate::diagnostics::DiagnosticLevel::Warning);
    }

    #[test]
    fn test_current_oauth_fields_are_accepted() {
        // agentLoader.ts on main also has issuer, audiences, redirect_uri,
        // token_param_name and registration_url; agnix used to error on them.
        let content = wrap_mcp(
            "  myserver:\n    auth:\n      type: oauth\n      issuer: https://accounts.example.com\n      audiences: [api]\n      redirect_uri: http://localhost:7777/callback\n      token_param_name: access_token\n      registration_url: https://accounts.example.com/register\n",
        );
        assert!(validate(&content).is_empty(), "{:?}", validate(&content));
    }

    #[test]
    fn test_new_oauth_field_types_are_checked() {
        let content = wrap_mcp(
            "  myserver:\n    auth:\n      type: oauth\n      issuer: not-a-url\n      audiences: api\n      token_param_name: 5\n",
        );
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 3, "{hits:?}");
        assert!(
            hits.iter()
                .all(|d| d.level == crate::diagnostics::DiagnosticLevel::Error)
        );
    }

    // ===== Value-type checks =====

    #[test]
    fn test_client_id_non_string_flags() {
        let content =
            wrap_mcp("  myserver:\n    auth:\n      type: oauth\n      client_id: 12345\n");
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("client_id"));
    }

    #[test]
    fn test_authorization_url_malformed_flags() {
        let content = wrap_mcp(
            "  myserver:\n    auth:\n      type: oauth\n      authorization_url: \"not a url\"\n",
        );
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("authorization_url"));
    }

    #[test]
    fn test_authorization_url_valid_https_accepted() {
        let content = wrap_mcp(
            "  myserver:\n    auth:\n      type: oauth\n      authorization_url: https://example.com/authorize\n",
        );
        let diagnostics = validate(&content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_scopes_not_array_flags() {
        let content = wrap_mcp(
            "  myserver:\n    auth:\n      type: google-credentials\n      scopes: one-scope\n",
        );
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn test_scope_non_string_flags() {
        let content = wrap_mcp(
            "  myserver:\n    auth:\n      type: google-credentials\n      scopes:\n        - 12345\n",
        );
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    // ===== Multiple servers =====

    #[test]
    fn test_per_server_independent_validation() {
        let content = wrap_mcp(
            "  s1:\n    auth:\n      type: google-credentials\n  s2:\n    auth:\n      type: basic-auth\n",
        );
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1, "only s2 should flag, got {:?}", hits);
        assert!(hits[0].message.contains("s2"));
    }

    // ===== Disable path =====

    #[test]
    fn test_can_be_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["GM-AG-001".to_string()];
        let content = wrap_mcp("  myserver:\n    auth:\n      type: basic-auth\n");
        let validator = GeminiAgentValidator;
        let diagnostics =
            validator.validate(&PathBuf::from(".gemini/agents/test.md"), &content, &config);
        // The agent has no tools list (GM-AG-006, info); nothing else.
        assert!(
            diagnostics.iter().all(|d| d.rule == "GM-AG-006"),
            "{diagnostics:?}"
        );
    }

    // ===== Helper unit tests =====

    #[test]
    fn test_looks_like_url() {
        assert!(looks_like_url("https://example.com"));
        assert!(looks_like_url("http://localhost:8080/path"));
        assert!(!looks_like_url("not-a-url"));
        assert!(!looks_like_url("ftp://example.com"));
        assert!(!looks_like_url("https://"));
        assert!(!looks_like_url("https:///empty-host"));
    }

    #[test]
    fn test_extract_frontmatter_basic() {
        let content = "---\nkey: value\n---\nBody\n";
        assert_eq!(extract_frontmatter(content), Some("key: value\n"));
    }

    #[test]
    fn test_extract_frontmatter_bom() {
        let content = "\u{FEFF}---\nkey: value\n---\nBody\n";
        assert_eq!(extract_frontmatter(content), Some("key: value\n"));
    }

    #[test]
    fn test_extract_frontmatter_absent() {
        assert_eq!(extract_frontmatter("# No frontmatter\n"), None);
    }

    #[test]
    fn test_extract_frontmatter_unterminated() {
        assert_eq!(extract_frontmatter("---\nkey: value\nno closing\n"), None);
    }
}

#[cfg(test)]
mod definition_tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate_at(path: &str, content: &str) -> Vec<Diagnostic> {
        GeminiAgentValidator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn validate(content: &str) -> Vec<Diagnostic> {
        validate_at(".gemini/agents/test.md", content)
    }

    fn rules(diagnostics: &[Diagnostic]) -> Vec<(&str, DiagnosticLevel, usize)> {
        diagnostics
            .iter()
            .map(|d| (d.rule.as_str(), d.level, d.line))
            .collect()
    }

    #[test]
    fn documented_examples_are_clean() {
        // The subagents page example, and the remote-agents examples.
        for content in [
            "---\nname: security-auditor\ndescription: Specialized in finding security vulnerabilities in code.\nkind: local\ntools:\n  - read_file\n  - grep_search\nmodel: gemini-3-flash-preview\ntemperature: 0.2\nmax_turns: 10\n---\n\nYou are a ruthless Security Auditor.\n",
            "---\nkind: remote\nname: my-remote-agent\nagent_card_url: https://example.com/agent-card\n---\n",
            "---\n- kind: remote\n  name: remote-1\n  agent_card_url: https://example.com/1\n- kind: remote\n  name: remote-2\n  agent_card_url: https://example.com/2\n---\n",
            "---\nkind: remote\nname: single-quotes-agent\nagent_card_json:\n  '{ \"protocolVersion\": \"0.3.0\", \"name\": \"Example Agent\", \"version\": \"1.0.0\",\n  \"url\": \"dummy-url\" }'\n---\n",
            "---\nkind: remote\nname: block-scalar-agent\nagent_card_json: |\n  {\n    \"protocolVersion\": \"0.3.0\",\n    \"name\": \"Example Agent Name\"\n  }\n---\n",
            "---\nkind: remote\nname: my-agent\nagent_card_url: https://example.com/agent-card\nauth:\n  type: apiKey\n  key: $MY_API_KEY\n---\n",
            "---\nkind: remote\nname: oauth-agent\nagent_card_url: https://example.com/.well-known/agent.json\nauth:\n  type: oauth\n  client_id: my-client-id.apps.example.com\n---\n",
            // A `!command` value must be quoted in YAML (an unquoted `!x` is
            // a YAML tag).
            "---\nkind: remote\nname: tok\nagent_card_url: https://example.com/a\nauth:\n  type: http\n  scheme: Bearer\n  token: \"!gcloud auth print-access-token\"\n---\n",
            // Trailing blanks after the markers, CRLF and a BOM are accepted.
            "\u{FEFF}---  \r\nname: crlf\r\ndescription: d\r\ntools: [read_file]\r\n--- \r\nBody\r\n",
        ] {
            assert!(
                validate(content).is_empty(),
                "{content}: {:?}",
                validate(content)
            );
        }
    }

    #[test]
    fn underscore_files_are_not_loaded() {
        // loadAgentsFromDirectory: "Ignores files starting with _".
        assert!(validate_at(".gemini/agents/_draft.md", "no frontmatter\n").is_empty());
    }

    #[test]
    fn gm_ag_002_frontmatter_must_load() {
        assert_eq!(
            rules(&validate("# Title\n\nno frontmatter\n")),
            [("GM-AG-002", DiagnosticLevel::Error, 1)]
        );
        // Unterminated, or the closing marker on the opening line's heels.
        assert_eq!(
            rules(&validate("---\nname: a\n")),
            [("GM-AG-002", DiagnosticLevel::Error, 1)]
        );
        assert_eq!(
            rules(&validate("---\n---\nbody\n")),
            [("GM-AG-002", DiagnosticLevel::Error, 1)]
        );
        // YAML error, at its line.
        let diagnostics = validate("---\nname: a\ndescription: [unclosed\ntools: [x]\n---\n");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, "GM-AG-002");
        assert!(diagnostics[0].line >= 3, "{diagnostics:?}");
        // A scalar instead of a mapping.
        assert_eq!(
            rules(&validate("---\njust text\n---\n")),
            [("GM-AG-002", DiagnosticLevel::Error, 1)]
        );
    }

    #[test]
    fn gm_ag_002_unquoted_command_value_is_a_yaml_tag() {
        // js-yaml (agentLoader.ts `load`) throws "unknown tag" for `!gcloud`;
        // serde_yaml reads it as a tagged value, so agnix reports the tag.
        let diagnostics = validate(
            "---\nkind: remote\nname: tok\nagent_card_url: https://example.com/a\nauth:\n  type: http\n  scheme: Bearer\n  token: !gcloud auth print-access-token\n---\n",
        );
        assert_eq!(
            rules(&diagnostics),
            [("GM-AG-002", DiagnosticLevel::Error, 8)]
        );
        assert!(
            diagnostics[0].message.contains("!gcloud"),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn gm_ag_002_name_and_description() {
        // The probe of the survey: no agnix diagnostic in 0.58.
        let diagnostics = validate("---\nname: Security Auditor\ntools: [read_file]\n---\n");
        assert_eq!(
            rules(&diagnostics),
            [
                ("GM-AG-002", DiagnosticLevel::Error, 2),
                ("GM-AG-002", DiagnosticLevel::Error, 2)
            ]
        );
        assert!(diagnostics[0].message.contains("slug"));
        assert!(diagnostics[1].message.contains("description"));

        let diagnostics = validate("---\ndescription: \"\"\ntools: [read_file]\n---\n");
        assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
        assert!(diagnostics.iter().all(|d| d.rule == "GM-AG-002"));

        // Underscores and digits are valid in a name.
        assert!(validate("---\nname: my_agent-2\ndescription: d\ntools: [x]\n---\n").is_empty());
        // A remote agent's description is optional.
        assert!(
            validate("---\nkind: remote\nname: r\nagent_card_url: https://e.com/a\n---\n")
                .is_empty()
        );
    }

    #[test]
    fn gm_ag_002_remote_agent_cards() {
        let missing = validate("---\nkind: remote\nname: r\n---\n");
        assert_eq!(rules(&missing), [("GM-AG-002", DiagnosticLevel::Error, 3)]);
        let both = validate(
            "---\nkind: remote\nname: r\nagent_card_url: https://e.com/a\nagent_card_json: '{}'\n---\n",
        );
        assert_eq!(rules(&both), [("GM-AG-002", DiagnosticLevel::Error, 5)]);
        let bad_url = validate("---\nkind: remote\nname: r\nagent_card_url: not a url\n---\n");
        assert_eq!(rules(&bad_url), [("GM-AG-002", DiagnosticLevel::Error, 4)]);
        let bad_json = validate("---\nkind: remote\nname: r\nagent_card_json: '{oops'\n---\n");
        assert_eq!(rules(&bad_json), [("GM-AG-002", DiagnosticLevel::Error, 4)]);
        // The list format is remote-only.
        let list = validate(
            "---\n- kind: remote\n  name: r1\n  agent_card_url: https://e.com/1\n- kind: local\n  name: l1\n  description: d\n---\n",
        );
        assert_eq!(rules(&list), [("GM-AG-002", DiagnosticLevel::Error, 5)]);
        assert!(list[0].message.contains("remote"));
        // kind must be local or remote.
        let kind = validate("---\nkind: Local\nname: a\ndescription: d\ntools: [x]\n---\n");
        assert_eq!(rules(&kind), [("GM-AG-002", DiagnosticLevel::Error, 2)]);
    }

    #[test]
    fn gm_ag_003_strict_keys() {
        // `system_prompt` in the frontmatter: the agnix 0.22 fixtures had it.
        let diagnostics = validate(
            "---\nname: a\ndescription: d\ntools: [x]\nsystem_prompt: You are a test.\n---\n",
        );
        assert_eq!(
            rules(&diagnostics),
            [("GM-AG-003", DiagnosticLevel::Error, 5)]
        );
        assert!(
            diagnostics[0]
                .suggestion
                .as_deref()
                .unwrap_or("")
                .contains("Markdown body")
        );

        // A local key on a remote agent.
        let diagnostics = validate(
            "---\nkind: remote\nname: r\nagent_card_url: https://e.com/a\ntools: [x]\n---\n",
        );
        assert_eq!(
            rules(&diagnostics),
            [("GM-AG-003", DiagnosticLevel::Error, 5)]
        );

        // The docs' `mcpServers`: a warning, not an error, since the
        // subagents page documents it.
        let diagnostics = validate(
            "---\nname: my-isolated-agent\ndescription: d\ntools:\n  - grep_search\nmcpServers:\n  my-custom-server:\n    command: 'node'\n---\n",
        );
        assert_eq!(
            rules(&diagnostics),
            [("GM-AG-003", DiagnosticLevel::Warning, 6)]
        );
    }

    #[test]
    fn gm_ag_004_numeric_fields() {
        let diagnostics = validate(
            "---\nname: a\ndescription: d\ntools: [x]\nmax_turns: 0\ntimeout_mins: 2.5\ntemperature: 3\n---\n",
        );
        assert_eq!(
            rules(&diagnostics),
            [
                ("GM-AG-004", DiagnosticLevel::Error, 5),
                ("GM-AG-004", DiagnosticLevel::Error, 6),
                ("GM-AG-004", DiagnosticLevel::Warning, 7)
            ]
        );
        let diagnostics = validate(
            "---\nname: a\ndescription: d\ntools: [x]\nmax_turns: \"10\"\ntemperature: hot\n---\n",
        );
        assert_eq!(
            rules(&diagnostics),
            [
                ("GM-AG-004", DiagnosticLevel::Error, 5),
                ("GM-AG-004", DiagnosticLevel::Error, 6)
            ]
        );
        // 10.0 is an integer to JavaScript; 0 and 2 are inside the range.
        assert!(
            validate(
                "---\nname: a\ndescription: d\ntools: [x]\nmax_turns: 10.0\ntemperature: 2\n---\n"
            )
            .is_empty()
        );
    }

    #[test]
    fn gm_ag_005_literal_secrets_in_remote_auth() {
        let diagnostics = validate(
            "---\nkind: remote\nname: r\nagent_card_url: https://e.com/a\nauth:\n  type: apiKey\n  key: sk-live-4f9a8b7c6d5e4f3a2b1c\n---\n",
        );
        assert_eq!(
            rules(&diagnostics),
            [("GM-AG-005", DiagnosticLevel::Warning, 7)]
        );
        let diagnostics = validate(
            "---\nkind: remote\nname: r\nagent_card_url: https://e.com/a\nauth:\n  type: http\n  scheme: Basic\n  username: admin\n  password: hunter2hunter2\n---\n",
        );
        assert_eq!(
            rules(&diagnostics),
            [("GM-AG-005", DiagnosticLevel::Warning, 9)]
        );
        // `$$` escapes a literal `$`: `$$MY_TOKEN` is the literal `$MY_TOKEN`,
        // which reads as a variable name rather than a secret.
        for value in [
            "$MY_TOKEN",
            "!pass show api",
            "$$MY_TOKEN",
            "your-api-key-here",
        ] {
            let content = format!(
                "---\nkind: remote\nname: r\nagent_card_url: https://e.com/a\nauth:\n  type: http\n  scheme: Bearer\n  token: \"{value}\"\n---\n"
            );
            assert!(
                validate(&content).is_empty(),
                "{value}: {:?}",
                validate(&content)
            );
        }
        // An escaped literal secret is still a secret.
        let content = "---\nkind: remote\nname: r\nagent_card_url: https://e.com/a\nauth:\n  type: http\n  scheme: Bearer\n  token: \"!!sk-live-4f9a8b7c6d5e\"\n---\n";
        assert_eq!(
            rules(&validate(content)),
            [("GM-AG-005", DiagnosticLevel::Warning, 8)]
        );
        // client_id and OAuth client_secret are not checked.
        assert!(
            validate("---\nkind: remote\nname: r\nagent_card_url: https://e.com/a\nauth:\n  type: oauth\n  client_id: abcdefgh12345678\n  client_secret: abcdefgh12345678\n---\n")
                .is_empty()
        );
    }

    #[test]
    fn gm_ag_006_tool_scope() {
        let diagnostics = validate("---\nname: a\ndescription: d\n---\n");
        assert_eq!(
            rules(&diagnostics),
            [("GM-AG-006", DiagnosticLevel::Info, 1)]
        );
        let diagnostics = validate("---\nname: a\ndescription: d\ntools:\n  - '*'\n---\n");
        assert_eq!(
            rules(&diagnostics),
            [("GM-AG-006", DiagnosticLevel::Info, 4)]
        );
        // Not for an agent the loader rejects (here: GM-AG-003).
        let diagnostics = validate("---\nname: a\ndescription: d\nsystem_prompt: x\n---\n");
        assert_eq!(
            rules(&diagnostics),
            [("GM-AG-003", DiagnosticLevel::Error, 4)]
        );
        // Remote agents have no tools.
        assert!(
            validate("---\nkind: remote\nname: r\nagent_card_url: https://e.com/a\n---\n")
                .is_empty()
        );
    }

    #[test]
    fn gm_011_agent_mcp_server_names() {
        let diagnostics = validate(
            "---\nname: a\ndescription: d\ntools: [x]\nmcp_servers:\n  ok-server:\n    url: https://e.com\n  my_server:\n    command: node\n---\n",
        );
        assert_eq!(
            rules(&diagnostics),
            [("GM-011", DiagnosticLevel::Warning, 8)]
        );
        assert_eq!(diagnostics[0].column, 3);
        assert!(diagnostics[0].message.contains("my-server"));
    }

    #[test]
    fn frontmatter_lines_find_nested_keys() {
        let yaml = "name: a\nauth:\n  type: http\n  token: x\nmcp_servers:\n  s_1:\n    token: y\n";
        let lines = FrontmatterLines {
            first_line: 2,
            lines: yaml.lines().collect(),
        };
        let all = 0..lines.lines.len();
        assert_eq!(lines.position(&["auth", "token"], all.clone()), (5, 3));
        assert_eq!(lines.position(&["mcp_servers", "s_1"], all.clone()), (7, 3));
        assert_eq!(
            lines.position(&["mcp_servers", "s_1", "token"], all.clone()),
            (8, 5)
        );
        assert_eq!(lines.position(&["missing"], all), (2, 1));
    }
}
