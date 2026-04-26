#![allow(clippy::collapsible_if, clippy::let_and_return)]
//! agnix CLI - The nginx of agent configs

rust_i18n::i18n!("locales", fallback = "en");

mod json;
mod locale;
mod sarif;
#[cfg(feature = "telemetry")]
pub mod telemetry;
#[cfg(not(feature = "telemetry"))]
mod telemetry_stub;
mod tools;
mod watch;
#[cfg(not(feature = "telemetry"))]
use telemetry_stub as telemetry;

use agnix_core::{
    ValidationOutcome, ValidationResult, ValidatorRegistry, apply_fixes_with_options,
    config::{LintConfig, TargetTool},
    diagnostics::{Diagnostic, DiagnosticLevel, FixConfidenceTier},
    eval::{EvalFormat, evaluate_manifest_file},
    fixes::{FixApplyMode, FixApplyOptions},
    generate_schema, validate_file_with_registry, validate_project, validate_project_with_registry,
};
use clap::{Parser, Subcommand, ValueEnum};
use colored::*;
use rust_i18n::t;
use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Sarif,
}

/// CLI target argument enum with kebab-case names for command line ergonomics.
/// Separate from TargetTool (which uses PascalCase for config file serialization).
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum TargetArg {
    #[default]
    Generic,
    #[value(name = "claude-code")]
    ClaudeCode,
    Cursor,
    Codex,
    Kiro,
}

impl From<TargetArg> for TargetTool {
    fn from(arg: TargetArg) -> Self {
        match arg {
            TargetArg::Generic => TargetTool::Generic,
            TargetArg::ClaudeCode => TargetTool::ClaudeCode,
            TargetArg::Cursor => TargetTool::Cursor,
            TargetArg::Codex => TargetTool::Codex,
            TargetArg::Kiro => TargetTool::Kiro,
        }
    }
}

#[derive(Parser)]
#[command(name = "agnix")]
#[command(author, version, about, long_about = None)]
#[command(
    about = "The nginx of agent configs",
    long_about = "Validate agent specifications across Claude Code, Cursor, Codex, and beyond.\n\nValidates: Skills • MCP • Hooks • Memory • Plugins"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Paths to validate (defaults to current directory).
    ///
    /// Accepts one or more files or directories. When multiple paths are
    /// passed (e.g. from a pre-commit hook), only those paths are checked
    /// instead of the full project walk.
    #[arg(default_value = ".", num_args = 1..)]
    paths: Vec<PathBuf>,

    /// Strict mode (treat warnings as errors)
    #[arg(short, long)]
    strict: bool,

    /// Target tool (generic, claude-code, cursor, codex, kiro)
    #[arg(short, long, value_enum, default_value_t = TargetArg::Generic)]
    target: TargetArg,

    /// Config file path
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Apply automatic fixes (HIGH and MEDIUM confidence)
    #[arg(long, group = "fix_mode")]
    fix: bool,

    /// Show what would be fixed without modifying files
    #[arg(long)]
    dry_run: bool,

    /// Apply only safe (HIGH certainty) fixes
    #[arg(long, group = "fix_mode")]
    fix_safe: bool,

    /// Apply all fixes, including LOW-confidence ones
    #[arg(long, group = "fix_mode")]
    fix_unsafe: bool,

    /// Show proposed fixes inline in text output
    #[arg(long)]
    show_fixes: bool,

    /// Output format (text, json, or sarif)
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    /// Watch mode - re-validate on file changes
    #[arg(short, long)]
    watch: bool,

    /// Set output locale (e.g., en, es, zh-CN)
    #[arg(long)]
    locale: Option<String>,

    /// List supported locales and exit
    #[arg(long)]
    list_locales: bool,

    /// Maximum number of files to validate (security limit)
    /// Default: 10,000. Set to 0 to disable the limit (not recommended).
    #[arg(long)]
    max_files: Option<usize>,
}

/// Output format for evaluation results
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum EvalOutputFormat {
    #[default]
    Markdown,
    Json,
    Csv,
}

impl From<EvalOutputFormat> for EvalFormat {
    fn from(f: EvalOutputFormat) -> Self {
        match f {
            EvalOutputFormat::Markdown => EvalFormat::Markdown,
            EvalOutputFormat::Json => EvalFormat::Json,
            EvalOutputFormat::Csv => EvalFormat::Csv,
        }
    }
}

/// Telemetry action for the CLI subcommand.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum TelemetryAction {
    /// Show current telemetry status
    #[default]
    Status,
    /// Enable telemetry (opt-in)
    Enable,
    /// Disable telemetry
    Disable,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate agent configs
    Validate {
        /// Paths to validate (one or more files/directories)
        #[arg(default_value = ".", num_args = 1..)]
        paths: Vec<PathBuf>,
    },

    /// Initialize config file
    Init {
        /// Output path for config
        #[arg(default_value = ".agnix.toml")]
        output: PathBuf,
    },

    /// Evaluate rule efficacy against labeled test cases
    Eval {
        /// Path to evaluation manifest (YAML file)
        path: PathBuf,

        /// Output format (markdown, json, csv)
        #[arg(long, short, value_enum, default_value_t = EvalOutputFormat::Markdown)]
        format: EvalOutputFormat,

        /// Filter to specific rule prefix (e.g., "AS-", "MCP-")
        #[arg(long)]
        filter: Option<String>,

        /// Show detailed results for each case
        #[arg(long, short)]
        verbose: bool,
    },

    /// Manage telemetry settings (opt-in usage analytics)
    Telemetry {
        /// Action to perform (status, enable, disable)
        #[arg(value_enum, default_value_t = TelemetryAction::Status)]
        action: TelemetryAction,
    },

    /// Output JSON Schema for configuration files
    Schema {
        /// Output file path (defaults to stdout, or `schemas/agnix.json` with --fix)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Regenerate the schema file in place iff it differs from the current
        /// binary's output. Silent when unchanged, prints a notice when it writes.
        ///
        /// Designed for pre-commit hooks: the hook calls `agnix schema --fix`,
        /// pre-commit fails automatically if the working tree has a post-hook
        /// diff. Defaults `--output` to `schemas/agnix.json` when that flag is
        /// absent.
        #[arg(long)]
        fix: bool,
    },

    /// Check or detect tool versions against `.tool_versions` in `.agnix.toml`
    #[command(subcommand)]
    Tools(ToolsCommand),
}

/// `agnix tools` subcommands.
///
/// Keeps version pins in `.agnix.toml` honest by cross-referencing them
/// against the CLIs actually installed on PATH.
#[derive(Subcommand)]
enum ToolsCommand {
    /// Compare pinned versions in .agnix.toml against installed CLIs on PATH
    Check {
        /// Exit non-zero when drift or missing CLI is detected. Without this
        /// flag, drift is reported as a warning and the command always exits 0.
        /// Useful in pre-commit or CI when you want to gate on version pin
        /// correctness.
        #[arg(long)]
        strict: bool,
    },
    /// Detect installed tool versions and emit a [tool_versions] TOML snippet
    Detect {
        /// Merge the detected versions into `.agnix.toml`'s [tool_versions]
        /// section in place. Preserves comments, blank lines, and unrelated
        /// keys inside the section. Appends the section if absent.
        #[arg(long)]
        write: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    // Handle --list-locales before anything else
    if cli.list_locales {
        locale::print_supported_locales();
        return;
    }

    // Initialize locale (--locale flag > env var > system locale > "en")
    // Config locale will be applied later when config is loaded
    locale::init(cli.locale.as_deref(), None);

    // Initialize tracing for verbose mode (only for text output to avoid corrupting JSON/SARIF)
    if cli.verbose && matches!(cli.format, OutputFormat::Text) {
        use tracing_subscriber::{EnvFilter, fmt, prelude::*};

        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("agnix=debug,agnix_core=debug"));

        tracing_subscriber::registry()
            .with(
                fmt::layer()
                    .with_target(true)
                    .with_level(true)
                    .with_writer(std::io::stderr),
            )
            .with(filter)
            .init();

        tracing::debug!("Verbose mode enabled");
    }

    // Load config early for watch mode to apply config-based locale
    // Watch mode doesn't allow format or fix flags, so we can safely load config here
    if cli.watch {
        let primary = primary_path(&cli.paths);
        let config_path = resolve_config_path(primary, cli.config.as_ref());
        let (config, _) = LintConfig::load_or_default(config_path.as_ref());

        // Re-initialize locale if config specifies one and no --locale flag was given
        if cli.locale.is_none() {
            if let Some(config_locale) = config.locale() {
                locale::init(None, Some(config_locale));
            }
        }
    }

    let result = match &cli.command {
        Some(Commands::Validate { paths }) => validate_command(paths, &cli),
        Some(Commands::Init { output }) => init_command(output),
        Some(Commands::Eval {
            path,
            format,
            filter,
            verbose,
        }) => eval_command(path, *format, filter.as_deref(), *verbose),
        Some(Commands::Telemetry { action }) => telemetry_command(*action),
        Some(Commands::Schema { output, fix }) => schema_command(output.as_ref(), *fix),
        Some(Commands::Tools(subcmd)) => tools_command(subcmd, &cli),
        None => validate_command(&cli.paths, &cli),
    };

    if let Err(e) = result {
        eprintln!("{} {}", t!("cli.error_label").red().bold(), e);
        process::exit(1);
    }
}

/// Pick a representative path for config resolution, SARIF roots, etc.
///
/// When the user passes multiple paths (common with pre-commit, which expands
/// the changed files into positional args), we still need a single path for
/// things like finding `.agnix.toml` or the git root. The first path is a
/// reasonable default; when no paths are given we fall back to `.`.
fn primary_path(paths: &[PathBuf]) -> &Path {
    paths
        .first()
        .map(PathBuf::as_path)
        .unwrap_or_else(|| Path::new("."))
}

fn format_paths_for_display(paths: &[PathBuf]) -> String {
    match paths.len() {
        0 => ".".to_string(),
        1 => paths[0].display().to_string(),
        _ => format!("{} paths", paths.len()),
    }
}

/// Dispatch validation across one or more paths.
///
/// - A single directory path uses the full project walk (`validate_project`).
/// - Otherwise each path is handled individually: files run through
///   `validate_file_with_registry`, directories fall back to the project walk.
///
/// This lets pre-commit style invocations like
/// `agnix --strict AGENTS.md CLAUDE.md` check only the changed files instead
/// of rescanning the entire repo.
///
/// Before iterating we resolve a workspace root and set it on a cloned
/// `LintConfig` so per-file validators see the same `[files]` include/exclude
/// semantics they'd see during a full project walk. The aggregate
/// `files_checked` count is also bounded by `max_files_to_validate` so a large
/// file list from pre-commit can't bypass the DoS guard.
fn run_validation(
    paths: &[PathBuf],
    config: &LintConfig,
) -> agnix_core::LintResult<ValidationResult> {
    use agnix_core::{CoreError, ValidationError};

    // Surface the same RootNotFound error as validate_project for paths that
    // don't exist, so CLI behaviour stays consistent.
    for path in paths {
        if !path.exists() {
            return Err(CoreError::Validation(ValidationError::RootNotFound {
                path: path.clone(),
            }));
        }
    }

    if paths.len() == 1 && paths[0].is_dir() {
        return validate_project(&paths[0], config);
    }

    // Resolve a workspace root so per-file validators can apply relative
    // `[files]` patterns and so diagnostics use a stable base path. Use the
    // parent of the first file path, or the first directory path, or cwd.
    let root_dir = paths
        .first()
        .map(|p| {
            if p.is_dir() {
                p.clone()
            } else {
                p.parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| PathBuf::from("."))
            }
        })
        .and_then(|p| std::fs::canonicalize(&p).ok())
        .unwrap_or_else(|| std::fs::canonicalize(".").unwrap_or_else(|_| PathBuf::from(".")));

    let mut config = config.clone();
    config.set_root_dir(root_dir);

    let mut registry = ValidatorRegistry::with_defaults();
    for name in &config.rules().disabled_validators {
        registry.disable_validator_owned(name);
    }

    let max_files = config.max_files_to_validate();
    let mut diagnostics = Vec::new();
    let mut files_checked = 0usize;
    for path in paths {
        if let Some(limit) = max_files {
            if files_checked >= limit {
                return Err(CoreError::Validation(ValidationError::TooManyFiles {
                    count: files_checked,
                    limit,
                }));
            }
        }
        if path.is_dir() {
            let r = validate_project_with_registry(path, &config, &registry)?;
            files_checked += r.files_checked;
            diagnostics.extend(r.diagnostics);
            continue;
        }
        match validate_file_with_registry(path, &config, &registry)? {
            ValidationOutcome::Success(d) => {
                files_checked += 1;
                diagnostics.extend(d);
            }
            ValidationOutcome::Skipped => {}
            ValidationOutcome::IoError(err) => {
                // Propagate so JSON/SARIF consumers see the failure in exit
                // code and text consumers get a clear error instead of a
                // silent warning that still exits 0.
                return Err(CoreError::File(err));
            }
            _ => {}
        }
    }
    Ok(ValidationResult::new(diagnostics, files_checked))
}

fn count_errors_warnings(diagnostics: &[Diagnostic]) -> (usize, usize) {
    let errors = diagnostics
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Warning)
        .count();
    (errors, warnings)
}

#[tracing::instrument(skip(cli), fields(paths_count = paths.len()))]
fn validate_command(paths: &[PathBuf], cli: &Cli) -> anyhow::Result<()> {
    tracing::debug!("Starting validation");

    // Watch mode validation
    if cli.watch {
        if !matches!(cli.format, OutputFormat::Text) {
            return Err(anyhow::anyhow!("{}", t!("cli.watch_error_text_only")));
        }
        let should_fix = cli.fix || cli.fix_safe || cli.fix_unsafe || cli.dry_run;
        if should_fix {
            return Err(anyhow::anyhow!("{}", t!("cli.watch_error_fix")));
        }
        if paths.len() > 1 {
            return Err(anyhow::anyhow!("{}", t!("cli.watch_error_single_path")));
        }

        let path = primary_path(paths).to_path_buf();
        let path_for_watch = path.clone();
        let strict = cli.strict;
        let verbose = cli.verbose;
        let target = cli.target;
        let config_override = cli.config.clone();

        return watch::watch_and_validate(&path_for_watch, move || {
            run_single_validation(&path, strict, verbose, target, config_override.as_ref())
        });
    }

    let primary = primary_path(paths);
    let config_path = resolve_config_path(primary, cli.config.as_ref());
    tracing::debug!(config_path = ?config_path, "Resolved config path");

    let (mut config, config_warning) = LintConfig::load_or_default(config_path.as_ref());

    // Re-initialize locale if config specifies one and no --locale flag was given
    if cli.locale.is_none() {
        if let Some(config_locale) = config.locale() {
            locale::init(None, Some(config_locale));
        }
    }

    // Display config warning before validation output
    if let Some(warning) = config_warning {
        eprintln!("{} {}", t!("cli.warning_label").yellow().bold(), warning);
        eprintln!();
    }
    config.set_target(cli.target.into());

    // Validate config semantics and display warnings (only for text output)
    if matches!(cli.format, OutputFormat::Text) {
        let config_warnings = config.validate();
        if !config_warnings.is_empty() {
            for warning in &config_warnings {
                eprintln!(
                    "{} [{}] {}",
                    t!("cli.config_warning_label").yellow().bold(),
                    warning.field.dimmed(),
                    warning.message
                );
                if let Some(suggestion) = &warning.suggestion {
                    eprintln!("  {} {}", t!("cli.hint_label").cyan(), suggestion);
                }
            }
            eprintln!();
        }
    }

    // Apply --max-files override if specified
    if let Some(max_files) = cli.max_files {
        // 0 means disable the limit (not recommended for security)
        if max_files == 0 {
            eprintln!(
                "{} --max-files=0 disables file count protection. This may allow DoS via large projects.",
                "Warning:".yellow().bold()
            );
            config.set_max_files_to_validate(None);
        } else if max_files > 1_000_000 {
            // Warn on very high limits (>1M files is likely a mistake or attack)
            eprintln!(
                "{} --max-files={} is very high. Consider using the default (10,000) for better performance.",
                "Warning:".yellow().bold(),
                max_files
            );
            config.set_max_files_to_validate(Some(max_files));
        } else {
            config.set_max_files_to_validate(Some(max_files));
        }
    }
    let should_fix = cli.fix || cli.fix_safe || cli.fix_unsafe || cli.dry_run;
    if should_fix && !matches!(cli.format, OutputFormat::Text) {
        return Err(anyhow::anyhow!("{}", t!("cli.fix_error_text_only")));
    }

    // Resolve absolute path for consistent relative output.
    // SARIF uses the git repository root so artifact URIs are relative to the
    // workspace root, which IDEs expect. Text/JSON use CWD for backwards compatibility.
    let base_path = if matches!(cli.format, OutputFormat::Sarif) {
        sarif::find_git_root(primary)
            .unwrap_or_else(|| std::fs::canonicalize(".").unwrap_or_else(|_| PathBuf::from(".")))
    } else {
        std::fs::canonicalize(".").unwrap_or_else(|_| PathBuf::from("."))
    };

    // For machine-readable output (JSON/SARIF), force English locale so that
    // diagnostic messages are always in English for tooling interoperability.
    // Save and restore the user's locale so that any subsequent stderr output
    // (e.g., error messages) remains in their chosen locale.
    let is_machine_output = matches!(cli.format, OutputFormat::Json | OutputFormat::Sarif);
    let saved_locale = if is_machine_output {
        let current = rust_i18n::locale().to_string();
        rust_i18n::set_locale("en");
        Some(current)
    } else {
        None
    };

    // Time the validation for telemetry
    let validation_start = Instant::now();

    let ValidationResult {
        diagnostics,
        files_checked,
        ..
    } = run_validation(paths, &config)?;

    // Restore user locale after validation so stderr messages use their language
    if let Some(ref locale) = saved_locale {
        rust_i18n::set_locale(locale);
    }

    let validation_duration = validation_start.elapsed();

    tracing::debug!(
        files_checked = files_checked,
        diagnostics_count = diagnostics.len(),
        "Validation complete"
    );

    // Record telemetry (non-blocking, respects opt-in)
    record_telemetry_event(&diagnostics, validation_duration);

    // Handle JSON output format
    if matches!(cli.format, OutputFormat::Json) {
        let json_output = json::diagnostics_to_json(&diagnostics, &base_path, files_checked);
        let json_str = serde_json::to_string_pretty(&json_output)?;
        println!("{}", json_str);

        // Exit with error code if there are errors (use summary to avoid re-iterating)
        if json_output.summary.errors > 0 || (cli.strict && json_output.summary.warnings > 0) {
            process::exit(1);
        }
        return Ok(());
    }

    // Handle SARIF output format
    if matches!(cli.format, OutputFormat::Sarif) {
        let sarif = sarif::diagnostics_to_sarif(&diagnostics, &base_path);
        let json = serde_json::to_string_pretty(&sarif)?;
        println!("{}", json);

        // Exit with error code if there are errors
        let has_errors = diagnostics
            .iter()
            .any(|d| d.level == DiagnosticLevel::Error);
        let has_warnings = diagnostics
            .iter()
            .any(|d| d.level == DiagnosticLevel::Warning);

        if has_errors || (cli.strict && has_warnings) {
            process::exit(1);
        }
        return Ok(());
    }

    // Text output format
    println!(
        "{} {}",
        t!("cli.validating").cyan().bold(),
        format_paths_for_display(paths)
    );
    println!();

    if diagnostics.is_empty() {
        println!("{}", t!("cli.no_issues_found").green().bold());
        return Ok(());
    }

    let (errors, warnings) = count_errors_warnings(&diagnostics);
    let infos = diagnostics
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Info)
        .count();
    let fixable = diagnostics.iter().filter(|d| d.has_fixes()).count();

    for diag in &diagnostics {
        let level_str = match diag.level {
            DiagnosticLevel::Error => "error".red().bold(),
            DiagnosticLevel::Warning => "warning".yellow().bold(),
            DiagnosticLevel::Info => "info".blue().bold(),
        };

        let fixable_marker = if diag.has_fixes() {
            format!(" {}", t!("cli.fixable")).green().to_string()
        } else {
            String::new()
        };

        println!(
            "{}:{}:{} {}: {}{}",
            diag.file.display().to_string().dimmed(),
            diag.line,
            diag.column,
            level_str,
            diag.message,
            fixable_marker
        );

        if cli.verbose {
            println!("  {} {}", t!("cli.rule_label").dimmed(), diag.rule.dimmed());
            if let Some(ref meta) = diag.metadata {
                let tool_info = match &meta.applies_to_tool {
                    Some(tool) => tool.as_str().into(),
                    None => t!("cli.generic_tool"),
                };
                println!(
                    "  {} {} | {} {} | {} {}",
                    t!("cli.category_label").dimmed(),
                    meta.category,
                    t!("cli.severity_label").dimmed(),
                    meta.severity,
                    t!("cli.tool_label").dimmed(),
                    tool_info
                );
            }
            if let Some(suggestion) = &diag.suggestion {
                println!("  {} {}", t!("cli.help_label").cyan(), suggestion);
            }
            if let Some(assumption) = &diag.assumption {
                println!("  {} {}", t!("cli.note_label").yellow(), assumption);
            }
        }

        if cli.verbose || cli.show_fixes {
            for fix in &diag.fixes {
                let tier = confidence_tier_label(fix.confidence_tier());
                let confidence_pct = (fix.confidence_score() * 100.0).round() as i32;
                let mut qualifiers = Vec::new();
                if let Some(group) = fix.group.as_deref() {
                    qualifiers.push(format!("group={group}"));
                }
                if let Some(depends_on) = fix.depends_on.as_deref() {
                    qualifiers.push(format!("depends_on={depends_on}"));
                }
                let qualifier_text = if qualifiers.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", qualifiers.join(", "))
                };
                println!(
                    "  {} {} ({} {}%){}",
                    t!("cli.fix_label").green(),
                    fix.description,
                    tier,
                    confidence_pct,
                    qualifier_text
                );
            }
        }
        println!();
    }

    println!("{}", "-".repeat(60).dimmed());
    println!(
        "{}",
        t!(
            "cli.found_errors_warnings",
            errors = errors,
            error_word = if errors == 1 {
                t!("cli.error_singular")
            } else {
                t!("cli.error_plural")
            },
            warnings = warnings,
            warning_word = if warnings == 1 {
                t!("cli.warning_singular")
            } else {
                t!("cli.warning_plural")
            }
        )
    );

    if infos > 0 {
        println!("{}", t!("cli.info_messages", count = infos));
    }

    if fixable > 0 {
        println!(
            "{}",
            t!(
                "cli.fixable_issues",
                count = fixable,
                word = if fixable == 1 {
                    t!("cli.issue_is")
                } else {
                    t!("cli.issues_are")
                }
            )
        );
    }

    let mut final_errors = errors;
    let mut final_warnings = warnings;

    if should_fix {
        let apply_mode = resolve_fix_mode(cli);
        println!();
        let action_mode = if cli.dry_run {
            t!("cli.preview")
        } else {
            t!("cli.applying")
        };
        let confidence_mode: String = match apply_mode {
            FixApplyMode::SafeOnly => t!("cli.safe_only").to_string(),
            FixApplyMode::SafeAndMedium => " (safe + medium)".to_string(),
            FixApplyMode::All => " (all confidence levels)".to_string(),
        };
        println!(
            "{}",
            t!(
                "cli.applying_fixes",
                mode = action_mode.cyan().bold(),
                safe_mode = confidence_mode
            )
        );

        let results =
            apply_fixes_with_options(&diagnostics, FixApplyOptions::new(cli.dry_run, apply_mode))?;

        if results.is_empty() {
            println!("{}", t!("cli.no_fixes"));
        } else {
            for result in &results {
                println!();
                println!(
                    "  {} {}",
                    if cli.dry_run {
                        t!("cli.would_fix")
                    } else {
                        t!("cli.fixed")
                    }
                    .green(),
                    result.path.display()
                );
                for desc in &result.applied {
                    println!("    - {}", desc);
                }

                if cli.dry_run && cli.verbose {
                    println!();
                    println!("  {}:", t!("cli.diff_label").yellow());
                    show_diff(&result.original, &result.fixed);
                }
            }

            println!();
            let action = if cli.dry_run {
                t!("cli.would_fix")
            } else {
                t!("cli.fixed")
            };
            println!(
                "{}",
                t!(
                    "cli.fix_summary",
                    action = action.green().bold(),
                    count = results.len(),
                    word = if results.len() == 1 {
                        t!("cli.file_singular")
                    } else {
                        t!("cli.file_plural")
                    }
                )
            );
        }

        // Re-run validation after applying fixes so exit code reflects remaining issues.
        if !cli.dry_run {
            let ValidationResult {
                diagnostics: post_fix_diagnostics,
                files_checked: _,
                ..
            } = run_validation(paths, &config)?;

            (final_errors, final_warnings) = count_errors_warnings(&post_fix_diagnostics);
        }
    } else if fixable > 0 {
        println!();
        println!(
            "{} {}",
            t!("cli.hint_label").cyan(),
            t!(
                "cli.hint_run_fix",
                flag = "--fix / --fix-safe / --fix-unsafe".bold()
            )
        );
    }

    // Exit with error if errors remain (even after fixing) or strict mode with warnings
    if final_errors > 0 || (cli.strict && final_warnings > 0) {
        process::exit(1);
    }

    Ok(())
}

/// Run a single validation pass (for watch mode)
/// Returns true if there are errors
fn run_single_validation(
    path: &Path,
    strict: bool,
    verbose: bool,
    target: TargetArg,
    config_override: Option<&PathBuf>,
) -> anyhow::Result<bool> {
    let config_path = resolve_config_path(path, config_override);

    let (mut config, config_warning) = LintConfig::load_or_default(config_path.as_ref());

    if let Some(warning) = config_warning {
        eprintln!("{} {}", t!("cli.warning_label").yellow().bold(), warning);
        eprintln!();
    }
    config.set_target(target.into());

    let ValidationResult {
        diagnostics,
        files_checked: _,
        ..
    } = validate_project(path, &config)?;

    println!("{} {}", t!("cli.validating").cyan().bold(), path.display());
    println!();

    if diagnostics.is_empty() {
        println!("{}", t!("cli.no_issues_found").green().bold());
        return Ok(false);
    }

    let errors = diagnostics
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Warning)
        .count();

    for diag in &diagnostics {
        let level_str = match diag.level {
            DiagnosticLevel::Error => "error".red().bold(),
            DiagnosticLevel::Warning => "warning".yellow().bold(),
            DiagnosticLevel::Info => "info".blue().bold(),
        };

        println!(
            "{}:{}:{} {}: {}",
            diag.file.display().to_string().dimmed(),
            diag.line,
            diag.column,
            level_str,
            diag.message,
        );

        if verbose {
            println!("  {} {}", t!("cli.rule_label").dimmed(), diag.rule.dimmed());
            if let Some(suggestion) = &diag.suggestion {
                println!("  {} {}", t!("cli.help_label").cyan(), suggestion);
            }
        }
        println!();
    }

    println!("{}", "-".repeat(60).dimmed());
    println!(
        "{}",
        t!(
            "cli.found_errors_warnings",
            errors = errors,
            error_word = if errors == 1 {
                t!("cli.error_singular")
            } else {
                t!("cli.error_plural")
            },
            warnings = warnings,
            warning_word = if warnings == 1 {
                t!("cli.warning_singular")
            } else {
                t!("cli.warning_plural")
            }
        )
    );

    Ok(errors > 0 || (strict && warnings > 0))
}

fn resolve_config_path(path: &Path, config_override: Option<&PathBuf>) -> Option<PathBuf> {
    if let Some(config) = config_override {
        return Some(config.clone());
    }

    let mut candidates = Vec::new();
    if path.is_dir() {
        candidates.push(path.to_path_buf());
    } else if let Some(parent) = path.parent() {
        candidates.push(parent.to_path_buf());
    }

    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd);
    }

    for dir in candidates {
        let candidate = dir.join(".agnix.toml");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

fn resolve_fix_mode(cli: &Cli) -> FixApplyMode {
    if cli.fix_safe {
        FixApplyMode::SafeOnly
    } else if cli.fix_unsafe {
        FixApplyMode::All
    } else {
        // Default for --fix and --dry-run.
        FixApplyMode::SafeAndMedium
    }
}

fn confidence_tier_label(tier: FixConfidenceTier) -> &'static str {
    match tier {
        FixConfidenceTier::High => "HIGH",
        FixConfidenceTier::Medium => "MEDIUM",
        FixConfidenceTier::Low => "LOW",
    }
}

fn show_diff(original: &str, fixed: &str) {
    let diff = TextDiff::from_lines(original, fixed);
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => print!("    {} {}", "-".red(), change.to_string().red()),
            ChangeTag::Insert => print!("    {} {}", "+".green(), change.to_string().green()),
            ChangeTag::Equal => {}
        }
    }
}

fn init_command(output: &PathBuf) -> anyhow::Result<()> {
    let default_config = LintConfig::default();
    let toml_content = toml::to_string_pretty(&default_config)?;

    std::fs::write(output, toml_content)?;

    println!("{} {}", t!("cli.created").green().bold(), output.display());

    Ok(())
}

/// Default schema file path when `--fix` is passed without `--output`.
const DEFAULT_SCHEMA_PATH: &str = "schemas/agnix.json";

fn schema_command(output: Option<&PathBuf>, fix: bool) -> anyhow::Result<()> {
    let schema = generate_schema();
    // `schema_for!` emits a trailing newline-less JSON object. Add a trailing
    // newline so the on-disk file looks like any other text file the user's
    // editor/formatter would produce - avoids spurious diffs.
    let json = format!("{}\n", serde_json::to_string_pretty(&schema)?);

    // Resolve the target path: explicit --output wins, otherwise --fix implies
    // the default path. Without either, we stream to stdout.
    let target: Option<PathBuf> = match (output, fix) {
        (Some(path), _) => Some(path.clone()),
        (None, true) => Some(PathBuf::from(DEFAULT_SCHEMA_PATH)),
        (None, false) => None,
    };

    match target {
        Some(path) => {
            if fix {
                // Conditional overwrite: read the current file, compare. Only
                // write when it differs. Pre-commit's contract ("fail if the
                // tree is dirty after hooks") handles the drift detection for
                // us - we just make sure the write is idempotent-in-effect.
                let current = std::fs::read_to_string(&path).ok();
                if current.as_deref() == Some(json.as_str()) {
                    // Silent on no-op so the hook stays quiet on clean runs.
                    return Ok(());
                }
                // Ensure parent directory exists so a fresh checkout with no
                // `schemas/` folder still succeeds on the first `--fix`.
                if let Some(parent) = path.parent()
                    && !parent.as_os_str().is_empty()
                {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, &json)?;
                let action_label = if current.is_some() {
                    t!("cli.schema_updated")
                } else {
                    t!("cli.schema_created")
                };
                println!("{} {}", action_label.green().bold(), path.display());
            } else {
                // Legacy behavior for `agnix schema --output <path>`:
                // always write, print "Schema written to: path".
                std::fs::write(&path, &json)?;
                println!(
                    "{} {}",
                    t!("cli.schema_written").green().bold(),
                    path.display()
                );
            }
        }
        None => {
            // stdout is not newline-terminated twice - println! adds one, and
            // json already ends with \n. Use print! to avoid a double newline.
            print!("{}", json);
        }
    }

    Ok(())
}

/// `agnix tools` dispatch. Loads the user's `.agnix.toml` (if present) for
/// `check`; `detect` doesn't need it (scans PATH independently). Returns
/// a non-zero exit code when `check --strict` finds drift.
fn tools_command(subcmd: &ToolsCommand, cli: &Cli) -> anyhow::Result<()> {
    match subcmd {
        ToolsCommand::Check { strict } => {
            // Prefer the user's explicit --config; otherwise search from cwd.
            let cwd = env::current_dir()?;
            let config_path = resolve_config_path(&cwd, cli.config.as_ref());
            let (config, config_warning) = LintConfig::load_or_default(config_path.as_ref());
            // Surface any config-load warning (parse error, unknown keys,
            // etc.). Without this, `check` would silently fall back to
            // defaults + report nothing-pinned even when the user's config
            // is broken.
            if let Some(warning) = config_warning {
                eprintln!("{} {}", t!("cli.warning_label").yellow().bold(), warning);
                eprintln!();
            }
            let issues_found = tools::check_command(&config, *strict)?;
            if *strict && issues_found {
                process::exit(1);
            }
        }
        ToolsCommand::Detect { write } => {
            // `detect --write` targets the same config path `check` would read.
            let cwd = env::current_dir()?;
            let config_path = resolve_config_path(&cwd, cli.config.as_ref());
            tools::detect_command(config_path.as_deref(), *write)?;
        }
    }
    Ok(())
}

fn eval_command(
    path: &Path,
    format: EvalOutputFormat,
    filter: Option<&str>,
    verbose: bool,
) -> anyhow::Result<()> {
    let config = LintConfig::default();

    println!("{} {}", t!("cli.evaluating").cyan().bold(), path.display());
    if let Some(f) = filter {
        println!("  {} {}", t!("cli.filter_label").dimmed(), f);
    }
    println!();

    let (results, summary) = evaluate_manifest_file(path, &config, filter)?;

    // Show verbose per-case results if requested
    if verbose {
        println!("{}", t!("cli.per_case_results").cyan().bold());
        println!("{}", "=".repeat(60).dimmed());

        for result in &results {
            let status = if result.passed() {
                t!("cli.pass").green().bold()
            } else {
                t!("cli.fail").red().bold()
            };

            println!("[{}] {}", status, result.case.file.display());

            if let Some(desc) = &result.case.description {
                println!("     {}", desc.dimmed());
            }

            if !result.passed() {
                if !result.false_positives.is_empty() {
                    println!(
                        "     {} {:?}",
                        t!("cli.unexpected_label").yellow(),
                        result.false_positives
                    );
                }
                if !result.false_negatives.is_empty() {
                    println!(
                        "     {} {:?}",
                        t!("cli.missing_label").red(),
                        result.false_negatives
                    );
                }
            }
            println!();
        }

        println!("{}", "=".repeat(60).dimmed());
        println!();
    }

    // Output summary in requested format
    let eval_format: EvalFormat = format.into();
    match eval_format {
        EvalFormat::Json => {
            let json = summary.to_json()?;
            println!("{}", json);
        }
        EvalFormat::Csv => {
            let csv = summary.to_csv();
            println!("{}", csv);
        }
        EvalFormat::Markdown => {
            let md = summary.to_markdown();
            println!("{}", md);
        }
    }

    // Print final status
    println!();
    if summary.cases_failed == 0 {
        println!(
            "{} {}",
            t!("cli.success").green().bold(),
            t!("cli.all_cases_passed", count = summary.cases_run)
        );
    } else {
        println!(
            "{} {}",
            t!("cli.failed").red().bold(),
            t!(
                "cli.cases_failed",
                failed = summary.cases_failed,
                total = summary.cases_run
            )
        );
        process::exit(1);
    }

    Ok(())
}

/// Record telemetry event for a validation run (non-blocking, respects opt-in).
fn record_telemetry_event(diagnostics: &[agnix_core::Diagnostic], duration: std::time::Duration) {
    use agnix_core::DiagnosticLevel;

    // Count diagnostics by level
    let mut error_count = 0u32;
    let mut warning_count = 0u32;
    let mut info_count = 0u32;

    // Count rule triggers (privacy-safe: only rule IDs, not paths or messages)
    let mut rule_trigger_counts: HashMap<String, u32> = HashMap::new();

    for diag in diagnostics {
        match diag.level {
            DiagnosticLevel::Error => error_count += 1,
            DiagnosticLevel::Warning => warning_count += 1,
            DiagnosticLevel::Info => info_count += 1,
        }

        // Validate rule ID format before including (defense-in-depth)
        // This prevents any bugs in validators from leaking paths/sensitive data
        if telemetry::is_valid_rule_id(&diag.rule) {
            *rule_trigger_counts.entry(diag.rule.clone()).or_insert(0) += 1;
        }
    }

    // File type counts would require exposing file type info from agnix-core
    // For now, we don't collect file type counts to avoid any path exposure
    let file_type_counts: HashMap<String, u32> = HashMap::new();

    // Record the event (spawns background thread, checks if enabled)
    telemetry::record_validation(
        file_type_counts,
        rule_trigger_counts,
        error_count,
        warning_count,
        info_count,
        duration.as_millis() as u64,
    );
}

fn telemetry_command(action: TelemetryAction) -> anyhow::Result<()> {
    use telemetry::TelemetryConfig;

    match action {
        TelemetryAction::Status => {
            let config = TelemetryConfig::load().unwrap_or_default();
            let effective = config.is_enabled();

            println!("{}", t!("cli.telemetry_status").cyan().bold());
            println!();
            println!(
                "  {} {}",
                t!("cli.telemetry_configured").dimmed(),
                if config.enabled {
                    t!("cli.telemetry_enabled")
                } else {
                    t!("cli.telemetry_disabled")
                }
            );
            println!(
                "  {} {}",
                t!("cli.telemetry_effective").dimmed(),
                if effective {
                    t!("cli.telemetry_enabled")
                } else {
                    t!("cli.telemetry_disabled")
                }
            );

            if config.enabled && !effective {
                println!();
                println!(
                    "  {} {}",
                    t!("cli.note_label").yellow(),
                    t!("cli.telemetry_env_note")
                );
            }

            if let Some(id) = &config.installation_id {
                // Show only first 8 chars for privacy
                let short_id = if id.len() > 8 { &id[..8] } else { id };
                println!(
                    "  {} {}...",
                    t!("cli.telemetry_installation_id").dimmed(),
                    short_id
                );
            }

            if let Some(ts) = &config.consent_timestamp {
                println!("  {} {}", t!("cli.telemetry_consent_given").dimmed(), ts);
            }

            println!();
            println!("{}", t!("cli.telemetry_privacy").cyan().bold());
            println!("{}", t!("cli.telemetry_privacy_1"));
            println!("{}", t!("cli.telemetry_privacy_2"));
            println!("{}", t!("cli.telemetry_privacy_3"));
            println!("{}", t!("cli.telemetry_privacy_4"));
            println!("{}", t!("cli.telemetry_privacy_5"));

            if let Ok(path) = TelemetryConfig::config_path() {
                println!();
                println!(
                    "  {} {}",
                    t!("cli.telemetry_config_file").dimmed(),
                    path.display()
                );
            }
        }

        TelemetryAction::Enable => {
            let mut config = TelemetryConfig::load().unwrap_or_default();

            if config.enabled {
                println!(
                    "{} {}",
                    t!("cli.note_label").cyan(),
                    t!("cli.telemetry_already_enabled")
                );
            } else {
                config.enable()?;
                println!("{} {}", "OK".green().bold(), t!("cli.telemetry_ok_enabled"));
                println!();
                println!("{}", t!("cli.telemetry_thanks"));
                println!();
                println!("{}", t!("cli.telemetry_collect").cyan());
                println!("{}", t!("cli.telemetry_collect_1"));
                println!("{}", t!("cli.telemetry_collect_2"));
                println!("{}", t!("cli.telemetry_collect_3"));
                println!("{}", t!("cli.telemetry_collect_4"));
                println!();
                println!("{}", t!("cli.telemetry_never_collect").cyan());
                println!("{}", t!("cli.telemetry_never_1"));
                println!("{}", t!("cli.telemetry_never_2"));
                println!("{}", t!("cli.telemetry_never_3"));
                println!();
                println!(
                    "{}",
                    t!(
                        "cli.telemetry_disable_hint",
                        cmd = "agnix telemetry disable".bold()
                    )
                );
            }
        }

        TelemetryAction::Disable => {
            let mut config = TelemetryConfig::load().unwrap_or_default();

            if !config.enabled {
                println!(
                    "{} {}",
                    t!("cli.note_label").cyan(),
                    t!("cli.telemetry_already_disabled")
                );
            } else {
                config.disable()?;
                println!(
                    "{} {}",
                    "OK".green().bold(),
                    t!("cli.telemetry_ok_disabled")
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod resolve_fix_mode_tests {
    use super::*;

    #[test]
    fn fix_safe_selects_safe_only_mode() {
        let cli = Cli::parse_from(["agnix", "--fix-safe"]);
        assert_eq!(resolve_fix_mode(&cli), FixApplyMode::SafeOnly);
    }

    #[test]
    fn fix_unsafe_selects_all_mode() {
        let cli = Cli::parse_from(["agnix", "--fix-unsafe"]);
        assert_eq!(resolve_fix_mode(&cli), FixApplyMode::All);
    }

    #[test]
    fn fix_selects_safe_and_medium_mode() {
        let cli = Cli::parse_from(["agnix", "--fix"]);
        assert_eq!(resolve_fix_mode(&cli), FixApplyMode::SafeAndMedium);
    }

    #[test]
    fn dry_run_selects_safe_and_medium_mode() {
        let cli = Cli::parse_from(["agnix", "--dry-run"]);
        assert_eq!(resolve_fix_mode(&cli), FixApplyMode::SafeAndMedium);
    }

    #[test]
    fn dry_run_with_fix_safe_selects_safe_only_mode() {
        let cli = Cli::parse_from(["agnix", "--dry-run", "--fix-safe"]);
        assert_eq!(resolve_fix_mode(&cli), FixApplyMode::SafeOnly);
    }

    #[test]
    fn dry_run_with_fix_unsafe_selects_all_mode() {
        let cli = Cli::parse_from(["agnix", "--dry-run", "--fix-unsafe"]);
        assert_eq!(resolve_fix_mode(&cli), FixApplyMode::All);
    }
}
