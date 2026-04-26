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

/// Mapping from agnix's ToolVersions field -> CLI binary + human-readable name.
///
/// The CLI binary is looked up on PATH and invoked with `--version`. The
/// version extractor parses the output with a shared semver-ish regex, but
/// each entry can override it if a CLI prints a non-standard format.
struct ToolDescriptor {
    /// `ToolVersions` field name as it appears in `.agnix.toml`.
    toml_key: &'static str,
    /// Display name for UI (e.g., "Claude Code").
    display: &'static str,
    /// Binary name on PATH (e.g., "claude" for Claude Code).
    binary: &'static str,
    /// Args to pass to the binary for a version dump. Most CLIs accept
    /// `--version`; some want `version`.
    version_args: &'static [&'static str],
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
        binary: "claude",
        version_args: &["--version"],
    },
    ToolDescriptor {
        toml_key: "codex",
        display: "Codex CLI",
        binary: "codex",
        version_args: &["--version"],
    },
    ToolDescriptor {
        toml_key: "cursor",
        display: "Cursor",
        binary: "cursor",
        version_args: &["--version"],
    },
    ToolDescriptor {
        toml_key: "copilot",
        display: "GitHub Copilot",
        // `gh copilot` is the CLI extension; also try bare `copilot` (npm
        // package shim) as a fallback.
        binary: "copilot",
        version_args: &["--version"],
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

/// Invoke `<binary> <args...>` and extract the first semver-shaped token
/// from its combined stdout+stderr. Returns None when the binary isn't on
/// PATH, the invocation errors, or no version-shaped token appears.
///
/// Separating stdout and stderr matters for tools like `claude --version`
/// that print to one, and potentially logs to the other; combining them
/// keeps the scan robust.
fn detect_installed(binary: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(binary).args(args).output().ok()?;
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    extract_version(&combined)
}

/// Extract the first SemVer-ish token from arbitrary output.
///
/// Matches `<digits>.<digits>.<digits>` with optional pre-release +
/// build-metadata suffix. Anchored to word boundaries so "v2.1.119" yields
/// "2.1.119" and "gh copilot 1.0.23" yields "1.0.23".
fn extract_version(s: &str) -> Option<String> {
    // Deliberately simple, deliberately lenient: capture the first group of
    // `N.N.N[-suffix]` anywhere in the text. We don't try to enforce
    // full SemVer 2.0.0 conformance - CLIs play fast and loose with the
    // suffix grammar, and the user is the one ultimately checking the
    // number against their intent.
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
            // Optionally consume pre-release / build-metadata: `[-+][0-9A-Za-z.-]+`.
            if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
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
        let installed = detect_installed(desc.binary, desc.version_args);
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
        match detect_installed(desc.binary, desc.version_args) {
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

/// Write detected versions into `.agnix.toml`'s `[tool_versions]` section,
/// preserving the rest of the file byte-for-byte. If the section doesn't
/// exist, append it. If it exists, rewrite only the fields we detected -
/// other fields inside `[tool_versions]` are left alone.
///
/// This is deliberately a light string-level edit rather than a toml
/// round-trip: `toml` crate round-trips lose comments, re-order keys, and
/// sometimes re-quote strings. Users keep comments in `.agnix.toml`
/// explaining their pins; we want to preserve those.
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

/// Pure string transformation that replaces or inserts the `[tool_versions]`
/// section. Extracted so it's unit-testable without filesystem I/O.
fn apply_tool_versions_section(content: &str, detected: &[(&ToolDescriptor, String)]) -> String {
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    // Locate the existing `[tool_versions]` section, if any. Section header
    // must be on its own line (possibly with trailing whitespace).
    let section_start = lines.iter().position(|line| {
        let t = line.trim();
        t == "[tool_versions]"
    });

    let new_block: Vec<String> = {
        // Collect the existing fields in the section we're NOT overwriting.
        // Any `key = value` whose key is in our detected list gets replaced;
        // everything else is preserved as-is.
        let detected_keys: std::collections::HashSet<&str> =
            detected.iter().map(|(d, _)| d.toml_key).collect();

        let mut block = vec!["[tool_versions]".to_string()];

        // If the section existed, preserve non-matching lines from it.
        if let Some(start) = section_start {
            let end = lines[start + 1..]
                .iter()
                .position(|line| {
                    let t = line.trim();
                    t.starts_with('[') && t.ends_with(']')
                })
                .map(|offset| start + 1 + offset)
                .unwrap_or(lines.len());
            for line in &lines[start + 1..end] {
                if let Some((k, _)) = parse_toml_key(line)
                    && detected_keys.contains(k.as_str())
                {
                    // Replacing this key with the detected version below.
                    continue;
                }
                // Preserve comments, blanks, and other keys as-is.
                block.push(line.clone());
            }
        }

        // Append the detected keys in descriptor order (stable), skipping
        // any we already preserved (we didn't preserve any; we skipped
        // matching ones above).
        for (desc, version) in detected {
            block.push(format!("{} = \"{}\"", desc.toml_key, version));
        }

        block
    };

    match section_start {
        Some(start) => {
            let end = lines[start + 1..]
                .iter()
                .position(|line| {
                    let t = line.trim();
                    t.starts_with('[') && t.ends_with(']')
                })
                .map(|offset| start + 1 + offset)
                .unwrap_or(lines.len());
            lines.splice(start..end, new_block);
            lines.join("\n")
        }
        None => {
            // Append, preceded by a blank line if the file has content.
            let mut out = content.trim_end().to_string();
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            out.push_str(&new_block.join("\n"));
            out.push('\n');
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
}
