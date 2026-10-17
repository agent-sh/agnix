//! Kiro steering file validation rules (KIRO-001 to KIRO-011, KIRO-014, KIRO-015)
//!
//! Validates:
//! - KIRO-001: Invalid steering file inclusion mode (HIGH/ERROR)
//! - KIRO-002: Inclusion mode without the field it needs to load (HIGH/ERROR)
//! - KIRO-003: fileMatchPattern that does not match as written (MEDIUM/WARNING)
//! - KIRO-004: Empty Kiro steering file (MEDIUM/WARNING)
//! - KIRO-005: Empty steering body after frontmatter (MEDIUM/WARNING)
//! - KIRO-006: Secrets detected in steering content (HIGH/ERROR)
//! - KIRO-007: fileMatchPattern present without inclusion: fileMatch (MEDIUM/WARNING)
//! - KIRO-008: Unknown frontmatter field (MEDIUM/WARNING)
//! - KIRO-009: Inline file or folder reference points to a missing target (MEDIUM/WARNING)
//! - KIRO-010: On-demand fields without an inclusion mode (MEDIUM/WARNING)
//! - KIRO-011: Steering doc excessively long (LOW/INFO)
//! - KIRO-014: Steering body without a Markdown heading (LOW/INFO)
//! - KIRO-015: Frontmatter Kiro cannot use (HIGH/ERROR, or WARNING when only
//!   Kiro CLI V3 rejects it and V2 is also targeted)
//!
//! Ground truth (Kiro CLI 2.32 V3 engine): the frontmatter is parsed only when
//! `---` is the very first line; a YAML error, a non-mapping document or a
//! schema failure (`inclusion` not one of the four modes, `name` or
//! `description` not a string, `fileMatchPattern` not a string or string
//! array) makes the engine use the whole raw file, frontmatter included, as
//! always-included content. A missing frontmatter or `inclusion` key means
//! `always`. The V2 engine reads only `inclusion` and loads only `always` files.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, Fix},
    parsers::frontmatter::split_frontmatter,
    rules::{
        Validator, ValidatorMetadata,
        kiro_engine::{KiroEngine, engines_for},
        line_col_at_offset, seems_plaintext_secret,
    },
};
use regex::Regex;
use rust_i18n::t;
use std::path::{Component, Path};
use std::sync::OnceLock;

const RULE_IDS: &[&str] = &[
    "KIRO-001", "KIRO-002", "KIRO-003", "KIRO-004", "KIRO-005", "KIRO-006", "KIRO-007", "KIRO-008",
    "KIRO-009", "KIRO-010", "KIRO-011", "KIRO-014", "KIRO-015",
];

/// agnix's size guidance for one steering document. Kiro documents no limit
/// for steering files; the only cap in the engine is the 50,000-character
/// truncation of the workspace-root AGENTS.md.
const MAX_STEERING_DOC_CHARS: usize = 50_000;
const VALID_INCLUSION_MODES: &[&str] = &["always", "fileMatch", "manual", "auto"];
const VALID_FRONTMATTER_FIELDS: &[&str] = &["inclusion", "name", "description", "fileMatchPattern"];
/// File-scoping keys of other tools' rule files (Cursor `globs`, Copilot
/// `applyTo`, Claude Code `paths`, ...). Kiro ignores them.
const FILE_SCOPE_FIELDS: &[&str] = &[
    "globs",
    "glob",
    "paths",
    "applyTo",
    "applicableTo",
    "filePatterns",
    "files",
    "include",
];

// ---------------------------------------------------------------------------
// YAML frontmatter span helpers (shared with the Kiro skill rules)
// ---------------------------------------------------------------------------

/// A top-level `key: value` entry of a YAML block mapping, located by a line
/// scanner. Offsets are relative to the scanned text. The scanner only finds
/// spans; callers confirm every edit with [`frontmatter_edit_yields`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct YamlTopLevelEntry {
    /// Key text without surrounding quotes.
    pub key: String,
    /// Start of the key line (the key starts in column 0).
    pub start: usize,
    /// End of the key token.
    pub key_end: usize,
    /// First non-blank byte after the colon on the key line (`line_end` when
    /// the value starts on a later line).
    pub value_start: usize,
    /// End of the key line, before its line break.
    pub line_end: usize,
    /// End of the last line that belongs to the value, before its line break.
    /// Trailing blank lines and column-0 comment lines are not part of it.
    pub value_end: usize,
}

impl YamlTopLevelEntry {
    /// True when the whole entry, key and value, sits on the key line.
    pub fn single_line(&self) -> bool {
        self.value_end == self.line_end && self.value_start < self.line_end
    }
}

/// Multi-line scalar state carried from one line to the next.
#[derive(Default, Clone, Copy)]
struct ScalarState {
    in_double: bool,
    in_single: bool,
    flow_depth: usize,
}

impl ScalarState {
    fn open(&self) -> bool {
        self.in_double || self.in_single || self.flow_depth > 0
    }

    /// Scan `text`; `at_value_start` is true when `text` starts where a
    /// scalar may start (after `key:`, a `- ` item marker or a flow opener).
    fn scan(&mut self, text: &str, mut at_value_start: bool) {
        let bytes = text.as_bytes();
        let mut i = 0;
        let mut prev_ws = true;
        while i < bytes.len() {
            let b = bytes[i];
            if self.in_double {
                if b == b'\\' {
                    i += 2;
                    continue;
                }
                if b == b'"' {
                    self.in_double = false;
                    at_value_start = false;
                }
                prev_ws = false;
                i += 1;
                continue;
            }
            if self.in_single {
                if b == b'\'' {
                    if bytes.get(i + 1) == Some(&b'\'') {
                        i += 2;
                        continue;
                    }
                    self.in_single = false;
                    at_value_start = false;
                }
                prev_ws = false;
                i += 1;
                continue;
            }
            let next_is_blank = bytes.get(i + 1).is_none_or(|n| *n == b' ' || *n == b'\t');
            match b {
                b' ' | b'\t' => {
                    prev_ws = true;
                    i += 1;
                    continue;
                }
                b'#' if prev_ws => return,
                b'"' if at_value_start => self.in_double = true,
                b'\'' if at_value_start => self.in_single = true,
                b'[' | b'{' if at_value_start || self.flow_depth > 0 => {
                    self.flow_depth += 1;
                    at_value_start = true;
                }
                b']' | b'}' if self.flow_depth > 0 => {
                    self.flow_depth -= 1;
                    at_value_start = false;
                }
                b',' if self.flow_depth > 0 => at_value_start = true,
                b':' if self.flow_depth > 0 || next_is_blank => at_value_start = true,
                b'-' if at_value_start && self.flow_depth == 0 && next_is_blank => {}
                _ => at_value_start = false,
            }
            prev_ws = false;
            i += 1;
        }
    }
}

/// Parse a column-0 `key:` line. Returns the key, the byte length of the key
/// token and the offset just past the colon.
fn top_level_key(line: &str) -> Option<(String, usize, usize)> {
    let first = *line.as_bytes().first()?;
    if first.is_ascii_whitespace() || b"#-[]{}?:|>%@`!&*,".contains(&first) {
        return None;
    }
    let is_value_colon = |idx: usize| {
        line.as_bytes()
            .get(idx + 1)
            .is_none_or(|n| *n == b' ' || *n == b'\t')
    };
    if first == b'"' || first == b'\'' {
        let close = line[1..].find(first as char)? + 1;
        let rest = &line[close + 1..];
        let colon = close + 1 + (rest.len() - rest.trim_start().len());
        if line.as_bytes().get(colon) != Some(&b':') || !is_value_colon(colon) {
            return None;
        }
        return Some((line[1..close].to_string(), close + 1, colon + 1));
    }
    let bytes = line.as_bytes();
    for (idx, b) in bytes.iter().enumerate() {
        if *b == b'#' && idx > 0 && bytes[idx - 1].is_ascii_whitespace() {
            return None;
        }
        if *b == b':' && is_value_colon(idx) {
            let key = line[..idx].trim_end();
            return Some((key.to_string(), key.len(), idx + 1));
        }
    }
    None
}

/// Locate the top-level entries of a YAML block mapping (frontmatter text).
pub(crate) fn yaml_top_level_entries(text: &str) -> Vec<YamlTopLevelEntry> {
    let mut entries = Vec::new();
    let mut current: Option<YamlTopLevelEntry> = None;
    let mut state = ScalarState::default();
    let mut offset = 0;
    for raw_line in text.split_inclusive('\n') {
        let line_start = offset;
        offset += raw_line.len();
        let line = raw_line
            .strip_suffix('\n')
            .unwrap_or(raw_line)
            .trim_end_matches('\r');
        let line_end = line_start + line.len();
        if state.open() {
            // Inside a flow collection every line starts at a token boundary.
            let at_token = state.flow_depth > 0 && !state.in_double && !state.in_single;
            state.scan(line, at_token);
            if let Some(entry) = current.as_mut() {
                entry.value_end = line_end;
            }
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        let indented = trimmed.len() != line.len();
        let sequence_item = line == "-" || line.starts_with("- ") || line.starts_with("-\t");
        if indented || sequence_item {
            if let Some(entry) = current.as_mut() {
                entry.value_end = line_end;
            }
            state.scan(line, true);
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        if let Some((key, key_len, colon_end)) = top_level_key(line) {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            let after = &line[colon_end..];
            let after_trimmed = after.trim_start();
            let value_start = if after_trimmed.is_empty() || after_trimmed.starts_with('#') {
                line_end
            } else {
                line_start + colon_end + (after.len() - after_trimmed.len())
            };
            current = Some(YamlTopLevelEntry {
                key,
                start: line_start,
                key_end: line_start + key_len,
                value_start,
                line_end,
                value_end: line_end,
            });
            state.scan(after, true);
            continue;
        }
        // Column-0 text that is not a key continues the current value.
        if let Some(entry) = current.as_mut() {
            entry.value_end = line_end;
        }
        state.scan(line, false);
    }
    if let Some(entry) = current.take() {
        entries.push(entry);
    }
    entries
}

/// Apply one edit to `content` and confirm that the result still has a closed
/// frontmatter block that parses to `expected`, with the body unchanged.
pub(crate) fn frontmatter_edit_yields(
    content: &str,
    start: usize,
    end: usize,
    replacement: &str,
    expected: &serde_yaml::Value,
) -> bool {
    if start > end
        || end > content.len()
        || !content.is_char_boundary(start)
        || !content.is_char_boundary(end)
    {
        return false;
    }
    let original = split_frontmatter(content);
    let mut edited = String::with_capacity(content.len() + replacement.len());
    edited.push_str(&content[..start]);
    edited.push_str(replacement);
    edited.push_str(&content[end..]);
    let parts = split_frontmatter(&edited);
    if !parts.has_frontmatter || !parts.has_closing || parts.body != original.body {
        return false;
    }
    serde_yaml::from_str::<serde_yaml::Value>(&parts.frontmatter)
        .is_ok_and(|value| &value == expected)
}

/// The scalar text of a single-line entry's value, without a trailing comment.
/// Returns `None` for values that are not a single scalar on the key line.
pub(crate) fn single_line_scalar_text<'a>(
    frontmatter: &'a str,
    entry: &YamlTopLevelEntry,
) -> Option<&'a str> {
    if !entry.single_line() {
        return None;
    }
    let value = &frontmatter[entry.value_start..entry.line_end];
    let first = value.chars().next()?;
    if first == '"' || first == '\'' {
        let bytes = value.as_bytes();
        let mut i = 1;
        while i < bytes.len() {
            if first == '"' && bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == first as u8 {
                if first == '\'' && bytes.get(i + 1) == Some(&b'\'') {
                    i += 2;
                    continue;
                }
                return Some(&value[..=i]);
            }
            i += 1;
        }
        return None;
    }
    if matches!(first, '[' | '{' | '|' | '>' | '&' | '*' | '!') {
        return None;
    }
    let end = value
        .char_indices()
        .find(|(idx, ch)| *ch == '#' && *idx > 0 && value[..*idx].ends_with([' ', '\t']))
        .map_or(value.len(), |(idx, _)| idx);
    Some(value[..end].trim_end())
}

/// Offset of the second occurrence of a repeated top-level key, which the
/// YAML parser reports at the mapping instead of at the repeated key.
fn duplicate_key_offset(frontmatter: &str) -> Option<usize> {
    let mut seen = std::collections::HashSet::new();
    yaml_top_level_entries(frontmatter)
        .into_iter()
        .find(|entry| !seen.insert(entry.key.clone()))
        .map(|entry| entry.start)
}

/// 1-based line and column of a byte offset inside `content`.
fn location(content: &str, offset: usize) -> (usize, usize) {
    line_col_at_offset(content, offset)
}

// ---------------------------------------------------------------------------
// Secret heuristics (shared with KR-PW-008)
// ---------------------------------------------------------------------------

fn secret_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?im)\b(?P<marker>api[_-]?key|token|password|[a-z0-9_-]+secret|secret[a-z0-9_-]+)\b\s*[:=]\s*(?P<value>[^\s#]+)",
        )
        .expect("secret pattern must compile")
    })
}

/// True for a value that names a variable, field, type or call instead of
/// holding a literal, for example `hashedPassword`, `user.password,`,
/// `SecretString` or `self.secrets.get('k')`. Code samples in steering and
/// power docs use these all the time; a quoted literal is never a reference.
pub(crate) fn looks_like_code_reference(raw: &str) -> bool {
    let value = raw.trim_end_matches([',', ';']);
    if value.starts_with(['"', '\'', '`']) {
        return false;
    }
    if value.starts_with('$') {
        return true;
    }
    if value.contains(['(', ')', '[', ']', '{', '}'])
        || value.contains("::")
        || value.contains("->")
    {
        return true;
    }
    let is_identifier = |s: &str| {
        s.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    if value.contains('.') && value.split('.').all(is_identifier) {
        return true;
    }
    is_identifier(value) && !value.chars().any(|c| c.is_ascii_digit())
}

/// Find the first likely hardcoded secret in `text`; returns the offset of the
/// match and the lowercased marker.
pub(crate) fn find_secret(text: &str) -> Option<(usize, String)> {
    for captures in secret_pattern().captures_iter(text) {
        let (Some(full), Some(value)) = (captures.get(0), captures.name("value")) else {
            continue;
        };
        if seems_plaintext_secret(value.as_str()) && !looks_like_code_reference(value.as_str()) {
            let marker = captures
                .name("marker")
                .map(|m| m.as_str().to_ascii_lowercase())
                .unwrap_or_else(|| "secret".to_string());
            return Some((full.start(), marker));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// fileMatchPattern helpers
// ---------------------------------------------------------------------------

/// Expand `{a,b}` alternatives the way minimatch's braceExpand does for the
/// patterns Kiro checks (numeric ranges are not expanded).
fn brace_expand(pattern: &str, out: &mut Vec<String>, budget: &mut usize) {
    if *budget == 0 {
        out.push(pattern.to_string());
        return;
    }
    *budget -= 1;
    let bytes = pattern.as_bytes();
    let mut open = None;
    let mut depth = 0usize;
    let mut commas = Vec::new();
    for (idx, b) in bytes.iter().enumerate() {
        match b {
            b'{' => {
                if depth == 0 {
                    open = Some(idx);
                    commas.clear();
                }
                depth += 1;
            }
            b',' if depth == 1 => commas.push(idx),
            b'}' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    let start = open.expect("brace depth tracks an opening brace");
                    if commas.is_empty() {
                        // `{a}` is not an alternation; keep scanning after it.
                        continue;
                    }
                    let (prefix, suffix) = (&pattern[..start], &pattern[idx + 1..]);
                    let mut part_start = start + 1;
                    for split in commas.iter().copied().chain(std::iter::once(idx)) {
                        let alternative = &pattern[part_start..split];
                        brace_expand(&format!("{prefix}{alternative}{suffix}"), out, budget);
                        part_start = split + 1;
                    }
                    return;
                }
            }
            _ => {}
        }
    }
    out.push(pattern.to_string());
}

/// True when `pattern` contains a comma that minimatch matches literally: a
/// comma outside `{...}` alternation and outside `[...]` classes. Mirrors the
/// V3 engine's `fileMatchPatternLiteralComma` check, including its limits.
fn has_literal_comma(pattern: &str) -> bool {
    let normalized = pattern.replace('\\', "/");
    if !normalized.contains(',') || normalized.len() > 1024 {
        return false;
    }
    static RANGE: OnceLock<Regex> = OnceLock::new();
    let range = RANGE.get_or_init(|| {
        Regex::new(r"\{-?\d+\.\.-?\d+(\.\.-?\d+)?\}").expect("range pattern must compile")
    });
    if range.is_match(&normalized) || normalized.matches('{').count() > 8 {
        return false;
    }
    let mut expanded = Vec::new();
    let mut budget = 256;
    brace_expand(&normalized, &mut expanded, &mut budget);
    expanded.iter().any(|candidate| {
        let mut in_class = false;
        for ch in candidate.chars() {
            if in_class {
                if ch == ']' {
                    in_class = false;
                }
            } else if ch == '[' {
                in_class = true;
            } else if ch == ',' {
                return true;
            }
        }
        false
    })
}

/// True when a `[` never gets a closing `]`, so minimatch matches the `[`
/// as a literal character.
fn has_unclosed_class(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'[' => {
                // A `]` right after `[` or `[!` is part of the class.
                let mut j = i + 1;
                if matches!(bytes.get(j), Some(b'!' | b'^')) {
                    j += 1;
                }
                if bytes.get(j) == Some(&b']') {
                    j += 1;
                }
                match bytes[j.min(bytes.len())..].iter().position(|b| *b == b']') {
                    Some(close) => i = j + close + 1,
                    None => return true,
                }
            }
            _ => i += 1,
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Markdown helpers
// ---------------------------------------------------------------------------

/// True when `body` has at least one ATX or `===` setext heading outside
/// fenced code blocks.
fn has_markdown_heading(body: &str) -> bool {
    static ATX: OnceLock<Regex> = OnceLock::new();
    let atx = ATX.get_or_init(|| Regex::new(r"^ {0,3}#{1,6}(?:[ \t]|$)").expect("atx heading"));
    let mut fence: Option<&str> = None;
    let mut previous_text = false;
    for line in body.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if indent <= 3 {
            let marker = if trimmed.starts_with("```") {
                Some("```")
            } else if trimmed.starts_with("~~~") {
                Some("~~~")
            } else {
                None
            };
            if let Some(marker) = marker {
                match fence {
                    Some(open) if open == marker => fence = None,
                    None => fence = Some(marker),
                    _ => {}
                }
                previous_text = false;
                continue;
            }
        }
        if fence.is_some() {
            continue;
        }
        if atx.is_match(line) {
            return true;
        }
        let is_setext =
            indent <= 3 && trimmed.len() >= 3 && trimmed.trim_end().chars().all(|c| c == '=');
        if is_setext && previous_text {
            return true;
        }
        previous_text = !trimmed.is_empty();
    }
    false
}

// ---------------------------------------------------------------------------
// KIRO-009 helpers
// ---------------------------------------------------------------------------

fn inline_reference_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"#\[\[(?P<kind>file|folder):(?P<path>[^\]\n]+)\]\]")
            .expect("inline reference pattern must compile")
    })
}

fn file_reference_path(reference: &str) -> String {
    static SELECTOR_RE: OnceLock<Regex> = OnceLock::new();
    let selector = SELECTOR_RE.get_or_init(|| {
        Regex::new(r":\d+(?:-\d+)?$").expect("file line selector pattern must compile")
    });
    selector.replace(reference, "").into_owned()
}

fn has_parent_dir_traversal(reference: &str) -> bool {
    Path::new(reference)
        .components()
        .any(|component| matches!(component, Component::ParentDir))
}

fn yaml_type_name(value: &serde_yaml::Value) -> &'static str {
    match value {
        serde_yaml::Value::Null => "null",
        serde_yaml::Value::Bool(_) => "a boolean",
        serde_yaml::Value::Number(_) => "a number",
        serde_yaml::Value::String(_) => "a string",
        serde_yaml::Value::Sequence(_) => "a list",
        serde_yaml::Value::Mapping(_) => "a mapping",
        serde_yaml::Value::Tagged(_) => "a tagged value",
    }
}

pub struct KiroSteeringValidator;

/// Per-file context for the frontmatter rules.
struct Steering<'a> {
    path: &'a Path,
    content: &'a str,
    frontmatter: &'a str,
    frontmatter_start: usize,
    entries: Vec<YamlTopLevelEntry>,
}

impl Steering<'_> {
    fn entry(&self, key: &str) -> Option<&YamlTopLevelEntry> {
        self.entries.iter().find(|entry| entry.key == key)
    }

    /// Location of a top-level key, or of the first frontmatter line.
    fn key_location(&self, key: &str) -> (usize, usize) {
        let offset = self.entry(key).map_or(0, |entry| entry.start);
        location(self.content, self.frontmatter_start + offset)
    }

    /// Location of a top-level key's value (the key when the value starts on
    /// a later line).
    fn value_location(&self, key: &str) -> (usize, usize) {
        match self.entry(key) {
            Some(entry) if entry.value_start < entry.line_end => {
                location(self.content, self.frontmatter_start + entry.value_start)
            }
            _ => self.key_location(key),
        }
    }

    /// Location of `needle` inside a key's entry, falling back to the value.
    fn location_within(&self, key: &str, needle: &str) -> (usize, usize) {
        if let Some(entry) = self.entry(key)
            && let Some(found) = self.frontmatter[entry.start..entry.value_end].find(needle)
        {
            return location(self.content, self.frontmatter_start + entry.start + found);
        }
        self.value_location(key)
    }
}

impl Validator for KiroSteeringValidator {
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

        // KIRO-004: Empty steering file (check first, return early)
        if config.is_rule_enabled("KIRO-004") && content.trim().is_empty() {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    1,
                    "KIRO-004",
                    t!("rules.kiro_004.message"),
                )
                .with_suggestion(t!("rules.kiro_004.suggestion")),
            );
            return diagnostics;
        }

        // KIRO-006: Secrets in steering content
        if config.is_rule_enabled("KIRO-006")
            && let Some((offset, marker)) = find_secret(content)
        {
            let (line, col) = location(content, offset);
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "KIRO-006",
                    t!("rules.kiro_006.message", marker = marker),
                )
                .with_suggestion(t!("rules.kiro_006.suggestion")),
            );
        }

        // KIRO-009: Broken inline file and folder references
        if config.is_rule_enabled("KIRO-009") {
            check_inline_references(path, content, config, &mut diagnostics);
        }

        // KIRO-011: Steering doc excessively long (applies with or without frontmatter)
        let char_count = content.chars().count();
        if config.is_rule_enabled("KIRO-011") && char_count > MAX_STEERING_DOC_CHARS {
            diagnostics.push(
                Diagnostic::info(
                    path.to_path_buf(),
                    1,
                    1,
                    "KIRO-011",
                    t!(
                        "rules.kiro_011.message",
                        len = char_count.to_string(),
                        limit = MAX_STEERING_DOC_CHARS.to_string()
                    ),
                )
                .with_suggestion(t!("rules.kiro_011.suggestion")),
            );
        }

        let parts = split_frontmatter(content);
        let without_bom = content.strip_prefix('\u{FEFF}').unwrap_or(content);
        let frontmatter_at_start = without_bom.starts_with("---");

        // KIRO-014: Markdown structure (body, or the whole file without frontmatter)
        let body = if parts.has_frontmatter && parts.has_closing && frontmatter_at_start {
            parts.body.as_str()
        } else {
            content
        };
        if config.is_rule_enabled("KIRO-014")
            && !body.trim().is_empty()
            && !has_markdown_heading(body)
        {
            diagnostics.push(
                Diagnostic::info(
                    path.to_path_buf(),
                    1,
                    1,
                    "KIRO-014",
                    t!("rules.kiro_014.message"),
                )
                .with_suggestion(t!("rules.kiro_014.suggestion")),
            );
        }

        if !parts.has_frontmatter {
            return diagnostics; // No frontmatter: always included, nothing to check
        }

        // KIRO-015: frontmatter not on the first line is not frontmatter for Kiro
        if !frontmatter_at_start {
            if config.is_rule_enabled("KIRO-015") {
                let (line, col) = location(content, parts.frontmatter_start.saturating_sub(4));
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        col,
                        "KIRO-015",
                        t!("rules.kiro_015_not_first.message"),
                    )
                    .with_suggestion(t!("rules.kiro_015_not_first.suggestion")),
                );
            }
            return diagnostics;
        }

        if !parts.has_closing {
            if config.is_rule_enabled("KIRO-015") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        1,
                        "KIRO-015",
                        t!("rules.kiro_015_unclosed.message"),
                    )
                    .with_suggestion(t!("rules.kiro_015_unclosed.suggestion")),
                );
            }
            return diagnostics;
        }

        // KIRO-005: Empty body after valid frontmatter delimiters
        if config.is_rule_enabled("KIRO-005") && parts.body.trim().is_empty() {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    1,
                    "KIRO-005",
                    t!("rules.kiro_005.message"),
                )
                .with_suggestion(t!("rules.kiro_005.suggestion")),
            );
        }

        // KIRO-015: YAML that does not parse or is not a mapping
        let yaml: serde_yaml::Value = match serde_yaml::from_str(&parts.frontmatter) {
            Ok(value) => value,
            Err(error) => {
                if config.is_rule_enabled("KIRO-015") {
                    let duplicate = error
                        .to_string()
                        .contains("duplicate")
                        .then(|| duplicate_key_offset(&parts.frontmatter))
                        .flatten();
                    let (line, col) = duplicate
                        .map(|offset| location(content, parts.frontmatter_start + offset))
                        .or_else(|| {
                            error.location().map(|mark| {
                                let (fm_line, _) = location(content, parts.frontmatter_start);
                                (
                                    fm_line + mark.line().saturating_sub(1),
                                    mark.column().max(1),
                                )
                            })
                        })
                        .unwrap_or_else(|| location(content, parts.frontmatter_start));
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            col,
                            "KIRO-015",
                            t!("rules.kiro_015_yaml.message", error = error.to_string()),
                        )
                        .with_suggestion(t!("rules.kiro_015_yaml.suggestion")),
                    );
                }
                return diagnostics;
            }
        };
        let mapping = match &yaml {
            serde_yaml::Value::Mapping(mapping) => mapping,
            // An empty or comments-only block is no frontmatter: always included.
            serde_yaml::Value::Null => return diagnostics,
            other => {
                if config.is_rule_enabled("KIRO-015") {
                    let (line, col) = location(content, parts.frontmatter_start);
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            col,
                            "KIRO-015",
                            t!(
                                "rules.kiro_015_not_mapping.message",
                                kind = yaml_type_name(other)
                            ),
                        )
                        .with_suggestion(t!("rules.kiro_015_not_mapping.suggestion")),
                    );
                }
                return diagnostics;
            }
        };

        let steering = Steering {
            path,
            content,
            frontmatter: &parts.frontmatter,
            frontmatter_start: parts.frontmatter_start,
            entries: yaml_top_level_entries(&parts.frontmatter),
        };
        validate_mapping(&steering, &yaml, mapping, config, &mut diagnostics);
        diagnostics
    }
}

fn check_inline_references(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let fs = config.fs();
    for captures in inline_reference_pattern().captures_iter(content) {
        let Some(full_match) = captures.get(0) else {
            continue;
        };
        let Some(path_match) = captures.name("path") else {
            continue;
        };

        let kind = captures.name("kind").map(|m| m.as_str()).unwrap_or("file");
        let reference = path_match.as_str().trim();
        let resolved_reference = if kind == "file" {
            file_reference_path(reference)
        } else {
            reference.to_string()
        };
        if reference.is_empty()
            || reference.starts_with("http://")
            || reference.starts_with("https://")
            || reference.starts_with('/')
            || Path::new(&resolved_reference).is_absolute()
            || has_parent_dir_traversal(&resolved_reference)
        {
            continue;
        }

        // Workspace steering resolves from the workspace root. Global
        // steering and AGENTS.md references use their containing folder.
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let workspace_steering = config.root_dir().filter(|root| {
            path.starts_with(root.join(".kiro/steering")) || path.starts_with(".kiro/steering")
        });
        let base = workspace_steering.map_or(parent, |root| root.as_path());
        let resolved = base.join(&resolved_reference);

        let valid_target = if kind == "folder" {
            fs.is_dir(&resolved)
        } else {
            fs.is_file(&resolved)
        };
        if !valid_target {
            let (line, col) = location(content, full_match.start());
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "KIRO-009",
                    t!("rules.kiro_009.message", reference = reference),
                )
                .with_suggestion(t!("rules.kiro_009.suggestion")),
            );
        }
    }
}

/// The KIRO-001 autofix: replace the exact top-level `inclusion` scalar with
/// the closest valid mode, keeping the quote style, when the edit re-parses to
/// the same frontmatter with only `inclusion` changed. Always unsafe: a new
/// mode changes when Kiro loads the file.
fn inclusion_fix(
    steering: &Steering<'_>,
    yaml: &serde_yaml::Value,
    current: &str,
    suggested: &str,
) -> Option<Fix> {
    let entry = steering.entry("inclusion")?;
    let scalar = single_line_scalar_text(steering.frontmatter, entry)?;
    let replacement = if scalar.starts_with('"') {
        format!("\"{suggested}\"")
    } else if scalar.starts_with('\'') {
        format!("'{suggested}'")
    } else {
        suggested.to_string()
    };
    let start = steering.frontmatter_start + entry.value_start;
    let end = start + scalar.len();
    let mut expected = yaml.clone();
    expected.as_mapping_mut()?.insert(
        serde_yaml::Value::String("inclusion".into()),
        serde_yaml::Value::String(suggested.into()),
    );
    frontmatter_edit_yields(steering.content, start, end, &replacement, &expected).then(|| {
        Fix::replace(
            start,
            end,
            replacement,
            format!("Replace inclusion mode '{current}' with '{suggested}'"),
            false,
        )
    })
}

fn validate_mapping(
    steering: &Steering<'_>,
    yaml: &serde_yaml::Value,
    mapping: &serde_yaml::Mapping,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let path = steering.path;
    let get = |key: &str| mapping.get(serde_yaml::Value::String(key.into()));
    let inclusion_val = get("inclusion").filter(|value| !value.is_null());
    let inclusion_str = inclusion_val.and_then(|v| v.as_str());
    let pattern_val = get("fileMatchPattern");

    // KIRO-001: Invalid inclusion mode (`inclusion: null` means always)
    if config.is_rule_enabled("KIRO-001")
        && let Some(val) = inclusion_val
        && !inclusion_str.is_some_and(|mode| VALID_INCLUSION_MODES.contains(&mode))
    {
        let display = inclusion_str.map_or_else(
            || {
                serde_yaml::to_string(val)
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            },
            str::to_string,
        );
        let (line, col) = steering.value_location("inclusion");
        let mut diagnostic = Diagnostic::error(
            path.to_path_buf(),
            line,
            col,
            "KIRO-001",
            t!("rules.kiro_001.message", value = display.as_str()),
        )
        .with_suggestion(t!("rules.kiro_001.suggestion"));
        if let Some(current) = inclusion_str
            && let Some(suggested) =
                crate::rules::find_closest_value(current, VALID_INCLUSION_MODES)
            && let Some(fix) = inclusion_fix(steering, yaml, current, suggested)
        {
            diagnostic = diagnostic.with_fix(fix);
        }
        diagnostics.push(diagnostic);
    }

    // KIRO-015: V3 schema types. V2 reads only `inclusion`, so these break
    // only the V3 engine: an error when only V3 is targeted, a warning that
    // names V3 when both are.
    let engines = engines_for(config, false, false);
    if config.is_rule_enabled("KIRO-015") && engines.includes(KiroEngine::V3) {
        let mut type_errors: Vec<(&str, &'static str, &str)> = Vec::new();
        for field in ["name", "description"] {
            if let Some(value) = get(field)
                && !value.is_string()
            {
                type_errors.push((field, yaml_type_name(value), "a string"));
            }
        }
        if let Some(value) = pattern_val {
            let valid = match value {
                serde_yaml::Value::Null | serde_yaml::Value::String(_) => true,
                serde_yaml::Value::Sequence(items) => items.iter().all(|item| item.is_string()),
                _ => false,
            };
            if !valid {
                let kind = if value.is_sequence() {
                    "a list with non-string items"
                } else {
                    yaml_type_name(value)
                };
                type_errors.push(("fileMatchPattern", kind, "a string or a list of strings"));
            }
        }
        for (field, kind, expected) in type_errors {
            let (line, col) = steering.value_location(field);
            let message = t!(
                "rules.kiro_015_type.message",
                field = field,
                kind = kind,
                expected = expected
            );
            let diagnostic = if engines.only(KiroEngine::V3) {
                Diagnostic::error(path.to_path_buf(), line, col, "KIRO-015", message)
            } else {
                Diagnostic::warning(path.to_path_buf(), line, col, "KIRO-015", message)
            };
            diagnostics.push(diagnostic.with_suggestion(t!("rules.kiro_015_type.suggestion")));
        }
    }

    // KIRO-002: the inclusion mode needs a field to ever load the file
    if config.is_rule_enabled("KIRO-002") {
        match inclusion_str {
            Some("auto") => {
                // V3 activates auto steering by description only; `name` is
                // optional (the file name is the fallback).
                let description = get("description");
                let missing = description.is_none_or(|d| d.as_str() == Some(""));
                if missing {
                    let (line, col) = steering.key_location("inclusion");
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            col,
                            "KIRO-002",
                            t!("rules.kiro_002_auto.message"),
                        )
                        .with_suggestion(t!("rules.kiro_002_auto.suggestion")),
                    );
                }
            }
            Some("fileMatch") => {
                let no_pattern = match pattern_val {
                    None | Some(serde_yaml::Value::Null) => true,
                    Some(serde_yaml::Value::String(pattern)) => pattern.is_empty(),
                    Some(serde_yaml::Value::Sequence(items)) => {
                        items.iter().all(|item| item.as_str() == Some(""))
                    }
                    Some(_) => false, // wrong type: KIRO-015
                };
                if no_pattern {
                    let (line, col) = if pattern_val.is_some() {
                        steering.key_location("fileMatchPattern")
                    } else {
                        steering.key_location("inclusion")
                    };
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            col,
                            "KIRO-002",
                            t!("rules.kiro_002_filematch.message"),
                        )
                        .with_suggestion(t!("rules.kiro_002_filematch.suggestion")),
                    );
                }
            }
            _ => {}
        }
    }

    // KIRO-003: patterns that do not match the way they read
    if config.is_rule_enabled("KIRO-003") {
        let patterns: Vec<&str> = match pattern_val {
            Some(serde_yaml::Value::String(pattern)) => vec![pattern.as_str()],
            Some(serde_yaml::Value::Sequence(items)) => {
                items.iter().filter_map(|item| item.as_str()).collect()
            }
            _ => Vec::new(),
        };
        for pattern in patterns {
            let problem = if has_literal_comma(pattern) {
                Some("kiro_003_comma")
            } else if has_unclosed_class(pattern) {
                Some("kiro_003_bracket")
            } else {
                None
            };
            if let Some(key) = problem {
                let (line, col) = steering.location_within("fileMatchPattern", pattern);
                let message_key = format!("rules.{key}.message");
                let suggestion_key = format!("rules.{key}.suggestion");
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        col,
                        "KIRO-003",
                        t!(&message_key, pattern = pattern),
                    )
                    .with_suggestion(t!(&suggestion_key)),
                );
            }
        }
    }

    // KIRO-007: fileMatchPattern without inclusion: fileMatch (pattern unused,
    // file loaded by its actual mode, `always` when unset)
    if config.is_rule_enabled("KIRO-007")
        && pattern_val.is_some_and(|value| !value.is_null())
        && inclusion_str != Some("fileMatch")
    {
        let inclusion_display = inclusion_str.unwrap_or("<missing>");
        let (line, col) = steering.key_location("fileMatchPattern");
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                col,
                "KIRO-007",
                t!("rules.kiro_007.message", inclusion = inclusion_display),
            )
            .with_suggestion(t!("rules.kiro_007.suggestion")),
        );
    }

    // KIRO-008: Unknown frontmatter fields (Kiro drops them)
    if config.is_rule_enabled("KIRO-008") {
        for key in mapping.keys() {
            let Some(field) = key.as_str() else {
                continue;
            };
            if VALID_FRONTMATTER_FIELDS.contains(&field) {
                continue;
            }
            let (line, col) = steering.key_location(field);
            // Other tools' file-scoping keys: Kiro loads such a file always.
            let suggestion = if FILE_SCOPE_FIELDS.contains(&field) {
                t!("rules.kiro_008_scope.suggestion", field = field)
            } else {
                t!("rules.kiro_008.suggestion")
            };
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "KIRO-008",
                    t!("rules.kiro_008.message", field = field),
                )
                .with_suggestion(suggestion),
            );
        }
    }

    // KIRO-010: on-demand fields without an inclusion mode. Missing inclusion
    // means `always`, so `name`/`description` (read only for auto steering)
    // suggest the author expected on-demand loading.
    if config.is_rule_enabled("KIRO-010") && inclusion_val.is_none() {
        let field = ["description", "name"]
            .into_iter()
            .find(|field| get(field).is_some_and(|value| !value.is_null()));
        if let Some(field) = field {
            let (line, col) = steering.key_location(field);
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "KIRO-010",
                    t!("rules.kiro_010.message", field = field),
                )
                .with_suggestion(t!("rules.kiro_010.suggestion")),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LintConfig, ToolVersions};
    use crate::diagnostics::DiagnosticLevel;
    use crate::fs::{FileSystem, MockFileSystem};
    use std::sync::Arc;

    fn validate(path: &str, content: &str) -> Vec<Diagnostic> {
        let validator = KiroSteeringValidator;
        validator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn validate_steering(content: &str) -> Vec<Diagnostic> {
        validate(".kiro/steering/test.md", content)
    }

    fn rule<'a>(diagnostics: &'a [Diagnostic], id: &str) -> Vec<&'a Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == id).collect()
    }

    fn pinned(version: &str) -> LintConfig {
        LintConfig::builder()
            .tool_versions(ToolVersions {
                kiro: Some(version.to_string()),
                ..ToolVersions::default()
            })
            .build()
            .unwrap()
    }

    /// Apply every fix (safe and unsafe) through the fix engine.
    fn apply_all(path: &str, content: &str, diagnostics: &[Diagnostic], safe_only: bool) -> String {
        let fs = Arc::new(MockFileSystem::new());
        fs.add_file(path, content);
        crate::fixes::apply_fixes_with_fs(diagnostics, false, safe_only, Some(fs.clone())).unwrap();
        fs.read_to_string(Path::new(path)).unwrap()
    }

    fn frontmatter_value(content: &str) -> serde_yaml::Value {
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter && parts.has_closing, "{content}");
        serde_yaml::from_str(&parts.frontmatter).expect("frontmatter must parse")
    }

    // ===== YAML span helpers =====

    #[test]
    fn top_level_entries_find_keys_values_and_continuations() {
        let fm = "name: x\ntags:\n- a\n- b\n# comment\nmeta:\n  inclusion: nested\ndesc: \"multi\nline: still\"\nlist: [a,\n  b]\nlast: v # c";
        let entries = yaml_top_level_entries(fm);
        let keys: Vec<&str> = entries.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, ["name", "tags", "meta", "desc", "list", "last"]);
        let tags = &entries[1];
        assert_eq!(&fm[tags.start..tags.value_end], "tags:\n- a\n- b");
        assert!(!tags.single_line());
        let desc = &entries[3];
        assert_eq!(
            &fm[desc.start..desc.value_end],
            "desc: \"multi\nline: still\""
        );
        assert!(entries[0].single_line());
        assert_eq!(single_line_scalar_text(fm, &entries[0]), Some("x"));
        assert_eq!(single_line_scalar_text(fm, &entries[5]), Some("v"));
        assert_eq!(single_line_scalar_text(fm, &entries[4]), None);
    }

    #[test]
    fn edit_check_rejects_broken_yaml_and_body_changes() {
        let content = "---\na: 1\nb: 2\n---\nbody\n";
        let expected: serde_yaml::Value = serde_yaml::from_str("a: 1").unwrap();
        // Deleting `b: 2\n` keeps a valid mapping.
        assert!(frontmatter_edit_yields(content, 9, 14, "", &expected));
        // Deleting only `b` breaks the YAML.
        assert!(!frontmatter_edit_yields(content, 9, 10, "", &expected));
        // Touching the body is rejected.
        assert!(!frontmatter_edit_yields(
            content,
            18,
            22,
            "",
            &frontmatter_value(content)
        ));
    }

    // ===== KIRO-001: Invalid inclusion mode =====

    #[test]
    fn test_kiro_001_invalid_mode() {
        let content = "---\ninclusion: invalid_mode\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_001 = rule(&diagnostics, "KIRO-001");
        assert_eq!(kiro_001.len(), 1);
        assert_eq!(kiro_001[0].level, DiagnosticLevel::Error);
        assert!(kiro_001[0].message.contains("invalid_mode"));
        // Points at the value, not line 1 column 0.
        assert_eq!((kiro_001[0].line, kiro_001[0].column), (2, 12));
    }

    #[test]
    fn test_kiro_001_valid_modes() {
        for frontmatter in [
            "inclusion: always",
            "inclusion: auto\nname: test\ndescription: test desc",
            "inclusion: fileMatch\nfileMatchPattern: \"**/*.ts\"",
            "inclusion: manual",
            // `inclusion: null` passes the V3 schema and means always.
            "inclusion:",
        ] {
            let content = format!("---\n{frontmatter}\n---\n# Steering\n");
            let diagnostics = validate_steering(&content);
            assert!(rule(&diagnostics, "KIRO-001").is_empty(), "{frontmatter}");
        }
    }

    #[test]
    fn test_kiro_001_has_fix() {
        // Use a case-insensitive mismatch that find_closest_value can match
        let content = "---\ninclusion: Always\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_001 = rule(&diagnostics, "KIRO-001");
        assert_eq!(kiro_001.len(), 1);
        assert!(
            kiro_001[0].has_fixes(),
            "KIRO-001 should have auto-fix for case-mismatched inclusion mode"
        );
        let fix = &kiro_001[0].fixes[0];
        assert!(!fix.safe, "KIRO-001 fix should be unsafe");
        assert_eq!(&content[fix.start_byte..fix.end_byte], "Always");
        assert_eq!(fix.replacement, "always");

        // Safe mode leaves the file alone; unsafe mode fixes it and it re-parses.
        let path = ".kiro/steering/test.md";
        assert_eq!(apply_all(path, content, &diagnostics, true), content);
        let fixed = apply_all(path, content, &diagnostics, false);
        assert_eq!(fixed, "---\ninclusion: always\n---\n# Steering\n");
        assert!(validate_steering(&fixed).is_empty());
    }

    #[test]
    fn test_kiro_001_fix_targets_top_level_key_not_nested_decoy() {
        let path = ".kiro/steering/test.md";
        let content =
            "---\nmeta:\n  inclusion: keepme\ninclusion: 'Manual' # mode\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_001 = rule(&diagnostics, "KIRO-001");
        assert_eq!(kiro_001.len(), 1);
        assert_eq!(kiro_001[0].line, 4);
        let fixed = apply_all(path, content, &diagnostics, false);
        let value = frontmatter_value(&fixed);
        assert_eq!(value["inclusion"], "manual");
        assert_eq!(value["meta"]["inclusion"], "keepme");
        assert!(fixed.contains("inclusion: 'manual' # mode"));
    }

    #[test]
    fn test_kiro_001_non_string_inclusion_flagged_without_fix() {
        let content = "---\ninclusion: 123\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        let kiro_001 = rule(&diagnostics, "KIRO-001");
        assert_eq!(kiro_001.len(), 1);
        assert_eq!(kiro_001[0].level, DiagnosticLevel::Error);
        assert!(kiro_001[0].message.contains("123"));
        assert!(!kiro_001[0].has_fixes());
    }

    #[test]
    fn test_kiro_001_case_sensitive() {
        // Inclusion modes are case-sensitive - "ALWAYS" is not valid
        let content = "---\ninclusion: ALWAYS\n---\n# Content\n";
        assert_eq!(rule(&validate_steering(content), "KIRO-001").len(), 1);
    }

    #[test]
    fn test_kiro_001_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KIRO-001".to_string()];
        let diagnostics = KiroSteeringValidator.validate(
            Path::new(".kiro/steering/test.md"),
            "---\ninclusion: invalid_mode\n---\n# Steering\n",
            &config,
        );
        assert!(rule(&diagnostics, "KIRO-001").is_empty());
    }

    // ===== KIRO-002: inclusion mode without the field it needs =====

    #[test]
    fn test_kiro_002_auto_missing_name_is_fine() {
        // Changed expectation: the V3 engine falls back to the file name, so a
        // missing `name` no longer reports KIRO-002 (only description matters).
        let content = "---\ninclusion: auto\ndescription: test desc\n---\n# Steering\n";
        assert!(rule(&validate_steering(content), "KIRO-002").is_empty());
    }

    #[test]
    fn test_kiro_002_auto_missing_description() {
        let content = "---\ninclusion: auto\nname: my-steering\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002 = rule(&diagnostics, "KIRO-002");
        assert_eq!(kiro_002.len(), 1);
        assert_eq!(kiro_002[0].level, DiagnosticLevel::Error);
        assert!(kiro_002[0].message.contains("description"));
        assert_eq!(kiro_002[0].line, 2);
    }

    #[test]
    fn test_kiro_002_auto_missing_both_reports_description_once() {
        // Changed expectation: one finding (description); name is optional.
        let content = "---\ninclusion: auto\n---\n# Steering\n";
        assert_eq!(rule(&validate_steering(content), "KIRO-002").len(), 1);
    }

    #[test]
    fn test_kiro_002_auto_empty_description_flagged() {
        let content = "---\ninclusion: auto\ndescription: \"\"\n---\n# Steering\n";
        assert_eq!(rule(&validate_steering(content), "KIRO-002").len(), 1);
    }

    #[test]
    fn test_kiro_002_auto_empty_name_is_fine() {
        // Changed expectation: an empty-string name passes the V3 schema and
        // the description is present, so nothing is reported.
        let content = "---\ninclusion: auto\nname: \"\"\ndescription: test desc\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        assert!(rule(&diagnostics, "KIRO-002").is_empty());
        assert!(rule(&diagnostics, "KIRO-015").is_empty());
    }

    #[test]
    fn test_kiro_002_auto_valid() {
        let content =
            "---\ninclusion: auto\nname: my-steering\ndescription: test desc\n---\n# Steering\n";
        assert!(rule(&validate_steering(content), "KIRO-002").is_empty());
    }

    #[test]
    fn test_kiro_002_filematch_missing_or_empty_pattern() {
        for frontmatter in [
            "inclusion: fileMatch",
            "inclusion: fileMatch\nfileMatchPattern:",
            "inclusion: fileMatch\nfileMatchPattern: \"\"",
            "inclusion: fileMatch\nfileMatchPattern: []",
        ] {
            let content = format!("---\n{frontmatter}\n---\n# Steering\n");
            let diagnostics = validate_steering(&content);
            let kiro_002 = rule(&diagnostics, "KIRO-002");
            assert_eq!(kiro_002.len(), 1, "{frontmatter}");
            assert!(kiro_002[0].message.contains("fileMatchPattern"));
        }
    }

    #[test]
    fn test_kiro_002_filematch_valid() {
        for pattern in [
            "\"**/*.ts\"",
            "[\"**/*.ts\", \"**/*.tsx\"]",
            "\n  - src/**\n  - lib/**",
        ] {
            let content = format!(
                "---\ninclusion: fileMatch\nfileMatchPattern: {pattern}\n---\n# Steering\n"
            );
            let diagnostics = validate_steering(&content);
            assert!(rule(&diagnostics, "KIRO-002").is_empty(), "{pattern}");
        }
    }

    #[test]
    fn test_kiro_002_always_and_manual_need_nothing() {
        for mode in ["always", "manual"] {
            let content = format!("---\ninclusion: {mode}\n---\n# Steering\n");
            assert!(rule(&validate_steering(&content), "KIRO-002").is_empty());
        }
    }

    #[test]
    fn test_kiro_002_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KIRO-002".to_string()];
        let diagnostics = KiroSteeringValidator.validate(
            Path::new(".kiro/steering/test.md"),
            "---\ninclusion: auto\n---\n# Steering\n",
            &config,
        );
        assert!(rule(&diagnostics, "KIRO-002").is_empty());
    }

    // ===== KIRO-003: fileMatchPattern that does not match as written =====

    #[test]
    fn test_kiro_003_documented_array_form_accepted() {
        // Corpus false positive: the documented array form is valid.
        for pattern in [
            "[\"**/*.ts\", \"**/*.tsx\", \"**/tsconfig.*.json\"]",
            "\n  - \"**/*.{py,java,js,jsx}\"\n  - \"**/{package.json,requirements.txt}\"",
            "[\"src/net/**/*.ts\"]",
        ] {
            let content = format!(
                "---\ninclusion: fileMatch\nfileMatchPattern: {pattern}\n---\n# Steering\n"
            );
            let diagnostics = validate_steering(&content);
            assert!(
                diagnostics.is_empty(),
                "array form must be accepted: {pattern}: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn test_kiro_003_minimatch_patterns_the_glob_crate_rejects_are_valid() {
        // `*gps*` style segments around `**` are fine for minimatch.
        let content =
            "---\ninclusion: fileMatch\nfileMatchPattern: \"src/**gps**/*.ts\"\n---\n# S\n";
        assert!(rule(&validate_steering(content), "KIRO-003").is_empty());
    }

    #[test]
    fn test_kiro_003_literal_comma_flagged_at_value() {
        let content =
            "---\ninclusion: fileMatch\nfileMatchPattern: \"**/*gps*,**/*nmea*\"\n---\n# S\n";
        let diagnostics = validate_steering(content);
        let kiro_003 = rule(&diagnostics, "KIRO-003");
        assert_eq!(kiro_003.len(), 1);
        assert_eq!(kiro_003[0].level, DiagnosticLevel::Warning);
        assert!(kiro_003[0].message.contains("comma"));
        assert_eq!((kiro_003[0].line, kiro_003[0].column), (3, 20));
    }

    #[test]
    fn test_kiro_003_literal_comma_in_array_item_located() {
        let content = "---\ninclusion: fileMatch\nfileMatchPattern:\n  - \"*.md\"\n  - \"*.ts,*.tsx\"\n---\n# S\n";
        let diagnostics = validate_steering(content);
        let kiro_003 = rule(&diagnostics, "KIRO-003");
        assert_eq!(kiro_003.len(), 1);
        assert_eq!(kiro_003[0].line, 5);
    }

    #[test]
    fn test_kiro_003_brace_and_class_commas_are_alternation() {
        for pattern in [
            "**/*.{ts,tsx}",
            "src/[a,b]/*.md",
            "{a,{b,c}}/*.md",
            "file{1..3},x",
        ] {
            let content =
                format!("---\ninclusion: fileMatch\nfileMatchPattern: \"{pattern}\"\n---\n# S\n");
            let diagnostics = validate_steering(&content);
            // The numeric-range form is skipped by Kiro's own check too.
            assert!(rule(&diagnostics, "KIRO-003").is_empty(), "{pattern}");
        }
        assert!(has_literal_comma("{a,b},c"));
        assert!(has_literal_comma("a,b"));
    }

    #[test]
    fn test_kiro_003_bad_glob() {
        let content = "---\nfileMatchPattern: \"[unclosed\"\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_003 = rule(&diagnostics, "KIRO-003");
        assert_eq!(kiro_003.len(), 1);
        assert_eq!(kiro_003[0].level, DiagnosticLevel::Warning);
        assert!(!has_unclosed_class("src/[ab]/*.md"));
        assert!(!has_unclosed_class("src/[]]/*.md"));
        assert!(has_unclosed_class("src/[]"));
    }

    #[test]
    fn test_kiro_003_valid_glob() {
        let content = "---\nfileMatchPattern: \"**/*.ts\"\n---\n# Steering\n";
        assert!(rule(&validate_steering(content), "KIRO-003").is_empty());
    }

    #[test]
    fn test_kiro_003_empty_string_pattern() {
        // Empty string is not a malformed pattern
        let content = "---\nfileMatchPattern: \"\"\n---\n# Content\n";
        assert!(rule(&validate_steering(content), "KIRO-003").is_empty());
    }

    #[test]
    fn test_kiro_003_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KIRO-003".to_string()];
        let diagnostics = KiroSteeringValidator.validate(
            Path::new(".kiro/steering/test.md"),
            "---\nfileMatchPattern: \"[unclosed\"\n---\n# Steering\n",
            &config,
        );
        assert!(rule(&diagnostics, "KIRO-003").is_empty());
    }

    // ===== KIRO-004: Empty steering file =====

    #[test]
    fn test_kiro_004_empty_file() {
        let diagnostics = validate_steering("");
        let kiro_004 = rule(&diagnostics, "KIRO-004");
        assert_eq!(kiro_004.len(), 1);
        assert_eq!(kiro_004[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_kiro_004_whitespace_only() {
        assert_eq!(rule(&validate_steering("   \n\n  "), "KIRO-004").len(), 1);
    }

    #[test]
    fn test_kiro_004_valid_file() {
        let diagnostics = validate_steering("---\ninclusion: always\n---\n# Guidelines\n");
        assert!(rule(&diagnostics, "KIRO-004").is_empty());
    }

    #[test]
    fn test_kiro_004_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KIRO-004".to_string()];
        let diagnostics =
            KiroSteeringValidator.validate(Path::new(".kiro/steering/test.md"), "", &config);
        assert!(rule(&diagnostics, "KIRO-004").is_empty());
    }

    // ===== Category disable =====

    #[test]
    fn test_kiro_steering_category_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().kiro_steering = false;
        let validator = KiroSteeringValidator;
        for content in [
            "",
            "---\ninclusion: invalid\n---\n# Test\n",
            "\n---\nbad: [\n---\nx",
        ] {
            let diagnostics =
                validator.validate(Path::new(".kiro/steering/test.md"), content, &config);
            assert!(
                diagnostics.iter().all(|d| !d.rule.starts_with("KIRO-")),
                "{content:?}: {diagnostics:?}"
            );
        }
    }

    // ===== Edge cases =====

    #[test]
    fn test_no_frontmatter_no_diagnostics() {
        let diagnostics = validate_steering("# Just a heading\nSome content.");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_empty_or_comment_only_frontmatter_is_no_frontmatter() {
        for content in ["---\n---\n# Body\n", "---\n# just a comment\n---\n# Body\n"] {
            assert!(validate_steering(content).is_empty(), "{content:?}");
        }
    }

    #[test]
    fn test_frontmatter_only_no_body_not_empty() {
        // File with frontmatter but no body is not "empty"
        let content = "---\ninclusion: always\n---\n";
        assert!(rule(&validate_steering(content), "KIRO-004").is_empty());
    }

    #[test]
    fn test_kiro_005_frontmatter_only_body() {
        let content = "---\ninclusion: always\n---\n";
        assert!(
            validate_steering(content)
                .iter()
                .any(|d| d.rule == "KIRO-005")
        );
    }

    // ===== KIRO-006: Secrets =====

    #[test]
    fn test_kiro_006_secrets_detected() {
        let content = "---\ninclusion: always\n---\nAPI_KEY=hardcodedsecret123\n";
        let diagnostics = validate_steering(content);
        let kiro_006 = rule(&diagnostics, "KIRO-006");
        assert_eq!(kiro_006.len(), 1);
        assert_eq!((kiro_006[0].line, kiro_006[0].column), (4, 1));
    }

    #[test]
    fn test_kiro_006_scans_past_template_values() {
        let content =
            "---\ninclusion: always\n---\nTOKEN=${ENV_TOKEN}\npassword=plaintextsecret123\n";
        assert!(
            validate_steering(content)
                .iter()
                .any(|d| d.rule == "KIRO-006")
        );
    }

    #[test]
    fn test_kiro_006_ignores_plain_prose_secret_label() {
        let content = "---\ninclusion: always\n---\nSecret: guidelines\n";
        assert!(
            validate_steering(content)
                .iter()
                .all(|d| d.rule != "KIRO-006")
        );
    }

    #[test]
    fn test_kiro_006_detects_identifier_style_secret_keys() {
        let content = "---\ninclusion: always\n---\nclient_secret: hardcodedsecret123\n";
        assert!(
            validate_steering(content)
                .iter()
                .any(|d| d.rule == "KIRO-006")
        );
    }

    #[test]
    fn test_kiro_006_all_template_values_no_fire() {
        let content = "---\ninclusion: always\n---\n# Config\napi_key= ${API_KEY}\ntoken= $(get_token)\npassword= {{VAULT_PW}}\nsecret= <from-env>\n";
        assert!(
            validate_steering(content)
                .iter()
                .all(|d| d.rule != "KIRO-006")
        );
    }

    #[test]
    fn test_kiro_006_ignores_identifiers_in_code_samples() {
        // Corpus false positives: variable names, fields, types and calls.
        for line in [
            "      password: hashedPassword",
            "  password: user.password,  // will be redacted",
            "        self.api_key = self.secrets.get('my_api_key', '')",
            "let password: SecretString = SecretString::new(value.into());",
            "token = process.env.GITHUB_TOKEN",
            "api_key: $OPENAI_API_KEY",
        ] {
            let content = format!("# Code\n```\n{line}\n```\n");
            assert!(
                validate_steering(&content)
                    .iter()
                    .all(|d| d.rule != "KIRO-006"),
                "{line}"
            );
        }
    }

    #[test]
    fn test_kiro_006_flags_quoted_literal_in_code_sample() {
        // Corpus true positive: a concrete password in a shell sample.
        let content = "# Deploy\n```bash\nPASSWORD=\"DeployTest!2025\"\n```\n";
        let diagnostics = validate_steering(content);
        let kiro_006 = rule(&diagnostics, "KIRO-006");
        assert_eq!(kiro_006.len(), 1);
        assert_eq!(kiro_006[0].line, 3);
    }

    // ===== KIRO-007 / KIRO-008 =====

    #[test]
    fn test_kiro_007_file_match_pattern_without_file_match_mode() {
        let content = "---\ninclusion: always\nfileMatchPattern: \"**/*.md\"\n---\n# body\n";
        let diagnostics = validate_steering(content);
        let kiro_007 = rule(&diagnostics, "KIRO-007");
        assert_eq!(kiro_007.len(), 1);
        assert_eq!((kiro_007[0].line, kiro_007[0].column), (3, 1));
    }

    #[test]
    fn test_kiro_007_null_pattern_ignored() {
        let content = "---\ninclusion: always\nfileMatchPattern:\n---\n# body\n";
        assert!(rule(&validate_steering(content), "KIRO-007").is_empty());
    }

    #[test]
    fn test_kiro_008_scope_keys_suggest_file_match() {
        let content = "---\npaths:\n  - \"manifests/**\"\n---\n# Manifests\n";
        let diagnostics = validate_steering(content);
        let kiro_008 = rule(&diagnostics, "KIRO-008");
        assert_eq!(kiro_008.len(), 1);
        let suggestion = kiro_008[0].suggestion.as_deref().unwrap();
        assert!(
            suggestion.contains("fileMatchPattern") && suggestion.contains("'paths'"),
            "{suggestion}"
        );
    }

    #[test]
    fn test_kiro_008_unknown_frontmatter_field() {
        let content = "---\ninclusion: always\ninclusions: true\n---\n# body\n";
        let diagnostics = validate_steering(content);
        let kiro_008 = rule(&diagnostics, "KIRO-008");
        assert_eq!(kiro_008.len(), 1);
        assert_eq!((kiro_008[0].line, kiro_008[0].column), (3, 1));
    }

    // ===== KIRO-009 =====

    #[test]
    fn test_kiro_009_missing_inline_file_reference() {
        let content = "---\ninclusion: always\n---\nUse #[[file:docs/missing.md]]\n";
        assert!(
            validate_steering(content)
                .iter()
                .any(|d| d.rule == "KIRO-009")
        );
    }

    #[test]
    fn test_kiro_009_skips_absolute_inline_file_reference() {
        let content = "---\ninclusion: always\n---\nUse #[[file:/etc/passwd]]\n";
        assert!(
            validate_steering(content)
                .iter()
                .all(|d| d.rule != "KIRO-009")
        );
    }

    #[test]
    fn test_kiro_009_skips_parent_dir_traversal_inline_file_reference() {
        let content = "---\ninclusion: always\n---\nUse #[[file:../../secrets.txt]]\n";
        assert!(
            validate_steering(content)
                .iter()
                .all(|d| d.rule != "KIRO-009")
        );
    }

    #[test]
    fn test_kiro_009_strips_line_selectors_before_lookup() {
        assert_eq!(file_reference_path("docs/api.md:12"), "docs/api.md");
        assert_eq!(file_reference_path("docs/api.md:12-28"), "docs/api.md");
        assert_eq!(file_reference_path("docs/api:v3.md"), "docs/api:v3.md");
    }

    #[test]
    fn test_kiro_009_workspace_targets_and_selectors() {
        let fs = Arc::new(MockFileSystem::new());
        fs.add_file("/project/docs/api.md", "one\ntwo\nthree\n");
        fs.add_dir("/project/config");
        let mut config = LintConfig::default();
        config.set_fs(fs);
        config.set_root_dir("/project".into());
        for reference in [
            "file:docs/api.md",
            "file:docs/api.md:2",
            "file:docs/api.md:1-3",
            "folder:config",
        ] {
            let content = format!("# Context\n#[[{reference}]]\n");
            for path in ["/project/.kiro/steering/test.md", ".kiro/steering/test.md"] {
                let diagnostics =
                    KiroSteeringValidator.validate(Path::new(path), &content, &config);
                assert!(
                    diagnostics.iter().all(|d| d.rule != "KIRO-009"),
                    "{reference} at {path}"
                );
            }
        }
    }

    #[test]
    fn test_kiro_009_rejects_missing_and_wrong_kind_targets() {
        let fs = Arc::new(MockFileSystem::new());
        fs.add_file("/project/docs/api.md", "content");
        fs.add_dir("/project/config");
        let mut config = LintConfig::default();
        config.set_fs(fs);
        config.set_root_dir("/project".into());
        for reference in [
            "file:missing.md:2",
            "folder:missing",
            "folder:docs/api.md",
            "file:config",
        ] {
            let diagnostics = KiroSteeringValidator.validate(
                Path::new("/project/.kiro/steering/test.md"),
                &format!("# Context\n#[[{reference}]]\n"),
                &config,
            );
            assert_eq!(
                diagnostics.iter().filter(|d| d.rule == "KIRO-009").count(),
                1,
                "{reference}"
            );
        }
    }

    #[test]
    fn test_kiro_009_global_and_agents_references_use_containing_folder() {
        let fs = Arc::new(MockFileSystem::new());
        fs.add_file("/global/.kiro/steering/style.md", "global");
        fs.add_file("/project/services/style.md", "service");
        let mut config = LintConfig::default();
        config.set_fs(fs);
        config.set_root_dir("/project".into());
        for path in [
            "/global/.kiro/steering/test.md",
            "/project/services/AGENTS.md",
        ] {
            let diagnostics = KiroSteeringValidator.validate(
                Path::new(path),
                "# Context\n#[[file:style.md:1]]\n",
                &config,
            );
            assert!(diagnostics.iter().all(|d| d.rule != "KIRO-009"), "{path}");
        }
    }

    // ===== KIRO-010: on-demand fields without an inclusion mode =====

    #[test]
    fn test_kiro_010_missing_inclusion_with_on_demand_fields() {
        for (frontmatter, line) in [
            ("name: test", 2),
            ("title: x\ndescription: when to load", 3),
            ("inclusion:\ndescription: when", 3),
        ] {
            let content = format!("---\n{frontmatter}\n---\n# Steering\n");
            let diagnostics = validate_steering(&content);
            let kiro_010 = rule(&diagnostics, "KIRO-010");
            assert_eq!(kiro_010.len(), 1, "{frontmatter}");
            assert_eq!(kiro_010[0].line, line, "{frontmatter}");
        }
    }

    #[test]
    fn test_kiro_010_default_always_is_not_flagged() {
        // Changed expectation: a missing inclusion is the documented default
        // (always); without on-demand fields there is nothing to report.
        for content in [
            "---\ntitle: Conventions\n---\n# Steering\n",
            "---\ninclusion: always\n---\n# Steering\n",
            "# No frontmatter\n",
        ] {
            assert!(
                rule(&validate_steering(content), "KIRO-010").is_empty(),
                "{content}"
            );
        }
    }

    // ===== KIRO-011: Steering doc excessively long =====

    #[test]
    fn test_kiro_011_excessively_long_doc_renders_numbers() {
        let body = "x".repeat(51_000);
        for content in [
            format!("---\ninclusion: always\n---\n# T\n{body}\n"),
            format!("# T\n{body}\n"),
        ] {
            let diagnostics = validate_steering(&content);
            let kiro_011 = rule(&diagnostics, "KIRO-011");
            assert_eq!(kiro_011.len(), 1);
            assert!(
                !kiro_011[0].message.contains("%{"),
                "{}",
                kiro_011[0].message
            );
            assert!(
                kiro_011[0].message.contains("50000"),
                "{}",
                kiro_011[0].message
            );
        }
    }

    #[test]
    fn test_kiro_011_counts_characters_not_bytes() {
        let content = format!("# T\n{}\n", "\u{4e2d}".repeat(20_000));
        assert!(rule(&validate_steering(&content), "KIRO-011").is_empty());
    }

    #[test]
    fn test_kiro_011_normal_length_no_diagnostic() {
        let content = "---\ninclusion: always\n---\n# Short doc\n";
        assert!(rule(&validate_steering(content), "KIRO-011").is_empty());
    }

    // ===== KIRO-014: Markdown structure =====

    #[test]
    fn test_kiro_014_no_heading_in_body() {
        for content in [
            "---\ninclusion: always\n---\nJust plain text without any heading.\n",
            "Plain text, no frontmatter.\n",
            // An inline reference or a hashtag is not a heading.
            "Intro\n#[[file:docs/a.md]]\n#tag\n",
            "Intro\n```\n# comment in code\n```\n",
        ] {
            assert_eq!(
                rule(&validate_steering(content), "KIRO-014").len(),
                1,
                "{content}"
            );
        }
    }

    #[test]
    fn test_kiro_014_has_heading_no_diagnostic() {
        for content in [
            "---\ninclusion: always\n---\n# Heading\nSome content.\n",
            "Intro\n\n## Section\ntext\n",
            "Title\n=====\ntext\n",
        ] {
            assert!(
                rule(&validate_steering(content), "KIRO-014").is_empty(),
                "{content}"
            );
        }
    }

    // ===== KIRO-015: frontmatter Kiro cannot use =====

    #[test]
    fn test_kiro_015_malformed_yaml_reported_at_error() {
        // Changed expectation: malformed YAML used to be silent.
        let content = "---\ninclusion: auto\n  bad: indentation\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        let kiro_015 = rule(&diagnostics, "KIRO-015");
        assert_eq!(kiro_015.len(), 1, "{diagnostics:?}");
        assert_eq!(kiro_015[0].level, DiagnosticLevel::Error);
        assert!(
            kiro_015[0].line >= 2 && kiro_015[0].line <= 3,
            "{:?}",
            kiro_015[0]
        );
    }

    #[test]
    fn test_kiro_015_duplicate_inclusion_key() {
        // Replaces the retired KIRO-013: duplicate keys make the YAML invalid.
        let content = "---\ninclusion: always\ninclusion: manual\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        let kiro_015 = rule(&diagnostics, "KIRO-015");
        assert_eq!(kiro_015.len(), 1);
        assert!(
            kiro_015[0].message.contains("inclusion"),
            "{}",
            kiro_015[0].message
        );
    }

    #[test]
    fn test_kiro_015_nested_inclusion_is_not_a_duplicate() {
        let content = "---\nmeta:\n  inclusion: x\ninclusion: always\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        assert!(rule(&diagnostics, "KIRO-015").is_empty());
        assert!(rule(&diagnostics, "KIRO-001").is_empty());
    }

    #[test]
    fn test_kiro_015_non_mapping_yaml() {
        let content = "---\n- item1\n- item2\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        assert_eq!(rule(&diagnostics, "KIRO-015").len(), 1);
    }

    #[test]
    fn test_kiro_015_frontmatter_not_on_first_line() {
        let content = "\n---\ninclusion: manual\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        let kiro_015 = rule(&diagnostics, "KIRO-015");
        assert_eq!(kiro_015.len(), 1);
        assert_eq!(kiro_015[0].line, 2);
        // The block is not frontmatter for Kiro, so no frontmatter rules run.
        assert!(rule(&diagnostics, "KIRO-001").is_empty());
    }

    #[test]
    fn test_kiro_015_unclosed_frontmatter() {
        let content = "---\ninclusion: manual\n# Content\n";
        assert_eq!(rule(&validate_steering(content), "KIRO-015").len(), 1);
    }

    #[test]
    fn test_kiro_015_v3_type_errors_depend_on_engine() {
        // Changed expectation: `name: null` used to be KIRO-002; it is a V3
        // schema failure (the frontmatter is dropped), and V2 never reads name.
        let content = "---\ninclusion: auto\nname: null\ndescription: test desc\n---\n# Steering\n";
        let path = Path::new(".kiro/steering/test.md");
        let both = KiroSteeringValidator.validate(path, content, &LintConfig::default());
        let kiro_015 = rule(&both, "KIRO-015");
        assert_eq!(kiro_015.len(), 1);
        assert_eq!(kiro_015[0].level, DiagnosticLevel::Warning);
        assert!(kiro_015[0].message.contains("V3"));
        assert_eq!(kiro_015[0].line, 3);
        assert!(rule(&both, "KIRO-002").is_empty());

        let v3 = KiroSteeringValidator.validate(path, content, &pinned("3.0.0"));
        assert_eq!(rule(&v3, "KIRO-015")[0].level, DiagnosticLevel::Error);
        let v2 = KiroSteeringValidator.validate(path, content, &pinned("2.32.1"));
        assert!(rule(&v2, "KIRO-015").is_empty());
    }

    #[test]
    fn test_kiro_015_pattern_type_errors() {
        // Changed expectation: a non-string pattern was KIRO-003; it is a V3
        // schema failure.
        for pattern in ["123", "{a: b}", "[1, \"*.md\"]"] {
            let content =
                format!("---\ninclusion: fileMatch\nfileMatchPattern: {pattern}\n---\n# S\n");
            let diagnostics = validate_steering(&content);
            assert_eq!(rule(&diagnostics, "KIRO-015").len(), 1, "{pattern}");
            assert!(rule(&diagnostics, "KIRO-003").is_empty(), "{pattern}");
            assert!(rule(&diagnostics, "KIRO-002").is_empty(), "{pattern}");
        }
    }

    #[test]
    fn test_steering_fixtures() {
        macro_rules! fixture {
            ($name:literal) => {
                (
                    $name,
                    include_str!(concat!(
                        "../../../../tests/fixtures/kiro-steering/.kiro/steering/",
                        $name
                    )),
                )
            };
        }
        let cases: &[((&str, &str), &[&str])] = &[
            (fixture!("bad-glob.md"), &["KIRO-003"]),
            (fixture!("code-samples.md"), &["KIRO-006"]),
            (fixture!("duplicate-inclusion.md"), &["KIRO-015"]),
            (fixture!("empty.md"), &["KIRO-004"]),
            (fixture!("frontmatter-not-first.md"), &["KIRO-015"]),
            (fixture!("invalid-mode.md"), &["KIRO-001"]),
            (fixture!("literal-comma-pattern.md"), &["KIRO-003"]),
            (fixture!("missing-fields-auto.md"), &["KIRO-002"]),
            (fixture!("missing-pattern-filematch.md"), &["KIRO-002"]),
            (fixture!("no-heading.md"), &["KIRO-009", "KIRO-014"]),
            (fixture!("on-demand-without-inclusion.md"), &["KIRO-010"]),
            (fixture!("unknown-fields.md"), &["KIRO-007", "KIRO-008"]),
            (fixture!("valid-always.md"), &[]),
            (fixture!("valid-array-pattern.md"), &[]),
            (fixture!("valid-auto.md"), &[]),
            (fixture!("valid-filematch.md"), &[]),
            (fixture!("valid-manual.md"), &[]),
        ];
        for ((name, content), expected) in cases {
            let diagnostics = validate(&format!(".kiro/steering/{name}"), content);
            let mut rules: Vec<&str> = diagnostics.iter().map(|d| d.rule.as_str()).collect();
            rules.sort_unstable();
            assert_eq!(&rules, expected, "{name}: {diagnostics:?}");
        }
    }

    // ===== Messages and metadata =====

    #[test]
    fn test_messages_render_all_placeholders() {
        let long = format!(
            "---\ninclusion: Bogus\nname: 1\nfoo: x\nfileMatchPattern: \"a,b\"\n---\n{}\n",
            "y".repeat(50_001)
        );
        let mut all = validate_steering(&long);
        all.extend(validate_steering("---\ninclusion: auto\n---\nx\n"));
        all.extend(validate_steering(
            "---\ninclusion: fileMatch\nfileMatchPattern: \"[x\"\n---\nx\n",
        ));
        all.extend(validate_steering("---\nname: x\n---\n# x\n"));
        all.extend(validate_steering("\n---\na: 1\n---\n"));
        all.extend(validate_steering("---\na: [\n---\n"));
        all.extend(validate_steering("---\n- a\n---\n"));
        all.extend(validate_steering("---\na: 1\n"));
        assert!(all.len() >= 12, "{all:?}");
        for diagnostic in &all {
            assert!(!diagnostic.message.contains("%{"), "{}", diagnostic.message);
            assert!(
                diagnostic
                    .suggestion
                    .as_deref()
                    .is_none_or(|s| !s.contains("%{")),
                "{:?}",
                diagnostic.suggestion
            );
            assert!(
                !diagnostic.message.starts_with("rules."),
                "{}",
                diagnostic.message
            );
        }
    }

    #[test]
    fn test_line_col_at_offset_is_one_based() {
        assert_eq!(line_col_at_offset("secret", 0), (1, 1));
        assert_eq!(line_col_at_offset("x\nsecret", 2), (2, 1));
    }

    #[test]
    fn test_metadata() {
        let v = KiroSteeringValidator;
        let meta = v.metadata();
        assert_eq!(meta.name, "KiroSteeringValidator");
        assert_eq!(
            meta.rule_ids,
            &[
                "KIRO-001", "KIRO-002", "KIRO-003", "KIRO-004", "KIRO-005", "KIRO-006", "KIRO-007",
                "KIRO-008", "KIRO-009", "KIRO-010", "KIRO-011", "KIRO-014", "KIRO-015",
            ]
        );
    }
}
