//! Windsurf (Devin Desktop) validation: workspace rules, workflows, the
//! legacy `.windsurfrules` file, global rules, AGENTS.md (WS-001 to WS-011)
//! and Cascade hooks (WS-HK-001 to WS-HK-005, in [`super::windsurf_hooks`]).
//!
//! - WS-001: Empty rule file (MEDIUM/WARNING)
//! - WS-002: Rule file over 12,000 characters (HIGH/ERROR)
//! - WS-003: Empty (WARNING) or oversized (ERROR) workflow file
//! - WS-004: Legacy `.windsurfrules` (LOW/INFO)
//! - WS-005: `trigger: model_decision` without a description (MEDIUM/WARNING)
//! - WS-006: Unknown or missing `trigger`, `glob` without `globs` (WARNING/INFO)
//! - WS-007: Legacy `.windsurf/` location (LOW/INFO, once per directory)
//! - WS-008: Generic instruction in a rule file (LOW/INFO)
//! - WS-009: Workflow the Devin Local agent does not run (LOW/INFO)
//! - WS-010: Global rules over 6,000 characters (HIGH/ERROR)
//! - WS-011: AGENTS.md over 12,000 characters in a Windsurf project
//!   (MEDIUM/WARNING, [`WindsurfAgentsMdValidator`])
//!
//! Sources (docs.devin.ai): desktop/cascade/memories ("Rules", "Rules
//! Discovery", "Activation Modes", "Best Practices"), desktop/cascade/workflows,
//! desktop/cascade/agents-md, desktop/cascade/hooks and cli/extensibility/rules
//! (Devin CLI, which also reads `.devin/rules` and accepts `trigger: agent`).
//! The docs state the limits in characters ("Limited to 12,000 characters per
//! file."), so agnix counts characters (Unicode scalar values), and counts the
//! whole file because the docs do not say whether frontmatter counts.

use crate::{
    FileType,
    config::{LintConfig, PerFileLintConfig, TargetTool},
    diagnostics::Diagnostic,
    fs::FileSystem,
    rules::{Validator, ValidatorMetadata, line_col_at_offset},
    schemas::{kiro_json::closest_name, prompt::find_redundant_instructions},
};
use rust_i18n::t;
use std::path::{Path, PathBuf};

const RULE_IDS: &[&str] = &[
    "WS-001",
    "WS-002",
    "WS-003",
    "WS-004",
    "WS-005",
    "WS-006",
    "WS-007",
    "WS-008",
    "WS-009",
    "WS-010",
    "WS-HK-001",
    "WS-HK-002",
    "WS-HK-003",
    "WS-HK-004",
    "WS-HK-005",
];

const AGENTS_MD_RULE_IDS: &[&str] = &["WS-011"];

/// "Workspace rule files are limited to 12,000 characters each."
pub(crate) const WORKSPACE_RULE_CHAR_LIMIT: usize = 12_000;
/// "Workflow files are limited to 12000 characters each."
const WORKFLOW_CHAR_LIMIT: usize = 12_000;
/// "The global rules file is limited to 6,000 characters."
const GLOBAL_RULES_CHAR_LIMIT: usize = 6_000;

/// Rule `trigger` values: the four Devin Desktop activation modes and
/// Devin CLI's `agent` ("the `trigger` values `always_on`, `manual`,
/// `model_decision`, `agent`, and `glob` all apply").
const TRIGGERS: &[&str] = &["always_on", "model_decision", "glob", "manual", "agent"];

/// Bounds for the WS-007 directory listing.
const LEGACY_LISTING_MAX_DEPTH: usize = 8;
const LEGACY_LISTING_MAX_ENTRIES: usize = 2_000;

pub struct WindsurfValidator;

impl Validator for WindsurfValidator {
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
            FileType::WindsurfRule => {
                // WS-001 and WS-002 are mutually exclusive: an empty file
                // cannot also exceed the character limit.
                if content.trim().is_empty() {
                    if config.is_rule_enabled("WS-001") {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "WS-001",
                                t!("rules.ws_001.message"),
                            )
                            .with_suggestion(t!("rules.ws_001.suggestion")),
                        );
                    }
                } else {
                    if config.is_rule_enabled("WS-002")
                        && let Some(over) = over_limit(content, WORKSPACE_RULE_CHAR_LIMIT)
                    {
                        diagnostics.push(over.diagnostic(
                            path,
                            content,
                            Level::Error,
                            "WS-002",
                            t!(
                                "rules.ws_002.message",
                                len = over.chars,
                                limit = WORKSPACE_RULE_CHAR_LIMIT
                            ),
                            t!("rules.ws_002.suggestion"),
                        ));
                    }
                    check_trigger(path, content, config, &mut diagnostics);
                    check_generic_instructions(path, content, config, &mut diagnostics);
                }
                check_legacy_directory(path, file_type, "rules", config, &mut diagnostics);
            }
            FileType::WindsurfWorkflow => {
                if config.is_rule_enabled("WS-003") {
                    if content.trim().is_empty() {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "WS-003",
                                t!("rules.ws_003_empty.message"),
                            )
                            .with_suggestion(t!("rules.ws_003_empty.suggestion")),
                        );
                    } else if let Some(over) = over_limit(content, WORKFLOW_CHAR_LIMIT) {
                        // The workflows page states this limit exactly as the
                        // memories page states WS-002's, so it is an error too.
                        diagnostics.push(over.diagnostic(
                            path,
                            content,
                            Level::Error,
                            "WS-003",
                            t!(
                                "rules.ws_003_too_long.message",
                                len = over.chars,
                                limit = WORKFLOW_CHAR_LIMIT
                            ),
                            t!("rules.ws_003_too_long.suggestion"),
                        ));
                    }
                }
                if config.is_rule_enabled("WS-009") {
                    diagnostics.push(
                        Diagnostic::info(
                            path.to_path_buf(),
                            1,
                            1,
                            "WS-009",
                            t!("rules.ws_009.message"),
                        )
                        .with_suggestion(t!("rules.ws_009.suggestion")),
                    );
                }
                check_legacy_directory(path, file_type, "workflows", config, &mut diagnostics);
            }
            FileType::WindsurfRulesLegacy => {
                if config.is_rule_enabled("WS-004") {
                    let suggestion = if cline_may_read(config) {
                        t!("rules.ws_004.suggestion_cline")
                    } else {
                        t!("rules.ws_004.suggestion")
                    };
                    diagnostics.push(
                        Diagnostic::info(
                            path.to_path_buf(),
                            1,
                            0,
                            "WS-004",
                            t!("rules.ws_004.message"),
                        )
                        .with_suggestion(suggestion),
                    );
                }
                check_generic_instructions(path, content, config, &mut diagnostics);
            }
            FileType::WindsurfGlobalRules => {
                if config.is_rule_enabled("WS-010")
                    && let Some(over) = over_limit(content, GLOBAL_RULES_CHAR_LIMIT)
                {
                    diagnostics.push(over.diagnostic(
                        path,
                        content,
                        Level::Error,
                        "WS-010",
                        t!(
                            "rules.ws_010.message",
                            len = over.chars,
                            limit = GLOBAL_RULES_CHAR_LIMIT
                        ),
                        t!("rules.ws_010.suggestion"),
                    ));
                }
                check_generic_instructions(path, content, config, &mut diagnostics);
            }
            FileType::WindsurfHooks => {
                diagnostics.extend(super::windsurf_hooks::validate(path, content, config));
            }
            _ => {}
        }

        diagnostics
    }
}

// ---------------------------------------------------------------------------
// Character limits (WS-002, WS-003, WS-010, WS-011)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
enum Level {
    Error,
    Warning,
    Info,
}

/// A file longer than a character limit: its length in characters and the
/// byte offset of the first character past the limit.
struct OverLimit {
    chars: usize,
    offset: usize,
}

fn over_limit(content: &str, limit: usize) -> Option<OverLimit> {
    let (offset, _) = content.char_indices().nth(limit)?;
    Some(OverLimit {
        chars: content.chars().count(),
        offset,
    })
}

impl OverLimit {
    /// The diagnostic points at the first character past the limit.
    fn diagnostic(
        &self,
        path: &Path,
        content: &str,
        level: Level,
        rule: &str,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Diagnostic {
        diagnostic_at(path, content, self.offset, level, rule, message).with_suggestion(suggestion)
    }
}

// ---------------------------------------------------------------------------
// Rule frontmatter (WS-005, WS-006)
// ---------------------------------------------------------------------------

/// One top-level `key: value` line of a rule's frontmatter.
#[derive(Debug)]
struct Field<'a> {
    key: &'a str,
    /// Inline value, trimmed, without surrounding quotes or a trailing comment.
    value: &'a str,
    key_offset: usize,
    value_offset: usize,
    /// Indented or list lines follow (a block list or block scalar).
    continued: bool,
}

impl Field<'_> {
    /// No usable value: empty, `null`, `~`, `[]`, or a block indicator with
    /// nothing below it.
    fn is_blank(&self) -> bool {
        let value = self.value.trim();
        if value.is_empty() || value.starts_with('|') || value.starts_with('>') {
            return !self.continued;
        }
        matches!(value, "null" | "~") || value.replace(' ', "") == "[]"
    }
}

/// The frontmatter fields of a rule file, or `None` when the file does not
/// start with a closed `---` block.
///
/// Read line by line instead of with a YAML parser: Devin Desktop's own
/// example, `globs: **/*.test.ts`, is not valid YAML (`*` starts an alias),
/// so a strict parser would reject documented rules.
fn rule_frontmatter(content: &str) -> Option<Vec<Field<'_>>> {
    let start = if content.starts_with('\u{FEFF}') {
        '\u{FEFF}'.len_utf8()
    } else {
        0
    };
    let mut offset = start;
    let mut lines = content[start..].split_inclusive('\n').map(|raw| {
        let line_offset = offset;
        offset += raw.len();
        (line_offset, raw.trim_end_matches(['\n', '\r']))
    });
    let (_, first) = lines.next()?;
    if first.trim_end() != "---" {
        return None;
    }
    let mut fields: Vec<Field<'_>> = Vec::new();
    for (line_offset, line) in lines {
        if line.trim_end() == "---" {
            return Some(fields);
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if line.starts_with([' ', '\t', '-']) {
            if let Some(last) = fields.last_mut() {
                last.continued = true;
            }
            continue;
        }
        let Some(colon) = line.find(':') else {
            continue;
        };
        let key = line[..colon].trim();
        if key.is_empty() {
            continue;
        }
        let after = &line[colon + 1..];
        let lead = after.len() - after.trim_start().len();
        let mut value = after.trim();
        let mut value_offset = line_offset + colon + 1 + lead;
        let quoted = value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')));
        if quoted {
            value = &value[1..value.len() - 1];
            value_offset += 1;
        } else if let Some(hash) = value.find(" #") {
            value = value[..hash].trim_end();
        }
        fields.push(Field {
            key,
            value,
            key_offset: line_offset + line.find(key).unwrap_or(0),
            value_offset,
            continued: false,
        });
    }
    None
}

fn field<'f, 'a>(fields: &'f [Field<'a>], key: &str) -> Option<&'f Field<'a>> {
    fields.iter().rev().find(|f| f.key == key)
}

/// Builds a diagnostic at a byte offset of the file.
fn diagnostic_at(
    path: &Path,
    content: &str,
    offset: usize,
    level: Level,
    rule: &str,
    message: impl Into<String>,
) -> Diagnostic {
    let (line, column) = line_col_at_offset(content, offset);
    let file = path.to_path_buf();
    match level {
        Level::Error => Diagnostic::error(file, line, column, rule, message),
        Level::Warning => Diagnostic::warning(file, line, column, rule, message),
        Level::Info => Diagnostic::info(file, line, column, rule, message),
    }
}

/// WS-005 and WS-006: the activation mode of a workspace rule ("Each
/// workspace rule declares an activation mode in its frontmatter via the
/// `trigger` field").
fn check_trigger(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let fields = rule_frontmatter(content);
    let trigger = fields
        .as_deref()
        .and_then(|fields| field(fields, "trigger"));
    let Some(trigger) = trigger else {
        // No frontmatter or no `trigger`: the docs do not say which mode
        // such a rule gets, so this is informational.
        if config.is_rule_enabled("WS-006") {
            let always_apply = fields
                .as_deref()
                .and_then(|fields| field(fields, "alwaysApply"));
            match always_apply {
                Some(cursor) => diagnostics.push(
                    diagnostic_at(
                        path,
                        content,
                        cursor.key_offset,
                        Level::Info,
                        "WS-006",
                        t!("rules.ws_006.cursor_always_apply"),
                    )
                    .with_suggestion(t!("rules.ws_006.cursor_always_apply_suggestion")),
                ),
                None => {
                    let offset = fields
                        .as_deref()
                        .and_then(<[Field<'_>]>::first)
                        .map_or(0, |f| f.key_offset);
                    diagnostics.push(
                        diagnostic_at(
                            path,
                            content,
                            offset,
                            Level::Info,
                            "WS-006",
                            t!("rules.ws_006.missing"),
                        )
                        .with_suggestion(t!("rules.ws_006.missing_suggestion")),
                    );
                }
            }
        }
        return;
    };
    let fields = fields.as_deref().unwrap_or_default();

    match trigger.value {
        "model_decision" => {
            let description = field(fields, "description");
            if config.is_rule_enabled("WS-005") && description.is_none_or(Field::is_blank) {
                let offset = description.map_or(trigger.value_offset, |d| d.key_offset);
                diagnostics.push(
                    diagnostic_at(
                        path,
                        content,
                        offset,
                        Level::Warning,
                        "WS-005",
                        t!("rules.ws_005.message"),
                    )
                    .with_suggestion(t!("rules.ws_005.suggestion")),
                );
            }
        }
        "glob" => {
            if config.is_rule_enabled("WS-006")
                && field(fields, "globs").is_none_or(Field::is_blank)
            {
                diagnostics.push(
                    diagnostic_at(
                        path,
                        content,
                        trigger.value_offset,
                        Level::Warning,
                        "WS-006",
                        t!("rules.ws_006.glob_without_globs"),
                    )
                    .with_suggestion(t!("rules.ws_006.glob_without_globs_suggestion")),
                );
            }
        }
        value if TRIGGERS.contains(&value) => {}
        value => {
            if config.is_rule_enabled("WS-006") {
                let suggestion = match closest_name(value, TRIGGERS) {
                    Some(closest) => {
                        t!("rules.ws_006.unknown_suggestion_closest", closest = closest)
                    }
                    None => t!("rules.ws_006.unknown_suggestion"),
                };
                diagnostics.push(
                    diagnostic_at(
                        path,
                        content,
                        trigger.value_offset,
                        Level::Warning,
                        "WS-006",
                        t!("rules.ws_006.unknown", value = value),
                    )
                    .with_suggestion(suggestion),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// WS-007: legacy `.windsurf/` directories
// ---------------------------------------------------------------------------

/// The innermost `.windsurf/<kind>` directory holding `path`, or `None` when
/// the innermost `<kind>` directory is under `.devin/` (or neither).
fn legacy_directory<'p>(path: &'p Path, kind: &str) -> Option<&'p Path> {
    for dir in path.ancestors().skip(1) {
        let name = dir.file_name().and_then(|name| name.to_str())?;
        if !name.eq_ignore_ascii_case(kind) {
            continue;
        }
        let parent = dir
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str());
        match parent {
            Some(parent) if parent.eq_ignore_ascii_case(".windsurf") => return Some(dir),
            Some(parent) if parent.eq_ignore_ascii_case(".devin") => return None,
            _ => {}
        }
    }
    None
}

/// Whether `path` is the first file of `file_type` below `dir` in path order,
/// so that WS-007 is reported once per directory. When the directory cannot
/// be listed, the file is the only one known and counts as first.
fn is_first_file_in(path: &Path, dir: &Path, file_type: FileType, fs: &dyn FileSystem) -> bool {
    let Ok(relative) = path.strip_prefix(dir) else {
        return true;
    };
    let mut first: Option<PathBuf> = None;
    let mut pending = vec![(dir.to_path_buf(), 0usize)];
    let mut seen = 0usize;
    while let Some((current, depth)) = pending.pop() {
        let Ok(entries) = fs.read_dir(&current) else {
            continue;
        };
        for entry in entries {
            seen += 1;
            if seen > LEGACY_LISTING_MAX_ENTRIES {
                break;
            }
            if entry.metadata.is_symlink {
                continue;
            }
            if entry.metadata.is_dir {
                if depth < LEGACY_LISTING_MAX_DEPTH {
                    pending.push((entry.path, depth + 1));
                }
            } else if entry.metadata.is_file
                && crate::detect_file_type(&entry.path) == file_type
                && let Ok(candidate) = entry.path.strip_prefix(dir)
                && first.as_deref().is_none_or(|best| candidate < best)
            {
                first = Some(candidate.to_path_buf());
            }
        }
    }
    first.is_none_or(|first| first == relative)
}

fn check_legacy_directory(
    path: &Path,
    file_type: FileType,
    kind: &str,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !config.is_rule_enabled("WS-007") {
        return;
    }
    let Some(dir) = legacy_directory(path, kind) else {
        return;
    };
    if !is_first_file_in(path, dir, file_type, config.fs().as_ref()) {
        return;
    }
    let legacy = format!(".windsurf/{kind}/");
    let preferred = format!(".devin/{kind}/");
    diagnostics.push(
        Diagnostic::info(
            path.to_path_buf(),
            1,
            1,
            "WS-007",
            t!(
                "rules.ws_007.message",
                legacy = legacy.as_str(),
                preferred = preferred.as_str()
            ),
        )
        .with_suggestion(t!(
            "rules.ws_007.suggestion",
            preferred = preferred.as_str()
        )),
    );
}

// ---------------------------------------------------------------------------
// WS-008: generic instructions
// ---------------------------------------------------------------------------

/// Devin Desktop's own examples of generic rules: "write good code"
/// (memories, "Best Practices") and the "Less Effective Example" list of the
/// AGENTS.md page ("Write good code", "Be careful with errors", "Use best
/// practices").
const VENDOR_GENERIC_PHRASES: &[&str] = &[
    "write good code",
    "write clean code",
    "be careful with errors",
    "use best practices",
    "follow best practices",
];

/// Words that do not make an instruction specific ("Always be concise").
const FILLER_WORDS: &[&str] = &[
    "always", "please", "and", "or", "also", "very", "really", "try", "to", "you", "should",
    "must", "be", "the", "a", "an", "your", "all", "times", "code",
];

/// Byte ranges of the vendor phrases in `line` (ASCII case-insensitive, at
/// word boundaries).
fn vendor_phrase_matches(line: &str) -> Vec<(usize, usize)> {
    let lower = line.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let boundary = |at: usize| at == 0 || at >= bytes.len() || !bytes[at].is_ascii_alphanumeric();
    let mut out = Vec::new();
    for phrase in VENDOR_GENERIC_PHRASES {
        for (at, _) in lower.match_indices(phrase) {
            let end = at + phrase.len();
            if (at == 0 || boundary(at - 1)) && boundary(end) {
                out.push((at, end));
            }
        }
    }
    out
}

/// The instruction is little more than the generic phrases: at most one
/// word remains once list markers, the phrases and filler words are removed.
/// "Write good code." and "Always write good code" qualify; "Write good code
/// in the parser module" does not.
fn is_generic_instruction(line: &str, phrases: &[(usize, usize)]) -> bool {
    let mut rest = line.to_ascii_lowercase().into_bytes();
    for (start, end) in phrases {
        for byte in &mut rest[*start..*end] {
            *byte = b' ';
        }
    }
    let rest = String::from_utf8_lossy(&rest);
    rest.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .filter(|word| !word.chars().all(|c| c.is_ascii_digit()))
        .filter(|word| !FILLER_WORDS.contains(word))
        .count()
        <= 1
}

fn check_generic_instructions(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !config.is_rule_enabled("WS-008") {
        return;
    }
    let lines: Vec<&str> = content.lines().collect();
    // Frontmatter lines (a description that says "be concise" is not a rule).
    let body_line = rule_frontmatter(content).map_or(0, |_| {
        lines
            .iter()
            .skip(1)
            .position(|line| line.trim_end() == "---")
            .map_or(0, |closing| closing + 2)
    });

    // PE-005's phrase list, plus the vendor's own examples.
    let mut per_line: std::collections::BTreeMap<usize, Vec<(usize, usize)>> =
        std::collections::BTreeMap::new();
    for found in find_redundant_instructions(content) {
        let start = found.column.saturating_sub(1);
        per_line
            .entry(found.line)
            .or_default()
            .push((start, start + found.phrase.len()));
    }
    let mut in_code = false;
    for (index, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if !in_code {
            let found = vendor_phrase_matches(line);
            if !found.is_empty() {
                per_line.entry(index + 1).or_default().extend(found);
            }
        }
    }

    for (line_number, mut phrases) in per_line {
        if line_number <= body_line {
            continue;
        }
        // Headings name a section; the instructions are below them.
        let Some(line) = lines
            .get(line_number - 1)
            .filter(|line| !line.trim_start().starts_with('#'))
        else {
            continue;
        };
        phrases.sort_unstable();
        if !is_generic_instruction(line, &phrases) {
            continue;
        }
        let (start, end) = phrases[0];
        let column = line
            .get(..start)
            .map_or(start, |prefix| prefix.chars().count())
            + 1;
        diagnostics.push(
            Diagnostic::info(
                path.to_path_buf(),
                line_number,
                column,
                "WS-008",
                t!(
                    "rules.ws_008.message",
                    phrase = line.get(start..end).unwrap_or_default()
                ),
            )
            .with_suggestion(t!("rules.ws_008.suggestion")),
        );
    }
}

// ---------------------------------------------------------------------------
// Scope helpers (WS-004, WS-011)
// ---------------------------------------------------------------------------

/// Cline still reads `.windsurfrules` and does not read `.windsurf/rules/` or
/// `.devin/rules/` (docs.cline.bot/customization/cline-rules). Without a
/// `tools` list agnix cannot tell, so the caveat is shown.
fn cline_may_read(config: &LintConfig) -> bool {
    let tools = config.tools();
    tools.is_empty() || tools.iter().any(|tool| tool.eq_ignore_ascii_case("cline"))
}

/// Whether Devin Desktop reads this AGENTS.md: windsurf is a selected tool, or
/// nothing is selected and the project shows Windsurf use (`.devin/rules`,
/// `.devin/workflows`, `.devin/hooks.json`, a `.windsurf/` directory or
/// `.windsurfrules`) next to the file or above it, up to the project root.
/// `.devin/` alone is not enough: it is also the Devin CLI config directory.
fn project_uses_windsurf(path: &Path, config: &LintConfig) -> bool {
    let tools = config.tools();
    if !tools.is_empty() {
        return tools
            .iter()
            .any(|tool| tool.eq_ignore_ascii_case("windsurf"));
    }
    if config.target() != TargetTool::Generic {
        return false;
    }
    let fs = config.fs();
    let root = config.root_dir();
    for dir in path.ancestors().skip(1) {
        let devin = dir.join(".devin");
        if fs.is_dir(&devin.join("rules"))
            || fs.is_dir(&devin.join("workflows"))
            || fs.is_file(&devin.join("hooks.json"))
            || fs.is_dir(&dir.join(".windsurf"))
            || fs.is_file(&dir.join(".windsurfrules"))
        {
            return true;
        }
        if root.is_some_and(|root| dir == root.as_path()) {
            break;
        }
    }
    false
}

/// WS-011: AGENTS.md files in projects that use Windsurf. Every other
/// ClaudeMd file (CLAUDE.md, AGENTS.local.md, AGENTS.override.md) is skipped.
pub struct WindsurfAgentsMdValidator;

impl Validator for WindsurfAgentsMdValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: AGENTS_MD_RULE_IDS,
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        if path.file_name().and_then(|name| name.to_str()) != Some("AGENTS.md")
            || !config.is_rule_enabled("WS-011")
        {
            return Vec::new();
        }
        let Some(over) = over_limit(content, WORKSPACE_RULE_CHAR_LIMIT) else {
            return Vec::new();
        };
        if !project_uses_windsurf(path, config) {
            return Vec::new();
        }
        vec![over.diagnostic(
            path,
            content,
            Level::Warning,
            "WS-011",
            t!(
                "rules.ws_011.message",
                len = over.chars,
                limit = WORKSPACE_RULE_CHAR_LIMIT
            ),
            t!("rules.ws_011.suggestion"),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;
    use crate::fs::MockFileSystem;
    use std::sync::Arc;

    fn validate_at(path: &str, content: &str) -> Vec<Diagnostic> {
        WindsurfValidator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn validate_rule(content: &str) -> Vec<Diagnostic> {
        validate_at(".devin/rules/test.md", content)
    }

    fn validate_workflow(content: &str) -> Vec<Diagnostic> {
        validate_at(".devin/workflows/test.md", content)
    }

    fn validate_legacy(content: &str) -> Vec<Diagnostic> {
        validate_at(".windsurfrules", content)
    }

    fn of<'d>(diagnostics: &'d [Diagnostic], rule: &str) -> Vec<&'d Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == rule).collect()
    }

    const ALWAYS_ON: &str = "---\ntrigger: always_on\n---\n";

    // ===== WS-001: Empty rule file =====

    #[test]
    fn test_ws_001_empty_file() {
        for path in [".devin/rules/test.md", ".windsurf/rules/test.md"] {
            let diagnostics = validate_at(path, "");
            let ws_001 = of(&diagnostics, "WS-001");
            assert_eq!(ws_001.len(), 1, "{path}");
            assert_eq!(ws_001[0].level, DiagnosticLevel::Warning);
            assert!(ws_001[0].message.contains("empty"));
        }
    }

    #[test]
    fn test_ws_001_whitespace_only() {
        let diagnostics = validate_rule("   \n\n  ");
        assert_eq!(of(&diagnostics, "WS-001").len(), 1);
        // An empty file gets no trigger note on top of WS-001.
        assert!(of(&diagnostics, "WS-006").is_empty());
    }

    #[test]
    fn test_ws_001_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["WS-001".to_string()];
        let diagnostics =
            WindsurfValidator.validate(Path::new(".devin/rules/test.md"), "", &config);
        assert!(of(&diagnostics, "WS-001").is_empty());
    }

    // ===== WS-002: Rule file exceeds 12,000 characters =====

    #[test]
    fn test_ws_002_exceeds_limit() {
        let content = format!("{ALWAYS_ON}{}", "x".repeat(WORKSPACE_RULE_CHAR_LIMIT));
        let diagnostics = validate_rule(&content);
        let ws_002 = of(&diagnostics, "WS-002");
        assert_eq!(ws_002.len(), 1);
        assert_eq!(ws_002[0].level, DiagnosticLevel::Error);
        let expected = format!("{} characters", content.chars().count());
        assert!(
            ws_002[0].message.contains(&expected),
            "{}",
            ws_002[0].message
        );
        // Points at the first character past the limit (line 4).
        assert_eq!(ws_002[0].line, 4);
    }

    #[test]
    fn test_ws_002_at_limit() {
        let body = "x".repeat(WORKSPACE_RULE_CHAR_LIMIT - ALWAYS_ON.len());
        let diagnostics = validate_rule(&format!("{ALWAYS_ON}{body}"));
        assert!(
            of(&diagnostics, "WS-002").is_empty(),
            "Exactly at limit should not trigger"
        );
    }

    #[test]
    fn test_ws_002_counts_characters_not_bytes() {
        // 7,029 CJK characters are 21,087 bytes: under the limit.
        let content = format!("{ALWAYS_ON}{}", "规".repeat(7_029));
        assert!(content.len() > WORKSPACE_RULE_CHAR_LIMIT);
        assert!(of(&validate_rule(&content), "WS-002").is_empty());
        let content = format!("{ALWAYS_ON}{}", "规".repeat(12_000));
        let diagnostics = validate_rule(&content);
        let ws_002 = of(&diagnostics, "WS-002");
        let expected = format!("{} characters", content.chars().count());
        assert!(
            ws_002[0].message.contains(&expected),
            "{}",
            ws_002[0].message
        );
    }

    #[test]
    fn test_ws_002_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["WS-002".to_string()];
        let content = "x".repeat(WORKSPACE_RULE_CHAR_LIMIT + 1);
        let diagnostics =
            WindsurfValidator.validate(Path::new(".devin/rules/test.md"), &content, &config);
        assert!(of(&diagnostics, "WS-002").is_empty());
    }

    // ===== WS-003: Workflow file =====

    #[test]
    fn test_ws_003_empty_workflow() {
        let diagnostics = validate_workflow("");
        let ws_003 = of(&diagnostics, "WS-003");
        assert_eq!(ws_003.len(), 1);
        assert_eq!(ws_003[0].level, DiagnosticLevel::Warning);
        assert!(ws_003[0].message.contains("empty"));
    }

    #[test]
    fn test_ws_003_workflow_too_long() {
        let content = "y".repeat(WORKFLOW_CHAR_LIMIT + 1);
        let diagnostics = validate_workflow(&content);
        let ws_003 = of(&diagnostics, "WS-003");
        assert_eq!(ws_003.len(), 1);
        assert!(ws_003[0].message.contains("12001 characters"));
        // Was a warning; the workflows page states this limit the same way the
        // memories page states WS-002's, so both are errors now.
        assert_eq!(ws_003[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_ws_003_whitespace_only() {
        let diagnostics = validate_workflow("   \n\n  ");
        assert_eq!(of(&diagnostics, "WS-003").len(), 1);
    }

    #[test]
    fn test_ws_003_workflow_at_limit_and_multibyte() {
        assert!(
            of(
                &validate_workflow(&"y".repeat(WORKFLOW_CHAR_LIMIT)),
                "WS-003"
            )
            .is_empty()
        );
        assert!(
            of(
                &validate_workflow(&"é".repeat(WORKFLOW_CHAR_LIMIT)),
                "WS-003"
            )
            .is_empty()
        );
    }

    #[test]
    fn test_ws_003_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["WS-003".to_string()];
        let diagnostics =
            WindsurfValidator.validate(Path::new(".devin/workflows/test.md"), "", &config);
        assert!(of(&diagnostics, "WS-003").is_empty());
    }

    // ===== WS-004: Legacy .windsurfrules =====

    #[test]
    fn test_ws_004_legacy_detected() {
        let diagnostics = validate_legacy("Some rules content");
        let ws_004 = of(&diagnostics, "WS-004");
        assert_eq!(ws_004.len(), 1);
        assert_eq!(ws_004[0].level, DiagnosticLevel::Info);
        assert!(
            ws_004[0].message.contains(".devin/rules/"),
            "{}",
            ws_004[0].message
        );
        let suggestion = ws_004[0].suggestion.as_deref().unwrap();
        assert!(suggestion.contains("AGENTS.md"), "{suggestion}");
        // No tools configured: Cline may read the file.
        assert!(suggestion.contains("Cline"), "{suggestion}");
    }

    #[test]
    fn test_ws_004_cline_caveat_only_when_cline_may_read() {
        let mut config = LintConfig::default();
        config.set_tools(vec!["windsurf".to_string()]);
        let diagnostics = WindsurfValidator.validate(Path::new(".windsurfrules"), "x", &config);
        let suggestion = diagnostics[0].suggestion.as_deref().unwrap();
        assert!(!suggestion.contains("Cline"), "{suggestion}");
        config.set_tools(vec!["windsurf".to_string(), "cline".to_string()]);
        let diagnostics = WindsurfValidator.validate(Path::new(".windsurfrules"), "x", &config);
        assert!(
            diagnostics[0]
                .suggestion
                .as_deref()
                .unwrap()
                .contains("Cline")
        );
    }

    #[test]
    fn test_ws_004_legacy_empty() {
        let diagnostics = validate_legacy("");
        let ws_004 = of(&diagnostics, "WS-004");
        assert_eq!(
            ws_004.len(),
            1,
            "WS-004 should trigger even on empty legacy file"
        );
    }

    #[test]
    fn test_ws_004_legacy_only_one_diagnostic() {
        let diagnostics = validate_legacy("Some rules content");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].rule, "WS-004");
    }

    #[test]
    fn test_ws_004_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["WS-004".to_string()];
        let diagnostics =
            WindsurfValidator.validate(Path::new(".windsurfrules"), "content", &config);
        assert!(of(&diagnostics, "WS-004").is_empty());
    }

    // ===== WS-005 / WS-006: trigger =====

    #[test]
    fn test_documented_rule_examples_are_clean() {
        // Devin Desktop's example (not valid YAML: `**` starts an alias).
        let glob = "---\ntrigger: glob\nglobs: **/*.test.ts\n---\n\nAll test files must use `describe`/`it` blocks and mock external API calls.\n";
        assert!(validate_rule(glob).is_empty(), "{:?}", validate_rule(glob));
        // Devin CLI's Windsurf example.
        let cli = "---\ndescription: \"API design rules\"\ntrigger: always_on\n---\n\nAll API endpoints must return JSON with a consistent envelope format.\n";
        assert!(validate_rule(cli).is_empty());
        for content in [
            "---\ntrigger: manual\n---\nRelease checklist.\n",
            "---\ntrigger: agent\ndescription: Database work\n---\nUse migrations.\n",
            "---\ntrigger: model_decision\ndescription: Use when editing SQL\n---\nUse migrations.\n",
            "---\ntrigger: 'model_decision'\ndescription: >\n  Use when editing SQL\n---\nUse migrations.\n",
            "---\ntrigger: glob\nglobs:\n  - \"src/**/*.ts\"\n---\nStrict mode.\n",
            "\u{FEFF}---\r\ntrigger: always_on # comment\r\n---\r\nUse bun.\r\n",
        ] {
            assert!(
                validate_rule(content).is_empty(),
                "{content:?}: {:?}",
                validate_rule(content)
            );
        }
    }

    #[test]
    fn test_ws_005_model_decision_without_description() {
        for content in [
            "---\ntrigger: model_decision\n---\nUse migrations.\n",
            "---\ntrigger: model_decision\ndescription: \"\"\n---\nUse migrations.\n",
            "---\ntrigger: model_decision\ndescription:\n---\nUse migrations.\n",
            "---\ntrigger: model_decision\ndescription: \"  \"\n---\nUse migrations.\n",
        ] {
            let diagnostics = validate_rule(content);
            assert_eq!(diagnostics.len(), 1, "{content:?}: {diagnostics:?}");
            assert_eq!(diagnostics[0].rule, "WS-005");
            assert_eq!(diagnostics[0].level, DiagnosticLevel::Warning);
        }
        let diagnostics = validate_rule("---\ntrigger: model_decision\n---\nx\n");
        assert_eq!((diagnostics[0].line, diagnostics[0].column), (2, 10));
        let diagnostics = validate_rule("---\ntrigger: model_decision\ndescription: ''\n---\nx\n");
        assert_eq!((diagnostics[0].line, diagnostics[0].column), (3, 1));
    }

    #[test]
    fn test_ws_006_unknown_trigger() {
        let diagnostics = validate_rule("---\ntrigger: always\n---\nUse bun.\n");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        let d = &diagnostics[0];
        assert_eq!(
            (d.rule.as_str(), d.level),
            ("WS-006", DiagnosticLevel::Warning)
        );
        assert_eq!((d.line, d.column), (2, 10));
        assert!(d.message.contains("'always'"), "{}", d.message);
        assert!(d.suggestion.as_deref().unwrap().contains("always_on"));
        // Case matters: the documented values are lowercase.
        let diagnostics = validate_rule("---\ntrigger: Always_On\n---\nUse bun.\n");
        assert_eq!(diagnostics[0].rule, "WS-006");
    }

    #[test]
    fn test_ws_006_glob_without_globs() {
        for content in [
            "---\ntrigger: glob\n---\nStrict mode.\n",
            "---\ntrigger: glob\nglobs: []\n---\nStrict mode.\n",
            "---\ntrigger: glob\nglobs: \"\"\n---\nStrict mode.\n",
        ] {
            let diagnostics = validate_rule(content);
            assert_eq!(diagnostics.len(), 1, "{content:?}: {diagnostics:?}");
            assert_eq!(diagnostics[0].rule, "WS-006");
            assert_eq!(diagnostics[0].level, DiagnosticLevel::Warning);
            assert!(diagnostics[0].message.contains("globs"));
        }
    }

    #[test]
    fn test_ws_006_missing_trigger_is_info() {
        let diagnostics = validate_rule("# TypeScript Guidelines\nUse strict mode.");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].rule, "WS-006");
        assert_eq!(diagnostics[0].level, DiagnosticLevel::Info);
        let diagnostics = validate_rule("---\ndescription: TypeScript\n---\nUse strict mode.");
        assert_eq!(diagnostics[0].rule, "WS-006");
        assert_eq!(diagnostics[0].level, DiagnosticLevel::Info);
        assert_eq!(diagnostics[0].line, 2);
    }

    #[test]
    fn test_ws_006_cursor_always_apply_hint() {
        let diagnostics = validate_rule(
            "---\ndescription: TS\nglobs: src/**\nalwaysApply: true\n---\nUse strict mode.",
        );
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        let d = &diagnostics[0];
        assert_eq!(
            (d.rule.as_str(), d.level),
            ("WS-006", DiagnosticLevel::Info)
        );
        assert_eq!(d.line, 4);
        assert!(d.message.contains("alwaysApply"), "{}", d.message);
        assert!(
            d.suggestion
                .as_deref()
                .unwrap()
                .contains("trigger: always_on")
        );
    }

    // ===== WS-007: legacy directories =====

    fn legacy_tree() -> LintConfig {
        let fs = Arc::new(MockFileSystem::new());
        for dir in [
            "/p/.windsurf/rules",
            "/p/.windsurf/rules/0",
            "/p/.windsurf/workflows",
            "/p/.devin/rules",
        ] {
            fs.add_dir(dir);
        }
        fs.add_file("/p/.windsurf/rules/b.md", ALWAYS_ON);
        fs.add_file("/p/.windsurf/rules/a.md", ALWAYS_ON);
        fs.add_file("/p/.windsurf/rules/0/z.md", ALWAYS_ON);
        fs.add_file("/p/.windsurf/rules/SKILL.md", "---\nname: x\n---\n");
        fs.add_file("/p/.windsurf/workflows/deploy.md", "# Deploy\n1. Ship\n");
        fs.add_file("/p/.devin/rules/c.md", ALWAYS_ON);
        let mut config = LintConfig::default();
        config.set_fs(fs);
        config
    }

    #[test]
    fn test_ws_007_once_per_legacy_directory() {
        let config = legacy_tree();
        let mut reported = Vec::new();
        for file in [
            "/p/.windsurf/rules/a.md",
            "/p/.windsurf/rules/b.md",
            "/p/.windsurf/rules/0/z.md",
            "/p/.devin/rules/c.md",
        ] {
            let diagnostics = WindsurfValidator.validate(Path::new(file), ALWAYS_ON, &config);
            if !of(&diagnostics, "WS-007").is_empty() {
                reported.push(file);
            }
        }
        // `0/z.md` sorts first (path components are compared in order).
        assert_eq!(reported, ["/p/.windsurf/rules/0/z.md"]);

        let diagnostics = WindsurfValidator.validate(
            Path::new("/p/.windsurf/workflows/deploy.md"),
            "# Deploy\n1. Ship\n",
            &config,
        );
        let ws_007 = of(&diagnostics, "WS-007");
        assert_eq!(ws_007.len(), 1);
        assert_eq!(ws_007[0].level, DiagnosticLevel::Info);
        assert!(
            ws_007[0].message.contains(".devin/workflows/"),
            "{}",
            ws_007[0].message
        );
    }

    #[test]
    fn test_ws_007_unlisted_directory_reports_the_file() {
        let diagnostics = validate_at(".windsurf/rules/test.md", ALWAYS_ON);
        assert_eq!(of(&diagnostics, "WS-007").len(), 1);
        assert!(of(&validate_rule(ALWAYS_ON), "WS-007").is_empty());
    }

    // ===== WS-008: generic instructions =====

    #[test]
    fn test_ws_008_generic_instructions() {
        let content = "---\ntrigger: always_on\ndescription: Be concise\n---\n# Be precise\n- Write good code\n- Be concise in commit messages\n- Always be helpful and accurate.\n```\nbe helpful\n```\n- Use pnpm, not npm\n";
        let diagnostics = validate_rule(content);
        let ws_008 = of(&diagnostics, "WS-008");
        let lines: Vec<usize> = ws_008.iter().map(|d| d.line).collect();
        assert_eq!(lines, [6, 8], "{diagnostics:?}");
        assert_eq!(ws_008[0].column, 3);
        assert_eq!(ws_008[0].level, DiagnosticLevel::Info);
        assert!(
            ws_008[0].message.contains("'Write good code'"),
            "{}",
            ws_008[0].message
        );
        assert!(ws_008[0].fixes.is_empty());
    }

    #[test]
    fn test_ws_008_on_legacy_and_global_rules() {
        let diagnostics = validate_legacy("- Be precise.\n");
        assert_eq!(of(&diagnostics, "WS-008").len(), 1);
        let diagnostics = validate_at(
            "/home/u/.codeium/windsurf/memories/global_rules.md",
            "- Use best practices\n",
        );
        assert_eq!(of(&diagnostics, "WS-008").len(), 1);
        // Not on workflows.
        assert!(of(&validate_workflow("- Be concise\n"), "WS-008").is_empty());
    }

    // ===== WS-009: workflows =====

    #[test]
    fn test_ws_009_every_workflow() {
        for path in [
            ".devin/workflows/deploy.md",
            "/home/u/.codeium/windsurf/global_workflows/deploy.md",
        ] {
            let diagnostics = validate_at(path, "# Deploy\n1. Run tests\n");
            assert_eq!(diagnostics.len(), 1, "{path}: {diagnostics:?}");
            assert_eq!(diagnostics[0].rule, "WS-009");
            assert_eq!(diagnostics[0].level, DiagnosticLevel::Info);
            assert!(diagnostics[0].message.contains("Devin Local"));
        }
    }

    // ===== WS-010: global rules =====

    #[test]
    fn test_ws_010_global_rules_limit() {
        let path = "/home/u/.codeium/windsurf/memories/global_rules.md";
        assert!(validate_at(path, &"x".repeat(GLOBAL_RULES_CHAR_LIMIT)).is_empty());
        assert!(validate_at(path, &"ü".repeat(GLOBAL_RULES_CHAR_LIMIT)).is_empty());
        let diagnostics = validate_at(path, &"x".repeat(GLOBAL_RULES_CHAR_LIMIT + 1));
        let ws_010 = of(&diagnostics, "WS-010");
        assert_eq!(ws_010.len(), 1);
        assert_eq!(ws_010[0].level, DiagnosticLevel::Error);
        assert!(
            ws_010[0].message.contains("6001 characters"),
            "{}",
            ws_010[0].message
        );
        // Devin CLI's project global rules file is not this file.
        assert_eq!(
            crate::detect_file_type(Path::new(".devin/global_rules.md")),
            FileType::GenericMarkdown
        );
    }

    // ===== WS-011: AGENTS.md =====

    fn agents_config(files: &[&str], tools: &[&str]) -> LintConfig {
        let fs = Arc::new(MockFileSystem::new());
        for file in files {
            if file.ends_with('/') {
                fs.add_dir(file.trim_end_matches('/'));
            } else {
                fs.add_file(file, "x");
            }
        }
        let mut config = LintConfig::default();
        config.set_fs(fs);
        config.set_root_dir(PathBuf::from("/p"));
        config.set_tools(tools.iter().map(|t| (*t).to_string()).collect());
        config
    }

    fn ws_011(path: &str, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        WindsurfAgentsMdValidator.validate(Path::new(path), content, config)
    }

    #[test]
    fn test_ws_011_only_in_windsurf_projects() {
        let long = "a".repeat(WORKSPACE_RULE_CHAR_LIMIT + 1);
        for markers in [
            &["/p/.windsurf/"][..],
            &["/p/.devin/rules/"][..],
            &["/p/.devin/workflows/"][..],
            &["/p/.devin/hooks.json"][..],
            &["/p/.windsurfrules"][..],
        ] {
            let config = agents_config(markers, &[]);
            let diagnostics = ws_011("/p/sub/AGENTS.md", &long, &config);
            assert_eq!(diagnostics.len(), 1, "{markers:?}");
            assert_eq!(diagnostics[0].level, DiagnosticLevel::Warning);
            assert!(diagnostics[0].message.contains("12001 characters"));
        }
        // `.devin/` alone is Devin CLI configuration.
        let config = agents_config(&["/p/.devin/config.json"], &[]);
        assert!(ws_011("/p/AGENTS.md", &long, &config).is_empty());
        // Selected tools decide.
        let config = agents_config(&[], &["windsurf"]);
        assert_eq!(ws_011("/p/AGENTS.md", &long, &config).len(), 1);
        let config = agents_config(&["/p/.windsurf/"], &["codex"]);
        assert!(ws_011("/p/AGENTS.md", &long, &config).is_empty());
    }

    #[test]
    fn test_ws_011_counts_characters_and_skips_other_files() {
        let config = agents_config(&[], &["windsurf"]);
        assert!(
            ws_011(
                "/p/AGENTS.md",
                &"规".repeat(WORKSPACE_RULE_CHAR_LIMIT),
                &config
            )
            .is_empty()
        );
        let long = "a".repeat(WORKSPACE_RULE_CHAR_LIMIT + 1);
        for name in ["CLAUDE.md", "AGENTS.local.md", "AGENTS.override.md"] {
            assert!(
                ws_011(&format!("/p/{name}"), &long, &config).is_empty(),
                "{name}"
            );
        }
    }

    // ===== Category disable =====

    #[test]
    fn test_windsurf_category_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().windsurf = false;
        for (path, content) in [
            (".devin/rules/test.md", ""),
            (".windsurf/rules/test.md", "---\ntrigger: nope\n---\nx"),
            (".windsurfrules", "content"),
            (".devin/workflows/x.md", "x"),
        ] {
            let diagnostics = WindsurfValidator.validate(Path::new(path), content, &config);
            assert!(diagnostics.is_empty(), "{path}: {diagnostics:?}");
        }
        let mut config = LintConfig::default();
        config.set_tools(vec!["windsurf".to_string()]);
        config.rules_mut().windsurf = false;
        let long = "a".repeat(WORKSPACE_RULE_CHAR_LIMIT + 1);
        assert!(ws_011("AGENTS.md", &long, &config).is_empty());
    }

    // ===== Valid files produce no diagnostics =====

    #[test]
    fn test_valid_rule_no_diagnostics() {
        let diagnostics = validate_rule(
            "---\ntrigger: always_on\n---\n# TypeScript Guidelines\nUse strict mode.",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    // ===== Metadata =====

    #[test]
    fn test_metadata() {
        let meta = WindsurfValidator.metadata();
        assert_eq!(meta.name, "WindsurfValidator");
        assert_eq!(meta.rule_ids, RULE_IDS);
        let meta = WindsurfAgentsMdValidator.metadata();
        assert_eq!(meta.name, "WindsurfAgentsMdValidator");
        assert_eq!(meta.rule_ids, &["WS-011"]);
    }
}
