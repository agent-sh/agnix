//! @import and markdown link reference validation
//!
//! This module validates:
//! - CC-MEM-001: @import references point to existing files (Claude Code specific)
//! - CC-MEM-002: Circular @import detection
//! - CC-MEM-003: @import depth exceeded
//! - REF-001: @import file not found (universal)
//! - REF-002: Broken markdown links (universal)
//! - REF-003: Duplicate @import detection
//! - REF-004: Non-markdown @import detection

use crate::{
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    fs::FileSystem,
    parsers::markdown::{extract_imports, extract_markdown_links},
    parsers::{Import, ImportCache},
    rules::{Validator, ValidatorMetadata, line_byte_range},
};
use rust_i18n::t;
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

const RULE_IDS: &[&str] = &[
    "CC-MEM-001",
    "CC-MEM-002",
    "CC-MEM-003",
    "REF-001",
    "REF-002",
    "REF-003",
    "REF-004",
];

/// Infrastructure rule ID for poisoned-lock recovery diagnostics.
/// Not included in RULE_IDS because it follows the `namespace::type` convention
/// used by pipeline-level diagnostics (like `config::glob`, `file::read`),
/// not the standard `[CATEGORY]-[NUMBER]` validation rule format.
const RULE_CACHE_POISON: &str = "lint::cache-poison";

pub struct ImportsValidator;

const MAX_IMPORT_DEPTH: usize = 5;
type DiagnosticKey = (PathBuf, usize, usize, String, String);

fn push_unique_diagnostic(
    diagnostics: &mut Vec<Diagnostic>,
    seen_diagnostics: &mut HashSet<DiagnosticKey>,
    diagnostic: Diagnostic,
) {
    let key = (
        diagnostic.file.clone(),
        diagnostic.line,
        diagnostic.column,
        diagnostic.rule.clone(),
        diagnostic.message.clone(),
    );
    if seen_diagnostics.insert(key) {
        diagnostics.push(diagnostic);
    }
}

/// Check if a URL is a local file link (not external or anchor-only)
fn is_local_file_link(url: &str) -> bool {
    const EXTERNAL_PREFIXES: &[&str] = &[
        "http://", "https://", "mailto:", "tel:", "data:", "ftp://", "file://", "//",
    ];

    if EXTERNAL_PREFIXES.iter().any(|p| url.starts_with(p)) {
        return false;
    }

    !url.is_empty() && !url.starts_with('#')
}

/// Strip URL fragment (e.g., "file.md#section" -> "file.md")
fn strip_fragment(url: &str) -> &str {
    match url.find('#') {
        Some(idx) => &url[..idx],
        None => url,
    }
}

impl Validator for ImportsValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check both new category flag and legacy flag for backward compatibility
        if !config.rules().imports || !config.rules().import_references {
            return diagnostics;
        }

        // Detect root file type for cycle/depth rules
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let is_claude_md = matches!(filename, "CLAUDE.md" | "CLAUDE.local.md");

        let fs = config.fs();
        let project_root = resolve_project_root(path, config, fs.as_ref());
        let root_path = normalize_existing_path(path, fs.as_ref());

        // Use shared cache if available (project-level validation),
        // otherwise create a local cache (single-file validation)
        let shared_cache = config.get_import_cache();
        let mut local_cache: HashMap<PathBuf, Vec<Import>> = HashMap::new();
        let mut visited_depth: HashMap<PathBuf, usize> = HashMap::new();
        let mut stack = Vec::new();
        let mut seen_diagnostics: HashSet<DiagnosticKey> = HashSet::new();

        // Insert the root file's imports into the appropriate cache (if not already present)
        let root_imports = extract_imports(content);

        // REF-003: Duplicate @import detection
        if config.is_rule_enabled("REF-003") {
            let mut seen_paths: HashSet<String> = HashSet::new();
            for import in &root_imports {
                // Normalize: strip leading "./" for comparison
                let normalized = import
                    .path
                    .strip_prefix("./")
                    .unwrap_or(&import.path)
                    .to_string();
                if !seen_paths.insert(normalized) {
                    let mut diagnostic = Diagnostic::warning(
                        path.to_path_buf(),
                        import.line,
                        import.column,
                        "REF-003",
                        t!("rules.ref_003.message", path = import.path.as_str()),
                    )
                    .with_suggestion(t!("rules.ref_003.suggestion"));

                    if let Some((start, end)) = line_byte_range(content, import.line) {
                        diagnostic = diagnostic.with_fix(Fix::delete(
                            start,
                            end,
                            format!("Remove duplicate import '{}'", import.path),
                            false,
                        ));
                    }

                    diagnostics.push(diagnostic);
                }
            }
        }

        // REF-004: Non-markdown @import detection
        if config.is_rule_enabled("REF-004") {
            for import in &root_imports {
                let import_path = Path::new(&import.path);
                if let Some(ext) = import_path.extension().and_then(|e| e.to_str()) {
                    if !ext.eq_ignore_ascii_case("md") {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                import.line,
                                import.column,
                                "REF-004",
                                t!(
                                    "rules.ref_004.message",
                                    path = import.path.as_str(),
                                    ext = ext
                                ),
                            )
                            .with_suggestion(t!("rules.ref_004.suggestion")),
                        );
                    }
                }
                // Extensionless imports are allowed (might be directories)
            }
        }

        if let Some(cache) = shared_cache {
            // Write to shared cache only if not already present
            let mut guard = match cache.write() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    push_unique_diagnostic(
                        &mut diagnostics,
                        &mut seen_diagnostics,
                        Diagnostic::warning(
                            path.to_path_buf(),
                            1,
                            0,
                            RULE_CACHE_POISON,
                            t!("rules.cache_poison.message"),
                        )
                        .with_suggestion(t!("rules.cache_poison.suggestion")),
                    );
                    poisoned.into_inner()
                }
            };
            guard.entry(root_path.clone()).or_insert(root_imports);
        } else {
            // Write to local cache
            local_cache.entry(root_path.clone()).or_insert(root_imports);
        }

        visit_imports(
            &root_path,
            None,
            shared_cache,
            &mut local_cache,
            &mut visited_depth,
            &mut stack,
            &mut diagnostics,
            &mut seen_diagnostics,
            config,
            is_claude_md,
            &project_root,
            fs.as_ref(),
            path,
        );

        // Validate markdown links (REF-002)
        // Only check agent config files, not generic markdown. Generic markdown
        // files (plans, research notes, etc.) commonly have broken relative links
        // that are project documentation issues, not agent configuration problems.
        let is_agent_config = matches!(
            filename,
            "CLAUDE.md"
                | "CLAUDE.local.md"
                | "AGENTS.md"
                | "AGENTS.local.md"
                | "AGENTS.override.md"
                | "SKILL.md"
                | "GEMINI.md"
                | "GEMINI.local.md"
        ) || filename.ends_with(".instructions.md")
            || filename == "copilot-instructions.md";
        if is_agent_config {
            validate_markdown_links(path, content, config, &mut diagnostics, fs.as_ref());
        }

        diagnostics
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_imports(
    file_path: &PathBuf,
    content_override: Option<&str>,
    shared_cache: Option<&ImportCache>,
    local_cache: &mut HashMap<PathBuf, Vec<Import>>,
    visited_depth: &mut HashMap<PathBuf, usize>,
    stack: &mut Vec<PathBuf>,
    diagnostics: &mut Vec<Diagnostic>,
    seen_diagnostics: &mut HashSet<DiagnosticKey>,
    config: &LintConfig,
    root_is_claude_md: bool,
    project_root: &Path,
    fs: &dyn FileSystem,
    validation_root: &Path,
) {
    let depth = stack.len();
    if let Some(prev_depth) = visited_depth.get(file_path) {
        // Skip only when we have already visited this file at an equal or
        // shallower depth. If we discover a shallower path later, revisit it
        // so traversal can continue with the tighter depth budget.
        if *prev_depth <= depth {
            return;
        }
    }
    visited_depth.insert(file_path.clone(), depth);

    let imports = get_imports_for_file(
        file_path,
        content_override,
        shared_cache,
        local_cache,
        fs,
        diagnostics,
        seen_diagnostics,
        validation_root,
    );
    let Some(imports) = imports else { return };

    let base_dir = file_path.parent().unwrap_or(Path::new("."));
    let normalized_base = normalize_existing_path(base_dir, fs);
    let normalized_root = project_root;

    // Determine file type for current file to route its own diagnostics
    let filename = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let is_claude_md = matches!(filename, "CLAUDE.md" | "CLAUDE.local.md");

    // Check rules based on CURRENT file type for missing imports
    // Check rules based on ROOT file type for cycles/depth (applies to entire chain)
    let check_not_found = (is_claude_md && config.is_rule_enabled("CC-MEM-001"))
        || (!is_claude_md && config.is_rule_enabled("REF-001"));
    let check_cycle = root_is_claude_md && config.is_rule_enabled("CC-MEM-002");
    let check_depth = root_is_claude_md && config.is_rule_enabled("CC-MEM-003");

    if !(check_not_found || check_cycle || check_depth) {
        return;
    }

    let rule_not_found = if is_claude_md {
        "CC-MEM-001"
    } else {
        "REF-001"
    };
    let rule_cycle = "CC-MEM-002";
    let rule_depth = "CC-MEM-003";

    stack.push(file_path.clone());

    for import in imports {
        let resolved = resolve_import_path(&import.path, base_dir);

        // Validate path to prevent traversal attacks
        // Reject absolute paths and paths that escape the project root
        let raw_path = Path::new(&import.path);
        if raw_path.is_absolute()
            || import.path.starts_with('/')
            || import.path.starts_with('\\')
            || import.path.starts_with('~')
        {
            if check_not_found {
                push_unique_diagnostic(
                    diagnostics,
                    seen_diagnostics,
                    Diagnostic::error(
                        file_path.clone(),
                        import.line,
                        import.column,
                        rule_not_found,
                        t!("rules.cc_mem_001.absolute", path = import.path.as_str()),
                    )
                    .with_suggestion(t!("rules.cc_mem_001.absolute_suggestion")),
                );
            }
            continue;
        }

        let normalized_resolved = normalize_join(&normalized_base, &import.path);
        if !normalized_resolved.starts_with(normalized_root) {
            if check_not_found {
                push_unique_diagnostic(
                    diagnostics,
                    seen_diagnostics,
                    Diagnostic::error(
                        file_path.clone(),
                        import.line,
                        import.column,
                        rule_not_found,
                        t!("rules.cc_mem_001.escapes", path = import.path.as_str()),
                    )
                    .with_suggestion(t!("rules.cc_mem_001.escapes_suggestion")),
                );
            }
            continue;
        }

        let normalized = if fs.exists(&resolved) {
            let canonical_resolved = normalize_existing_path(&resolved, fs);
            if !canonical_resolved.starts_with(normalized_root) {
                if check_not_found {
                    push_unique_diagnostic(
                        diagnostics,
                        seen_diagnostics,
                        Diagnostic::error(
                            file_path.clone(),
                            import.line,
                            import.column,
                            rule_not_found,
                            t!("rules.cc_mem_001.escapes", path = import.path.as_str()),
                        )
                        .with_suggestion(t!("rules.cc_mem_001.escapes_suggestion")),
                    );
                }
                continue;
            }
            canonical_resolved
        } else {
            resolved
        };

        // Try file-relative resolution first, then project-root resolution.
        // Claude Code resolves @imports relative to the project root, not
        // the importing file's directory.
        let normalized = if fs.exists(&normalized) {
            normalized
        } else {
            // Fallback: try resolving relative to project root
            let root_resolved = project_root.join(&import.path);
            if fs.exists(&root_resolved) {
                root_resolved
            } else {
                normalized
            }
        };

        let import_exists = fs.exists(&normalized);

        if !import_exists {
            if check_not_found {
                push_unique_diagnostic(
                    diagnostics,
                    seen_diagnostics,
                    Diagnostic::error(
                        file_path.clone(),
                        import.line,
                        import.column,
                        rule_not_found,
                        t!("rules.cc_mem_001.not_found", path = import.path.as_str()),
                    )
                    .with_suggestion(format!(
                        "Check that the file exists: {}",
                        normalized.display()
                    )),
                );
            }
            continue;
        }

        // Always check for cycles/depth to prevent infinite recursion
        let has_cycle = stack.contains(&normalized);
        let exceeds_depth = depth + 1 > MAX_IMPORT_DEPTH;

        // Emit diagnostics if rules are enabled for this file type
        if check_cycle && has_cycle {
            let cycle = format_cycle(stack, &normalized);
            push_unique_diagnostic(
                diagnostics,
                seen_diagnostics,
                Diagnostic::error(
                    file_path.clone(),
                    import.line,
                    import.column,
                    rule_cycle,
                    t!("rules.cc_mem_002.message", chain = cycle),
                )
                .with_suggestion(t!("rules.cc_mem_002.suggestion")),
            );
            continue;
        }

        if check_depth && exceeds_depth {
            push_unique_diagnostic(
                diagnostics,
                seen_diagnostics,
                Diagnostic::error(
                    file_path.clone(),
                    import.line,
                    import.column,
                    rule_depth,
                    t!(
                        "rules.cc_mem_003.message",
                        depth = depth + 1,
                        max = MAX_IMPORT_DEPTH
                    ),
                )
                .with_suggestion(t!("rules.cc_mem_003.suggestion")),
            );
            continue;
        }

        // Only recurse if no cycle/depth issues
        if !has_cycle && !exceeds_depth {
            visit_imports(
                &normalized,
                None,
                shared_cache,
                local_cache,
                visited_depth,
                stack,
                diagnostics,
                seen_diagnostics,
                config,
                root_is_claude_md,
                project_root,
                fs,
                validation_root,
            );
        }
    }

    stack.pop();
}

/// Get imports for a file, using shared cache if available, otherwise local cache.
///
/// This function uses a read-then-write lock pattern for the shared cache:
/// 1. Try to read from cache (read lock)
/// 2. If miss, drop read lock, parse file, then write (write lock)
///
/// This avoids holding locks during file I/O and parsing.
///
/// Note: There's a small window for duplicate work where two threads could both
/// miss the cache and parse the same file. This is acceptable because:
/// - The extra work is bounded (only one extra parse per file per thread)
/// - Using entry() API prevents duplicate insertions
/// - Lock-free parsing enables better parallelism than holding locks during I/O
#[allow(clippy::too_many_arguments)]
fn get_imports_for_file(
    file_path: &Path,
    content_override: Option<&str>,
    shared_cache: Option<&ImportCache>,
    local_cache: &mut HashMap<PathBuf, Vec<Import>>,
    fs: &dyn FileSystem,
    diagnostics: &mut Vec<Diagnostic>,
    seen_diagnostics: &mut HashSet<DiagnosticKey>,
    validation_root: &Path,
) -> Option<Vec<Import>> {
    // Try shared cache first if available
    if let Some(cache) = shared_cache {
        // Read lock - check if already cached
        {
            let guard = match cache.read() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    push_unique_diagnostic(
                        diagnostics,
                        seen_diagnostics,
                        Diagnostic::warning(
                            validation_root.to_path_buf(),
                            1,
                            0,
                            RULE_CACHE_POISON,
                            t!("rules.cache_poison.message"),
                        )
                        .with_suggestion(t!("rules.cache_poison.suggestion")),
                    );
                    poisoned.into_inner()
                }
            };
            if let Some(imports) = guard.get(file_path) {
                return Some(imports.clone());
            }
        }
        // Cache miss - read lock dropped here before I/O

        // Parse the file outside of any lock
        let content = match content_override {
            Some(content) => content.to_string(),
            // Silently skip files that can't be read (symlinks, too large, missing).
            // This is intentional: import chains often reference optional/external files,
            // and failing noisily on each would overwhelm the user.
            None => fs.read_to_string(file_path).ok()?,
        };
        let imports = extract_imports(&content);

        // Write lock - use entry() to handle race condition where another thread
        // may have already inserted while we were parsing
        let mut guard = match cache.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                push_unique_diagnostic(
                    diagnostics,
                    seen_diagnostics,
                    Diagnostic::warning(
                        validation_root.to_path_buf(),
                        1,
                        0,
                        RULE_CACHE_POISON,
                        t!("rules.cache_poison.message"),
                    )
                    .with_suggestion(t!("rules.cache_poison.suggestion")),
                );
                poisoned.into_inner()
            }
        };
        guard
            .entry(file_path.to_path_buf())
            .or_insert_with(|| imports.clone());
        return Some(imports);
    }

    // Fallback to local cache (single-file validation)
    if !local_cache.contains_key(file_path) {
        let content = match content_override {
            Some(content) => content.to_string(),
            None => fs.read_to_string(file_path).ok()?,
        };
        let imports = extract_imports(&content);
        local_cache.insert(file_path.to_path_buf(), imports);
    }
    local_cache.get(file_path).cloned()
}

fn resolve_import_path(import_path: &str, base_dir: &Path) -> PathBuf {
    if import_path.starts_with("~/") || import_path.starts_with("~\\") {
        #[cfg(feature = "filesystem")]
        if let Some(home) = dirs::home_dir() {
            return home.join(&import_path[2..]);
        }
    }

    let raw = PathBuf::from(import_path);
    if raw.is_absolute() {
        raw
    } else {
        base_dir.join(raw)
    }
}

fn normalize_join(base_dir: &Path, import_path: &str) -> PathBuf {
    let mut result = PathBuf::from(base_dir);
    for component in Path::new(import_path).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(segment) => {
                result.push(segment);
            }
            Component::RootDir | Component::Prefix(_) => {
                result = PathBuf::from(component.as_os_str());
            }
        }
    }
    result
}

fn normalize_existing_path(path: &Path, fs: &dyn FileSystem) -> PathBuf {
    fs.canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn resolve_project_root(path: &Path, config: &LintConfig, fs: &dyn FileSystem) -> PathBuf {
    if let Some(root) = config.get_root_dir() {
        return normalize_existing_path(root, fs);
    }

    find_repo_root(path, fs).unwrap_or_else(|| {
        let fallback = path.parent().unwrap_or(Path::new("."));
        normalize_existing_path(fallback, fs)
    })
}

fn find_repo_root(path: &Path, fs: &dyn FileSystem) -> Option<PathBuf> {
    for ancestor in path.ancestors() {
        if ancestor.as_os_str().is_empty() {
            continue;
        }
        let git_marker = ancestor.join(".git");
        if fs.is_dir(&git_marker) || fs.is_file(&git_marker) {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

fn format_cycle(stack: &[PathBuf], target: &Path) -> String {
    let mut cycle = Vec::new();
    let mut in_cycle = false;
    for path in stack {
        if path == target {
            in_cycle = true;
        }
        if in_cycle {
            cycle.push(path.display().to_string());
        }
    }
    cycle.push(target.display().to_string());
    cycle.join(" -> ")
}

/// Validate markdown links in content (REF-002)
fn validate_markdown_links(
    path: &Path,
    content: &str,
    config: &LintConfig,
    diagnostics: &mut Vec<Diagnostic>,
    fs: &dyn FileSystem,
) {
    if !config.is_rule_enabled("REF-002") {
        return;
    }

    let links = extract_markdown_links(content);
    let base_dir = path.parent().unwrap_or(Path::new("."));

    // Precompute containment boundary and its canonical form once, outside the loop.
    // Both values depend only on config.root_dir() and base_dir, which are
    // loop-invariant, so this eliminates N-1 redundant canonicalize syscalls.
    let containment_dir = config
        .root_dir()
        .cloned()
        .unwrap_or_else(|| base_dir.to_path_buf());
    let canonical_base = fs.canonicalize(&containment_dir).ok();

    for link in links {
        // Skip non-local links (external URLs, anchors, etc.)
        if !is_local_file_link(&link.url) {
            continue;
        }

        // Skip template placeholders like {url}, {repoUrl}, {brackets}
        if link.url.starts_with('{') && link.url.ends_with('}') {
            continue;
        }

        // Skip single-word "links" that don't look like file paths
        // (no extension, no directory separator) - likely wiki-style links or examples
        if !link.url.contains('/')
            && !link.url.contains('\\')
            && !link.url.contains('.')
            && !link.url.contains('#')
        {
            continue;
        }

        // Strip fragment to get the file path
        let file_path = strip_fragment(&link.url);

        // Resolve the path relative to the file's directory
        let resolved = resolve_import_path(file_path, base_dir);

        // Security: Verify resolved path stays within project root
        if let Some(ref canonical_base) = canonical_base {
            if let Ok(canonical_resolved) = fs.canonicalize(&resolved) {
                if !canonical_resolved.starts_with(canonical_base) {
                    continue;
                }
            }
        }

        // Check if file exists
        if !fs.exists(&resolved) {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    link.line,
                    link.column,
                    "REF-002",
                    t!(
                        "rules.ref_002.message",
                        url = link.url.as_str(),
                        resolved = resolved.display().to_string()
                    ),
                )
                .with_suggestion(t!("rules.ref_002.suggestion")),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_config_disabled_imports_category() {
        let mut config = LintConfig::default();
        config.rules_mut().imports = false;

        let content = "@nonexistent-file.md";
        let validator = ImportsValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &config);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_legacy_import_references_flag() {
        let mut config = LintConfig::default();
        config.rules_mut().import_references = false;

        let content = "@nonexistent-file.md";
        let validator = ImportsValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &config);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_missing_import_in_claude_md() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("CLAUDE.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
    }

    #[test]
    fn test_cycle_detection_in_claude_md() {
        let temp = TempDir::new().unwrap();
        let a = temp.path().join("CLAUDE.md");
        let b = temp.path().join("b.md");
        fs::write(&a, "See @b.md").unwrap();
        fs::write(&b, "See @CLAUDE.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&a, "See @b.md", &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "CC-MEM-002"));
    }

    #[test]
    fn test_depth_exceeded_in_claude_md() {
        let temp = TempDir::new().unwrap();
        let claude_md = temp.path().join("CLAUDE.md");
        let paths: Vec<PathBuf> = (1..7)
            .map(|i| temp.path().join(format!("{}.md", i)))
            .collect();

        fs::write(&claude_md, "See @1.md").unwrap();
        for (i, path) in paths.iter().enumerate().take(5) {
            let content = format!("See @{}.md", i + 2);
            fs::write(path, content).unwrap();
        }
        fs::write(&paths[5], "End").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&claude_md, "See @1.md", &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "CC-MEM-003"));
    }

    #[test]
    fn test_missing_import_in_skill_md() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("SKILL.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "REF-001"));
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
    }

    #[test]
    fn test_missing_import_in_agents_md() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("AGENTS.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "REF-001"));
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
    }

    #[test]
    fn test_missing_import_in_generic_md() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("README.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &LintConfig::default());

        assert!(diagnostics.iter().any(|d| d.rule == "REF-001"));
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
    }

    #[test]
    fn test_cycle_in_skill_md() {
        let temp = TempDir::new().unwrap();
        let a = temp.path().join("SKILL.md");
        let b = temp.path().join("b.md");
        fs::write(&a, "See @b.md").unwrap();
        fs::write(&b, "See @SKILL.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&a, "See @b.md", &LintConfig::default());

        // Non-CLAUDE files don't check cycles, so no diagnostics expected
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_depth_exceeded_in_skill_md() {
        let temp = TempDir::new().unwrap();
        let skill_md = temp.path().join("SKILL.md");
        let paths: Vec<PathBuf> = (1..7)
            .map(|i| temp.path().join(format!("{}.md", i)))
            .collect();

        fs::write(&skill_md, "See @1.md").unwrap();
        for (i, path) in paths.iter().enumerate().take(5) {
            let content = format!("See @{}.md", i + 2);
            fs::write(path, content).unwrap();
        }
        fs::write(&paths[5], "End").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&skill_md, "See @1.md", &LintConfig::default());

        // Non-CLAUDE files don't check depth, so no diagnostics expected
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_ref_001_disabled_suppresses_skill_md_errors() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("SKILL.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let mut config = LintConfig::default();
        config
            .rules_mut()
            .disabled_rules
            .push("REF-001".to_string());

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &config);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_cc_mem_disabled_still_allows_ref() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("SKILL.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let mut config = LintConfig::default();
        config
            .rules_mut()
            .disabled_rules
            .push("CC-MEM-001".to_string());
        config
            .rules_mut()
            .disabled_rules
            .push("CC-MEM-002".to_string());
        config
            .rules_mut()
            .disabled_rules
            .push("CC-MEM-003".to_string());

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &config);

        assert!(diagnostics.iter().any(|d| d.rule == "REF-001"));
    }

    #[test]
    fn test_ref_disabled_still_allows_cc_mem() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("CLAUDE.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let mut config = LintConfig::default();
        config
            .rules_mut()
            .disabled_rules
            .push("REF-001".to_string());

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &config);

        // CLAUDE.md should still emit CC-MEM-001 even when REF-001 is disabled
        assert!(diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
    }

    #[test]
    fn test_nested_file_type_detection() {
        // Test for critical fix: file type should be determined per-file in recursion
        let temp = TempDir::new().unwrap();
        let skill_md = temp.path().join("SKILL.md");
        let claude_md = temp.path().join("CLAUDE.md");

        // SKILL.md imports CLAUDE.md which has a missing import
        fs::write(&skill_md, "See @CLAUDE.md").unwrap();
        fs::write(&claude_md, "See @missing.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&skill_md, "See @CLAUDE.md", &LintConfig::default());

        // CLAUDE.md's missing import should emit CC-MEM-001, not REF-001
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule == "CC-MEM-001" && d.file.ends_with("CLAUDE.md"))
        );
        assert!(
            !diagnostics
                .iter()
                .any(|d| d.rule == "REF-001" && d.file.ends_with("CLAUDE.md"))
        );
    }

    #[test]
    fn test_absolute_path_rejection() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("CLAUDE.md");
        fs::write(&file_path, "See @/etc/passwd").unwrap();

        let validator = ImportsValidator;
        let diagnostics =
            validator.validate(&file_path, "See @/etc/passwd", &LintConfig::default());

        // Absolute paths should be rejected
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("Absolute import paths not allowed"))
        );
    }

    #[test]
    fn test_path_escape_rejection() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("root");
        let docs = root.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(temp.path().join("outside.md"), "Outside content").unwrap();

        let file_path = docs.join("CLAUDE.md");
        fs::write(&file_path, "See @../../outside.md").unwrap();

        let mut config = LintConfig::default();
        config.set_root_dir(root);

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @../../outside.md", &config);

        assert!(
            diagnostics
                .iter()
                .any(|d| { d.rule == "CC-MEM-001" && d.message.contains("escapes project root") })
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_symlink_escape_rejection() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let root = temp.path().join("root");
        let docs = root.join("docs");
        let outside = temp.path().join("outside");

        fs::create_dir_all(&docs).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.md"), "Secret content").unwrap();

        let link_path = root.join("link");
        symlink(&outside, &link_path).unwrap();

        let file_path = docs.join("CLAUDE.md");
        fs::write(&file_path, "See @../link/secret.md").unwrap();

        let mut config = LintConfig::default();
        config.set_root_dir(root);

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @../link/secret.md", &config);

        assert!(
            diagnostics
                .iter()
                .any(|d| { d.rule == "CC-MEM-001" && d.message.contains("escapes project root") })
        );
    }

    // ===== Helper Function Tests =====

    #[test]
    fn test_is_local_file_link_true() {
        assert!(is_local_file_link("file.md"));
        assert!(is_local_file_link("docs/guide.md"));
        assert!(is_local_file_link("./relative.md"));
        assert!(is_local_file_link("../parent.md"));
        assert!(is_local_file_link("file.md#section"));
    }

    #[test]
    fn test_is_local_file_link_false() {
        assert!(!is_local_file_link("https://example.com"));
        assert!(!is_local_file_link("http://example.com"));
        assert!(!is_local_file_link("mailto:test@example.com"));
        assert!(!is_local_file_link("tel:+1234567890"));
        assert!(!is_local_file_link("data:text/plain,hello"));
        assert!(!is_local_file_link("ftp://files.example.com"));
        assert!(!is_local_file_link("//cdn.example.com/file.js"));
        assert!(!is_local_file_link("#section"));
        assert!(!is_local_file_link(""));
    }

    #[test]
    fn test_strip_fragment() {
        assert_eq!(strip_fragment("file.md#section"), "file.md");
        assert_eq!(strip_fragment("file.md"), "file.md");
        assert_eq!(strip_fragment("#section"), "");
        assert_eq!(strip_fragment("docs/guide.md#heading"), "docs/guide.md");
    }

    // ===== REF-001 Tests =====

    #[test]
    fn test_ref_001_missing_import() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &LintConfig::default());

        // Non-CLAUDE.md files emit REF-001 only (not CC-MEM-001)
        assert!(diagnostics.iter().any(|d| d.rule == "REF-001"));
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
    }

    #[test]
    fn test_ref_001_existing_import() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("exists.md");
        let file_path = temp.path().join("test.md");
        fs::write(&target, "Target content").unwrap();
        fs::write(&file_path, "See @exists.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @exists.md", &LintConfig::default());

        // Should not emit any not-found errors
        assert!(!diagnostics.iter().any(|d| d.rule == "REF-001"));
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-MEM-001"));
    }

    #[test]
    fn test_ref_001_disabled() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["REF-001".to_string()];

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &config);

        // Non-CLAUDE.md with REF-001 disabled emits nothing
        assert!(diagnostics.is_empty());
    }

    // ===== REF-002 Tests =====

    #[test]
    fn test_ref_002_broken_link() {
        let temp = TempDir::new().unwrap();
        // REF-002 only fires on agent config files, not generic markdown
        let file_path = temp.path().join("CLAUDE.md");
        fs::write(&file_path, "See [guide](missing.md) for more.").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(
            &file_path,
            "See [guide](missing.md) for more.",
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "REF-002"));
        let ref_002 = diagnostics.iter().find(|d| d.rule == "REF-002").unwrap();
        assert!(ref_002.message.contains("Broken markdown link"));
    }

    #[test]
    fn test_ref_002_valid_link() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("exists.md");
        let file_path = temp.path().join("test.md");
        fs::write(&target, "Target content").unwrap();
        fs::write(&file_path, "See [guide](exists.md) for more.").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(
            &file_path,
            "See [guide](exists.md) for more.",
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "REF-002"));
    }

    #[test]
    fn test_ref_002_skips_external_links() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        let content = "See [GitHub](https://github.com) and [mail](mailto:test@example.com).";
        fs::write(&file_path, content).unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, content, &LintConfig::default());

        // External links should not trigger REF-002
        assert!(!diagnostics.iter().any(|d| d.rule == "REF-002"));
    }

    #[test]
    fn test_ref_002_skips_anchor_links() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        let content = "See [section](#section-name) for more.";
        fs::write(&file_path, content).unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, content, &LintConfig::default());

        // Pure anchor links should not trigger REF-002
        assert!(!diagnostics.iter().any(|d| d.rule == "REF-002"));
    }

    #[test]
    fn test_ref_002_link_with_fragment() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("exists.md");
        let file_path = temp.path().join("test.md");
        fs::write(&target, "# Section\nContent").unwrap();
        fs::write(&file_path, "See [section](exists.md#section) for more.").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(
            &file_path,
            "See [section](exists.md#section) for more.",
            &LintConfig::default(),
        );

        // File exists, fragment validation is not implemented - no error
        assert!(!diagnostics.iter().any(|d| d.rule == "REF-002"));
    }

    #[test]
    fn test_ref_002_missing_file_with_fragment() {
        let temp = TempDir::new().unwrap();
        // REF-002 only fires on agent config files, not generic markdown
        let file_path = temp.path().join("CLAUDE.md");
        fs::write(&file_path, "See [section](missing.md#section) for more.").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(
            &file_path,
            "See [section](missing.md#section) for more.",
            &LintConfig::default(),
        );

        // File doesn't exist, should error
        assert!(diagnostics.iter().any(|d| d.rule == "REF-002"));
    }

    #[test]
    fn test_ref_002_broken_image() {
        let temp = TempDir::new().unwrap();
        // REF-002 only fires on agent config files, not generic markdown
        let file_path = temp.path().join("CLAUDE.md");
        fs::write(&file_path, "![logo](images/logo.png)").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(
            &file_path,
            "![logo](images/logo.png)",
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "REF-002"));
        let ref_002 = diagnostics.iter().find(|d| d.rule == "REF-002").unwrap();
        assert!(ref_002.message.contains("Broken markdown link"));
    }

    #[test]
    fn test_ref_002_disabled() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "See [guide](missing.md) for more.").unwrap();

        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["REF-002".to_string()];

        let validator = ImportsValidator;
        let diagnostics =
            validator.validate(&file_path, "See [guide](missing.md) for more.", &config);

        assert!(!diagnostics.iter().any(|d| d.rule == "REF-002"));
    }

    #[test]
    fn test_ref_002_imports_category_disabled() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "See [guide](missing.md) for more.").unwrap();

        let mut config = LintConfig::default();
        config.rules_mut().imports = false;

        let validator = ImportsValidator;
        let diagnostics =
            validator.validate(&file_path, "See [guide](missing.md) for more.", &config);

        // When imports category is disabled, no validation happens
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_ref_002_relative_path() {
        let temp = TempDir::new().unwrap();
        let subdir = temp.path().join("docs");
        fs::create_dir(&subdir).unwrap();
        let target = subdir.join("guide.md");
        let file_path = temp.path().join("test.md");
        fs::write(&target, "Guide content").unwrap();
        fs::write(&file_path, "See [guide](docs/guide.md) for more.").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(
            &file_path,
            "See [guide](docs/guide.md) for more.",
            &LintConfig::default(),
        );

        // Relative path should resolve correctly
        assert!(!diagnostics.iter().any(|d| d.rule == "REF-002"));
    }

    #[test]
    fn test_ref_002_path_traversal_blocked_by_root_dir() {
        let temp = TempDir::new().unwrap();

        // Create directory structure: temp/sub/CLAUDE.md and temp/outside.md
        let sub = temp.path().join("sub");
        fs::create_dir(&sub).unwrap();
        let outside = temp.path().join("outside.md");
        fs::write(&outside, "Outside content").unwrap();

        let file_path = sub.join("CLAUDE.md");
        let content =
            "See [escape](../outside.md) for more.\nSee [missing](nonexistent.md) for more.";
        fs::write(&file_path, content).unwrap();

        // Set root_dir to sub/ so ../outside.md escapes the boundary
        let mut config = LintConfig::default();
        config.set_root_dir(sub.clone());

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, content, &config);

        // The ../outside.md link should be silently skipped (path traversal blocked)
        // but nonexistent.md should still produce REF-002 (within root, but missing)
        let ref_002_diags: Vec<_> = diagnostics.iter().filter(|d| d.rule == "REF-002").collect();
        assert_eq!(
            ref_002_diags.len(),
            1,
            "Expected exactly 1 REF-002 diagnostic, but found {}: {:?}",
            ref_002_diags.len(),
            ref_002_diags
        );
        assert!(ref_002_diags[0].message.contains("nonexistent.md"));
        assert!(
            !ref_002_diags
                .iter()
                .any(|d| d.message.contains("outside.md")),
            "outside.md should be silently skipped, not reported"
        );
    }

    #[test]
    fn test_ref_002_nonexistent_root_dir_skips_traversal_check() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("CLAUDE.md");
        let content = "See [missing](nonexistent.md) for more.";
        fs::write(&file_path, content).unwrap();

        // Set root_dir to a path that does not exist - canonical_base will be None
        let mut config = LintConfig::default();
        config.set_root_dir(PathBuf::from("/nonexistent/root/path"));

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, content, &config);

        // Traversal check is skipped (canonical_base is None), but existence check still runs
        assert!(
            diagnostics.iter().any(|d| d.rule == "REF-002"),
            "Expected at least one REF-002 diagnostic, but found none in: {:?}",
            diagnostics
        );
    }

    // ===== Shared Import Cache Tests =====

    #[test]
    fn test_single_file_validation_works_without_shared_cache() {
        // Single-file validation should work without a shared cache
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target.md");
        let file_path = temp.path().join("test.md");
        fs::write(&target, "Target content").unwrap();
        fs::write(&file_path, "See @target.md").unwrap();

        let config = LintConfig::default();
        // No shared cache set - should use local cache
        assert!(config.get_import_cache().is_none());

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @target.md", &config);

        // Should succeed with no errors
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_shared_cache_is_populated_after_first_parse() {
        use std::collections::HashMap;
        use std::sync::{Arc, RwLock};

        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target.md");
        let file_path = temp.path().join("test.md");
        fs::write(&target, "Target has @nested.md import").unwrap();
        fs::write(temp.path().join("nested.md"), "Nested content").unwrap();
        fs::write(&file_path, "See @target.md").unwrap();

        // Create shared cache
        let cache: crate::parsers::ImportCache = Arc::new(RwLock::new(HashMap::new()));

        let mut config = LintConfig::default();
        config.set_import_cache(cache.clone());

        // Verify cache is empty before validation
        {
            let guard = cache.read().unwrap();
            assert!(guard.is_empty());
        }

        let validator = ImportsValidator;
        let _ = validator.validate(&file_path, "See @target.md", &config);

        // Verify cache is populated after validation
        {
            let guard = cache.read().unwrap();
            // Should have at least the root file and target file
            assert!(
                guard.len() >= 2,
                "Expected at least 2 entries, got {}",
                guard.len()
            );
        }
    }

    #[test]
    fn test_shared_cache_second_access_uses_cached_result() {
        use std::collections::HashMap;
        use std::sync::{Arc, RwLock};

        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target.md");
        let file_path = temp.path().join("test.md");
        fs::write(&target, "Target content").unwrap();
        fs::write(&file_path, "See @target.md").unwrap();

        // Create shared cache
        let cache: crate::parsers::ImportCache = Arc::new(RwLock::new(HashMap::new()));

        let mut config = LintConfig::default();
        config.set_import_cache(cache.clone());

        let validator = ImportsValidator;

        // First validation - populates cache
        let _ = validator.validate(&file_path, "See @target.md", &config);
        let cache_size_after_first;
        {
            let guard = cache.read().unwrap();
            cache_size_after_first = guard.len();
        }

        // Second validation - should use cached results
        let _ = validator.validate(&file_path, "See @target.md", &config);
        let cache_size_after_second;
        {
            let guard = cache.read().unwrap();
            cache_size_after_second = guard.len();
        }

        // Cache size should be the same (entries reused, not duplicated)
        assert_eq!(cache_size_after_first, cache_size_after_second);
    }

    #[test]
    fn test_shared_cache_concurrent_access() {
        use std::collections::HashMap;
        use std::sync::{Arc, RwLock};
        use std::thread;

        let temp = TempDir::new().unwrap();

        // Create multiple files that reference each other
        for i in 0..5 {
            let content = if i < 4 {
                format!("Content with @file{}.md import", i + 1)
            } else {
                "End of chain".to_string()
            };
            fs::write(temp.path().join(format!("file{}.md", i)), content).unwrap();
        }

        // Create shared cache
        let cache: crate::parsers::ImportCache = Arc::new(RwLock::new(HashMap::new()));

        // Spawn multiple threads that validate different files with the same cache
        let handles: Vec<_> = (0..5)
            .map(|i| {
                let cache = cache.clone();
                let temp_path = temp.path().to_path_buf();
                thread::spawn(move || {
                    let mut config = LintConfig::default();
                    config.set_import_cache(cache);

                    let file_path = temp_path.join(format!("file{}.md", i));
                    let content = fs::read_to_string(&file_path).unwrap();

                    let validator = ImportsValidator;
                    validator.validate(&file_path, &content, &config)
                })
            })
            .collect();

        // All threads should complete without panic (no deadlock)
        for handle in handles {
            let result = handle.join();
            assert!(result.is_ok(), "Thread should complete without panic");
        }

        // Cache should have entries
        {
            let guard = cache.read().unwrap();
            assert!(
                !guard.is_empty(),
                "Cache should have entries after concurrent access"
            );
        }
    }

    #[test]
    fn test_shared_cache_poisoned_lock_does_not_panic() {
        use std::collections::HashMap;
        use std::sync::{Arc, RwLock};
        use std::thread;

        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target.md");
        let file_path = temp.path().join("test.md");
        fs::write(&target, "Target content").unwrap();
        fs::write(&file_path, "See @target.md").unwrap();

        let cache: crate::parsers::ImportCache = Arc::new(RwLock::new(HashMap::new()));

        let cache_for_poison = cache.clone();
        let _ = thread::spawn(move || {
            let _guard = cache_for_poison.write().unwrap();
            panic!("poison import cache lock");
        })
        .join();
        assert!(cache.read().is_err(), "Cache lock should be poisoned");

        let mut config = LintConfig::default();
        config.set_import_cache(cache);

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @target.md", &config);
        let poison_diag = diagnostics.iter().find(|d| d.rule == RULE_CACHE_POISON);
        assert!(
            poison_diag.is_some(),
            "Expected lint::cache-poison warning in diagnostics"
        );
        let d = poison_diag.unwrap();
        assert_eq!(
            d.level,
            DiagnosticLevel::Warning,
            "lint::cache-poison should be a Warning, not {:?}",
            d.level
        );
        assert!(
            d.suggestion.is_some(),
            "lint::cache-poison diagnostic should include a suggestion"
        );
    }

    #[test]
    fn test_shared_cache_poisoned_lock_still_reports_missing_import() {
        use std::collections::HashMap;
        use std::sync::{Arc, RwLock};
        use std::thread;

        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "See @missing.md").unwrap();

        let cache: crate::parsers::ImportCache = Arc::new(RwLock::new(HashMap::new()));

        let cache_for_poison = cache.clone();
        let _ = thread::spawn(move || {
            let _guard = cache_for_poison.write().unwrap();
            panic!("poison import cache lock");
        })
        .join();
        assert!(cache.read().is_err(), "Cache lock should be poisoned");

        let mut config = LintConfig::default();
        config.set_import_cache(cache);

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @missing.md", &config);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule == "REF-001" && d.message.contains("@missing.md")),
            "Validation should still report missing imports with a poisoned shared cache lock"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == RULE_CACHE_POISON),
            "Expected lint::cache-poison warning alongside REF-001"
        );
    }

    #[test]
    fn test_shared_cache_poisoned_lock_warning_is_deduplicated() {
        use std::collections::HashMap;
        use std::sync::{Arc, RwLock};
        use std::thread;

        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        // Multiple imports so the validator hits the poisoned lock multiple times
        fs::write(&file_path, "See @a.md and @b.md and @c.md").unwrap();
        fs::write(temp.path().join("a.md"), "A content").unwrap();
        fs::write(temp.path().join("b.md"), "B content").unwrap();
        fs::write(temp.path().join("c.md"), "C content").unwrap();

        let cache: crate::parsers::ImportCache = Arc::new(RwLock::new(HashMap::new()));

        let cache_for_poison = cache.clone();
        let _ = thread::spawn(move || {
            let _guard = cache_for_poison.write().unwrap();
            panic!("poison import cache lock");
        })
        .join();
        assert!(cache.read().is_err(), "Cache lock should be poisoned");

        let mut config = LintConfig::default();
        config.set_import_cache(cache);

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "See @a.md and @b.md and @c.md", &config);

        let poison_count = diagnostics
            .iter()
            .filter(|d| d.rule == RULE_CACHE_POISON)
            .count();
        assert_eq!(
            poison_count, 1,
            "Expected exactly 1 lint::cache-poison diagnostic (deduplication), got {}",
            poison_count
        );
    }

    #[test]
    fn test_shared_cache_poisoned_lock_deduplication_across_recursive_tree() {
        // Verifies deduplication holds across recursive import traversal:
        // root.md -> a.md -> b.md (nested chain, not just siblings)
        use std::collections::HashMap;
        use std::sync::{Arc, RwLock};
        use std::thread;

        let temp = TempDir::new().unwrap();
        let root = temp.path().join("root.md");
        let a = temp.path().join("a.md");
        let b = temp.path().join("b.md");
        fs::write(&root, "See @a.md").unwrap();
        fs::write(&a, "See @b.md").unwrap();
        fs::write(&b, "B content").unwrap();

        let cache: crate::parsers::ImportCache = Arc::new(RwLock::new(HashMap::new()));

        let cache_for_poison = cache.clone();
        let _ = thread::spawn(move || {
            let _guard = cache_for_poison.write().unwrap();
            panic!("poison import cache lock");
        })
        .join();
        assert!(cache.read().is_err(), "Cache lock should be poisoned");

        let mut config = LintConfig::default();
        config.set_import_cache(cache);

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&root, "See @a.md", &config);

        let poison_count = diagnostics
            .iter()
            .filter(|d| d.rule == RULE_CACHE_POISON)
            .count();
        assert_eq!(
            poison_count, 1,
            "Expected exactly 1 lint::cache-poison across recursive traversal, got {}",
            poison_count
        );
    }

    #[test]
    fn test_revisits_file_when_later_seen_at_shallower_depth() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("CLAUDE.md");
        let a = temp.path().join("a.md");
        let b = temp.path().join("b.md");
        let c = temp.path().join("c.md");
        let d = temp.path().join("d.md");
        let shared = temp.path().join("shared.md");
        let leaf = temp.path().join("leaf.md");

        fs::write(&root, "@a.md\n@shared.md").unwrap();
        fs::write(&a, "@b.md").unwrap();
        fs::write(&b, "@c.md").unwrap();
        fs::write(&c, "@d.md").unwrap();
        fs::write(&d, "@shared.md").unwrap();
        fs::write(&shared, "@leaf.md").unwrap();
        fs::write(&leaf, "@missing.md").unwrap();

        let mut config = LintConfig::default();
        config.set_root_dir(temp.path().to_path_buf());

        let validator = ImportsValidator;
        let content = fs::read_to_string(&root).unwrap();
        let diagnostics = validator.validate(&root, &content, &config);

        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule == "REF-001" && d.message.contains("@missing.md")),
            "Traversal should revisit shared.md at shallower depth and report downstream missing imports"
        );
    }

    #[test]
    fn test_shallower_revisit_does_not_duplicate_missing_import_diagnostics() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("CLAUDE.md");
        let a = temp.path().join("a.md");
        let b = temp.path().join("b.md");
        let shared = temp.path().join("shared.md");

        fs::write(&root, "@a.md\n@shared.md").unwrap();
        fs::write(&a, "@b.md").unwrap();
        fs::write(&b, "@shared.md").unwrap();
        fs::write(&shared, "@missing.md").unwrap();

        let mut config = LintConfig::default();
        config.set_root_dir(temp.path().to_path_buf());

        let validator = ImportsValidator;
        let content = fs::read_to_string(&root).unwrap();
        let diagnostics = validator.validate(&root, &content, &config);

        let missing: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "REF-001" && d.message.contains("@missing.md"))
            .collect();
        assert_eq!(
            missing.len(),
            1,
            "Expected a single REF-001 diagnostic for @missing.md, got {}",
            missing.len()
        );
    }

    // ===== REF-003: Duplicate @import =====

    #[test]
    fn test_ref_003_duplicate_import() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target.md");
        let file_path = temp.path().join("test.md");
        fs::write(&target, "Target content").unwrap();
        fs::write(&file_path, "@target.md\n@target.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics =
            validator.validate(&file_path, "@target.md\n@target.md", &LintConfig::default());

        let ref_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "REF-003").collect();
        assert_eq!(ref_003.len(), 1, "Should detect one duplicate import");
        assert!(ref_003[0].message.contains("target.md"));
    }

    #[test]
    fn test_ref_003_has_fix() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target.md");
        let file_path = temp.path().join("test.md");
        fs::write(&target, "Target content").unwrap();
        fs::write(&file_path, "@target.md\n@target.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics =
            validator.validate(&file_path, "@target.md\n@target.md", &LintConfig::default());

        let ref_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "REF-003").collect();
        assert_eq!(ref_003.len(), 1);
        assert!(
            ref_003[0].has_fixes(),
            "REF-003 should have auto-fix to delete duplicate import"
        );
        let fix = &ref_003[0].fixes[0];
        assert!(!fix.safe, "REF-003 fix should be unsafe");
        // The fix should be a deletion (replacement is empty)
        assert!(
            fix.replacement.is_empty(),
            "Fix should delete the duplicate line"
        );
    }

    #[test]
    fn test_ref_003_no_duplicate() {
        let temp = TempDir::new().unwrap();
        let a = temp.path().join("a.md");
        let b = temp.path().join("b.md");
        let file_path = temp.path().join("test.md");
        fs::write(&a, "A content").unwrap();
        fs::write(&b, "B content").unwrap();
        fs::write(&file_path, "@a.md\n@b.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "@a.md\n@b.md", &LintConfig::default());

        let ref_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "REF-003").collect();
        assert!(ref_003.is_empty(), "No duplicate imports");
    }

    #[test]
    fn test_ref_003_normalized_paths() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target.md");
        let file_path = temp.path().join("test.md");
        fs::write(&target, "Target content").unwrap();
        fs::write(&file_path, "@target.md\n@./target.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(
            &file_path,
            "@target.md\n@./target.md",
            &LintConfig::default(),
        );

        let ref_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "REF-003").collect();
        assert_eq!(
            ref_003.len(),
            1,
            "Should detect ./target.md as duplicate of target.md"
        );
    }

    #[test]
    fn test_ref_003_disabled() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target.md");
        let file_path = temp.path().join("test.md");
        fs::write(&target, "Target content").unwrap();
        fs::write(&file_path, "@target.md\n@target.md").unwrap();

        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["REF-003".to_string()];

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "@target.md\n@target.md", &config);

        let ref_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "REF-003").collect();
        assert!(ref_003.is_empty(), "REF-003 should be disabled");
    }

    // ===== REF-004: Non-Markdown @import =====

    #[test]
    fn test_ref_004_json_import() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "@config.json").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "@config.json", &LintConfig::default());

        let ref_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "REF-004").collect();
        assert_eq!(ref_004.len(), 1, "Should detect non-markdown import");
        assert!(ref_004[0].message.contains("config.json"));
        assert!(ref_004[0].message.contains("json"));
    }

    #[test]
    fn test_ref_004_python_import() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "@scripts/deploy.py").unwrap();

        let validator = ImportsValidator;
        let diagnostics =
            validator.validate(&file_path, "@scripts/deploy.py", &LintConfig::default());

        let ref_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "REF-004").collect();
        assert_eq!(ref_004.len(), 1);
    }

    #[test]
    fn test_ref_004_markdown_import_ok() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("guide.md");
        let file_path = temp.path().join("test.md");
        fs::write(&target, "Guide content").unwrap();
        fs::write(&file_path, "@guide.md").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "@guide.md", &LintConfig::default());

        let ref_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "REF-004").collect();
        assert!(ref_004.is_empty(), "Markdown imports should be OK");
    }

    #[test]
    fn test_ref_004_multiple_non_markdown() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "@config.json\n@script.py\n@utils.ts").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(
            &file_path,
            "@config.json\n@script.py\n@utils.ts",
            &LintConfig::default(),
        );

        let ref_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "REF-004").collect();
        assert_eq!(ref_004.len(), 3);
    }

    #[test]
    fn test_ref_004_disabled() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "@config.json").unwrap();

        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["REF-004".to_string()];

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "@config.json", &config);

        let ref_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "REF-004").collect();
        assert!(ref_004.is_empty(), "REF-004 should be disabled");
    }

    #[test]
    fn test_ref_004_extensionless_import_ok() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "@utils").unwrap();

        let validator = ImportsValidator;
        let diagnostics = validator.validate(&file_path, "@utils", &LintConfig::default());

        let ref_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "REF-004").collect();
        assert!(
            ref_004.is_empty(),
            "Extensionless imports should not trigger REF-004"
        );
    }

    #[test]
    fn test_cycle_detection_three_file_chain() {
        let temp = TempDir::new().unwrap();
        let claude = temp.path().join("CLAUDE.md");
        let b = temp.path().join("b.md");
        let c = temp.path().join("c.md");

        fs::write(&claude, "@b.md").unwrap();
        fs::write(&b, "@c.md").unwrap();
        fs::write(&c, "@CLAUDE.md").unwrap();

        let validator = ImportsValidator;
        let content = fs::read_to_string(&claude).unwrap();
        let diagnostics = validator.validate(&claude, &content, &LintConfig::default());

        let cycle_diag = diagnostics.iter().find(|d| d.rule == "CC-MEM-002");
        assert!(
            cycle_diag.is_some(),
            "Three-file cycle (CLAUDE.md -> b.md -> c.md -> CLAUDE.md) should trigger CC-MEM-002"
        );
        let msg = &cycle_diag.unwrap().message;
        assert!(
            msg.contains("b.md") && msg.contains("c.md"),
            "CC-MEM-002 message should contain the full cycle path, got: {msg}"
        );
    }

    #[test]
    fn test_cycle_detection_four_file_chain() {
        let temp = TempDir::new().unwrap();
        let claude = temp.path().join("CLAUDE.md");
        let b = temp.path().join("b.md");
        let c = temp.path().join("c.md");
        let d = temp.path().join("d.md");

        fs::write(&claude, "@b.md").unwrap();
        fs::write(&b, "@c.md").unwrap();
        fs::write(&c, "@d.md").unwrap();
        fs::write(&d, "@CLAUDE.md").unwrap();

        let validator = ImportsValidator;
        let content = fs::read_to_string(&claude).unwrap();
        let diagnostics = validator.validate(&claude, &content, &LintConfig::default());

        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-MEM-002"),
            "Four-file cycle should trigger CC-MEM-002"
        );
        assert!(
            !diagnostics.iter().any(|d| d.rule == "CC-MEM-003"),
            "Four-file cycle should not trigger CC-MEM-003 (cycle detection short-circuits traversal before depth is evaluated)"
        );
    }

    #[test]
    fn test_depth_below_boundary_no_trigger() {
        // 5 files, 4 hops deep. MAX_IMPORT_DEPTH = 5, check is depth+1 > 5.
        // Deepest point: depth=4, check 4+1=5 > 5 is false. Should not trigger CC-MEM-003.
        let temp = TempDir::new().unwrap();
        let claude = temp.path().join("CLAUDE.md");
        let a = temp.path().join("a.md");
        let b = temp.path().join("b.md");
        let c = temp.path().join("c.md");
        let leaf = temp.path().join("leaf.md");

        fs::write(&claude, "@a.md").unwrap();
        fs::write(&a, "@b.md").unwrap();
        fs::write(&b, "@c.md").unwrap();
        fs::write(&c, "@leaf.md").unwrap();
        fs::write(&leaf, "End of chain").unwrap();

        let validator = ImportsValidator;
        let content = fs::read_to_string(&claude).unwrap();
        let diagnostics = validator.validate(&claude, &content, &LintConfig::default());

        assert!(
            !diagnostics.iter().any(|d| d.rule == "CC-MEM-003"),
            "Chain of 5 files (4 imports deep) should not trigger CC-MEM-003"
        );
        assert!(
            !diagnostics.iter().any(|d| d.rule == "CC-MEM-002"),
            "Linear chain with no cycle should not trigger CC-MEM-002"
        );
    }

    #[test]
    fn test_depth_at_boundary_no_trigger() {
        // 6 files in a linear chain: CLAUDE -> a -> b -> c -> d -> leaf.
        // There are 5 import hops, so the deepest file (leaf) is at depth=5.
        // With MAX_IMPORT_DEPTH = 5 and the check `depth + 1 > MAX_IMPORT_DEPTH`:
        // - At depth=4 (file d), recursing into leaf uses 4+1=5 > 5 (false), so it is allowed.
        // - If leaf tried to import another file, that recursion would use 5+1=6 > 5 (true),
        //   and CC-MEM-003 would trigger.
        // This test verifies the boundary: a chain that reaches depth 5 is allowed as long as
        // the leaf file has no further imports, so CC-MEM-003 should NOT fire.
        let temp = TempDir::new().unwrap();
        let claude = temp.path().join("CLAUDE.md");
        let a = temp.path().join("a.md");
        let b = temp.path().join("b.md");
        let c = temp.path().join("c.md");
        let d = temp.path().join("d.md");
        let leaf = temp.path().join("leaf.md");

        fs::write(&claude, "@a.md").unwrap();
        fs::write(&a, "@b.md").unwrap();
        fs::write(&b, "@c.md").unwrap();
        fs::write(&c, "@d.md").unwrap();
        fs::write(&d, "@leaf.md").unwrap();
        fs::write(&leaf, "End of chain").unwrap();

        let validator = ImportsValidator;
        let content = fs::read_to_string(&claude).unwrap();
        let diagnostics = validator.validate(&claude, &content, &LintConfig::default());

        assert!(
            !diagnostics.iter().any(|d| d.rule == "CC-MEM-003"),
            "Chain of 6 files (depth reaches MAX_IMPORT_DEPTH=5) should not trigger CC-MEM-003"
        );
        assert!(
            !diagnostics.iter().any(|d| d.rule == "CC-MEM-002"),
            "Linear chain with no cycle should not trigger CC-MEM-002"
        );
    }

    #[test]
    fn test_concurrent_cycle_detection_no_deadlock() {
        use std::collections::HashMap;
        use std::sync::{Arc, RwLock, mpsc};
        use std::thread;
        use std::time::Duration;

        let temp = TempDir::new().unwrap();
        let claude = temp.path().join("CLAUDE.md");
        let b = temp.path().join("b.md");

        fs::write(&claude, "@b.md").unwrap();
        fs::write(&b, "@CLAUDE.md").unwrap();

        let cache: crate::parsers::ImportCache = Arc::new(RwLock::new(HashMap::new()));
        let (tx, rx) = mpsc::channel();

        for _ in 0..8 {
            let cache = cache.clone();
            let path = claude.clone();
            let content = fs::read_to_string(&path).unwrap();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut config = LintConfig::default();
                config.set_import_cache(cache);
                config.set_root_dir(path.parent().unwrap().to_path_buf());
                let validator = ImportsValidator;
                let result = validator.validate(&path, &content, &config);
                tx.send(result).ok();
            });
        }
        drop(tx);

        for _ in 0..8 {
            let diagnostics = rx
                .recv_timeout(Duration::from_secs(10))
                .expect("Thread did not complete within 10s (possible deadlock)");
            assert!(
                diagnostics.iter().any(|d| d.rule == "CC-MEM-002"),
                "Each thread should detect CC-MEM-002 in two-file cycle"
            );
        }
    }

    #[test]
    fn test_concurrent_three_file_cycle_shared_cache() {
        use std::collections::HashMap;
        use std::sync::{Arc, RwLock};
        use std::thread;

        let temp = TempDir::new().unwrap();
        let claude = temp.path().join("CLAUDE.md");
        let b = temp.path().join("b.md");
        let c = temp.path().join("c.md");

        fs::write(&claude, "@b.md").unwrap();
        fs::write(&b, "@c.md").unwrap();
        fs::write(&c, "@CLAUDE.md").unwrap();

        let cache: crate::parsers::ImportCache = Arc::new(RwLock::new(HashMap::new()));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let cache = cache.clone();
                let path = claude.clone();
                let content = fs::read_to_string(&path).unwrap();
                thread::spawn(move || {
                    let mut config = LintConfig::default();
                    config.set_import_cache(cache);
                    config.set_root_dir(path.parent().unwrap().to_path_buf());
                    let validator = ImportsValidator;
                    validator.validate(&path, &content, &config)
                })
            })
            .collect();

        for handle in handles {
            let diagnostics = handle
                .join()
                .expect("Thread panicked during three-file cycle detection");
            assert!(
                diagnostics.iter().any(|d| d.rule == "CC-MEM-002"),
                "Each thread should detect CC-MEM-002 in three-file cycle"
            );
        }

        let guard = cache.read().unwrap();
        assert!(
            guard.len() >= 3,
            "Shared cache should contain entries for all 3 files, found {}",
            guard.len()
        );
    }

    #[test]
    fn test_concurrent_cycle_near_depth_limit() {
        use std::collections::HashMap;
        use std::sync::{Arc, RwLock};
        use std::thread;

        let temp = TempDir::new().unwrap();
        let claude = temp.path().join("CLAUDE.md");
        let a = temp.path().join("a.md");
        let b = temp.path().join("b.md");
        let c = temp.path().join("c.md");
        let d = temp.path().join("d.md");

        fs::write(&claude, "@a.md").unwrap();
        fs::write(&a, "@b.md").unwrap();
        fs::write(&b, "@c.md").unwrap();
        fs::write(&c, "@d.md").unwrap();
        fs::write(&d, "@CLAUDE.md").unwrap();

        let cache: crate::parsers::ImportCache = Arc::new(RwLock::new(HashMap::new()));

        let handles: Vec<_> = (0..5)
            .map(|_| {
                let cache = cache.clone();
                let path = claude.clone();
                let content = fs::read_to_string(&path).unwrap();
                thread::spawn(move || {
                    let mut config = LintConfig::default();
                    config.set_import_cache(cache);
                    config.set_root_dir(path.parent().unwrap().to_path_buf());
                    let validator = ImportsValidator;
                    validator.validate(&path, &content, &config)
                })
            })
            .collect();

        for handle in handles {
            let diagnostics = handle
                .join()
                .expect("Thread panicked during near-depth-limit cycle detection");
            assert!(
                diagnostics.iter().any(|d| d.rule == "CC-MEM-002"),
                "Cycle at depth 4 should trigger CC-MEM-002"
            );
            assert!(
                !diagnostics.iter().any(|d| d.rule == "CC-MEM-003"),
                "Cycle at depth 4 should not trigger CC-MEM-003"
            );
        }
    }

    #[test]
    fn test_concurrent_diamond_dependency_no_duplicate_diagnostics() {
        use std::collections::HashMap;
        use std::sync::{Arc, RwLock};
        use std::thread;

        let temp = TempDir::new().unwrap();
        let claude = temp.path().join("CLAUDE.md");
        let b = temp.path().join("b.md");
        let c = temp.path().join("c.md");
        let shared = temp.path().join("shared.md");

        fs::write(&claude, "@b.md\n@c.md").unwrap();
        fs::write(&b, "@shared.md").unwrap();
        fs::write(&c, "@shared.md").unwrap();
        fs::write(&shared, "@missing.md").unwrap();

        let cache: crate::parsers::ImportCache = Arc::new(RwLock::new(HashMap::new()));

        let handles: Vec<_> = (0..5)
            .map(|_| {
                let cache = cache.clone();
                let path = claude.clone();
                let content = fs::read_to_string(&path).unwrap();
                thread::spawn(move || {
                    let mut config = LintConfig::default();
                    config.set_import_cache(cache);
                    config.set_root_dir(path.parent().unwrap().to_path_buf());
                    let validator = ImportsValidator;
                    validator.validate(&path, &content, &config)
                })
            })
            .collect();

        for handle in handles {
            let diagnostics = handle
                .join()
                .expect("Thread panicked during diamond dependency validation");
            let missing_count = diagnostics
                .iter()
                .filter(|d| {
                    (d.rule == "CC-MEM-001" || d.rule == "REF-001")
                        && d.message.contains("@missing.md")
                })
                .count();
            assert_eq!(
                missing_count, 1,
                "Diamond dependency should produce exactly one missing-import diagnostic for @missing.md (deduplication check)"
            );
        }
    }

    #[test]
    fn test_missing_transitive_import_stops_traversal() {
        let temp = TempDir::new().unwrap();
        let claude = temp.path().join("CLAUDE.md");
        let a = temp.path().join("a.md");

        fs::write(&claude, "@a.md").unwrap();
        fs::write(&a, "@b.md").unwrap();

        let validator = ImportsValidator;
        let content = fs::read_to_string(&claude).unwrap();
        let diagnostics = validator.validate(&claude, &content, &LintConfig::default());

        assert!(
            diagnostics.iter().any(|d| {
                (d.rule == "CC-MEM-001" || d.rule == "REF-001") && d.message.contains("@b.md")
            }),
            "Should report missing import for b.md"
        );
        assert!(
            !diagnostics.iter().any(|d| d.message.contains("c.md")),
            "Should not reference c.md since b.md does not exist"
        );
    }

    #[test]
    fn test_concurrent_different_roots_shared_cache() {
        use std::collections::HashMap;
        use std::sync::{Arc, RwLock};
        use std::thread;

        let temp = TempDir::new().unwrap();
        let claude = temp.path().join("CLAUDE.md");
        let skill = temp.path().join("SKILL.md");
        let b = temp.path().join("b.md");

        fs::write(&claude, "@b.md").unwrap();
        fs::write(&skill, "@b.md").unwrap();
        fs::write(&b, "@CLAUDE.md").unwrap();

        let cache: crate::parsers::ImportCache = Arc::new(RwLock::new(HashMap::new()));

        // Interleave CLAUDE.md and SKILL.md handles in one Vec so both groups race
        // against each other and against the shared cache simultaneously.
        let handles: Vec<(bool, _)> = (0..10)
            .map(|i| {
                let cache = cache.clone();
                let is_claude = i % 2 == 0;
                let path = if is_claude {
                    claude.clone()
                } else {
                    skill.clone()
                };
                let content = fs::read_to_string(&path).unwrap();
                let handle = thread::spawn(move || {
                    let mut config = LintConfig::default();
                    config.set_import_cache(cache);
                    config.set_root_dir(path.parent().unwrap().to_path_buf());
                    let validator = ImportsValidator;
                    validator.validate(&path, &content, &config)
                });
                (is_claude, handle)
            })
            .collect();

        for (is_claude, handle) in handles {
            let diagnostics = handle.join().expect("Thread panicked");
            if is_claude {
                assert!(
                    diagnostics.iter().any(|d| d.rule == "CC-MEM-002"),
                    "CLAUDE.md threads should detect cycle (CC-MEM-002)"
                );
            } else {
                assert!(
                    !diagnostics.iter().any(|d| d.rule == "CC-MEM-002"),
                    "SKILL.md threads should not get CC-MEM-002 (cycle rules only apply to CLAUDE.md roots)"
                );
            }
        }
    }

    #[test]
    fn test_cycle_detection_three_file_chain_with_non_claude_root() {
        let temp = TempDir::new().unwrap();
        let skill = temp.path().join("SKILL.md");
        let b = temp.path().join("b.md");
        let c = temp.path().join("c.md");

        fs::write(&skill, "@b.md").unwrap();
        fs::write(&b, "@c.md").unwrap();
        fs::write(&c, "@SKILL.md").unwrap();

        let validator = ImportsValidator;
        let content = fs::read_to_string(&skill).unwrap();
        let diagnostics = validator.validate(&skill, &content, &LintConfig::default());

        assert!(
            !diagnostics.iter().any(|d| d.rule == "CC-MEM-002"),
            "SKILL.md root should not trigger CC-MEM-002 (cycle rules only apply to CLAUDE.md roots)"
        );
        assert!(
            !diagnostics.iter().any(|d| d.rule == "CC-MEM-003"),
            "SKILL.md root should not trigger CC-MEM-003 (depth rules only apply to CLAUDE.md roots)"
        );
    }
}
