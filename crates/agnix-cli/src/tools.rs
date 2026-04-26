//! `agnix tools` subcommand family.
//!
//! - `agnix tools check [--strict]` - compare `.tool_versions` in `.agnix.toml`
//!   against versions of the corresponding CLIs on PATH. Warn by default,
//!   fail with `--strict`.
//! - `agnix tools detect [--write]` - run `<cli> --version` for each supported
//!   tool found on PATH and print a TOML snippet. With `--write`, merge the
//!   detected versions into `.agnix.toml`'s `[tool_versions]` section.
//!
//! Why this exists: the `.tool_versions` block in `.agnix.toml` is easy to
//! forget to update when an upstream CLI is bumped (via mise, asdf, brew,
//! npm, cargo, etc.). This command family keeps the pin in sync so future
//! version-aware validators (tracked separately) have trustworthy inputs.
//!
//! Design decisions resolved with @petemounce in #717:
//! - Check mode defaults to warn, `--strict` flag fails (petemounce preferred
//!   fail-by-default; we went with warn-by-default + flag so CI workflows that
//!   don't pin versions don't all break on upgrade - `--strict` in pre-commit
//!   or strict CI gives the fail-by-default experience).
//! - Exact-match only. Range matching would need a decision on range syntax
//!   and how to map `~=0.21` to a validator's version-specific rule set; out
//!   of scope here.
//! - Tool discovery is PATH-based. Mise/asdf shim PATH automatically, so
//!   users of those toolchain managers get it for free without agnix
//!   depending on any specific tool.

use anyhow::{Context, Result};
use colored::Colorize;
use rust_i18n::t;
use std::path::{Path, PathBuf};
use std::process::Command;

use agnix_core::config::LintConfig;

/// Mapping from agnix's ToolVersions field -> one or more CLI invocations.
///
/// Each entry in `invocations` is tried in order; the first that produces
/// a semver-shaped token wins. Needed because some tools have multiple
/// installation paths (e.g., GitHub Copilot ships as both a standalone
/// `copilot` npm shim and as a `gh copilot` extension).
struct ToolDescriptor {
    /// `ToolVersions` field name as it appears in `.agnix.toml`.
    toml_key: &'static str,
    /// Display name for UI (e.g., "Claude Code").
    display: &'static str,
    /// Ordered list of (binary, args) pairs to try. First hit wins.
    invocations: &'static [(&'static str, &'static [&'static str])],
}

/// Supported tools for `agnix tools check` / `detect`. Deliberately scoped
/// to the fields that exist in `ToolVersions` today (claude_code, codex,
/// cursor, copilot). Expanding `ToolVersions` to cover all 11 validated
/// tools is a separate refactor; this command family follows the config
/// struct rather than leading it.
const DESCRIPTORS: &[ToolDescriptor] = &[
    ToolDescriptor {
        toml_key: "claude_code",
        display: "Claude Code",
        invocations: &[("claude", &["--version"])],
    },
    ToolDescriptor {
        toml_key: "codex",
        display: "Codex CLI",
        invocations: &[("codex", &["--version"])],
    },
    ToolDescriptor {
        toml_key: "cursor",
        display: "Cursor",
        invocations: &[("cursor", &["--version"])],
    },
    ToolDescriptor {
        toml_key: "copilot",
        display: "GitHub Copilot",
        // Try the standalone npm shim first, then fall back to the
        // `gh copilot` extension that many users install via
        // `gh extension install github/gh-copilot`.
        invocations: &[
            ("copilot", &["--version"]),
            ("gh", &["copilot", "--version"]),
        ],
    },
];

/// What the user has pinned in `.agnix.toml`, per tool.
fn config_version_for(config: &LintConfig, key: &str) -> Option<String> {
    let tv = config.tool_versions();
    match key {
        "claude_code" => tv.claude_code.clone(),
        "codex" => tv.codex.clone(),
        "cursor" => tv.cursor.clone(),
        "copilot" => tv.copilot.clone(),
        _ => None,
    }
}

/// Try each `(binary, args)` pair in `invocations` in order. The first
/// one that runs successfully AND produces a semver-shaped token wins.
/// Returns None when every invocation fails (binary not on PATH or
/// command error) or when no output contains a version token.
///
/// Combining stdout and stderr matters for tools like `claude --version`
/// that print to one, and potentially logs to the other.
fn detect_installed(invocations: &[(&'static str, &'static [&'static str])]) -> Option<String> {
    for (binary, args) in invocations {
        let Ok(out) = Command::new(binary).args(*args).output() else {
            continue;
        };
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if let Some(v) = extract_version(&combined) {
            return Some(v);
        }
    }
    None
}

/// Extract the first SemVer-ish token from arbitrary output.
///
/// Matches `<digits>.<digits>.<digits>` with optional pre-release AND
/// build-metadata suffix per SemVer 2.0.0:
///   version = core [ "-" pre-release ] [ "+" build ]
/// So `1.2.3-alpha+build` captures the full string. Either suffix is
/// optional; both can appear in order.
///
/// Deliberately lenient about the pre-release/build grammar itself
/// (accepting `[0-9A-Za-z.-]+` / `[0-9A-Za-z.+-]+`) - CLIs play fast and
/// loose with these.
fn extract_version(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            // Scan major.
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i >= bytes.len() || bytes[i] != b'.' {
                continue;
            }
            i += 1;
            // Scan minor.
            let minor_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i == minor_start || i >= bytes.len() || bytes[i] != b'.' {
                continue;
            }
            i += 1;
            // Scan patch (required for N.N.N match).
            let patch_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i == patch_start {
                continue;
            }
            // Pre-release: `-` followed by `[0-9A-Za-z.-]+`. SemVer 2.0.0
            // forbids `+` inside the pre-release identifier (that's where
            // build metadata starts), so stop the pre-release scan at `+`.
            if i < bytes.len() && bytes[i] == b'-' {
                i += 1;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'.' || bytes[i] == b'-')
                {
                    i += 1;
                }
            }
            // Build metadata: `+` followed by `[0-9A-Za-z.-]+`. Can follow
            // pre-release or stand alone (e.g., `1.2.3+build.5`).
            if i < bytes.len() && bytes[i] == b'+' {
                i += 1;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'.' || bytes[i] == b'-')
                {
                    i += 1;
                }
            }
            return Some(s[start..i].to_string());
        }
        i += 1;
    }
    None
}

/// Outcome of comparing one tool's pinned vs. installed version.
#[derive(Debug, PartialEq, Eq)]
enum CheckOutcome {
    /// Both pinned and installed, and they match. No diagnostic.
    Match { version: String },
    /// Pinned value differs from installed. Needs a warning (or error under --strict).
    Drift { pinned: String, installed: String },
    /// Installed but nothing pinned. Informational - detect would offer to fill it in.
    Unpinned { installed: String },
    /// Pinned but CLI is not on PATH. Potential silent drift; warn.
    Missing { pinned: String },
    /// Neither pinned nor installed. Silent.
    Neither,
}

fn classify(pinned: Option<String>, installed: Option<String>) -> CheckOutcome {
    match (pinned, installed) {
        (Some(p), Some(i)) if p == i => CheckOutcome::Match { version: p },
        (Some(p), Some(i)) => CheckOutcome::Drift {
            pinned: p,
            installed: i,
        },
        (None, Some(i)) => CheckOutcome::Unpinned { installed: i },
        (Some(p), None) => CheckOutcome::Missing { pinned: p },
        (None, None) => CheckOutcome::Neither,
    }
}

/// Result of running `check` across every descriptor.
struct CheckReport {
    /// Whether any Drift or Missing was found.
    has_issues: bool,
}

/// Print a report line for one descriptor + outcome. Kept short + aligned
/// so `--strict` mode's failure summary is grep-able.
fn print_check_line(descriptor: &ToolDescriptor, outcome: &CheckOutcome) {
    match outcome {
        CheckOutcome::Match { version } => {
            println!(
                "  {} {} pinned={} installed={}",
                "[ok]".green().bold(),
                descriptor.display,
                version,
                version
            );
        }
        CheckOutcome::Drift { pinned, installed } => {
            println!(
                "  {} {} pinned={} installed={} {}",
                "[drift]".yellow().bold(),
                descriptor.display,
                pinned,
                installed,
                t!("cli.tools_check_drift_hint").dimmed()
            );
        }
        CheckOutcome::Unpinned { installed } => {
            println!(
                "  {} {} installed={} ({})",
                "[unpinned]".dimmed(),
                descriptor.display,
                installed,
                t!("cli.tools_check_unpinned_hint")
            );
        }
        CheckOutcome::Missing { pinned } => {
            println!(
                "  {} {} pinned={} {}",
                "[missing]".yellow().bold(),
                descriptor.display,
                pinned,
                t!("cli.tools_check_missing_hint")
            );
        }
        CheckOutcome::Neither => { /* silent */ }
    }
}

/// Run `check`. Returns Ok(true) if any issues were found (for --strict
/// exit code decision), Ok(false) otherwise. Errors propagate through
/// `?` for unexpected I/O problems.
pub fn check_command(config: &LintConfig, strict: bool) -> Result<bool> {
    println!("{}", t!("cli.tools_check_header").bold());
    let report = run_check(config);

    if report.has_issues {
        let msg = t!("cli.tools_check_issues_found");
        if strict {
            eprintln!("\n{} {}", "[error]".red().bold(), msg);
            return Ok(true);
        } else {
            eprintln!("\n{} {}", "[warn]".yellow().bold(), msg);
            eprintln!("        {}", t!("cli.tools_check_strict_hint").dimmed());
        }
    } else {
        println!(
            "\n{} {}",
            "[ok]".green().bold(),
            t!("cli.tools_check_all_aligned")
        );
    }
    Ok(report.has_issues)
}

fn run_check(config: &LintConfig) -> CheckReport {
    let mut has_issues = false;
    for desc in DESCRIPTORS {
        let pinned = config_version_for(config, desc.toml_key);
        let installed = detect_installed(desc.invocations);
        let outcome = classify(pinned, installed);
        if matches!(
            outcome,
            CheckOutcome::Drift { .. } | CheckOutcome::Missing { .. }
        ) {
            has_issues = true;
        }
        print_check_line(desc, &outcome);
    }
    CheckReport { has_issues }
}

/// Run `detect`. When write=false, prints a TOML snippet users can copy
/// into `.agnix.toml`. When write=true and `config_path` is Some, writes
/// the `[tool_versions]` section back to the config file in place.
pub fn detect_command(config_path: Option<&Path>, write: bool) -> Result<()> {
    println!("{}", t!("cli.tools_detect_header").bold());

    // Scan PATH for each supported tool. Unlike `check`, which needs the
    // user's current config, `detect` only cares about what's installed.
    let mut detected: Vec<(&ToolDescriptor, String)> = Vec::new();
    for desc in DESCRIPTORS {
        match detect_installed(desc.invocations) {
            Some(version) => {
                println!(
                    "  {} {} = {}",
                    "[found]".green().bold(),
                    desc.display,
                    version
                );
                detected.push((desc, version));
            }
            None => {
                println!(
                    "  {} {} {}",
                    "[skip]".dimmed(),
                    desc.display,
                    t!("cli.tools_detect_not_on_path").dimmed()
                );
            }
        }
    }

    if detected.is_empty() {
        println!("\n{}", t!("cli.tools_detect_none_found"));
        return Ok(());
    }

    // Produce the TOML snippet.
    let mut snippet = String::from("[tool_versions]\n");
    for (desc, version) in &detected {
        snippet.push_str(&format!("{} = \"{}\"\n", desc.toml_key, version));
    }

    if write {
        let target = config_path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".agnix.toml"));
        write_tool_versions(&target, &detected)?;
        println!(
            "\n{} {}",
            t!("cli.tools_detect_wrote").green().bold(),
            target.display()
        );
    } else {
        println!("\n{}", t!("cli.tools_detect_snippet_header").bold());
        println!("{snippet}");
        println!("{}", t!("cli.tools_detect_write_hint").dimmed());
    }

    Ok(())
}

/// Write detected versions into `.agnix.toml`'s `[tool_versions]` section.
///
/// Preservation guarantees (line-based, not strictly byte-for-byte):
/// - Dominant line ending (LF vs CRLF) is re-emitted as detected.
/// - Comments (including inline `[section] # comment` headers), blank
///   lines, and every key inside `[tool_versions]` that wasn't detected
///   are kept as-is.
/// - Sections before and after `[tool_versions]` are untouched; the rewrite
///   only touches lines from the section header through its terminator.
///
/// Not preserved:
/// - Trailing whitespace on lines (`lines()` strips it on read).
/// - Mixed line endings (normalized to the dominant one).
///
/// Deliberately a string-level edit rather than a toml round-trip: the
/// `toml` crate loses comments, re-orders keys, and sometimes re-quotes
/// strings. Users keep comments in `.agnix.toml` explaining their pins;
/// we want to preserve those.
fn write_tool_versions(path: &Path, detected: &[(&ToolDescriptor, String)]) -> Result<()> {
    let existing = if path.exists() {
        std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?
    } else {
        String::new()
    };

    let mut updated = apply_tool_versions_section(&existing, detected);

    // Ensure trailing newline for cleanliness.
    if !updated.ends_with('\n') {
        updated.push('\n');
    }

    if updated == existing {
        // No-op: avoid touching the file (same principle as `agnix schema --fix`).
        return Ok(());
    }

    std::fs::write(path, updated).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

/// Strip an inline `# comment` tail from a line and trim whitespace.
/// TOML allows comments anywhere on a key-value or section-header line
/// outside of quoted strings; our inputs here are either section headers
/// or simple key=value, neither of which contains `#` inside a string
/// value we'd need to preserve. Good enough for this narrow use.
fn strip_inline_comment(line: &str) -> &str {
    match line.find('#') {
        Some(idx) => line[..idx].trim_end(),
        None => line.trim_end(),
    }
    .trim_start()
}

/// Does this line contain a section header with the given exact name?
/// Tolerates trailing comments and whitespace: `[tool_versions] # pins`
/// and `[tool_versions]   ` both match `"[tool_versions]"`.
fn is_section_header(line: &str, expected: &str) -> bool {
    strip_inline_comment(line) == expected
}

/// Does this line start any TOML section header at all? Used to detect the
/// end of the `[tool_versions]` section so we don't accidentally consume
/// subsequent ones. Handles `[foo]` with trailing comments.
fn is_any_section_header(line: &str) -> bool {
    let stripped = strip_inline_comment(line);
    stripped.starts_with('[') && stripped.ends_with(']') && stripped.len() >= 2
}

/// Detect the dominant line ending in the content. Returns "\r\n" when
/// more than half of the line breaks are CRLF, "\n" otherwise. Preserves
/// the user's line-ending style on write - Windows developers with CRLF-
/// configured git don't want us rewriting their config to LF.
fn detect_line_ending(content: &str) -> &'static str {
    let crlf = content.matches("\r\n").count();
    let total = content.matches('\n').count();
    if total > 0 && crlf * 2 >= total {
        "\r\n"
    } else {
        "\n"
    }
}

/// Pure string transformation that replaces or inserts the `[tool_versions]`
/// section. Extracted so it's unit-testable without filesystem I/O.
///
/// Preserves the dominant line ending of the input (CRLF vs LF), comments
/// (including inline comments on section headers), blank lines, and any
/// keys inside `[tool_versions]` that weren't detected. Section end is
/// detected by the next `[section]` header on its own line (tolerating
/// trailing inline comments); everything after is left untouched.
fn apply_tool_versions_section(content: &str, detected: &[(&ToolDescriptor, String)]) -> String {
    let line_ending = detect_line_ending(content);
    let lines: Vec<&str> = content.lines().collect();

    // Locate the existing `[tool_versions]` section, tolerating trailing
    // inline comments on the header line.
    let section_start = lines
        .iter()
        .position(|line| is_section_header(line, "[tool_versions]"));

    // Compute section bounds once in the existing-section branch - used by
    // both the block builder (to preserve non-matching lines) and the
    // splice. Avoids recomputing the same end index twice.
    let section_end = section_start.map(|start| {
        lines[start + 1..]
            .iter()
            .position(|line| is_any_section_header(line))
            .map(|offset| start + 1 + offset)
            .unwrap_or(lines.len())
    });

    let detected_keys: std::collections::HashSet<&str> =
        detected.iter().map(|(d, _)| d.toml_key).collect();

    // Build the new `[tool_versions]` block: preserve everything inside the
    // old section except keys we're replacing; append detected keys last.
    let mut block: Vec<String> = vec!["[tool_versions]".to_string()];
    if let (Some(start), Some(end)) = (section_start, section_end) {
        for line in &lines[start + 1..end] {
            if let Some((k, _)) = parse_toml_key(line)
                && detected_keys.contains(k.as_str())
            {
                continue; // Will be rewritten below with the detected version.
            }
            block.push((*line).to_string());
        }
    }
    for (desc, version) in detected {
        block.push(format!("{} = \"{}\"", desc.toml_key, version));
    }

    match (section_start, section_end) {
        (Some(start), Some(end)) => {
            // Splice the new block over the old section.
            let mut out_lines: Vec<&str> = lines[..start].to_vec();
            let block_refs: Vec<&str> = block.iter().map(|s| s.as_str()).collect();
            out_lines.extend(block_refs);
            out_lines.extend(&lines[end..]);
            out_lines.join(line_ending)
        }
        _ => {
            // No existing section - append. Preserve a blank separator if
            // the file has content, preserve the file's line ending.
            let mut out = content.trim_end_matches(&['\r', '\n'][..]).to_string();
            if !out.is_empty() {
                out.push_str(line_ending);
                out.push_str(line_ending);
            }
            out.push_str(&block.join(line_ending));
            out.push_str(line_ending);
            out
        }
    }
}

/// Parse a `key = value` line, returning (key, value) trimmed. Returns None
/// on comment-only lines, blank lines, or section headers.
fn parse_toml_key(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('[') {
        return None;
    }
    let (k, v) = trimmed.split_once('=')?;
    Some((k.trim().to_string(), v.trim().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_version_plain_semver() {
        assert_eq!(extract_version("2.1.119").as_deref(), Some("2.1.119"));
    }

    #[test]
    fn extract_version_with_prefix() {
        assert_eq!(
            extract_version("Claude Code v2.1.119 (build abc)").as_deref(),
            Some("2.1.119")
        );
    }

    #[test]
    fn extract_version_with_prerelease() {
        assert_eq!(
            extract_version("codex 0.125.0-beta.3").as_deref(),
            Some("0.125.0-beta.3")
        );
    }

    #[test]
    fn extract_version_with_build_metadata() {
        assert_eq!(
            extract_version("cursor 3.2.11+1234").as_deref(),
            Some("3.2.11+1234")
        );
    }

    #[test]
    fn extract_version_ignores_two_segment_versions() {
        // MAJOR.MINOR without patch is not semver; skip it and find the
        // next valid triple.
        assert_eq!(
            extract_version("node v20.11 (claude 2.1.119)").as_deref(),
            Some("2.1.119")
        );
    }

    #[test]
    fn extract_version_returns_none_on_empty() {
        assert_eq!(extract_version("").as_deref(), None);
        assert_eq!(extract_version("no version here").as_deref(), None);
    }

    #[test]
    fn classify_match() {
        let r = classify(Some("1.0.0".into()), Some("1.0.0".into()));
        assert!(matches!(r, CheckOutcome::Match { .. }));
    }

    #[test]
    fn classify_drift() {
        let r = classify(Some("1.0.0".into()), Some("1.0.1".into()));
        assert!(matches!(r, CheckOutcome::Drift { .. }));
    }

    #[test]
    fn classify_unpinned() {
        let r = classify(None, Some("1.0.0".into()));
        assert!(matches!(r, CheckOutcome::Unpinned { .. }));
    }

    #[test]
    fn classify_missing() {
        let r = classify(Some("1.0.0".into()), None);
        assert!(matches!(r, CheckOutcome::Missing { .. }));
    }

    #[test]
    fn classify_neither() {
        let r = classify(None, None);
        assert!(matches!(r, CheckOutcome::Neither));
    }

    #[test]
    fn apply_tool_versions_section_appends_to_empty_file() {
        let detected: Vec<(&ToolDescriptor, String)> = vec![(&DESCRIPTORS[0], "2.1.119".into())];
        let result = apply_tool_versions_section("", &detected);
        assert!(result.contains("[tool_versions]"));
        assert!(result.contains("claude_code = \"2.1.119\""));
    }

    #[test]
    fn apply_tool_versions_section_appends_to_existing_content() {
        let existing = "[rules]\nxml = true\n";
        let detected: Vec<(&ToolDescriptor, String)> = vec![(&DESCRIPTORS[0], "2.1.119".into())];
        let result = apply_tool_versions_section(existing, &detected);
        assert!(
            result.contains("[rules]\nxml = true"),
            "must preserve existing [rules] section, got: {result}"
        );
        assert!(result.contains("[tool_versions]\nclaude_code = \"2.1.119\""));
    }

    #[test]
    fn apply_tool_versions_section_replaces_existing_keys() {
        let existing = "[tool_versions]\nclaude_code = \"1.0.0\"\ncodex = \"0.1.0\"\n";
        let detected: Vec<(&ToolDescriptor, String)> = vec![(&DESCRIPTORS[0], "2.1.119".into())];
        let result = apply_tool_versions_section(existing, &detected);
        assert!(
            result.contains("claude_code = \"2.1.119\""),
            "claude_code should be updated, got: {result}"
        );
        // codex entry was NOT in `detected`, so it should be preserved.
        assert!(
            result.contains("codex = \"0.1.0\""),
            "codex entry should be preserved, got: {result}"
        );
    }

    #[test]
    fn apply_tool_versions_section_preserves_comments_in_section() {
        let existing = "\
[tool_versions]
# Pinned per team standard
claude_code = \"1.0.0\"
codex = \"0.1.0\"
";
        let detected: Vec<(&ToolDescriptor, String)> = vec![(&DESCRIPTORS[0], "2.1.119".into())];
        let result = apply_tool_versions_section(existing, &detected);
        assert!(
            result.contains("# Pinned per team standard"),
            "comment should survive, got: {result}"
        );
        assert!(result.contains("claude_code = \"2.1.119\""));
        assert!(result.contains("codex = \"0.1.0\""));
    }

    #[test]
    fn apply_tool_versions_section_preserves_trailing_sections() {
        let existing = "\
[tool_versions]
claude_code = \"1.0.0\"

[rules]
xml = true
";
        let detected: Vec<(&ToolDescriptor, String)> = vec![(&DESCRIPTORS[0], "2.1.119".into())];
        let result = apply_tool_versions_section(existing, &detected);
        assert!(result.contains("[rules]\nxml = true"));
        assert!(result.contains("claude_code = \"2.1.119\""));
    }

    #[test]
    fn parse_toml_key_basic() {
        assert_eq!(
            parse_toml_key("key = \"value\""),
            Some(("key".into(), "\"value\"".into()))
        );
    }

    #[test]
    fn parse_toml_key_with_indent() {
        assert_eq!(
            parse_toml_key("  key=\"value\""),
            Some(("key".into(), "\"value\"".into()))
        );
    }

    #[test]
    fn parse_toml_key_rejects_comments_and_headers() {
        assert_eq!(parse_toml_key("# comment"), None);
        assert_eq!(parse_toml_key("[section]"), None);
        assert_eq!(parse_toml_key(""), None);
    }

    // Review #823 - extract_version was truncating at `+` when both
    // pre-release and build-metadata were present. SemVer 2.0.0 grammar
    // allows "1.2.3-alpha+build"; the extractor must capture the full
    // string.
    #[test]
    fn extract_version_accepts_prerelease_plus_build_metadata() {
        assert_eq!(
            extract_version("1.2.3-alpha+build").as_deref(),
            Some("1.2.3-alpha+build")
        );
        assert_eq!(
            extract_version("release v1.2.3-rc.1+sha.5114f85").as_deref(),
            Some("1.2.3-rc.1+sha.5114f85")
        );
    }

    // Review #823 - section header parser had two bugs: it would miss a
    // `[tool_versions]` header that had an inline `# pin-rationale`
    // comment, AND it would miss a later `[rules] # comment` as a
    // section terminator, so the rewrite could accidentally consume the
    // following section. Both fixed by strip_inline_comment.

    #[test]
    fn is_section_header_tolerates_inline_comment_on_target() {
        assert!(is_section_header("[tool_versions]", "[tool_versions]"));
        assert!(is_section_header(
            "[tool_versions] # pinned per team",
            "[tool_versions]"
        ));
        assert!(is_section_header("  [tool_versions]  ", "[tool_versions]"));
        assert!(!is_section_header("[tools]", "[tool_versions]"));
    }

    #[test]
    fn is_any_section_header_tolerates_inline_comment() {
        assert!(is_any_section_header("[rules]"));
        assert!(is_any_section_header("[rules] # category block"));
        assert!(!is_any_section_header("key = \"value\""));
        assert!(!is_any_section_header("# just a comment"));
        assert!(!is_any_section_header(""));
    }

    #[test]
    fn apply_tool_versions_section_handles_inline_comment_on_header() {
        // Header line has an inline comment - must still be recognized
        // as the existing section and updated in place (not appended as
        // a duplicate).
        let existing = "\
[tool_versions] # pinned per team
claude_code = \"1.0.0\"
";
        let detected: Vec<(&ToolDescriptor, String)> = vec![(&DESCRIPTORS[0], "2.1.119".into())];
        let result = apply_tool_versions_section(existing, &detected);
        // Only one [tool_versions] - no duplicate appended.
        assert_eq!(
            result.matches("[tool_versions]").count(),
            1,
            "must not append duplicate section, got: {result}"
        );
        assert!(result.contains("claude_code = \"2.1.119\""));
    }

    #[test]
    fn apply_tool_versions_section_stops_at_trailing_section_with_inline_comment() {
        // If the section after [tool_versions] has an inline comment, the
        // parser must still see it as a terminator so the rewrite doesn't
        // eat the following section's keys.
        let existing = "\
[tool_versions]
claude_code = \"1.0.0\"

[rules] # category gate
xml = true
";
        let detected: Vec<(&ToolDescriptor, String)> = vec![(&DESCRIPTORS[0], "2.1.119".into())];
        let result = apply_tool_versions_section(existing, &detected);
        assert!(
            result.contains("[rules] # category gate"),
            "trailing section with inline comment must survive, got: {result}"
        );
        assert!(
            result.contains("xml = true"),
            "keys after the trailing section must survive, got: {result}"
        );
    }

    // Review #823 - line-ending preservation. Windows users with CRLF
    // git config shouldn't have their .agnix.toml normalized to LF on
    // --write.

    #[test]
    fn detect_line_ending_prefers_crlf_when_dominant() {
        let crlf = "a = 1\r\nb = 2\r\n";
        assert_eq!(detect_line_ending(crlf), "\r\n");
    }

    #[test]
    fn detect_line_ending_prefers_lf_when_dominant() {
        let lf = "a = 1\nb = 2\n";
        assert_eq!(detect_line_ending(lf), "\n");
    }

    #[test]
    fn detect_line_ending_defaults_to_lf_on_empty() {
        assert_eq!(detect_line_ending(""), "\n");
        assert_eq!(detect_line_ending("no newline at all"), "\n");
    }

    #[test]
    fn apply_tool_versions_section_preserves_crlf() {
        let existing = "[tool_versions]\r\nclaude_code = \"1.0.0\"\r\n";
        let detected: Vec<(&ToolDescriptor, String)> = vec![(&DESCRIPTORS[0], "2.1.119".into())];
        let result = apply_tool_versions_section(existing, &detected);
        assert!(
            result.contains("\r\n"),
            "CRLF input must produce CRLF output, got: {result:?}"
        );
        assert!(result.contains("claude_code = \"2.1.119\""));
    }
}
