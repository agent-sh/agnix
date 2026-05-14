//! End-to-end integration tests for `[[overrides]]` (PRD #53 / upstream
//! agnix issue #909).
//!
//! These tests exercise the full pipeline: project walk → file type
//! detection → validator dispatch → `PerFileLintConfig` view → rule
//! filtering. They mirror the original use case where a CLAUDE.md-style
//! file legitimately contains quoted-example patterns that would otherwise
//! trip generic-instruction detection (CC-MEM-005).

use agnix_core::{LintConfig, OverrideConfig, validate_project};
use std::fs;
use std::path::Path;

/// Content that reliably triggers CC-MEM-005 (generic-instruction
/// detection). "make sure to" and "be helpful" both match patterns in
/// `crates/agnix-core/src/schemas/claude_md.rs::generic_patterns()`.
const TRIGGER_CONTENT: &str = "\
# Test memory file

- make sure to verify the input is valid
- be helpful and concise
";

/// Project layout used by both tests:
///
/// ```text
/// <root>/CLAUDE.md            (top-level — override pattern targets this)
/// <root>/nested/CLAUDE.md     (sub-directory — override pattern does NOT match)
/// ```
fn setup_project(temp: &tempfile::TempDir) {
    fs::write(temp.path().join("CLAUDE.md"), TRIGGER_CONTENT).unwrap();
    fs::create_dir_all(temp.path().join("nested")).unwrap();
    fs::write(
        temp.path().join("nested").join("CLAUDE.md"),
        TRIGGER_CONTENT,
    )
    .unwrap();
}

/// True if `file` ends with the two components `nested/CLAUDE.md`.
fn is_nested(file: &Path) -> bool {
    file.ends_with(Path::new("nested").join("CLAUDE.md"))
}

/// True if `file` ends with just `CLAUDE.md` AND is NOT under `nested/`.
fn is_root_level(file: &Path) -> bool {
    file.file_name().and_then(|n| n.to_str()) == Some("CLAUDE.md") && !is_nested(file)
}

#[test]
fn baseline_without_override_rule_fires_on_both_files() {
    let temp = tempfile::TempDir::new().unwrap();
    setup_project(&temp);

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).expect("validate_project");

    let hits: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "CC-MEM-005")
        .collect();
    let root_hits = hits.iter().filter(|d| is_root_level(&d.file)).count();
    let nested_hits = hits.iter().filter(|d| is_nested(&d.file)).count();

    assert!(
        root_hits > 0,
        "baseline: expected CC-MEM-005 on root CLAUDE.md, got hits on: {:?}",
        hits.iter().map(|d| &d.file).collect::<Vec<_>>()
    );
    assert!(
        nested_hits > 0,
        "baseline: expected CC-MEM-005 on nested/CLAUDE.md, got hits on: {:?}",
        hits.iter().map(|d| &d.file).collect::<Vec<_>>()
    );
}

#[test]
fn overrides_suppress_rule_on_matching_path_only() {
    let temp = tempfile::TempDir::new().unwrap();
    setup_project(&temp);

    // Override the top-level CLAUDE.md only. The pattern `CLAUDE.md` does
    // NOT match `nested/CLAUDE.md` because `FILES_MATCH_OPTIONS` sets
    // `require_literal_separator = true`.
    let config = LintConfig::builder()
        .overrides(vec![OverrideConfig {
            paths: vec!["CLAUDE.md".to_string()],
            disabled_rules: vec!["CC-MEM-005".to_string()],
        }])
        .build()
        .expect("valid config");

    let result = validate_project(temp.path(), &config).expect("validate_project");

    let hits: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "CC-MEM-005")
        .collect();
    let root_hits = hits.iter().filter(|d| is_root_level(&d.file)).count();
    let nested_hits = hits.iter().filter(|d| is_nested(&d.file)).count();

    assert_eq!(
        root_hits,
        0,
        "override should suppress CC-MEM-005 on root CLAUDE.md, but got hits: {:?}",
        hits.iter()
            .filter(|d| is_root_level(&d.file))
            .collect::<Vec<_>>()
    );
    assert!(
        nested_hits > 0,
        "override pattern `CLAUDE.md` must NOT recurse — expected CC-MEM-005 still on nested/CLAUDE.md, got hits on: {:?}",
        hits.iter().map(|d| &d.file).collect::<Vec<_>>()
    );
}

#[test]
fn overrides_recursive_glob_suppresses_everywhere() {
    let temp = tempfile::TempDir::new().unwrap();
    setup_project(&temp);

    // `**/CLAUDE.md` matches at any depth.
    let config = LintConfig::builder()
        .overrides(vec![OverrideConfig {
            paths: vec!["**/CLAUDE.md".to_string()],
            disabled_rules: vec!["CC-MEM-005".to_string()],
        }])
        .build()
        .expect("valid config");

    let result = validate_project(temp.path(), &config).expect("validate_project");

    let cc_mem_005_hits = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "CC-MEM-005")
        .count();

    assert_eq!(
        cc_mem_005_hits,
        0,
        "recursive override `**/CLAUDE.md` should suppress CC-MEM-005 on both files, got diagnostics: {:?}",
        result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-005")
            .collect::<Vec<_>>()
    );
}

/// Regression for project-level rule AGM-006: per-file `[[overrides]]`
/// must suppress AGM-006 just like normal file-validator rules. Mirrors
/// the repro provided by @avifenesh on PR #915.
#[test]
fn overrides_suppress_agm006_on_matching_paths() {
    let temp = tempfile::TempDir::new().unwrap();
    fs::write(temp.path().join("AGENTS.md"), "# Root\n").unwrap();
    fs::create_dir_all(temp.path().join("nested")).unwrap();
    fs::write(temp.path().join("nested").join("AGENTS.md"), "# Nested\n").unwrap();

    // Baseline: AGM-006 fires on both AGENTS.md files when no override is set.
    let baseline =
        validate_project(temp.path(), &LintConfig::default()).expect("validate_project (baseline)");
    let baseline_hits: Vec<_> = baseline
        .diagnostics
        .iter()
        .filter(|d| d.rule == "AGM-006")
        .collect();
    assert_eq!(
        baseline_hits.len(),
        2,
        "baseline: expected AGM-006 on both AGENTS.md files, got: {baseline_hits:?}"
    );

    // With override: AGM-006 must be fully suppressed on both files.
    let config = LintConfig::builder()
        .overrides(vec![OverrideConfig {
            paths: vec!["**/AGENTS.md".to_string()],
            disabled_rules: vec!["AGM-006".to_string()],
        }])
        .build()
        .expect("valid config");

    let result = validate_project(temp.path(), &config).expect("validate_project");

    let agm006_hits: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "AGM-006")
        .collect();
    assert!(
        agm006_hits.is_empty(),
        "override on `**/AGENTS.md` must suppress AGM-006 on every matching file, got: {agm006_hits:?}"
    );
}

/// Regression for project-level rule XP-004: per-file `[[overrides]]`
/// must suppress cross-file build-command conflict diagnostics for files
/// covered by the override. Extra-safe coverage requested by @avifenesh.
#[test]
fn overrides_suppress_xp004_build_conflict() {
    let temp = tempfile::TempDir::new().unwrap();
    fs::write(
        temp.path().join("CLAUDE.md"),
        "# Claude\n\nTo install dependencies, run `npm install`.\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("AGENTS.md"),
        "# Agents\n\nTo install dependencies, run `yarn install`.\n",
    )
    .unwrap();

    // Baseline: XP-004 detects the npm-vs-yarn install conflict across files.
    let baseline =
        validate_project(temp.path(), &LintConfig::default()).expect("validate_project (baseline)");
    let baseline_hits: Vec<_> = baseline
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-004")
        .collect();
    assert!(
        !baseline_hits.is_empty(),
        "baseline: expected XP-004 build-conflict diagnostic, got: {:?}",
        baseline.diagnostics.iter().collect::<Vec<_>>()
    );

    // With an override covering both files, XP-004 must be fully suppressed
    // regardless of which file the conflict detector reports on.
    let config = LintConfig::builder()
        .overrides(vec![OverrideConfig {
            paths: vec!["CLAUDE.md".to_string(), "AGENTS.md".to_string()],
            disabled_rules: vec!["XP-004".to_string()],
        }])
        .build()
        .expect("valid config");

    let result = validate_project(temp.path(), &config).expect("validate_project");

    let xp004_hits: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-004")
        .collect();
    assert!(
        xp004_hits.is_empty(),
        "override covering both files must suppress XP-004 build-conflict, got: {xp004_hits:?}"
    );
}

/// Regression for project-level rule VER-001: per-file `[[overrides]]`
/// must suppress VER-001 when the override targets the diagnostic's
/// report path (`.agnix.toml` when present, project root otherwise).
/// Matches the maintainer's stated preference on PR #915: "if the
/// diagnostic has a report path, use `for_path(report_path)`".
#[test]
fn overrides_suppress_ver001_on_agnix_toml() {
    let temp = tempfile::TempDir::new().unwrap();
    // `.agnix.toml` must exist on disk so `run_project_level_checks`
    // picks it as the report path (vs. falling back to the project root).
    // Content is irrelevant here — config is built via the builder below.
    fs::write(temp.path().join(".agnix.toml"), "# placeholder\n").unwrap();

    // Baseline: with no tool versions pinned and no override, VER-001
    // fires and reports on `.agnix.toml`.
    let baseline =
        validate_project(temp.path(), &LintConfig::default()).expect("validate_project (baseline)");
    let baseline_hits: Vec<_> = baseline
        .diagnostics
        .iter()
        .filter(|d| d.rule == "VER-001")
        .collect();
    assert_eq!(
        baseline_hits.len(),
        1,
        "baseline: expected one VER-001 diagnostic when no versions are pinned, got: {baseline_hits:?}"
    );
    assert!(
        baseline_hits[0].file.ends_with(".agnix.toml"),
        "baseline: VER-001 should report on `.agnix.toml`, got: {}",
        baseline_hits[0].file.display()
    );

    // With override targeting `.agnix.toml`, VER-001 must be suppressed.
    let config = LintConfig::builder()
        .overrides(vec![OverrideConfig {
            paths: vec![".agnix.toml".to_string()],
            disabled_rules: vec!["VER-001".to_string()],
        }])
        .build()
        .expect("valid config");

    let result = validate_project(temp.path(), &config).expect("validate_project");

    let ver001_hits: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "VER-001")
        .collect();
    assert!(
        ver001_hits.is_empty(),
        "override on `.agnix.toml` must suppress VER-001, got: {ver001_hits:?}"
    );
}
