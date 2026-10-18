//! Amp validation rules: code review checks, AGENTS.md @-mentions and
//! settings files.
//!
//! - AMP-001: invalid check file (ERROR): no frontmatter, invalid YAML, no
//!   `name`, no instructions; unknown or mistyped keys are warnings
//! - AMP-002: invalid `severity-default` value (WARNING)
//! - AMP-003: invalid `globs` frontmatter in a file AGENTS.md @-mentions
//!   (WARNING)
//! - AMP-004, AMP-007 (and AMP-006, AMP-010 on `amp.mcpServers`): settings
//!   files, see `amp_settings`
//!
//! Sources: ampcode.com/docs/review ("Check File Format"),
//! ampcode.com/docs/customize/agents-md ("Writing AGENTS.md Files",
//! "Granular Guidance"), ampcode.com/docs/cli/settings.

use super::amp_mcp::{Sink, yaml_node};
use crate::{
    FileType,
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel, Fix},
    fs::FileSystem,
    parsers::frontmatter::split_frontmatter,
    rules::{Validator, ValidatorMetadata},
    schemas::kiro_json::{JsonMember, JsonNode},
};
use rust_i18n::t;
use serde_yaml::Value as YamlValue;
use std::collections::HashSet;
use std::ops::Range;
use std::path::{Component, Path, PathBuf};

/// AMP-006 is also emitted by the Amp skill validator (skill `mcp.json` and
/// SKILL.md `mcpServers`); AMP-010 is owned by that validator and emitted
/// here for `amp.mcpServers`.
const RULE_IDS: &[&str] = &[
    "AMP-001", "AMP-002", "AMP-003", "AMP-004", "AMP-006", "AMP-007",
];

const VALID_SEVERITY_DEFAULT: &[&str] = &["low", "medium", "high", "critical"];
/// Documented check keys, plus `tools`, which the current page no longer
/// lists. Whether the review-checks plugin still reads `tools` is
/// unverified, so it is accepted (and type-checked) rather than reported.
const VALID_CHECK_KEYS: &[&str] = &["name", "description", "severity-default", "tools"];

/// Agent files whose @-mentions AMP-003 follows.
const AGENTS_FILES: &[&str] = &["AGENTS.md", "AGENTS.local.md", "AGENTS.override.md"];
/// Mentioned files read per agent file.
const MAX_MENTIONED_FILES: usize = 64;
/// Directory entries visited per agent file when expanding glob mentions.
const MAX_GLOB_ENTRIES: usize = 4096;
/// Directories a glob mention never descends into.
const SKIPPED_DIRS: &[&str] = &[".git", "node_modules", "target"];

pub struct AmpValidator;

impl Validator for AmpValidator {
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
        match crate::file_types::detect_file_type(path) {
            FileType::AmpCheck => validate_amp_check(path, content, config),
            FileType::AmpSettings => {
                super::amp_settings::validate_amp_settings(path, content, config)
            }
            FileType::ClaudeMd => validate_amp_mentions(path, content, config),
            _ => Vec::new(),
        }
    }
}

/// Byte range of the frontmatter text (between the `---` lines).
fn frontmatter_range(parts: &crate::parsers::frontmatter::FrontmatterParts) -> Range<usize> {
    parts.frontmatter_start..parts.frontmatter_start + parts.frontmatter.len()
}

/// Parse frontmatter YAML into a node tree with spans into `content`. An
/// empty frontmatter is an empty mapping.
fn frontmatter_node(
    content: &str,
    parts: &crate::parsers::frontmatter::FrontmatterParts,
) -> Result<Option<JsonNode>, serde_yaml::Error> {
    let yaml: YamlValue = if parts.frontmatter.trim().is_empty() {
        YamlValue::Mapping(Default::default())
    } else {
        serde_yaml::from_str(&parts.frontmatter)?
    };
    if !yaml.is_mapping() {
        return Ok(None);
    }
    let range = frontmatter_range(parts);
    let anchor = (parts.frontmatter_start, parts.frontmatter_start);
    Ok(Some(yaml_node(content, &yaml, range, anchor)))
}

/// Span that deletes a frontmatter key with its value lines: from the key's
/// line through the lines indented deeper than it.
fn frontmatter_entry_span(content: &str, fm: &Range<usize>, key_start: usize) -> (usize, usize) {
    let line_start = content[..key_start].rfind('\n').map_or(0, |i| i + 1);
    let indent = key_start - line_start;
    let mut end = content[key_start..fm.end]
        .find('\n')
        .map_or(fm.end, |i| key_start + i + 1);
    while end < fm.end {
        let line_end = content[end..fm.end]
            .find('\n')
            .map_or(fm.end, |i| end + i + 1);
        let line = &content[end..line_end];
        let text = line.trim_end_matches(['\n', '\r']);
        let deeper = text.len() - text.trim_start_matches([' ', '\t']).len() > indent;
        if !(text.trim().is_empty() || deeper) {
            break;
        }
        end = line_end;
    }
    // The frontmatter text ends before the newline of the closing `---`.
    (
        line_start,
        (end + usize::from(end == fm.end)).min(content.len()),
    )
}

fn validate_amp_check(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let amp_001 = config.is_rule_enabled("AMP-001");
    let amp_002 = config.is_rule_enabled("AMP-002");
    if !amp_001 && !amp_002 {
        return Vec::new();
    }
    let mut sink = Sink::new(path, content, config);
    let file_start = (0, 0);

    let parts = split_frontmatter(content);
    if !parts.has_frontmatter || !parts.has_closing {
        sink.push(
            DiagnosticLevel::Error,
            "AMP-001",
            file_start,
            t!("rules.amp_001.missing_frontmatter").to_string(),
            t!("rules.amp_001.format_suggestion").to_string(),
        );
        return sink.diagnostics;
    }

    let fm = frontmatter_range(&parts);
    let node = match frontmatter_node(content, &parts) {
        Ok(Some(node)) => node,
        Ok(None) => {
            sink.push(
                DiagnosticLevel::Error,
                "AMP-001",
                (fm.start, fm.start),
                t!("rules.amp_001.not_mapping").to_string(),
                t!("rules.amp_001.format_suggestion").to_string(),
            );
            return sink.diagnostics;
        }
        Err(error) => {
            if amp_001 {
                // serde_yaml lines are relative to the frontmatter text.
                let (fm_line, _) = sink.line_col(fm.start);
                let line = error
                    .location()
                    .map_or(fm_line, |loc| fm_line + loc.line() - 1);
                let column = error.location().map_or(0, |loc| loc.column());
                sink.diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        column,
                        "AMP-001",
                        t!("rules.amp_001.invalid_yaml", error = error.to_string()),
                    )
                    .with_suggestion(t!("rules.amp_001.invalid_yaml_suggestion")),
                );
            }
            return sink.diagnostics;
        }
    };
    let members = node.value.as_object().unwrap_or_default();

    for member in members {
        check_check_key(&mut sink, content, &fm, member);
    }

    let name = members.iter().rev().find(|member| member.key == "name");
    if !name.is_some_and(|name| {
        name.value
            .value
            .as_str()
            .is_some_and(|text| !text.trim().is_empty())
    }) {
        let span = name.map_or((fm.start, fm.start), |name| (name.key_start, name.key_end));
        sink.push(
            DiagnosticLevel::Error,
            "AMP-001",
            span,
            t!("rules.amp_001.missing_name").to_string(),
            t!("rules.amp_001.format_suggestion").to_string(),
        );
    }

    if parts.body.trim().is_empty() {
        // At the closing `---` (body_start is just after it).
        let end = parts.body_start.min(content.len());
        sink.push(
            DiagnosticLevel::Error,
            "AMP-001",
            (end.saturating_sub(3), end),
            t!("rules.amp_001.empty_body").to_string(),
            t!("rules.amp_001.format_suggestion").to_string(),
        );
    }

    if let Some(severity) = members
        .iter()
        .rev()
        .find(|member| member.key == "severity-default")
    {
        check_severity_default(&mut sink, content, severity);
    }

    sink.diagnostics
}

fn check_check_key(sink: &mut Sink<'_, '_>, content: &str, fm: &Range<usize>, member: &JsonMember) {
    let key = member.key.as_str();
    let span = (member.key_start, member.key_end);
    if !VALID_CHECK_KEYS.contains(&key) {
        let (start, end) = frontmatter_entry_span(content, fm, member.key_start);
        // Unsafe: the plugin may read keys the page does not document. Only
        // offered when the key was located in the text (block style).
        let fix = (span.1 > span.0).then(|| {
            Fix::delete(
                start,
                end,
                t!("rules.amp_001.fix_remove", key = key).to_string(),
                false,
            )
        });
        sink.push_with_fix(
            DiagnosticLevel::Warning,
            "AMP-001",
            span,
            t!("rules.amp_001.unknown_key", key = key).to_string(),
            t!("rules.amp_001.unknown_key_suggestion").to_string(),
            fix,
        );
        return;
    }
    let value = &member.value.value;
    let mistyped = match key {
        "description" => value.as_str().is_none(),
        "tools" => {
            value.as_str().is_none()
                && !value
                    .as_array()
                    .is_some_and(|items| items.iter().all(|item| item.value.as_str().is_some()))
        }
        _ => false,
    };
    if mistyped {
        let message = if key == "tools" {
            t!("rules.amp_001.tools_type")
        } else {
            t!("rules.amp_001.description_type")
        };
        sink.push(
            DiagnosticLevel::Warning,
            "AMP-001",
            span,
            message.to_string(),
            t!("rules.amp_001.format_suggestion").to_string(),
        );
    }
}

/// AMP-002: "`severity-default` is optional. Its value must be `low`,
/// `medium`, `high`, or `critical`." A missing field is fine.
fn check_severity_default(sink: &mut Sink<'_, '_>, content: &str, member: &JsonMember) {
    let span = (member.key_start, member.key_end);
    let Some(severity) = member.value.value.as_str() else {
        sink.push(
            DiagnosticLevel::Warning,
            "AMP-002",
            span,
            t!("rules.amp_002.not_string").to_string(),
            t!("rules.amp_002.suggestion").to_string(),
        );
        return;
    };
    if VALID_SEVERITY_DEFAULT.contains(&severity) {
        return;
    }
    // Unsafe: the closest value is a guess at the intended severity.
    let fix = super::find_closest_value(severity, VALID_SEVERITY_DEFAULT).and_then(|suggested| {
        let (start, end) = (member.value.start, member.value.end);
        let slice = content.get(start..end)?;
        let replacement = match slice.chars().next() {
            Some(quote @ ('"' | '\'')) => format!("{quote}{suggested}{quote}"),
            _ => suggested.to_string(),
        };
        (end > start).then(|| {
            Fix::replace(
                start,
                end,
                replacement,
                t!("rules.amp_002.fix", value = suggested).to_string(),
                false,
            )
        })
    });
    sink.push_with_fix(
        DiagnosticLevel::Warning,
        "AMP-002",
        (member.value.start, member.value.end),
        t!("rules.amp_002.invalid", value = severity).to_string(),
        t!("rules.amp_002.suggestion").to_string(),
        fix,
    );
}

// ---------------------------------------------------------------------------
// AMP-003: `globs` in files that AGENTS.md @-mentions
// ---------------------------------------------------------------------------

/// Characters of an @-mention: path characters and the glob characters `*`,
/// `?`, `[` and `]`.
fn is_mention_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '~' | '*' | '?' | '[' | ']')
}

/// @-mentions outside code (`masked` has code blocks and spans blanked).
/// Like Amp's examples (`@doc/style.md`, `@doc/*.md`, `@.agent/**/*.md`), a
/// mention starts at `@` after a non-word character and runs over path
/// characters; trailing sentence punctuation is dropped.
fn scan_mentions(masked: &str) -> Vec<String> {
    let mut mentions = Vec::new();
    let mut previous: Option<char> = None;
    let mut chars = masked.char_indices().peekable();
    while let Some((index, c)) = chars.next() {
        let boundary = previous
            .is_none_or(|p| !(p.is_alphanumeric() || matches!(p, '_' | '-' | '.' | '@' | '/')));
        previous = Some(c);
        if c != '@' || !boundary {
            continue;
        }
        let start = index + 1;
        let mut end = start;
        while let Some(&(next_index, next)) = chars.peek() {
            if !is_mention_char(next) {
                break;
            }
            end = next_index + next.len_utf8();
            previous = Some(next);
            chars.next();
        }
        let mention = masked[start..end].trim_end_matches(['.', ',', ';', ':', '!', '?', ']']);
        if !mention.is_empty() {
            mentions.push(mention.to_string());
        }
    }
    mentions
}

fn has_glob_meta(text: &str) -> bool {
    text.contains(['*', '?', '['])
}

fn is_markdown_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".md") || lower.ends_with(".markdown")
}

/// Lexical normalization: drop `.`, resolve `..` against earlier parts.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Mentioned files to read, deduplicated, inside `root` when it is known.
struct Targets<'a> {
    fs: &'a dyn FileSystem,
    root: Option<PathBuf>,
    files: Vec<PathBuf>,
    seen: HashSet<PathBuf>,
    entries_left: usize,
}

impl Targets<'_> {
    fn inside_root(&self, path: &Path) -> bool {
        self.root.as_ref().is_none_or(|root| path.starts_with(root))
    }

    fn full(&self) -> bool {
        self.files.len() >= MAX_MENTIONED_FILES
    }

    fn add(&mut self, path: PathBuf) {
        if !self.full() && self.seen.insert(path.clone()) {
            self.files.push(path);
        }
    }

    fn add_literal(&mut self, base: &Path, mention: &str) {
        if !is_markdown_name(mention) {
            return;
        }
        let path = normalize(&base.join(mention));
        if self.inside_root(&path) && self.fs.is_file(&path) && !self.fs.is_symlink(&path) {
            self.add(path);
        }
    }

    /// Expand a glob mention relative to `base`: walk from the literal
    /// directory prefix and match the rest of the pattern.
    fn add_glob(&mut self, base: &Path, mention: &str) {
        let parts: Vec<&str> = mention.split('/').filter(|part| !part.is_empty()).collect();
        let Some(first_glob) = parts.iter().position(|part| has_glob_meta(part)) else {
            return;
        };
        let dir = normalize(&base.join(parts[..first_glob].join("/")));
        if !self.inside_root(&dir) {
            return;
        }
        let pattern_text = parts[first_glob..].join("/");
        let Ok(pattern) = glob::Pattern::new(&pattern_text) else {
            return;
        };
        let options = glob::MatchOptions {
            case_sensitive: true,
            require_literal_separator: true,
            require_literal_leading_dot: true,
        };
        let recursive = pattern_text.contains("**");
        let max_depth = parts.len() - first_glob;
        let mut stack = vec![(dir.clone(), 1usize)];
        while let Some((current, depth)) = stack.pop() {
            if self.full() {
                return;
            }
            let Ok(mut entries) = self.fs.read_dir(&current) else {
                continue;
            };
            entries.sort_by(|a, b| a.path.cmp(&b.path));
            for entry in entries {
                if self.entries_left == 0 {
                    return;
                }
                self.entries_left -= 1;
                if entry.metadata.is_symlink {
                    continue;
                }
                let name = entry
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if entry.metadata.is_dir {
                    if (recursive || depth < max_depth) && !SKIPPED_DIRS.contains(&name) {
                        stack.push((entry.path.clone(), depth + 1));
                    }
                    continue;
                }
                if !entry.metadata.is_file || !is_markdown_name(name) {
                    continue;
                }
                let Ok(relative) = entry.path.strip_prefix(&dir) else {
                    continue;
                };
                let relative = relative.to_string_lossy().replace('\\', "/");
                if pattern.matches_with(&relative, options) {
                    self.add(entry.path.clone());
                }
            }
        }
    }
}

/// AMP-003: Amp reads `globs` from the front matter of files that agent
/// files @-mention ("you can specify `globs` in YAML front matter of
/// mentioned files"), so the check runs on those files. `globs` on
/// AGENTS.md itself is not reported.
fn validate_amp_mentions(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    if !config.is_rule_enabled("AMP-003") {
        return Vec::new();
    }
    let Some(agents) = path.file_name().and_then(|name| name.to_str()) else {
        return Vec::new();
    };
    if !AGENTS_FILES.contains(&agents) {
        return Vec::new();
    }
    let Some(base) = path.parent() else {
        return Vec::new();
    };
    let base = normalize(base);
    let masked = crate::parsers::markdown::mask_code_spans(content);
    let fs = config.fs();
    let mut targets = Targets {
        fs: fs.as_ref(),
        root: config.root_dir().map(|root| normalize(root)),
        files: Vec::new(),
        seen: HashSet::new(),
        entries_left: MAX_GLOB_ENTRIES,
    };
    for mention in scan_mentions(&masked) {
        if targets.full() {
            break;
        }
        // `@~/...` is the home directory and absolute mentions point outside
        // the project; neither is followed.
        if mention.starts_with('~') || mention.starts_with('/') || mention.contains(':') {
            continue;
        }
        if has_glob_meta(&mention) {
            targets.add_glob(&base, &mention);
        } else {
            targets.add_literal(&base, &mention);
        }
    }

    let mut diagnostics = Vec::new();
    for target in &targets.files {
        let Ok(raw) = fs.read_to_string(target) else {
            continue;
        };
        let text = crate::parsers::frontmatter::normalize_line_endings(&raw);
        diagnostics.extend(check_mentioned_globs(target, &text, agents, config));
    }
    diagnostics
}

/// `globs` of one mentioned file: a string or a list of strings, each a
/// valid glob after Amp's implicit `**/` prefix ("Globs are implicitly
/// prefixed with `**/` unless they start with `../` or `./`").
fn check_mentioned_globs(
    path: &Path,
    content: &str,
    agents: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let parts = split_frontmatter(content);
    if !parts.has_frontmatter || !parts.has_closing {
        return Vec::new();
    }
    let Ok(Some(node)) = frontmatter_node(content, &parts) else {
        return Vec::new();
    };
    let Some(globs) = node.value.get("globs") else {
        return Vec::new();
    };
    let mut sink = Sink::new(path, content, config);
    let span = (globs.key_start, globs.key_end);
    let suggestion = t!("rules.amp_003.suggestion", agents = agents).to_string();
    let patterns: Vec<&str> = match &globs.value.value {
        crate::schemas::kiro_json::JsonValue::String(pattern) => vec![pattern.as_str()],
        crate::schemas::kiro_json::JsonValue::Array(items) => {
            if items.iter().any(|item| item.value.as_str().is_none()) {
                sink.push(
                    DiagnosticLevel::Warning,
                    "AMP-003",
                    span,
                    t!("rules.amp_003.non_string_item", agents = agents).to_string(),
                    suggestion,
                );
                return sink.diagnostics;
            }
            items
                .iter()
                .filter_map(|item| item.value.as_str())
                .collect()
        }
        _ => {
            sink.push(
                DiagnosticLevel::Warning,
                "AMP-003",
                span,
                t!("rules.amp_003.not_list", agents = agents).to_string(),
                suggestion,
            );
            return sink.diagnostics;
        }
    };
    for pattern in patterns {
        if let Err(error) = glob::Pattern::new(&normalize_amp_glob(pattern)) {
            sink.push(
                DiagnosticLevel::Warning,
                "AMP-003",
                span,
                t!(
                    "rules.amp_003.invalid_pattern",
                    pattern = pattern,
                    error = error.to_string(),
                    agents = agents
                )
                .to_string(),
                suggestion.clone(),
            );
        }
    }
    sink.diagnostics
}

fn normalize_amp_glob(pattern: &str) -> String {
    if pattern.starts_with("./") || pattern.starts_with("../") || pattern.starts_with("**/") {
        pattern.to_string()
    } else {
        format!("**/{pattern}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::MockFileSystem;
    use crate::{config::LintConfig, diagnostics::DiagnosticLevel};
    use std::sync::Arc;

    fn validate(path: &str, content: &str) -> Vec<Diagnostic> {
        let validator = AmpValidator;
        validator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn validate_with_config(path: &str, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = AmpValidator;
        validator.validate(Path::new(path), content, config)
    }

    fn rule<'a>(diagnostics: &'a [Diagnostic], id: &str) -> Vec<&'a Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == id).collect()
    }

    // ===== AMP-001 =====

    #[test]
    fn test_amp_001_missing_frontmatter() {
        let diagnostics = validate(".agents/checks/security.md", "# Security check");
        let amp_001 = rule(&diagnostics, "AMP-001");
        assert_eq!(amp_001.len(), 1);
        assert_eq!(amp_001[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_amp_001_invalid_yaml_frontmatter() {
        let content = "---\nname: [unclosed\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_001 = rule(&diagnostics, "AMP-001");
        assert_eq!(amp_001.len(), 1);
        assert!(amp_001[0].message.contains("Invalid YAML frontmatter"));
        assert_eq!(
            amp_001[0].line, 3,
            "serde_yaml reports loc.line()=2 for this error (libyaml places the unclosed-bracket error at EOF, serde_yaml adds its own +1); the frontmatter starts on line 2, giving line 3"
        );
    }

    #[test]
    fn test_amp_001_frontmatter_must_be_mapping() {
        let content = "---\n- security\n- check\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_001 = rule(&diagnostics, "AMP-001");
        assert_eq!(amp_001.len(), 1);
        assert!(amp_001[0].message.contains("must be a YAML mapping"));
    }

    #[test]
    fn test_amp_001_missing_name() {
        let content = "---\ndescription: Security checks\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_001 = rule(&diagnostics, "AMP-001");
        assert_eq!(amp_001.len(), 1);
        assert!(amp_001[0].message.contains("non-empty `name`"));
        assert_eq!(amp_001[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_amp_001_empty_name_points_at_key() {
        let content = "---\nname: \"  \"\n---\nReport emoji.";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_001 = rule(&diagnostics, "AMP-001");
        assert_eq!(amp_001.len(), 1);
        assert_eq!((amp_001[0].line, amp_001[0].column), (2, 1));
    }

    #[test]
    fn test_amp_001_empty_frontmatter_reports_missing_name() {
        let content = "---\n---\nReport emoji.";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_001 = rule(&diagnostics, "AMP-001");
        assert_eq!(amp_001.len(), 1);
        assert!(amp_001[0].message.contains("non-empty `name`"));
    }

    #[test]
    fn test_amp_001_empty_body_is_error() {
        // "Each check must have valid YAML frontmatter with a non-empty
        // `name`, followed by non-empty check instructions".
        for content in [
            "---\nname: no-emojis\n---\n",
            "---\nname: no-emojis\n---",
            "---\nname: no-emojis\n---\n\n   \n\t\n",
        ] {
            let diagnostics = validate(".agents/checks/no-emojis.md", content);
            let amp_001 = rule(&diagnostics, "AMP-001");
            assert_eq!(amp_001.len(), 1, "{content:?}: {diagnostics:?}");
            assert_eq!(amp_001[0].level, DiagnosticLevel::Error);
            assert!(amp_001[0].message.contains("no check instructions"));
            assert_eq!((amp_001[0].line, amp_001[0].column), (3, 1));
        }
    }

    #[test]
    fn test_amp_001_docs_example_is_valid() {
        let content = "---\nname: no-emojis\ndescription: Flags emoji characters in user-facing text\nseverity-default: medium\n---\n\nReport emoji characters in user-facing strings.\n";
        assert!(validate(".agents/checks/no-emojis.md", content).is_empty());
    }

    #[test]
    fn test_amp_001_unknown_frontmatter_key() {
        // Was an error; an unknown key is not a reason the plugin rejects
        // the check, so it is now a warning.
        let content = "---\nname: security\ndescription: Security checks\nfoo: bar\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_001 = rule(&diagnostics, "AMP-001");
        assert_eq!(amp_001.len(), 1);
        assert!(
            amp_001[0]
                .message
                .contains("Unknown Amp check frontmatter key")
        );
        assert_eq!(amp_001[0].level, DiagnosticLevel::Warning);
        assert_eq!((amp_001[0].line, amp_001[0].column), (4, 1));
    }

    #[test]
    fn test_amp_001_non_string_frontmatter_key_is_unknown() {
        // A non-string key such as `1` is never a documented key; it is now
        // reported as an unknown key (warning) instead of an error.
        let content = "---\n1: true\nname: security\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_001 = rule(&diagnostics, "AMP-001");
        assert_eq!(amp_001.len(), 1);
        assert!(amp_001[0].message.contains("'1'"));
        assert_eq!(amp_001[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_amp_001_description_must_be_string() {
        let content = "---\nname: security\ndescription: [not, string]\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_001 = rule(&diagnostics, "AMP-001");
        assert_eq!(amp_001.len(), 1);
        assert!(
            amp_001[0]
                .message
                .contains("`description` must be a string")
        );
        assert_eq!(amp_001[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_amp_001_tools_must_be_string_or_string_array() {
        let content = "---\nname: security\ntools:\n  - rg\n  - 7\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_001 = rule(&diagnostics, "AMP-001");
        assert_eq!(amp_001.len(), 1);
        assert!(amp_001[0].message.contains("`tools` must be a string"));
    }

    #[test]
    fn test_amp_check_valid_file_with_tools() {
        // `tools` is not on the current page; it stays accepted (unverified
        // whether the plugin still reads it).
        let content = "---\nname: security\ndescription: Security checks\nseverity-default: high\ntools:\n  - rg\n---\n# Body";
        assert!(validate(".agents/checks/security.md", content).is_empty());
    }

    #[test]
    fn test_amp_001_does_not_apply_to_agents_file_under_checks() {
        let diagnostics = validate(".agents/checks/AGENTS.md", "# Missing frontmatter");
        assert!(rule(&diagnostics, "AMP-001").is_empty());
    }

    #[test]
    fn test_amp_001_unknown_key_fix_removes_value_lines() {
        let content = "---\nname: security\nfoo:\n  - a\n  - b\ndescription: x\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_001 = rule(&diagnostics, "AMP-001");
        assert_eq!(amp_001.len(), 1);
        let fix = &amp_001[0].fixes[0];
        assert!(!fix.safe, "AMP-001 fix should be unsafe");
        assert!(fix.is_deletion());
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        assert_eq!(fixed, "---\nname: security\ndescription: x\n---\n# Body");
        assert!(validate(".agents/checks/security.md", &fixed).is_empty());
    }

    #[test]
    fn test_amp_001_unknown_last_key_fix_keeps_closing_delimiter() {
        let content = "---\nname: security\nfoo: bar\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let fix = &rule(&diagnostics, "AMP-001")[0].fixes[0];
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        assert_eq!(fixed, "---\nname: security\n---\n# Body");
        assert!(validate(".agents/checks/security.md", &fixed).is_empty());
    }

    // ===== AMP-002 =====

    #[test]
    fn test_amp_002_invalid_severity_default() {
        let content = "---\nname: security\nseverity-default: urgent\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_002 = rule(&diagnostics, "AMP-002");
        assert_eq!(amp_002.len(), 1);
        assert_eq!(amp_002[0].level, DiagnosticLevel::Warning);
        assert_eq!((amp_002[0].line, amp_002[0].column), (3, 19));
    }

    #[test]
    fn test_amp_002_severity_default_must_be_string() {
        let content = "---\nname: security\nseverity-default:\n  level: high\n---\n# Body";
        let diagnostics = validate(".agents/checks/security.md", content);
        let amp_002 = rule(&diagnostics, "AMP-002");
        assert_eq!(amp_002.len(), 1);
        assert!(amp_002[0].message.contains("must be a string"));
    }

    #[test]
    fn test_amp_002_missing_severity_default_is_fine() {
        // Was a warning ("missing required `severity-default`"). The review
        // page: "`severity-default` is optional."
        let content = "---\nname: security\n---\n# Body";
        assert!(validate(".agents/checks/security.md", content).is_empty());
    }

    #[test]
    fn test_amp_002_severity_default_has_fix() {
        // "High" -> closest match is "high" (case-insensitive)
        for (content, expected) in [
            (
                "---\nname: test\nseverity-default: High\n---\n# Body",
                "severity-default: high\n",
            ),
            (
                "---\nname: test\nseverity-default: \"Critical\" # top\n---\n# Body",
                "severity-default: \"critical\" # top\n",
            ),
        ] {
            let diagnostics = validate(".agents/checks/test.md", content);
            let amp_002 = rule(&diagnostics, "AMP-002");
            assert_eq!(amp_002.len(), 1);
            assert!(amp_002[0].has_fixes(), "AMP-002 should have a fix");
            assert!(!amp_002[0].fixes[0].safe, "AMP-002 fix should be unsafe");
            let fix = &amp_002[0].fixes[0];
            let mut fixed = content.to_string();
            fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
            assert!(fixed.contains(expected), "{fixed:?}");
            assert!(validate(".agents/checks/test.md", &fixed).is_empty());
        }
    }

    // ===== AMP-003 =====

    fn mentions_config(files: &[(&str, &str)]) -> LintConfig {
        let fs = Arc::new(MockFileSystem::new());
        for (path, content) in files {
            fs.add_file(*path, *content);
            for dir in Path::new(path).ancestors().skip(1) {
                fs.add_dir(dir);
            }
        }
        let mut config = LintConfig::default();
        config.set_fs(fs);
        config.set_root_dir(PathBuf::from("/p"));
        config
    }

    const BROKEN: &str = "---\nglobs:\n  - \"[broken\"\n---\n\nTS rules\n";

    #[test]
    fn test_amp_003_reports_on_mentioned_file() {
        let config = mentions_config(&[
            ("/p/doc/ts.md", BROKEN),
            (
                "/p/doc/ok.md",
                "---\nglobs: ['**/*.ts', 'src/**']\n---\nok\n",
            ),
        ]);
        let content = "# Project\n\nSee @doc/ts.md and @doc/ok.md.\n";
        let diagnostics = validate_with_config("/p/AGENTS.md", content, &config);
        let amp_003 = rule(&diagnostics, "AMP-003");
        assert_eq!(amp_003.len(), 1, "{diagnostics:?}");
        assert_eq!(amp_003[0].file, PathBuf::from("/p/doc/ts.md"));
        assert_eq!((amp_003[0].line, amp_003[0].column), (2, 1));
        assert_eq!(amp_003[0].level, DiagnosticLevel::Warning);
        assert!(amp_003[0].message.contains("AGENTS.md"));
    }

    #[test]
    fn test_amp_003_glob_mentions_and_dedup() {
        let config = mentions_config(&[
            ("/p/doc/ts.md", BROKEN),
            ("/p/specs/a/b.md", "---\nglobs: true\n---\nx\n"),
            ("/p/specs/a/c.txt", BROKEN),
            ("/p/specs/top.md", "---\nglobs:\n  - 42\n---\nx\n"),
        ]);
        let content = "See @doc/*.md, @doc/ts.md and @specs/**/*.md.\n";
        let diagnostics = validate_with_config("/p/AGENTS.md", content, &config);
        let amp_003 = rule(&diagnostics, "AMP-003");
        let files: Vec<_> = amp_003
            .iter()
            .map(|d| d.file.to_string_lossy().to_string())
            .collect();
        assert_eq!(
            files,
            ["/p/doc/ts.md", "/p/specs/top.md", "/p/specs/a/b.md"],
            "{diagnostics:?}"
        );
        assert!(amp_003[1].message.contains("only string patterns"));
        assert!(amp_003[2].message.contains("a string or an array"));
    }

    #[test]
    fn test_amp_003_relative_to_mentioning_file_and_root_bound() {
        let config = mentions_config(&[
            ("/p/pkg/docs/ts.md", BROKEN),
            ("/p/shared.md", BROKEN),
            ("/outside.md", BROKEN),
        ]);
        let content = "@docs/ts.md @../shared.md @../../outside.md\n";
        let diagnostics = validate_with_config("/p/pkg/AGENTS.md", content, &config);
        let files: Vec<_> = rule(&diagnostics, "AMP-003")
            .iter()
            .map(|d| d.file.as_path())
            .collect();
        assert_eq!(
            files,
            [Path::new("/p/pkg/docs/ts.md"), Path::new("/p/shared.md")]
        );
    }

    #[test]
    fn test_amp_003_skips_code_home_and_emails() {
        let config = mentions_config(&[("/p/doc/ts.md", BROKEN)]);
        let content =
            "```\n@doc/ts.md\n```\n\nInline `@doc/ts.md`, mail me@doc/ts.md, home @~/doc/ts.md.\n";
        let diagnostics = validate_with_config("/p/AGENTS.md", content, &config);
        assert!(rule(&diagnostics, "AMP-003").is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn test_amp_003_globs_on_agents_md_itself_not_reported() {
        // AGENTS.md's own `globs` was reported; Amp documents `globs` for
        // mentioned files only, so it is no longer checked here.
        let content = "---\nglobs: \"[unclosed\"\n---\n# Instructions";
        for name in ["AGENTS.md", "AGENTS.local.md", "AGENTS.override.md"] {
            assert!(rule(&validate(name, content), "AMP-003").is_empty());
        }
    }

    #[test]
    fn test_amp_003_only_agent_files_follow_mentions() {
        let config = mentions_config(&[("/p/doc/ts.md", BROKEN)]);
        let diagnostics = validate_with_config("/p/CLAUDE.md", "@doc/ts.md\n", &config);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_amp_003_implicit_prefix() {
        assert_eq!(normalize_amp_glob("*.ts"), "**/*.ts");
        assert_eq!(normalize_amp_glob("./src/*.ts"), "./src/*.ts");
        assert_eq!(normalize_amp_glob("../x/*.ts"), "../x/*.ts");
        assert_eq!(normalize_amp_glob("**/*.ts"), "**/*.ts");
    }

    #[test]
    fn test_scan_mentions() {
        let found = scan_mentions(
            "See @doc/style.md and @specs/**/*.md. (@a.md) x@y.md @@z.md\n@.agent/**/*.md!",
        );
        assert_eq!(
            found,
            ["doc/style.md", "specs/**/*.md", "a.md", ".agent/**/*.md"]
        );
    }

    // ===== Config =====

    #[test]
    fn test_amp_rules_respect_disabled_config() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["AMP-004".to_string()];
        let diagnostics = validate_with_config(".amp/settings.json", "{ invalid json", &config);
        assert!(diagnostics.is_empty());
    }
}
