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
