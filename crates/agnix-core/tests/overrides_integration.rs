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
/// <root>/CLAUDE.md            (top-level - override pattern targets this)
/// <root>/nested/CLAUDE.md     (sub-directory - override pattern does NOT match)
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
        "override pattern `CLAUDE.md` must NOT recurse - expected CC-MEM-005 still on nested/CLAUDE.md, got hits on: {:?}",
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

/// Regression for the cross-file determinism bug surfaced in PR #915
/// round-2 review: when only one side of an XP-004 build conflict carries
/// the override, the previous push-site gate suppressed the diagnostic only
/// when the detector happened to pick the overridden file as the report
/// path (~50% of runs due to HashMap iteration order in the conflict
/// detector). After filtering the candidate set up front, the overridden
/// file is invisible to the detector, so the result is a deterministic
/// zero XP-004 diagnostics.
#[test]
fn overrides_partial_xp004_deterministic_suppression() {
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

    // Override ONLY CLAUDE.md. AGENTS.md continues to participate fully.
    // With CLAUDE.md filtered out of the candidate set, no two files remain
    // to conflict - XP-004 must produce zero diagnostics deterministically.
    let config = LintConfig::builder()
        .overrides(vec![OverrideConfig {
            paths: vec!["CLAUDE.md".to_string()],
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
        "partial override on CLAUDE.md must suppress XP-004 build-conflict deterministically (filter-at-input semantics), got: {xp004_hits:?}"
    );
}

/// Regression for AGM-006 partial-override semantics: a file that disables
/// AGM-006 is invisible to the rule - it neither fires nor appears in
/// other files' "other AGENTS.md files exist at:" listings. When the only
/// remaining unfiltered AGENTS.md leaves the participating set below the
/// `len() > 1` threshold, AGM-006 is fully suppressed.
#[test]
fn overrides_partial_agm006_no_cross_mention() {
    let temp = tempfile::TempDir::new().unwrap();
    fs::write(temp.path().join("AGENTS.md"), "# Root\n").unwrap();
    fs::create_dir_all(temp.path().join("nested")).unwrap();
    fs::write(temp.path().join("nested").join("AGENTS.md"), "# Nested\n").unwrap();

    // Override ONLY the nested AGENTS.md. After filtering, the root file is
    // the only AGM-006 participant - `len() > 1` fails, so AGM-006 produces
    // zero diagnostics. Crucially, the root file must NOT fire AGM-006 with
    // a message that mentions the (filtered-out) nested file.
    let config = LintConfig::builder()
        .overrides(vec![OverrideConfig {
            paths: vec!["nested/AGENTS.md".to_string()],
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
        "partial override on nested/AGENTS.md must remove it from the AGM-006 participating set, dropping below len() > 1 - got: {agm006_hits:?}"
    );
}

/// Regression for upstream issue #1277: `[[overrides]]` had no effect on any
/// rule emitted by the skill validator (the whole AS-*/CC-SK-* family),
/// because its internal `ValidationContext` stored the `&LintConfig` that
/// `PerFileLintConfig` derefs to, dropping the per-file override layer before
/// any `is_rule_enabled` call. Reported as Windows-specific but platform
/// independent.
#[test]
fn overrides_suppress_skill_validator_rules() {
    let temp = tempfile::TempDir::new().unwrap();
    let skill_dir = temp.path().join("skills").join("picky");
    fs::create_dir_all(&skill_dir).unwrap();
    // `FooBarTool` is not a Claude Code tool, so CC-SK-008 fires.
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: picky\ndescription: Use when testing per-file overrides on skills\nallowed-tools: Read, FooBarTool\n---\n\nBody\n",
    )
    .unwrap();

    // Baseline: CC-SK-008 fires without an override.
    let baseline =
        validate_project(temp.path(), &LintConfig::default()).expect("validate_project (baseline)");
    let baseline_hits: Vec<_> = baseline
        .diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();
    assert_eq!(
        baseline_hits.len(),
        1,
        "baseline: expected CC-SK-008 for the unknown tool, got: {baseline_hits:?}"
    );

    let config = LintConfig::builder()
        .overrides(vec![OverrideConfig {
            paths: vec!["skills/picky/SKILL.md".to_string()],
            disabled_rules: vec!["CC-SK-008".to_string()],
        }])
        .build()
        .expect("valid config");

    let result = validate_project(temp.path(), &config).expect("validate_project");

    let hits: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();
    assert!(
        hits.is_empty(),
        "override on `skills/picky/SKILL.md` must suppress CC-SK-008, got: {hits:?}"
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
    // Content is irrelevant here - config is built via the builder below.
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

/// XP-009: Codex's `project_doc_max_bytes` cap is cumulative across the
/// instruction chain, not per-file. XP-007 checks each file alone, so a project
/// split into several mid-size AGENTS.md files was silently truncated with no
/// diagnostic at all (issue #1289).
#[test]
fn xp009_flags_cumulative_chain_over_the_cap() {
    let temp = tempfile::TempDir::new().unwrap();
    // Three files, each well under the 32 KiB per-file limit, 14 KB each.
    // Combined 42 KB, so Codex stops appending partway through.
    let body = "x".repeat(14 * 1024);
    fs::write(temp.path().join("AGENTS.md"), &body).unwrap();
    fs::create_dir_all(temp.path().join("api")).unwrap();
    fs::write(temp.path().join("api").join("AGENTS.md"), &body).unwrap();
    fs::create_dir_all(temp.path().join("api").join("v2")).unwrap();
    fs::write(temp.path().join("api").join("v2").join("AGENTS.md"), &body).unwrap();

    let result = validate_project(temp.path(), &LintConfig::default()).expect("validate_project");

    let xp007: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-007")
        .collect();
    assert!(
        xp007.is_empty(),
        "each file is under the per-file limit, so XP-007 must stay quiet - that is the gap XP-009 fills: {xp007:?}"
    );

    let xp009: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-009")
        .collect();
    assert_eq!(
        xp009.len(),
        1,
        "a 42 KB chain exceeds the 32768-byte cap and must be reported once: {:?}",
        result.diagnostics
    );
}

/// A chain that fits stays clean, so the rule is not simply counting files.
#[test]
fn xp009_quiet_when_chain_fits_under_the_cap() {
    let temp = tempfile::TempDir::new().unwrap();
    let body = "x".repeat(6 * 1024);
    fs::write(temp.path().join("AGENTS.md"), &body).unwrap();
    fs::create_dir_all(temp.path().join("api")).unwrap();
    fs::write(temp.path().join("api").join("AGENTS.md"), &body).unwrap();

    let result = validate_project(temp.path(), &LintConfig::default()).expect("validate_project");

    let xp009: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-009")
        .collect();
    assert!(
        xp009.is_empty(),
        "12 KB combined is under the cap and must not be reported: {xp009:?}"
    );
}

/// At most one file per directory counts, preferring `AGENTS.override.md`. A
/// shadowed `AGENTS.md` must not inflate the total, or a project using overrides
/// would be reported for bytes Codex never reads.
#[test]
fn xp009_counts_one_file_per_directory_preferring_override() {
    let temp = tempfile::TempDir::new().unwrap();
    // Override is small; the shadowed AGENTS.md alongside it is huge. If both
    // counted, the chain would blow the cap.
    fs::write(temp.path().join("AGENTS.override.md"), "small override\n").unwrap();
    fs::write(temp.path().join("AGENTS.md"), "y".repeat(40 * 1024)).unwrap();

    let result = validate_project(temp.path(), &LintConfig::default()).expect("validate_project");

    let xp009: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-009")
        .collect();
    assert!(
        xp009.is_empty(),
        "the shadowed AGENTS.md is not in the chain, so its bytes must not count: {xp009:?}"
    );
}

/// Per-file `[[overrides]]` must suppress XP-009 like any other rule.
#[test]
fn xp009_respects_per_file_overrides() {
    let temp = tempfile::TempDir::new().unwrap();
    let body = "x".repeat(20 * 1024);
    fs::write(temp.path().join("AGENTS.md"), &body).unwrap();
    fs::create_dir_all(temp.path().join("api")).unwrap();
    fs::write(temp.path().join("api").join("AGENTS.md"), &body).unwrap();

    let baseline =
        validate_project(temp.path(), &LintConfig::default()).expect("validate_project baseline");
    assert!(
        baseline.diagnostics.iter().any(|d| d.rule == "XP-009"),
        "baseline: a 40 KB chain must be reported, got: {:?}",
        baseline.diagnostics
    );

    let config = LintConfig::builder()
        .overrides(vec![OverrideConfig {
            paths: vec!["**/AGENTS.md".to_string()],
            disabled_rules: vec!["XP-009".to_string()],
        }])
        .build()
        .expect("valid config");

    let result = validate_project(temp.path(), &config).expect("validate_project");
    let xp009: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-009")
        .collect();
    assert!(
        xp009.is_empty(),
        "an override on the reported file must suppress XP-009: {xp009:?}"
    );
}

/// A Codex chain is a single root-to-cwd path, so sibling subtrees are never
/// concatenated. Summing every discovered file into one total blamed a sibling
/// for bytes it does not share, and scaled the wrong way: enough packages would
/// cross the cap however small each one is. It also contradicted AGM-006, which
/// recommends splitting across nested directories to stay under this very cap.
#[test]
fn xp009_does_not_sum_across_sibling_subtrees() {
    let temp = tempfile::TempDir::new().unwrap();
    // root 10 KB + two 12 KB siblings. Real chains are 22 KB each, both fine;
    // a naive whole-tree sum reaches 34 KB and reports the second sibling.
    fs::write(temp.path().join("AGENTS.md"), "r".repeat(10 * 1024)).unwrap();
    for pkg in ["pkg-a", "pkg-b"] {
        fs::create_dir_all(temp.path().join(pkg)).unwrap();
        fs::write(
            temp.path().join(pkg).join("AGENTS.md"),
            "x".repeat(12 * 1024),
        )
        .unwrap();
    }

    let result = validate_project(temp.path(), &LintConfig::default()).expect("validate_project");
    let xp009: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-009")
        .collect();
    assert!(
        xp009.is_empty(),
        "neither root->pkg-a nor root->pkg-b exceeds the cap, so nothing may be reported: {xp009:?}"
    );
}

/// A single deep chain over the cap is still reported exactly once. With three
/// equal-sized files the attribution tiebreak picks the shallowest, so the root
/// is named - see `xp009_blames_a_deep_file_when_it_is_the_biggest` for the case
/// where a leaf dominates.
#[test]
fn xp009_reports_a_single_deep_chain_once() {
    let temp = tempfile::TempDir::new().unwrap();
    let body = "x".repeat(14 * 1024);
    fs::write(temp.path().join("AGENTS.md"), &body).unwrap();
    fs::create_dir_all(temp.path().join("api").join("v2")).unwrap();
    fs::write(temp.path().join("api").join("AGENTS.md"), &body).unwrap();
    fs::write(temp.path().join("api").join("v2").join("AGENTS.md"), &body).unwrap();

    let result = validate_project(temp.path(), &LintConfig::default()).expect("validate_project");
    let xp009: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-009")
        .collect();
    assert_eq!(
        xp009.len(),
        1,
        "a 42 KB root->api->v2 chain must be reported exactly once: {:?}",
        result.diagnostics
    );
    // Compare structurally rather than against `temp.path()`: on macOS a
    // TempDir under /var resolves to /private/var, so equality on the parent
    // fails for a reason unrelated to what this asserts.
    assert!(
        !xp009[0].file.components().any(|c| c.as_os_str() == "api"),
        "with equal sizes the shallowest (root) file is named, got: {}",
        xp009[0].file.display()
    );
}

/// Codex "includes at most one file per directory", checking
/// `AGENTS.override.md` first, so a shadowed `AGENTS.md` is never loaded - the
/// doc's tree labels it "Ignored because an override exists". Comparing the two
/// as peers reported a conflict between a file and the thing whose purpose is to
/// differ from it.
#[test]
fn xp004_ignores_agents_md_shadowed_by_an_override() {
    let temp = tempfile::TempDir::new().unwrap();
    fs::write(
        temp.path().join("AGENTS.md"),
        "# Agents\n\nTo install dependencies, run `npm install`.\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("AGENTS.override.md"),
        "# Override\n\nTo install dependencies, run `yarn install`.\n",
    )
    .unwrap();

    let result = validate_project(temp.path(), &LintConfig::default()).expect("validate_project");
    let conflicts: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-004")
        .collect();
    assert!(
        conflicts.is_empty(),
        "an override is meant to differ from the file it shadows: {conflicts:?}"
    );
}

/// A genuine conflict between two files Codex actually loads together still
/// fires, so the exclusion above is not simply disabling XP-004.
#[test]
fn xp004_still_flags_a_real_cross_file_conflict() {
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

    let result = validate_project(temp.path(), &LintConfig::default()).expect("validate_project");
    assert!(
        result.diagnostics.iter().any(|d| d.rule == "XP-004"),
        "a real npm-vs-yarn conflict must still be reported: {:?}",
        result.diagnostics
    );
}

/// Attribution follows the largest contributor, not the file where the running
/// total happens to cross. A big root with many small packages crosses inside
/// each package, so blaming the crossing file produced one diagnostic per
/// package against files that cannot fix the problem - trimming a 5 KB package
/// removes 5 KB from a chain that needs 30 KB removed. This is the sibling bug
/// one step along: N reports against the wrong file instead of one report with
/// the wrong total.
#[test]
fn xp009_blames_the_largest_file_not_every_sibling() {
    let temp = tempfile::TempDir::new().unwrap();
    fs::write(temp.path().join("AGENTS.md"), "r".repeat(30 * 1024)).unwrap();
    for i in 0..20 {
        let dir = temp.path().join(format!("pkg{i}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("AGENTS.md"), "p".repeat(5 * 1024)).unwrap();
    }

    let result = validate_project(temp.path(), &LintConfig::default()).expect("validate_project");
    let xp009: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-009")
        .collect();

    assert_eq!(
        xp009.len(),
        1,
        "20 chains sharing one oversized root must collapse to a single report, got: {:?}",
        xp009
            .iter()
            .map(|d| d.file.display().to_string())
            .collect::<Vec<_>>()
    );
    assert!(
        xp009[0].file.ends_with("AGENTS.md") && xp009[0].file.parent() == Some(temp.path()),
        "the 30 KB root is the only edit that fixes every chain, got: {}",
        xp009[0].file.display()
    );
}

/// When a deep file is the largest contributor, it is named rather than the root.
#[test]
fn xp009_blames_a_deep_file_when_it_is_the_biggest() {
    let temp = tempfile::TempDir::new().unwrap();
    fs::write(temp.path().join("AGENTS.md"), "r".repeat(4 * 1024)).unwrap();
    let deep = temp.path().join("api").join("v2");
    fs::create_dir_all(&deep).unwrap();
    fs::write(
        temp.path().join("api").join("AGENTS.md"),
        "a".repeat(4 * 1024),
    )
    .unwrap();
    fs::write(deep.join("AGENTS.md"), "v".repeat(30 * 1024)).unwrap();

    let result = validate_project(temp.path(), &LintConfig::default()).expect("validate_project");
    let xp009: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-009")
        .collect();
    assert_eq!(xp009.len(), 1, "got: {:?}", result.diagnostics);
    assert!(
        xp009[0].file.ends_with("v2/AGENTS.md"),
        "the 30 KB leaf is the biggest contributor, got: {}",
        xp009[0].file.display()
    );
}

/// `is_instruction_file()` collects `AGENTS.override.md` case-insensitively, so
/// the shadow filter and the chain builder have to agree. They matched exactly
/// before, which left a lowercase `agents.override.md` treated as an instruction
/// file while shadowing nothing - the 40 KB file it should have hidden was still
/// summed.
#[test]
fn xp009_override_shadowing_is_case_insensitive() {
    for override_name in ["AGENTS.override.md", "agents.override.md"] {
        let temp = tempfile::TempDir::new().unwrap();
        fs::write(temp.path().join(override_name), "small override\n").unwrap();
        fs::write(temp.path().join("AGENTS.md"), "y".repeat(40 * 1024)).unwrap();

        let result =
            validate_project(temp.path(), &LintConfig::default()).expect("validate_project");
        let xp009: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.rule == "XP-009")
            .collect();
        assert!(
            xp009.is_empty(),
            "'{override_name}' must shadow AGENTS.md, so its 40 KB must not count: {xp009:?}"
        );
    }
}
