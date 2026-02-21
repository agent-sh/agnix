//! Validation pipeline: file and project validation.

#[cfg(feature = "filesystem")]
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
#[cfg(feature = "filesystem")]
use std::path::PathBuf;
#[cfg(feature = "filesystem")]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[cfg(feature = "filesystem")]
use rayon::iter::ParallelBridge;
#[cfg(feature = "filesystem")]
use rayon::prelude::*;
#[cfg(feature = "filesystem")]
use rust_i18n::t;

use crate::config::LintConfig;
use crate::diagnostics::Diagnostic;
#[cfg(feature = "filesystem")]
use crate::diagnostics::{ConfigError, CoreError, LintResult, ValidationError, ValidationOutcome};
use crate::file_types::{FileType, detect_file_type};
#[cfg(feature = "filesystem")]
use crate::file_utils;
use crate::parsers::frontmatter::normalize_line_endings;
use crate::registry::ValidatorRegistry;
#[cfg(feature = "filesystem")]
use crate::rules::project_level::run_project_level_checks;
#[cfg(feature = "filesystem")]
use crate::schemas;

/// Result of validating a project, including diagnostics and metadata.
///
/// All fields are public. Use [`ValidationResult::new`] for convenient construction when only
/// `diagnostics` and `files_checked` are known; struct literal construction is also supported.
/// Note: adding a new public field in the future would be a breaking change for struct literals
/// and exhaustive destructuring patterns. For forward-compatible code, prefer
/// [`ValidationResult::new`] and use `..` in destructure patterns.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Diagnostics found during validation.
    pub diagnostics: Vec<Diagnostic>,
    /// Number of files that were checked (excludes Unknown file types).
    pub files_checked: usize,
    /// Wall-clock time spent in validation, in milliseconds.
    pub validation_time_ms: Option<u64>,
    /// Number of validator instances registered in the registry (not the count of validators executed).
    /// The field name uses "factories" for backward compatibility; since v0.12.2 this counts
    /// pre-built cached instances rather than factory invocations.
    pub validator_factories_registered: usize,
}

impl ValidationResult {
    /// Create a new `ValidationResult` with the given diagnostics and file count.
    ///
    /// Metadata fields (`validation_time_ms`, `validator_factories_registered`) default to
    /// `None` / `0` and can be set with the builder-style helpers.
    pub fn new(diagnostics: Vec<Diagnostic>, files_checked: usize) -> Self {
        Self {
            diagnostics,
            files_checked,
            validation_time_ms: None,
            validator_factories_registered: 0,
        }
    }

    /// Set the wall-clock validation time (builder pattern).
    pub fn with_timing(mut self, ms: u64) -> Self {
        self.validation_time_ms = Some(ms);
        self
    }

    /// Set the total number of validator factories registered (builder pattern).
    pub fn with_validator_factories_registered(mut self, count: usize) -> Self {
        self.validator_factories_registered = count;
        self
    }
}

/// Pre-compiled file inclusion/exclusion patterns for efficient matching.
///
/// Used internally by `validate_project_with_registry` to avoid re-compiling
/// glob patterns for every file during parallel validation.
#[derive(Default)]
pub(crate) struct CompiledFilesConfig {
    include_as_memory: Vec<glob::Pattern>,
    include_as_generic: Vec<glob::Pattern>,
    exclude: Vec<glob::Pattern>,
}

impl CompiledFilesConfig {
    fn is_empty(&self) -> bool {
        self.include_as_memory.is_empty()
            && self.include_as_generic.is_empty()
            && self.exclude.is_empty()
    }
}

/// Compile glob patterns, collecting any invalid patterns as [`Diagnostic`] warnings
/// instead of printing to stderr.
///
/// Returns the successfully compiled patterns alongside diagnostics for any
/// patterns that failed to compile. Invalid patterns are excluded from the
/// compiled output and reported as Diagnostic warnings instead.
///
/// `config_file` is the path used in the diagnostic `file` field - callers should
/// pass an absolute path (e.g. `root_dir.join(".agnix.toml")`) so that diagnostics
/// are consistent with other config-level diagnostics in the pipeline.
#[cfg(feature = "filesystem")]
fn compile_patterns_with_diagnostics(
    patterns: &[String],
    config_file: &Path,
) -> (Vec<glob::Pattern>, Vec<Diagnostic>) {
    let mut compiled = Vec::with_capacity(patterns.len());
    let mut diagnostics = Vec::new();
    for p in patterns {
        let normalized = p.replace('\\', "/");
        match glob::Pattern::new(&normalized) {
            Ok(pat) => compiled.push(pat),
            Err(e) => {
                diagnostics.push(
                    Diagnostic::warning(
                        config_file.to_path_buf(),
                        1,
                        0,
                        "config::glob",
                        t!(
                            "rules.invalid_glob_pattern",
                            pattern = p,
                            error = e.to_string()
                        ),
                    )
                    .with_suggestion(t!("rules.invalid_glob_pattern_suggestion")),
                );
            }
        }
    }
    (compiled, diagnostics)
}

/// Compile glob patterns leniently, discarding diagnostics for invalid patterns.
///
/// Used in code paths where diagnostics cannot be surfaced (e.g. the public
/// [`resolve_file_type`] API, which must not change its return type). Invalid
/// patterns are silently skipped.
fn compile_patterns_lenient(patterns: &[String]) -> Vec<glob::Pattern> {
    patterns
        .iter()
        .filter_map(|p| {
            let normalized = p.replace('\\', "/");
            glob::Pattern::new(&normalized).ok()
        })
        .collect()
}

fn compile_files_config(files: &crate::config::FilesConfig) -> CompiledFilesConfig {
    CompiledFilesConfig {
        include_as_memory: compile_patterns_lenient(&files.include_as_memory),
        include_as_generic: compile_patterns_lenient(&files.include_as_generic),
        exclude: compile_patterns_lenient(&files.exclude),
    }
}

/// Compile `[files]` config patterns, surfacing invalid patterns as diagnostics.
///
/// Used by [`validate_project_with_registry`] where diagnostics can be
/// propagated to the caller. Returns both the compiled config and any
/// diagnostics for malformed glob patterns.
///
/// `config_file` is forwarded to [`compile_patterns_with_diagnostics`] for the
/// diagnostic `file` field.
#[cfg(feature = "filesystem")]
fn compile_files_config_with_diagnostics(
    files: &crate::config::FilesConfig,
    config_file: &Path,
) -> (CompiledFilesConfig, Vec<Diagnostic>) {
    let mut all_diagnostics = Vec::new();

    let (include_as_memory, diags) =
        compile_patterns_with_diagnostics(&files.include_as_memory, config_file);
    all_diagnostics.extend(diags);

    let (include_as_generic, diags) =
        compile_patterns_with_diagnostics(&files.include_as_generic, config_file);
    all_diagnostics.extend(diags);

    let (exclude, diags) = compile_patterns_with_diagnostics(&files.exclude, config_file);
    all_diagnostics.extend(diags);

    (
        CompiledFilesConfig {
            include_as_memory,
            include_as_generic,
            exclude,
        },
        all_diagnostics,
    )
}

/// Match options for file inclusion/exclusion glob patterns.
///
/// `require_literal_separator` is `true` so that `*` only matches within a
/// single path component. Users must use `**` for recursive matching (e.g.
/// `dir/**/*.md` instead of `dir/*.md` to match nested files).
const FILES_MATCH_OPTIONS: glob::MatchOptions = glob::MatchOptions {
    case_sensitive: true,
    require_literal_separator: true,
    require_literal_leading_dot: false,
};

fn resolve_with_compiled(
    path: &Path,
    root_dir: Option<&Path>,
    compiled: &CompiledFilesConfig,
) -> FileType {
    if compiled.is_empty() {
        return detect_file_type(path);
    }

    let rel_path = if let Some(root) = root_dir {
        normalize_rel_path(path, root)
    } else {
        // No root_dir: use filename only
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    };

    // Priority: exclude > include_as_memory > include_as_generic > detect
    for pattern in &compiled.exclude {
        if pattern.matches_with(&rel_path, FILES_MATCH_OPTIONS) {
            return FileType::Unknown;
        }
    }
    for pattern in &compiled.include_as_memory {
        if pattern.matches_with(&rel_path, FILES_MATCH_OPTIONS) {
            return FileType::ClaudeMd;
        }
    }
    for pattern in &compiled.include_as_generic {
        if pattern.matches_with(&rel_path, FILES_MATCH_OPTIONS) {
            return FileType::GenericMarkdown;
        }
    }

    detect_file_type(path)
}

/// Resolve file type with config-based overrides.
///
/// Applies `[files]` config patterns on top of [`detect_file_type`]:
/// - `files.exclude` patterns map to [`FileType::Unknown`] (skip validation)
/// - `files.include_as_memory` patterns map to [`FileType::ClaudeMd`]
/// - `files.include_as_generic` patterns map to [`FileType::GenericMarkdown`]
/// - Otherwise falls through to [`detect_file_type`]
///
/// Priority: exclude > include_as_memory > include_as_generic > built-in detection.
///
/// When no `[files]` patterns are configured, this is equivalent to
/// calling `detect_file_type(path)` directly.
pub fn resolve_file_type(path: &Path, config: &LintConfig) -> FileType {
    let files = config.files_config();
    if files.include_as_memory.is_empty()
        && files.include_as_generic.is_empty()
        && files.exclude.is_empty()
    {
        return detect_file_type(path);
    }

    // Compile patterns on-demand for single-file validation. Invalid patterns
    // are silently skipped (no diagnostics) because this public API returns only
    // a FileType. Use validate_project() for diagnostic surfacing, or
    // LintConfigBuilder::build() / LintConfig::validate() at config load time
    // for strict validation.
    let compiled = compile_files_config(files);
    resolve_with_compiled(path, config.root_dir().map(|p| p.as_path()), &compiled)
}

/// Validate a single file.
///
/// Returns [`ValidationOutcome::Success`] with diagnostics when validation runs,
/// [`ValidationOutcome::IoError`] when the file cannot be read, or
/// [`ValidationOutcome::Skipped`] when the file type is unknown.
///
/// The `Err` path is reserved for config-level errors only (e.g. those that
/// occur during `LintConfig` construction).
///
/// Note: This function creates a new [`ValidatorRegistry`] on every call. For
/// bulk validation of multiple files, use
/// [`validate_file_with_registry()`] with a pre-built shared registry for
/// significantly better performance. Unlike [`validate_file_with_registry()`],
/// this function applies `config.rules().disabled_validators` to the
/// freshly-created registry at construction time.
#[cfg(feature = "filesystem")]
pub fn validate_file(path: &Path, config: &LintConfig) -> LintResult<ValidationOutcome> {
    let mut registry = ValidatorRegistry::with_defaults();
    for name in &config.rules().disabled_validators {
        registry.disable_validator_owned(name);
    }
    validate_file_with_registry(path, config, &registry)
}

/// Validate a single file with a custom validator registry.
///
/// Returns [`ValidationOutcome::Success`] with diagnostics when validation runs,
/// [`ValidationOutcome::IoError`] when the file cannot be read, or
/// [`ValidationOutcome::Skipped`] when the file type is unknown.
///
/// The `Err` path is reserved for config-level errors only.
///
/// `config.rules().disabled_validators` is applied at runtime, so callers
/// may share a single `ValidatorRegistry` across configs that differ only
/// in their disabled-validator sets (e.g. the LSP path). This is consistent
/// with [`validate_content()`].
#[cfg(feature = "filesystem")]
pub fn validate_file_with_registry(
    path: &Path,
    config: &LintConfig,
    registry: &ValidatorRegistry,
) -> LintResult<ValidationOutcome> {
    let file_type = resolve_file_type(path, config);
    validate_file_with_type(path, file_type, config, registry)
}

/// Validate a single file with a pre-resolved [`FileType`].
///
/// This avoids re-compiling `[files]` glob patterns when the file type has
/// already been determined (e.g. in `validate_project_with_registry` where
/// patterns are pre-compiled for the entire walk).
#[cfg(feature = "filesystem")]
fn validate_file_with_type(
    path: &Path,
    file_type: FileType,
    config: &LintConfig,
    registry: &ValidatorRegistry,
) -> LintResult<ValidationOutcome> {
    if file_type == FileType::Unknown {
        return Ok(ValidationOutcome::Skipped);
    }

    let raw_content = match file_utils::safe_read_file(path) {
        Ok(content) => content,
        Err(CoreError::File(file_error)) => {
            return Ok(ValidationOutcome::IoError(file_error));
        }
        Err(other) => return Err(other),
    };
    let content = normalize_line_endings(&raw_content);

    let validators = registry.validators_for(file_type);
    let disabled = &config.rules().disabled_validators;
    let mut diagnostics = Vec::new();

    if disabled.is_empty() {
        for validator in validators {
            diagnostics.extend(validator.validate(path, &content, config));
        }
    } else {
        let disabled_set: HashSet<&str> = disabled.iter().map(|s| s.as_str()).collect();
        for validator in validators {
            if disabled_set.contains(validator.name()) {
                continue;
            }
            diagnostics.extend(validator.validate(path, &content, config));
        }
    }

    Ok(ValidationOutcome::Success(diagnostics))
}

/// Validate in-memory content for a given path.
///
/// This function performs no filesystem I/O -- the content is provided directly.
/// File type is resolved from the path using [`resolve_file_type`], then all
/// matching validators are run against the content.
///
/// Returns an empty `Vec` if the file type is unknown.
pub fn validate_content(
    path: &Path,
    content: &str,
    config: &LintConfig,
    registry: &ValidatorRegistry,
) -> Vec<Diagnostic> {
    let file_type = resolve_file_type(path, config);
    if file_type == FileType::Unknown {
        return vec![];
    }

    let content = normalize_line_endings(content);

    let validators = registry.validators_for(file_type);
    let disabled = &config.rules().disabled_validators;
    let mut diagnostics = Vec::new();

    // Runtime disabled_validators check: honours per-config disabled_validators
    // without requiring them to be pre-applied to the registry. The LSP creates
    // a single shared registry via with_defaults() and relies on this check to
    // respect per-workspace disabled_validators from the user's LintConfig.
    if disabled.is_empty() {
        for validator in validators {
            diagnostics.extend(validator.validate(path, &content, config));
        }
    } else {
        let disabled_set: HashSet<&str> = disabled.iter().map(|s| s.as_str()).collect();
        for validator in validators {
            if disabled_set.contains(validator.name()) {
                continue;
            }
            diagnostics.extend(validator.validate(path, &content, config));
        }
    }

    diagnostics
}

/// Main entry point for validating a project
#[cfg(feature = "filesystem")]
pub fn validate_project(path: &Path, config: &LintConfig) -> LintResult<ValidationResult> {
    let mut registry = ValidatorRegistry::with_defaults();
    for name in &config.rules().disabled_validators {
        registry.disable_validator_owned(name);
    }
    validate_project_with_registry(path, config, &registry)
}

#[cfg(feature = "filesystem")]
struct ExcludePattern {
    pattern: glob::Pattern,
    dir_only_prefix: Option<String>,
    allow_probe: bool,
}

fn normalize_rel_path(entry_path: &Path, root: &Path) -> String {
    let rel_path = entry_path.strip_prefix(root).unwrap_or(entry_path);
    let path_str = rel_path.to_string_lossy().replace('\\', "/");
    match path_str.strip_prefix("./") {
        Some(stripped) => stripped.to_string(),
        None => path_str,
    }
}

#[cfg(feature = "filesystem")]
fn compile_exclude_patterns(excludes: &[String]) -> LintResult<Vec<ExcludePattern>> {
    excludes
        .iter()
        .map(|pattern| {
            let normalized = pattern.replace('\\', "/");
            let (glob_str, dir_only_prefix) = if let Some(prefix) = normalized.strip_suffix('/') {
                (format!("{}/**", prefix), Some(prefix.to_string()))
            } else {
                (normalized.clone(), None)
            };
            let allow_probe = dir_only_prefix.is_some() || glob_str.contains("**");
            let compiled = glob::Pattern::new(&glob_str).map_err(|e| {
                CoreError::Config(ConfigError::InvalidExcludePattern {
                    pattern: pattern.clone(),
                    message: e.to_string(),
                })
            })?;
            Ok(ExcludePattern {
                pattern: compiled,
                dir_only_prefix,
                allow_probe,
            })
        })
        .collect()
}

#[cfg(feature = "filesystem")]
fn should_prune_dir(rel_dir: &str, exclude_patterns: &[ExcludePattern]) -> bool {
    if rel_dir.is_empty() {
        return false;
    }
    // Probe path used to detect patterns that match files inside a directory.
    // Only apply it for recursive patterns (e.g. ** or dir-only prefix).
    let probe = format!("{}/__agnix_probe__", rel_dir.trim_end_matches('/'));
    exclude_patterns
        .iter()
        .any(|p| p.pattern.matches(rel_dir) || (p.allow_probe && p.pattern.matches(&probe)))
}

#[cfg(feature = "filesystem")]
fn is_excluded_file(path_str: &str, exclude_patterns: &[ExcludePattern]) -> bool {
    exclude_patterns
        .iter()
        .any(|p| p.pattern.matches(path_str) && p.dir_only_prefix.as_deref() != Some(path_str))
}

/// Run only project-level validation checks without per-file validation.
///
/// This is a lightweight alternative to [`validate_project`] that only runs
/// cross-file analysis rules (AGM-006, XP-004/005/006, VER-001). It does
/// not validate individual file contents.
///
/// Designed for the LSP server to provide project-level diagnostics that
/// require workspace-wide analysis, without the overhead of full per-file
/// validation (which the LSP handles incrementally via `did_open`/`did_change`).
#[cfg(feature = "filesystem")]
pub fn validate_project_rules(root: &Path, config: &LintConfig) -> LintResult<Vec<Diagnostic>> {
    use ignore::WalkBuilder;
    use std::sync::Arc;

    let root_dir = resolve_validation_root(root)?;
    let mut config = config.clone();
    config.set_root_dir(root_dir.clone());

    // Pre-compile exclude patterns once (Arc for filter_entry 'static bound)
    let exclude_patterns = Arc::new(compile_exclude_patterns(config.exclude())?);

    let walk_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let root_path = root_dir.clone();

    let mut agents_md_paths: Vec<PathBuf> = Vec::new();
    let mut instruction_file_paths: Vec<PathBuf> = Vec::new();
    let max_files = config.max_files_to_validate();

    // Walk directory tree collecting only paths relevant to project-level checks.
    // No per-file validation is performed -- this walk is lightweight.
    // Respects the same max_files_to_validate limit as validate_project_with_registry
    // to prevent unbounded directory traversal in large workspaces.
    for (files_seen, entry) in WalkBuilder::new(&walk_root)
        .hidden(false)
        .git_ignore(true)
        .git_exclude(false)
        .filter_entry({
            let exclude_patterns = Arc::clone(&exclude_patterns);
            let root_path = root_path.clone();
            move |entry| {
                let entry_path = entry.path();
                if entry_path == root_path {
                    return true;
                }
                if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                    let rel_path = normalize_rel_path(entry_path, &root_path);
                    return !should_prune_dir(&rel_path, exclude_patterns.as_slice());
                }
                true
            }
        })
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .enumerate()
    {
        // Enforce file count limit to prevent unbounded traversal
        if let Some(limit) = max_files {
            if files_seen >= limit {
                return Err(CoreError::Validation(ValidationError::TooManyFiles {
                    count: files_seen,
                    limit,
                }));
            }
        }
        let file_path = entry.path().to_path_buf();

        let path_str = normalize_rel_path(&file_path, &root_path);
        if is_excluded_file(&path_str, exclude_patterns.as_slice()) {
            continue;
        }

        // Collect AGENTS.md paths for AGM-006 check
        if file_path.file_name().and_then(|n| n.to_str()) == Some("AGENTS.md") {
            agents_md_paths.push(file_path.clone());
        }

        // Collect instruction file paths for XP-004/005/006 checks
        if schemas::cross_platform::is_instruction_file(&file_path) {
            instruction_file_paths.push(file_path);
        }
    }

    // Sort for deterministic ordering
    agents_md_paths.sort();
    instruction_file_paths.sort();

    Ok(run_project_level_checks(
        &agents_md_paths,
        &instruction_file_paths,
        &config,
        &root_dir,
    ))
}

/// Main entry point for validating a project with a custom validator registry
#[cfg(feature = "filesystem")]
pub fn validate_project_with_registry(
    path: &Path,
    config: &LintConfig,
    registry: &ValidatorRegistry,
) -> LintResult<ValidationResult> {
    use ignore::WalkBuilder;
    use std::sync::Arc;
    use std::time::Instant;

    let validation_start = Instant::now();

    let root_dir = resolve_validation_root(path)?;
    let mut config = config.clone();
    config.set_root_dir(root_dir.clone());

    // Initialize shared import cache for project-level validation (if not already set).
    // This cache is shared across all file validations, allowing the ImportsValidator
    // to avoid redundant parsing when traversing import chains that reference the same files.
    if config.get_import_cache().is_none() {
        let import_cache: crate::parsers::ImportCache =
            std::sync::Arc::new(std::sync::RwLock::new(HashMap::new()));
        config.set_import_cache(import_cache);
    }

    // Pre-compile exclude patterns once (avoids N+1 pattern compilation)
    let exclude_patterns = compile_exclude_patterns(config.exclude())?;
    let exclude_patterns = Arc::new(exclude_patterns);

    // Pre-compile files config patterns once for the parallel walk.
    // Invalid patterns produce Warning diagnostics that are prepended to results.
    let config_file = root_dir.join(".agnix.toml");
    let (compiled_files_inner, config_diags) =
        compile_files_config_with_diagnostics(config.files_config(), &config_file);
    let compiled_files = Arc::new(compiled_files_inner);

    let root_path = root_dir.clone();

    // Fallback to relative path is safe: symlink checks and size limits still apply per-file
    let walk_root = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    // Shared atomic state for file-limit enforcement across parallel workers.
    // These must remain atomic (not fold/reduce) because the limit check must
    // be visible immediately to all threads to stop work promptly.
    let files_checked = Arc::new(AtomicUsize::new(0));
    let limit_exceeded = Arc::new(AtomicBool::new(false));

    // Get the file limit from config (None means no limit)
    let max_files = config.max_files_to_validate();

    // Stream file walk directly into parallel validation (no intermediate Vec)
    // Note: hidden(false) includes .github, .codex, .claude, .cursor directories
    // Note: git_exclude(false) prevents .git/info/exclude from hiding config dirs
    //       that users may locally exclude (e.g. .codex/) but still need linting.
    //       Trade-off: this may surface files the user intentionally excluded locally,
    //       but security is still enforced via symlink rejection (file_utils::safe_read)
    //       and file size limits, so the exposure is limited to lint noise, not unsafe I/O.
    //
    // Uses fold/reduce instead of Mutex-protected Vecs to accumulate paths and
    // diagnostics thread-locally, eliminating lock contention in the hot loop.
    let (mut diagnostics, mut agents_md_paths, mut instruction_file_paths) =
        WalkBuilder::new(&walk_root)
            .hidden(false)
            .git_ignore(true)
            .git_exclude(false)
            .filter_entry({
                let exclude_patterns = Arc::clone(&exclude_patterns);
                let root_path = root_path.clone();
                move |entry| {
                    let entry_path = entry.path();
                    if entry_path == root_path {
                        return true;
                    }
                    if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                        let rel_path = normalize_rel_path(entry_path, &root_path);
                        return !should_prune_dir(&rel_path, exclude_patterns.as_slice());
                    }
                    true
                }
            })
            .build()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_file())
            .filter(|entry| {
                let entry_path = entry.path();
                let path_str = normalize_rel_path(entry_path, &root_path);
                !is_excluded_file(&path_str, exclude_patterns.as_slice())
            })
            .map(|entry| entry.path().to_path_buf())
            .par_bridge()
            .fold(
                || {
                    (
                        Vec::<Diagnostic>::new(),
                        Vec::<PathBuf>::new(),
                        Vec::<PathBuf>::new(),
                    )
                },
                |(mut diags, mut agents, mut instructions), file_path| {
                    // Security: Check if file limit has been exceeded
                    // Once exceeded, skip processing additional files
                    // Use SeqCst ordering for consistency with store operations
                    if limit_exceeded.load(Ordering::SeqCst) {
                        return (diags, agents, instructions);
                    }

                    // Count recognized files (resolve_with_compiled is string-only, no I/O)
                    let file_type =
                        resolve_with_compiled(&file_path, Some(&root_path), &compiled_files);
                    if file_type != FileType::Unknown {
                        let count = files_checked.fetch_add(1, Ordering::SeqCst) + 1;
                        // Security: Enforce file count limit to prevent DoS
                        if let Some(limit) = max_files {
                            if count > limit {
                                limit_exceeded.store(true, Ordering::SeqCst);
                                return (diags, agents, instructions);
                            }
                        }
                    }

                    // Collect AGENTS.md paths for AGM-006 check (thread-local, no lock).
                    if file_path.file_name().and_then(|n| n.to_str()) == Some("AGENTS.md") {
                        agents.push(file_path.clone());
                    }

                    // Collect instruction file paths for XP-004/005/006 checks (thread-local, no lock).
                    if schemas::cross_platform::is_instruction_file(&file_path) {
                        instructions.push(file_path.clone());
                    }

                    // Validate the file using the pre-resolved file_type to avoid
                    // re-compiling [files] glob patterns for every file.
                    match validate_file_with_type(&file_path, file_type, &config, registry) {
                        Ok(ValidationOutcome::Success(file_diagnostics)) => {
                            diags.extend(file_diagnostics);
                        }
                        Ok(ValidationOutcome::IoError(file_error)) => {
                            diags.push(
                                Diagnostic::error(
                                    file_path,
                                    0,
                                    0,
                                    "file::read",
                                    t!("rules.file_read_error", error = file_error.to_string()),
                                )
                                .with_suggestion(t!("rules.file_read_error_suggestion")),
                            );
                        }
                        Ok(ValidationOutcome::Skipped) => {
                            // File type unknown - no validation needed
                        }
                        Err(e) => {
                            diags.push(
                                Diagnostic::error(
                                    file_path,
                                    0,
                                    0,
                                    "file::read",
                                    t!("rules.file_read_error", error = e.to_string()),
                                )
                                .with_suggestion(t!("rules.file_read_error_suggestion")),
                            );
                        }
                    }

                    (diags, agents, instructions)
                },
            )
            .reduce(
                || (Vec::new(), Vec::new(), Vec::new()),
                |(mut d1, mut a1, mut i1), (d2, a2, i2)| {
                    d1.extend(d2);
                    a1.extend(a2);
                    i1.extend(i2);
                    (d1, a1, i1)
                },
            );

    // Surface config-level diagnostics (e.g. invalid glob patterns in [files])
    // before the TooManyFiles check so they are included on successful validation.
    diagnostics.extend(config_diags);

    // Check if limit was exceeded and return error
    if limit_exceeded.load(Ordering::Relaxed) {
        if let Some(limit) = max_files {
            return Err(CoreError::Validation(ValidationError::TooManyFiles {
                count: files_checked.load(Ordering::Relaxed),
                limit,
            }));
        }
    }

    // Run project-level checks (AGM-006, XP-004/005/006, VER-001)
    {
        agents_md_paths.sort();
        instruction_file_paths.sort();

        diagnostics.extend(run_project_level_checks(
            &agents_md_paths,
            &instruction_file_paths,
            &config,
            &root_dir,
        ));
    }

    // Sort by severity (errors first), then by file path, then by line/rule for full determinism
    diagnostics.sort_by(|a, b| {
        a.level
            .cmp(&b.level)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.rule.cmp(&b.rule))
    });

    // Extract final count from atomic counter
    let files_checked = files_checked.load(Ordering::Relaxed);

    // as_millis() returns u128; clamp to u64 for the public API contract.
    let elapsed_ms = validation_start.elapsed().as_millis().min(u64::MAX as u128) as u64;
    let validator_factories_registered = registry.total_validator_count();

    Ok(ValidationResult::new(diagnostics, files_checked)
        .with_timing(elapsed_ms)
        .with_validator_factories_registered(validator_factories_registered))
}

#[cfg(feature = "filesystem")]
fn resolve_validation_root(path: &Path) -> LintResult<PathBuf> {
    let metadata = match path.metadata() {
        Ok(m) => m,
        Err(_) => {
            // Any I/O failure (not found, permission denied, etc.) is treated
            // uniformly as RootNotFound. For a local linter running as the
            // invoking user, the distinction is not actionable at this level.
            return Err(CoreError::Validation(ValidationError::RootNotFound {
                path: path.to_path_buf(),
            }));
        }
    };
    let candidate = if metadata.is_file() {
        path.parent().unwrap_or(Path::new("."))
    } else {
        path
    };
    Ok(std::fs::canonicalize(candidate).unwrap_or_else(|_| candidate.to_path_buf()))
}

#[cfg(test)]
mod validate_content_tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::registry::ValidatorRegistry;

    #[test]
    fn returns_diagnostics_for_known_file_type() {
        let config = LintConfig::default();
        let registry = ValidatorRegistry::with_defaults();
        let path = Path::new("CLAUDE.md");
        let content = "<unclosed>";
        let diags = validate_content(path, content, &config, &registry);
        assert!(
            !diags.is_empty(),
            "Should find diagnostics for unclosed XML tag"
        );
    }

    #[test]
    fn returns_empty_for_unknown_file_type() {
        let config = LintConfig::default();
        let registry = ValidatorRegistry::with_defaults();
        let path = Path::new("main.rs");
        let diags = validate_content(path, "", &config, &registry);
        assert!(
            diags.is_empty(),
            "Unknown file type should produce no diagnostics"
        );
    }

    #[test]
    fn returns_empty_for_empty_content_with_known_type() {
        let config = LintConfig::default();
        let registry = ValidatorRegistry::with_defaults();
        let path = Path::new("CLAUDE.md");
        let diags = validate_content(path, "", &config, &registry);
        // Empty CLAUDE.md is valid (no content to violate rules).
        assert!(
            diags.is_empty(),
            "Empty content for a known file type should not produce diagnostics"
        );
    }

    #[test]
    fn respects_tool_filter() {
        let config = LintConfig::builder()
            .tools(vec!["cursor".to_string()])
            .build()
            .unwrap();
        let registry = ValidatorRegistry::with_defaults();
        let path = Path::new("CLAUDE.md");
        let content = "# Project\n\nSome instructions.";
        // Should not panic with tool filter
        let _ = validate_content(path, content, &config, &registry);
    }

    #[test]
    fn crlf_content_produces_same_diagnostics_as_lf() {
        let config = LintConfig::default();
        let registry = ValidatorRegistry::with_defaults();
        let path = Path::new("skill.md");

        let lf_content =
            "---\nname: test-skill\ndescription: A test\n---\n\n# Instructions\n\n<unclosed>\n";
        let crlf_content = "---\r\nname: test-skill\r\ndescription: A test\r\n---\r\n\r\n# Instructions\r\n\r\n<unclosed>\r\n";

        let lf_diags = validate_content(path, lf_content, &config, &registry);
        let crlf_diags = validate_content(path, crlf_content, &config, &registry);

        assert_eq!(
            lf_diags.len(),
            crlf_diags.len(),
            "CRLF and LF content should produce the same number of diagnostics.\nLF: {:?}\nCRLF: {:?}",
            lf_diags
                .iter()
                .map(|d| (&d.rule, d.line, d.column))
                .collect::<Vec<_>>(),
            crlf_diags
                .iter()
                .map(|d| (&d.rule, d.line, d.column))
                .collect::<Vec<_>>(),
        );

        for (lf_d, crlf_d) in lf_diags.iter().zip(crlf_diags.iter()) {
            assert_eq!(
                lf_d.rule, crlf_d.rule,
                "Same rules should fire for LF and CRLF content"
            );
            assert_eq!(
                lf_d.line, crlf_d.line,
                "Line numbers should match between LF and CRLF for rule {}",
                lf_d.rule
            );
            assert_eq!(
                lf_d.column, crlf_d.column,
                "Column numbers should match between LF and CRLF for rule {}",
                lf_d.rule
            );
        }
    }

    #[test]
    fn lf_validation_is_stable() {
        let config = LintConfig::default();
        let registry = ValidatorRegistry::with_defaults();
        let path = Path::new("CLAUDE.md");

        // Already-normalized content should produce the same result on repeated calls.
        let content = "# Project\n\nInstructions here.\n";
        let diags1 = validate_content(path, content, &config, &registry);
        let diags2 = validate_content(path, content, &config, &registry);

        assert_eq!(
            diags1.len(),
            diags2.len(),
            "Repeated validation of LF content should be stable"
        );
    }

    #[test]
    fn crlf_validation_is_idempotent() {
        let config = LintConfig::default();
        let registry = ValidatorRegistry::with_defaults();
        let path = Path::new("skill.md");

        // Validating CRLF content twice should produce identical diagnostics each time.
        let crlf_content =
            "---\r\nname: test-skill\r\ndescription: A test\r\n---\r\n\r\n# Instructions\r\n";
        let diags1 = validate_content(path, crlf_content, &config, &registry);
        let diags2 = validate_content(path, crlf_content, &config, &registry);

        assert_eq!(
            diags1.len(),
            diags2.len(),
            "Repeated validation of CRLF content should be stable"
        );
        for (d1, d2) in diags1.iter().zip(diags2.iter()) {
            assert_eq!(d1.rule, d2.rule);
            assert_eq!(d1.line, d2.line);
            assert_eq!(d1.column, d2.column);
        }
    }

    #[test]
    fn lone_cr_content_produces_same_diagnostics_as_lf() {
        // Lone CR (\r without following \n) is the old Mac line ending format.
        // normalize_line_endings handles it in its single-pass char iterator,
        // which converts any bare \r (not followed by \n) to \n.
        let config = LintConfig::default();
        let registry = ValidatorRegistry::with_defaults();
        let path = Path::new("skill.md");

        let lf_content = "---\nname: test-skill\ndescription: A test\n---\n\n# Instructions\n";
        // Same content with lone CR instead of LF
        let cr_content = "---\rname: test-skill\rdescription: A test\r---\r\r# Instructions\r";

        let lf_diags = validate_content(path, lf_content, &config, &registry);
        let cr_diags = validate_content(path, cr_content, &config, &registry);

        assert_eq!(
            lf_diags.len(),
            cr_diags.len(),
            "Lone-CR and LF content should produce the same number of diagnostics.\nLF: {:?}\nCR: {:?}",
            lf_diags
                .iter()
                .map(|d| (&d.rule, d.line, d.column))
                .collect::<Vec<_>>(),
            cr_diags
                .iter()
                .map(|d| (&d.rule, d.line, d.column))
                .collect::<Vec<_>>(),
        );
        for (lf_d, cr_d) in lf_diags.iter().zip(cr_diags.iter()) {
            assert_eq!(lf_d.rule, cr_d.rule);
            assert_eq!(lf_d.line, cr_d.line);
            assert_eq!(lf_d.column, cr_d.column);
        }
    }
}

#[cfg(all(test, feature = "filesystem"))]
mod tests {
    use super::*;

    #[test]
    fn test_should_prune_dir_with_globbed_patterns() {
        let patterns =
            compile_exclude_patterns(&vec!["target/**".to_string(), "**/target/**".to_string()])
                .unwrap();
        assert!(
            should_prune_dir("target", &patterns),
            "Expected target/** to prune target directory"
        );
        assert!(
            should_prune_dir("sub/target", &patterns),
            "Expected **/target/** to prune nested target directory"
        );
    }

    #[test]
    fn test_should_prune_dir_for_bare_pattern() {
        let patterns = compile_exclude_patterns(&vec!["target".to_string()]).unwrap();
        assert!(
            should_prune_dir("target", &patterns),
            "Bare pattern should prune directory"
        );
        assert!(
            !should_prune_dir("sub/target", &patterns),
            "Bare pattern should not prune nested directories"
        );
    }

    #[test]
    fn test_should_prune_dir_for_trailing_slash_pattern() {
        let patterns = compile_exclude_patterns(&vec!["target/".to_string()]).unwrap();
        assert!(
            should_prune_dir("target", &patterns),
            "Trailing slash pattern should prune directory"
        );
    }

    #[test]
    fn test_should_not_prune_root_dir() {
        let patterns = compile_exclude_patterns(&vec!["target/**".to_string()]).unwrap();
        assert!(
            !should_prune_dir("", &patterns),
            "Root directory should never be pruned"
        );
    }

    #[test]
    fn test_should_not_prune_dir_for_single_level_glob() {
        let patterns = compile_exclude_patterns(&vec!["target/*".to_string()]).unwrap();
        assert!(
            !should_prune_dir("target", &patterns),
            "Single-level glob should not prune directory"
        );
    }

    #[test]
    fn test_dir_only_pattern_does_not_exclude_file_named_dir() {
        let patterns = compile_exclude_patterns(&vec!["target/".to_string()]).unwrap();
        assert!(
            !is_excluded_file("target", &patterns),
            "Directory-only pattern should not exclude a file named target"
        );
    }

    #[test]
    fn test_dir_only_pattern_excludes_files_under_dir() {
        let patterns = compile_exclude_patterns(&vec!["target/".to_string()]).unwrap();
        assert!(
            is_excluded_file("target/file.txt", &patterns),
            "Directory-only pattern should exclude files under target/"
        );
    }

    #[test]
    fn test_compile_exclude_patterns_invalid_pattern_returns_error() {
        let result = compile_exclude_patterns(&vec!["[".to_string()]);
        assert!(matches!(
            result,
            Err(CoreError::Config(ConfigError::InvalidExcludePattern { .. }))
        ));
    }

    // ===== compile_patterns_with_diagnostics tests =====

    #[test]
    fn compile_patterns_with_diagnostics_all_valid() {
        let patterns = vec!["*.md".to_string(), "src/**/*.rs".to_string()];
        let config_file = Path::new(".agnix.toml");
        let (compiled, diags) = compile_patterns_with_diagnostics(&patterns, config_file);
        assert_eq!(compiled.len(), 2, "All valid patterns should compile");
        assert!(
            diags.is_empty(),
            "No diagnostics expected for valid patterns, got: {diags:?}"
        );
    }

    #[test]
    fn compile_patterns_with_diagnostics_invalid_pattern() {
        let patterns = vec!["[invalid".to_string()];
        let config_file = Path::new(".agnix.toml");
        let (compiled, diags) = compile_patterns_with_diagnostics(&patterns, config_file);
        assert!(
            compiled.is_empty(),
            "Invalid pattern should not produce a compiled pattern"
        );
        assert_eq!(
            diags.len(),
            1,
            "Expected exactly one diagnostic for the invalid pattern"
        );
        assert_eq!(
            diags[0].level,
            crate::DiagnosticLevel::Warning,
            "Invalid glob diagnostic should be Warning level"
        );
        assert_eq!(
            diags[0].rule, "config::glob",
            "Invalid glob diagnostic should use rule config::glob"
        );
        assert!(
            diags[0].suggestion.is_some(),
            "Diagnostic should include a suggestion"
        );
        assert!(
            diags[0].message.contains("[invalid"),
            "diagnostic message should include the pattern"
        );
    }

    #[test]
    fn compile_patterns_with_diagnostics_mixed_valid_and_invalid() {
        let patterns = vec![
            "*.md".to_string(),
            "[bad".to_string(),
            "src/**/*.rs".to_string(),
            "[also-bad".to_string(),
        ];
        let config_file = Path::new(".agnix.toml");
        let (compiled, diags) = compile_patterns_with_diagnostics(&patterns, config_file);
        assert_eq!(
            compiled.len(),
            2,
            "Only valid patterns should compile, got {}",
            compiled.len()
        );
        assert_eq!(
            diags.len(),
            2,
            "Expected 2 diagnostics for 2 invalid patterns, got {}",
            diags.len()
        );
        for d in &diags {
            assert_eq!(d.rule, "config::glob");
            assert_eq!(d.level, crate::DiagnosticLevel::Warning);
        }
    }

    #[test]
    fn compile_patterns_with_diagnostics_empty_input() {
        let patterns: Vec<String> = vec![];
        let config_file = Path::new(".agnix.toml");
        let (compiled, diags) = compile_patterns_with_diagnostics(&patterns, config_file);
        assert!(compiled.is_empty());
        assert!(diags.is_empty());
    }

    #[test]
    fn compile_files_config_with_diagnostics_aggregates_all_lists() {
        use crate::config::FilesConfig;

        let files = FilesConfig {
            include_as_memory: vec!["*.md".to_string(), "[bad-memory".to_string()],
            include_as_generic: vec!["[bad-generic".to_string()],
            exclude: vec!["valid/**".to_string(), "[bad-exclude".to_string()],
        };
        let config_file = Path::new(".agnix.toml");
        let (compiled, diags) = compile_files_config_with_diagnostics(&files, config_file);
        // Valid patterns: *.md (memory), valid/** (exclude) = 2 compiled total
        assert_eq!(compiled.include_as_memory.len(), 1);
        assert_eq!(compiled.include_as_generic.len(), 0);
        assert_eq!(compiled.exclude.len(), 1);
        // Invalid patterns: [bad-memory, [bad-generic, [bad-exclude = 3 diagnostics
        assert_eq!(
            diags.len(),
            3,
            "Expected 3 diagnostics from all 3 pattern lists, got: {diags:?}"
        );
        for d in &diags {
            assert_eq!(d.rule, "config::glob");
        }
    }

    #[test]
    fn crlf_file_on_disk_produces_same_diagnostics_as_lf() {
        // validate_file() reads from disk and normalizes CRLF in validate_file_with_type.
        // Verify the on-disk path produces the same diagnostics as the in-memory path.
        use crate::diagnostics::ValidationOutcome;

        let temp = tempfile::TempDir::new().unwrap();
        let lf_path = temp.path().join("skill_lf.md");
        let crlf_path = temp.path().join("skill_crlf.md");

        let lf_content =
            "---\nname: test-skill\ndescription: A test\n---\n\n# Instructions\n\n<unclosed>\n";
        let crlf_content = "---\r\nname: test-skill\r\ndescription: A test\r\n---\r\n\r\n# Instructions\r\n\r\n<unclosed>\r\n";

        std::fs::write(&lf_path, lf_content).unwrap();
        std::fs::write(&crlf_path, crlf_content).unwrap();

        let config = LintConfig::default();

        let lf_outcome = validate_file(&lf_path, &config).unwrap();
        let crlf_outcome = validate_file(&crlf_path, &config).unwrap();

        let lf_diags = match lf_outcome {
            ValidationOutcome::Success(d) => d,
            other => panic!("Expected Success, got {other:?}"),
        };
        let crlf_diags = match crlf_outcome {
            ValidationOutcome::Success(d) => d,
            other => panic!("Expected Success, got {other:?}"),
        };

        assert_eq!(
            lf_diags.len(),
            crlf_diags.len(),
            "On-disk CRLF file should produce same diagnostic count as LF file.\nLF: {:?}\nCRLF: {:?}",
            lf_diags
                .iter()
                .map(|d| (&d.rule, d.line, d.column))
                .collect::<Vec<_>>(),
            crlf_diags
                .iter()
                .map(|d| (&d.rule, d.line, d.column))
                .collect::<Vec<_>>(),
        );
        for (lf_d, crlf_d) in lf_diags.iter().zip(crlf_diags.iter()) {
            assert_eq!(lf_d.rule, crlf_d.rule, "Same rules should fire");
            assert_eq!(
                lf_d.line, crlf_d.line,
                "Line numbers should match for rule {}",
                lf_d.rule
            );
            assert_eq!(
                lf_d.column, crlf_d.column,
                "Column numbers should match for rule {}",
                lf_d.rule
            );
        }
    }
}
