//! Claude Code memory-file validation.
//!
//! Runs on CLAUDE.md and CLAUDE.local.md (`FileType::ClaudeMd`; AGENTS.md
//! files share that type and are skipped here), on `.claude/rules/**/*.md`
//! (`FileType::ClaudeRule`, length and emphasis only), on MEMORY.md indexes
//! (`FileType::ClaudeMemoryIndex`), on `.claudeignore`
//! (`FileType::ClaudeIgnore`) and on `.claude/settings.local.json`
//! (`FileType::Hooks`, CC-MEM-020 only). Every rule follows the Claude Code
//! memory and best-practices pages; nothing here runs on other vendors'
//! files.
//!
//! - CC-MEM-004: `npm run <script>` names a script package.json lacks
//! - CC-MEM-010: CLAUDE.md repeats README.md (agnix heuristic threshold)
//! - CC-MEM-014: an instruction file, or an `@path` import, over 200 lines
//! - CC-MEM-015: always-loaded instruction files add up past a combined size
//! - CC-MEM-016: MEMORY.md past its 200-line or 25KB load limit
//! - CC-MEM-017: emphasis such as IMPORTANT or MUST on many lines
//! - CC-MEM-018: CLAUDE.md asks Claude to read AGENTS.md instead of importing
//!   it, or CLAUDE.local.md stops Claude from reading AGENTS.md
//! - CC-MEM-019: an `@path` import that is quoted or has an unescaped space
//! - CC-MEM-020: CLAUDE.local.md or local subagent memory not gitignored
//! - CC-MEM-021: a long directory tree or file-by-file listing
//! - CC-SET-038: `.claudeignore`, which Claude Code does not read

use crate::{
    FileType,
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    fs::FileSystem,
    parsers::markdown::extract_imports,
    rules::{Validator, ValidatorMetadata},
    schemas::claude_md::{
        EMPHASIS_LINE_LIMIT, LineCountMode, MAX_COMBINED_INSTRUCTION_LINES,
        MAX_INSTRUCTION_FILE_LINES, MEMORY_INDEX_MAX_BYTES, MEMORY_INDEX_MAX_LINES,
        check_readme_duplication, extract_npm_scripts, find_agents_md_deferral,
        find_directory_tree, find_emphasis_lines, find_file_listing, find_quoted_imports,
        find_spaced_imports, loaded_line_count,
    },
};
use rust_i18n::t;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const RULE_IDS: &[&str] = &[
    "CC-MEM-004",
    "CC-MEM-010",
    "CC-MEM-014",
    "CC-MEM-015",
    "CC-MEM-016",
    "CC-MEM-017",
    "CC-MEM-018",
    "CC-MEM-019",
    "CC-MEM-020",
    "CC-MEM-021",
    "CC-SET-038",
];

/// Claude Code's import depth: "a maximum depth of four hops".
const MAX_IMPORT_HOPS: usize = 4;

pub struct ClaudeMdValidator;

/// Which memory file a path is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemoryFile {
    /// `CLAUDE.md`, including `.claude/CLAUDE.md`.
    ClaudeMd,
    /// `CLAUDE.local.md`.
    ClaudeLocalMd,
    /// `.claude/rules/**/*.md`.
    RuleFile,
    /// `.claude/settings.local.json`.
    LocalSettings,
    /// A `MEMORY.md` index Claude Code loads at session start.
    MemoryIndex,
    /// `.claudeignore`.
    ClaudeIgnore,
}

fn classify(path: &Path) -> Option<MemoryFile> {
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    match filename {
        "CLAUDE.md" => Some(MemoryFile::ClaudeMd),
        "CLAUDE.local.md" => Some(MemoryFile::ClaudeLocalMd),
        "settings.local.json"
            if path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                == Some(".claude") =>
        {
            Some(MemoryFile::LocalSettings)
        }
        _ => match crate::detect_file_type(path) {
            FileType::ClaudeRule => Some(MemoryFile::RuleFile),
            FileType::ClaudeMemoryIndex => Some(MemoryFile::MemoryIndex),
            FileType::ClaudeIgnore => Some(MemoryFile::ClaudeIgnore),
            _ => None,
        },
    }
}

/// The directory a memory file belongs to: `./.claude/CLAUDE.md` belongs to
/// `./`, every other file to its own directory.
fn project_dir(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or(Path::new(""));
    if parent.file_name().and_then(|n| n.to_str()) == Some(".claude") {
        parent.parent().unwrap_or(Path::new("")).to_path_buf()
    } else {
        parent.to_path_buf()
    }
}

fn display_name(path: &Path) -> String {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let in_claude_dir = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        == Some(".claude");
    if in_claude_dir {
        format!(".claude/{name}")
    } else {
        name.to_string()
    }
}

impl Validator for ClaudeMdValidator {
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
        let Some(kind) = classify(path) else {
            return diagnostics;
        };
        let fs = config.fs().as_ref();

        match kind {
            MemoryFile::MemoryIndex => {
                if config.is_rule_enabled("CC-MEM-016") {
                    check_memory_index(path, content, &mut diagnostics);
                }
                if config.is_rule_enabled("CC-MEM-020") {
                    check_not_gitignored(path, fs, &mut diagnostics);
                }
                return diagnostics;
            }
            MemoryFile::ClaudeIgnore => {
                if config.is_rule_enabled("CC-SET-038") {
                    check_claudeignore(path, content, &mut diagnostics);
                }
                return diagnostics;
            }
            MemoryFile::LocalSettings => {
                if config.is_rule_enabled("CC-MEM-020") {
                    check_not_gitignored(path, fs, &mut diagnostics);
                }
                return diagnostics;
            }
            MemoryFile::RuleFile => {
                if config.is_rule_enabled("CC-MEM-014") {
                    check_rule_file_length(path, content, &mut diagnostics);
                }
                if config.is_rule_enabled("CC-MEM-017") {
                    let body_start = crate::schemas::claude_md::frontmatter_body_start(content);
                    let offset = content[..body_start].lines().count();
                    if let Some(diagnostic) =
                        emphasis_diagnostic(path, &content[body_start..], offset)
                    {
                        diagnostics.push(diagnostic);
                    }
                }
                return diagnostics;
            }
            MemoryFile::ClaudeMd | MemoryFile::ClaudeLocalMd => {}
        }

        if config.is_rule_enabled("CC-MEM-004") {
            check_npm_scripts(path, content, &mut diagnostics);
        }
        if config.is_rule_enabled("CC-MEM-010") {
            check_readme(path, content, &mut diagnostics);
        }
        if config.is_rule_enabled("CC-MEM-014") {
            check_file_length(path, content, &mut diagnostics);
            check_import_lengths(path, content, fs, &mut diagnostics);
        }
        if config.is_rule_enabled("CC-MEM-015") {
            check_combined_length(path, content, fs, &mut diagnostics);
        }
        if config.is_rule_enabled("CC-MEM-017") {
            check_emphasis(path, content, &mut diagnostics);
        }
        if config.is_rule_enabled("CC-MEM-018") {
            check_agents_md_reach(path, kind, content, fs, &mut diagnostics);
        }
        if config.is_rule_enabled("CC-MEM-019") {
            check_import_syntax(path, content, fs, &mut diagnostics);
        }
        if kind == MemoryFile::ClaudeLocalMd && config.is_rule_enabled("CC-MEM-020") {
            check_not_gitignored(path, fs, &mut diagnostics);
        }
        if kind == MemoryFile::ClaudeMd && config.is_rule_enabled("CC-MEM-021") {
            check_listings(path, content, &mut diagnostics);
        }

        diagnostics
    }
}

// ============================================================================
// CC-MEM-004 and CC-MEM-010
// ============================================================================

fn check_npm_scripts(path: &Path, content: &str, diagnostics: &mut Vec<Diagnostic>) {
    let npm_refs = extract_npm_scripts(content);
    if npm_refs.is_empty() {
        return;
    }
    // package.json of the project the memory file belongs to (the parent of
    // `.claude/` for `.claude/CLAUDE.md`), read with the safe reader.
    let Ok(pkg_content) =
        crate::file_utils::safe_read_file(&project_dir(path).join("package.json"))
    else {
        return;
    };
    let Ok(pkg_json) = serde_json::from_str::<serde_json::Value>(&pkg_content) else {
        return;
    };
    let available_scripts: Vec<String> = pkg_json
        .get("scripts")
        .and_then(|s| s.as_object())
        .map(|scripts| scripts.keys().cloned().collect())
        .unwrap_or_default();

    for npm_ref in npm_refs {
        if available_scripts.contains(&npm_ref.script_name) {
            continue;
        }
        let suggestion = if available_scripts.is_empty() {
            t!("rules.cc_mem_004.suggestion_no_scripts").to_string()
        } else {
            t!(
                "rules.cc_mem_004.suggestion_available",
                scripts = available_scripts.join(", ")
            )
            .to_string()
        };
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                npm_ref.line,
                npm_ref.column,
                "CC-MEM-004",
                t!(
                    "rules.cc_mem_004.message",
                    script = npm_ref.script_name.as_str()
                ),
            )
            .with_suggestion(suggestion),
        );
    }
}

fn check_readme(path: &Path, content: &str, diagnostics: &mut Vec<Diagnostic>) {
    let Ok(readme_content) =
        crate::file_utils::safe_read_file(&project_dir(path).join("README.md"))
    else {
        return;
    };
    let Some(dup) = check_readme_duplication(content, &readme_content) else {
        return;
    };
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            1,
            0,
            "CC-MEM-010",
            t!(
                "rules.cc_mem_010.message",
                file = display_name(path),
                overlap = format!("{:.0}", dup.overlap_percent),
                threshold = format!("{:.0}", dup.threshold)
            ),
        )
        .with_suggestion(t!("rules.cc_mem_010.suggestion")),
    );
}

// ============================================================================
// CC-MEM-014 and CC-MEM-015: instruction file length
// ============================================================================

fn check_file_length(path: &Path, content: &str, diagnostics: &mut Vec<Diagnostic>) {
    let lines = loaded_line_count(content, LineCountMode::MemoryFile);
    if lines <= MAX_INSTRUCTION_FILE_LINES {
        return;
    }
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            1,
            0,
            "CC-MEM-014",
            t!(
                "rules.cc_mem_014.message",
                file = display_name(path),
                lines = lines,
                limit = MAX_INSTRUCTION_FILE_LINES
            ),
        )
        .with_suggestion(t!("rules.cc_mem_014.suggestion")),
    );
}

/// CC-MEM-014 for a rule file: "Each CLAUDE.md, rules file, and `@path`
/// import counts as a separate file". The frontmatter is removed before the
/// rule loads, so only the body counts.
fn check_rule_file_length(path: &Path, content: &str, diagnostics: &mut Vec<Diagnostic>) {
    let lines = loaded_line_count(content, LineCountMode::RuleFile);
    if lines <= MAX_INSTRUCTION_FILE_LINES {
        return;
    }
    let path_scoped = crate::schemas::claude_rules::parse_frontmatter(content)
        .and_then(|parsed| parsed.schema)
        .is_some_and(|schema| !schema.paths.is_empty());
    let suggestion = if path_scoped {
        t!("rules.cc_mem_014.rule_scoped_suggestion")
    } else {
        t!("rules.cc_mem_014.rule_suggestion")
    };
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            1,
            0,
            "CC-MEM-014",
            t!(
                "rules.cc_mem_014.rule_message",
                lines = lines,
                limit = MAX_INSTRUCTION_FILE_LINES
            ),
        )
        .with_suggestion(suggestion),
    );
}

/// A file an `@path` import loads, with the line of the import that names it.
struct LoadedImport {
    path: PathBuf,
    /// The import as written, without the `@`.
    written: String,
    line: usize,
    column: usize,
    content: String,
}

/// Resolve an import the way Claude Code does: relative to the file holding
/// it, absolute paths as given. Home-directory imports are personal files
/// outside the project and are not read here.
fn resolve_import(import: &str, base_dir: &Path) -> Option<PathBuf> {
    if import.starts_with('~') {
        return None;
    }
    let raw = Path::new(import);
    Some(if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        base_dir.join(raw)
    })
}

/// Files that a memory file's direct imports load and that exist.
fn direct_imports(path: &Path, content: &str, fs: &dyn FileSystem) -> Vec<LoadedImport> {
    let base_dir = path.parent().unwrap_or(Path::new(""));
    extract_imports(content)
        .into_iter()
        .filter_map(|import| {
            let resolved = resolve_import(&import.path, base_dir)?;
            if !fs.is_file(&resolved) {
                return None;
            }
            let content = fs.read_to_string(&resolved).ok()?;
            Some(LoadedImport {
                path: resolved,
                written: import.path,
                line: import.line,
                column: import.column,
                content,
            })
        })
        .collect()
}

/// Imports that agnix checks as files of their own (CLAUDE.md, rules).
fn is_checked_directly(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("CLAUDE.md" | "CLAUDE.local.md")
    ) || crate::file_types::path_contains_consecutive_components(path, ".claude", "rules")
}

fn check_import_lengths(
    path: &Path,
    content: &str,
    fs: &dyn FileSystem,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for import in direct_imports(path, content, fs) {
        if is_checked_directly(&import.path) {
            continue;
        }
        let lines = loaded_line_count(&import.content, LineCountMode::Plain);
        if lines <= MAX_INSTRUCTION_FILE_LINES {
            continue;
        }
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                import.line,
                import.column,
                "CC-MEM-014",
                t!(
                    "rules.cc_mem_014.import_message",
                    import = import.written.as_str(),
                    lines = lines,
                    limit = MAX_INSTRUCTION_FILE_LINES
                ),
            )
            .with_suggestion(t!("rules.cc_mem_014.import_suggestion")),
        );
    }
}

/// Lines of every file reachable from `start` through imports, up to the
/// import depth Claude Code follows. Each file counts once.
fn add_with_imports(
    start: &Path,
    content: &str,
    mode: LineCountMode,
    fs: &dyn FileSystem,
    seen: &mut HashSet<PathBuf>,
    parts: &mut Vec<(String, usize)>,
    root: &Path,
) {
    let key = fs
        .canonicalize(start)
        .unwrap_or_else(|_| start.to_path_buf());
    if !seen.insert(key) {
        return;
    }
    parts.push((
        relative_label(start, root),
        loaded_line_count(content, mode),
    ));

    let mut frontier = vec![(start.to_path_buf(), content.to_string(), 0usize)];
    while let Some((file, text, depth)) = frontier.pop() {
        if depth >= MAX_IMPORT_HOPS {
            continue;
        }
        for import in direct_imports(&file, &text, fs) {
            let key = fs
                .canonicalize(&import.path)
                .unwrap_or_else(|_| import.path.clone());
            if !seen.insert(key) {
                continue;
            }
            parts.push((
                format!("@{}", import.written),
                loaded_line_count(&import.content, LineCountMode::Plain),
            ));
            frontier.push((import.path, import.content, depth + 1));
        }
    }
}

fn relative_label(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Rule files under `.claude/rules/` that load at launch: those without a
/// `paths` field, including rules whose frontmatter does not parse ("loads
/// the rule as if it had no `paths`").
fn unconditional_rule_files(rules_dir: &Path, fs: &dyn FileSystem) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    let mut stack = vec![rules_dir.to_path_buf()];
    let mut visited = 0usize;
    while let Some(dir) = stack.pop() {
        visited += 1;
        if visited > 256 {
            break;
        }
        let Ok(entries) = fs.read_dir(&dir) else {
            continue;
        };
        for entry in entries {
            if entry.metadata.is_symlink {
                continue;
            }
            if entry.metadata.is_dir {
                stack.push(entry.path);
                continue;
            }
            if entry.path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Ok(text) = fs.read_to_string(&entry.path) else {
                continue;
            };
            let scoped = crate::schemas::claude_rules::parse_frontmatter(&text)
                .and_then(|parsed| parsed.schema)
                .is_some_and(|schema| !schema.paths.is_empty());
            if !scoped {
                found.push((entry.path, text));
            }
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

fn check_combined_length(
    path: &Path,
    content: &str,
    fs: &dyn FileSystem,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let dir = project_dir(path);
    let candidates = [
        dir.join("CLAUDE.md"),
        dir.join(".claude").join("CLAUDE.md"),
        dir.join("CLAUDE.local.md"),
    ];
    // Report once per directory, on its first memory file.
    let Some(owner) = candidates.iter().find(|c| fs.is_file(c)).cloned() else {
        return;
    };
    let same_file = |a: &Path, b: &Path| {
        fs.canonicalize(a)
            .ok()
            .zip(fs.canonicalize(b).ok())
            .is_some_and(|(x, y)| x == y)
            || a == b
    };
    if !same_file(&owner, path) {
        return;
    }

    let mut seen = HashSet::new();
    let mut parts: Vec<(String, usize)> = Vec::new();
    for candidate in &candidates {
        let text = if same_file(candidate, path) {
            Some(content.to_string())
        } else if fs.is_file(candidate) {
            fs.read_to_string(candidate).ok()
        } else {
            None
        };
        if let Some(text) = text {
            add_with_imports(
                candidate,
                &text,
                LineCountMode::MemoryFile,
                fs,
                &mut seen,
                &mut parts,
                &dir,
            );
        }
    }
    for (rule, text) in unconditional_rule_files(&dir.join(".claude").join("rules"), fs) {
        add_with_imports(
            &rule,
            &text,
            LineCountMode::RuleFile,
            fs,
            &mut seen,
            &mut parts,
            &dir,
        );
    }

    let total: usize = parts.iter().map(|(_, lines)| lines).sum();
    // One file over the limit is CC-MEM-014's finding.
    if parts.len() < 2 || total <= MAX_COMBINED_INSTRUCTION_LINES {
        return;
    }
    let mut largest = parts.clone();
    largest.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let breakdown = largest
        .iter()
        .take(4)
        .map(|(name, lines)| format!("{name} {lines}"))
        .collect::<Vec<_>>()
        .join(", ");
    diagnostics.push(
        Diagnostic::info(
            path.to_path_buf(),
            1,
            0,
            "CC-MEM-015",
            t!(
                "rules.cc_mem_015.message",
                lines = total,
                files = parts.len(),
                breakdown = breakdown,
                limit = MAX_COMBINED_INSTRUCTION_LINES
            ),
        )
        .with_suggestion(t!("rules.cc_mem_015.suggestion")),
    );
}

// ============================================================================
// CC-MEM-016: MEMORY.md load limit
// ============================================================================

fn check_memory_index(path: &Path, content: &str, diagnostics: &mut Vec<Diagnostic>) {
    let lines = content.lines().count();
    let bytes = content.len();
    if lines <= MEMORY_INDEX_MAX_LINES && bytes <= MEMORY_INDEX_MAX_BYTES {
        return;
    }
    // The first line Claude does not see: past line 200, or the line holding
    // the byte where 25KB runs out, whichever comes first.
    let byte_line = if bytes > MEMORY_INDEX_MAX_BYTES {
        content[..MEMORY_INDEX_MAX_BYTES.min(content.len())]
            .bytes()
            .filter(|&b| b == b'\n')
            .count()
            + 1
    } else {
        usize::MAX
    };
    let first_dropped = (MEMORY_INDEX_MAX_LINES + 1).min(byte_line);
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            first_dropped,
            0,
            "CC-MEM-016",
            t!(
                "rules.cc_mem_016.message",
                lines = lines,
                bytes = bytes,
                line = first_dropped
            ),
        )
        .with_suggestion(t!("rules.cc_mem_016.suggestion")),
    );
}

// ============================================================================
// CC-MEM-017: emphasis overuse
// ============================================================================

fn check_emphasis(path: &Path, content: &str, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(diagnostic) = emphasis_diagnostic(path, content, 0) {
        diagnostics.push(diagnostic);
    }
}

/// CC-MEM-017 for a memory or rule file. `line_offset` shifts lines when the
/// caller scanned only part of the file.
fn emphasis_diagnostic(path: &Path, content: &str, line_offset: usize) -> Option<Diagnostic> {
    let lines = find_emphasis_lines(content);
    if lines.len() <= EMPHASIS_LINE_LIMIT {
        return None;
    }
    let first = &lines[0];
    let listed = lines
        .iter()
        .take(6)
        .map(|l| (l.line + line_offset).to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Some(
        Diagnostic::info(
            path.to_path_buf(),
            first.line + line_offset,
            first.column,
            "CC-MEM-017",
            t!(
                "rules.cc_mem_017.message",
                count = lines.len(),
                lines = listed
            ),
        )
        .with_suggestion(t!("rules.cc_mem_017.suggestion")),
    )
}

// ============================================================================
// CC-MEM-018: AGENTS.md that Claude may not read
// ============================================================================

/// True when a CLAUDE.md, `.claude/CLAUDE.md` or (if `count_local`)
/// CLAUDE.local.md exists in `dir` or any directory above it. Those are the
/// files that make Claude read CLAUDE.md "instead of AGENTS.md".
/// The user's `~/.claude/CLAUDE.md` does not count ("Don't count, and keep
/// loading alongside `AGENTS.md`: your `~/.claude/CLAUDE.md`").
fn claude_md_at_or_above(dir: &Path, fs: &dyn FileSystem, count_local: bool) -> bool {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    let absolute = if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(dir))
            .unwrap_or_else(|_| dir.to_path_buf())
    };
    absolute.ancestors().take(64).any(|ancestor| {
        let is_home = home.as_deref() == Some(ancestor);
        fs.is_file(&ancestor.join("CLAUDE.md"))
            || (!is_home && fs.is_file(&ancestor.join(".claude").join("CLAUDE.md")))
            || (count_local && fs.is_file(&ancestor.join("CLAUDE.local.md")))
    })
}

fn check_agents_md_reach(
    path: &Path,
    kind: MemoryFile,
    content: &str,
    fs: &dyn FileSystem,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let dir = project_dir(path);
    let agents_md = dir.join("AGENTS.md");
    if !fs.is_file(&agents_md) {
        return;
    }
    let imports_agents_md = extract_imports(content).iter().any(|import| {
        Path::new(&import.path).file_name().and_then(|n| n.to_str()) == Some("AGENTS.md")
    });
    if imports_agents_md {
        return;
    }

    match kind {
        MemoryFile::ClaudeMd => {
            let Some(deferral) = find_agents_md_deferral(content) else {
                return;
            };
            diagnostics.push(
                Diagnostic::info(
                    path.to_path_buf(),
                    deferral.line,
                    deferral.column,
                    "CC-MEM-018",
                    t!(
                        "rules.cc_mem_018.prose_message",
                        file = display_name(path),
                        text = deferral.text.as_str()
                    ),
                )
                .with_suggestion(t!("rules.cc_mem_018.prose_suggestion")),
            );
        }
        MemoryFile::ClaudeLocalMd => {
            // A CLAUDE.md anywhere on the path already replaces AGENTS.md.
            if claude_md_at_or_above(&dir, fs, false) {
                return;
            }
            diagnostics.push(
                Diagnostic::info(
                    path.to_path_buf(),
                    1,
                    0,
                    "CC-MEM-018",
                    t!("rules.cc_mem_018.local_message"),
                )
                .with_suggestion(t!("rules.cc_mem_018.local_suggestion")),
            );
        }
        MemoryFile::RuleFile
        | MemoryFile::LocalSettings
        | MemoryFile::MemoryIndex
        | MemoryFile::ClaudeIgnore => {}
    }
}

// ============================================================================
// CC-MEM-019: import syntax Claude Code does not import
// ============================================================================

fn check_import_syntax(
    path: &Path,
    content: &str,
    fs: &dyn FileSystem,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let base_dir = path.parent().unwrap_or(Path::new(""));
    for quoted in find_quoted_imports(content) {
        // Only quoted names of files that exist: "@'alice.smith'" is a
        // mention, "@'docs/api.md'" with docs/api.md present is an import.
        if !resolve_import(&quoted.path, base_dir).is_some_and(|p| fs.exists(&p)) {
            continue;
        }
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                quoted.line,
                quoted.column,
                "CC-MEM-019",
                t!(
                    "rules.cc_mem_019.quoted_message",
                    import = quoted.raw.as_str()
                ),
            )
            .with_suggestion(t!(
                "rules.cc_mem_019.quoted_suggestion",
                escaped = escape_spaces(&quoted.path)
            )),
        );
    }

    // `@Design Docs/x.md`: the import ends at the space, so Claude Code loads
    // `Design`. Report it when that name is missing and the name with the
    // following words exists.
    for spaced in find_spaced_imports(content) {
        let Some(first) = resolve_import(&spaced.first, base_dir) else {
            continue;
        };
        if fs.exists(&first) {
            continue;
        }
        let mut candidate = spaced.first.clone();
        for word in &spaced.words {
            candidate.push(' ');
            candidate.push_str(word);
            let Some(joined) = resolve_import(&candidate, base_dir) else {
                break;
            };
            if fs.is_file(&joined) {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        spaced.line,
                        spaced.column,
                        "CC-MEM-019",
                        t!(
                            "rules.cc_mem_019.space_message",
                            import = spaced.first.as_str(),
                            intended = candidate.as_str()
                        ),
                    )
                    .with_suggestion(t!(
                        "rules.cc_mem_019.space_suggestion",
                        escaped = escape_spaces(&candidate)
                    )),
                );
                break;
            }
        }
    }
}

fn escape_spaces(path: &str) -> String {
    format!("@{}", path.replace(' ', "\\ "))
}

// ============================================================================
// CC-MEM-020: personal files not gitignored
// ============================================================================

/// The repository root above `path`, found by its `.git` directory or file.
fn find_repo_root(path: &Path, fs: &dyn FileSystem) -> Option<PathBuf> {
    path.ancestors().skip(1).take(64).find_map(|ancestor| {
        let marker = ancestor.join(".git");
        (fs.is_dir(&marker) || fs.is_file(&marker)).then(|| ancestor.to_path_buf())
    })
}

/// Whether git ignores `path`: the repository's `.gitignore` files from the
/// root down to the file, and `.git/info/exclude`. With `include_global`, the
/// user's global excludes file counts too.
#[cfg(feature = "filesystem")]
fn is_gitignored(path: &Path, repo_root: &Path, include_global: bool) -> bool {
    use ignore::gitignore::{Gitignore, GitignoreBuilder};

    let mut builder = GitignoreBuilder::new(repo_root);
    if let Ok(relative_dir) = path.parent().unwrap_or(repo_root).strip_prefix(repo_root) {
        let mut dir = repo_root.to_path_buf();
        builder.add(dir.join(".gitignore"));
        for component in relative_dir.components() {
            dir.push(component);
            builder.add(dir.join(".gitignore"));
        }
    }
    builder.add(repo_root.join(".git").join("info").join("exclude"));
    let ignored_by_repo = builder.build().ok().is_some_and(|gitignore| {
        gitignore
            .matched_path_or_any_parents(path, false)
            .is_ignore()
    });
    if ignored_by_repo || !include_global {
        return ignored_by_repo;
    }
    let (global, _) = Gitignore::global();
    global.matched_path_or_any_parents(path, false).is_ignore()
}

#[cfg(not(feature = "filesystem"))]
fn is_gitignored(_path: &Path, _repo_root: &Path, _include_global: bool) -> bool {
    // Without the filesystem feature there is no gitignore matcher; report
    // nothing rather than guess.
    true
}

/// CC-MEM-020 for CLAUDE.local.md, local subagent memory and
/// `.claude/settings.local.json`. Reports only inside a git repository.
fn check_not_gitignored(path: &Path, fs: &dyn FileSystem, diagnostics: &mut Vec<Diagnostic>) {
    let is_local_agent_memory = crate::file_types::path_contains_consecutive_components(
        path,
        ".claude",
        "agent-memory-local",
    );
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let personal = match filename {
        "CLAUDE.local.md" | "settings.local.json" => true,
        "MEMORY.md" => is_local_agent_memory,
        _ => false,
    };
    if !personal {
        return;
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(path),
            Err(_) => return,
        }
    };
    let Some(repo_root) = find_repo_root(&absolute, fs) else {
        return;
    };
    // The memory page asks for CLAUDE.local.md in ".gitignore". Claude Code
    // itself adds settings.local.json to the global git excludes file, so
    // that file also counts as ignored when the global excludes cover it.
    if is_gitignored(&absolute, &repo_root, filename == "settings.local.json") {
        return;
    }
    let shown = if is_local_agent_memory {
        ".claude/agent-memory-local/".to_string()
    } else if filename == "settings.local.json" {
        ".claude/settings.local.json".to_string()
    } else {
        filename.to_string()
    };
    diagnostics.push(
        Diagnostic::info(
            path.to_path_buf(),
            1,
            0,
            "CC-MEM-020",
            t!("rules.cc_mem_020.message", file = shown.as_str()),
        )
        .with_suggestion(t!("rules.cc_mem_020.suggestion", file = shown.as_str())),
    );
}

// ============================================================================
// CC-MEM-021: directory trees and file-by-file listings
// ============================================================================

fn check_listings(path: &Path, content: &str, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(tree) = find_directory_tree(content) {
        diagnostics.push(
            Diagnostic::info(
                path.to_path_buf(),
                tree.line,
                0,
                "CC-MEM-021",
                t!("rules.cc_mem_021.tree_message", lines = tree.size),
            )
            .with_suggestion(t!("rules.cc_mem_021.suggestion")),
        );
    } else if let Some(listing) = find_file_listing(content) {
        diagnostics.push(
            Diagnostic::info(
                path.to_path_buf(),
                listing.line,
                0,
                "CC-MEM-021",
                t!("rules.cc_mem_021.listing_message", items = listing.size),
            )
            .with_suggestion(t!("rules.cc_mem_021.suggestion")),
        );
    }
}

// ============================================================================
// CC-SET-038: .claudeignore
// ============================================================================

fn check_claudeignore(path: &Path, content: &str, diagnostics: &mut Vec<Diagnostic>) {
    let entries = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .count();
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            1,
            0,
            "CC-SET-038",
            t!("rules.cc_set_038.message", entries = entries),
        )
        .with_suggestion(t!("rules.cc_set_038.suggestion")),
    );
}

#[cfg(test)]
mod tests;
