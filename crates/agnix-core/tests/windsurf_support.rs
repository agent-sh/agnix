//! Windsurf (Devin Desktop) rules, workflows, global rules, AGENTS.md and
//! Cascade hooks through the real pipeline (path detection, the registry and
//! the validators) on the fixture packs tests/fixtures/windsurf (legacy
//! `.windsurf/` layout), tests/fixtures/windsurf-legacy (`.windsurfrules`) and
//! tests/fixtures/devin (preferred `.devin/` layout). Only WS rules are
//! asserted; other families have their own tests.
use agnix_core::{DiagnosticLevel, FileType, LintConfig, resolve_file_type, validate_project};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixtures(dir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(dir)
}

type Findings = BTreeMap<String, Vec<(String, DiagnosticLevel)>>;

/// WS (rule, level) pairs per file (relative path), from `validate_project`.
/// Repeated findings are kept, so counts are asserted too.
fn ws_findings_with(root: &Path, config: &LintConfig) -> Findings {
    let canonical = std::fs::canonicalize(root).unwrap();
    let result = validate_project(root, config).unwrap();
    let mut out = Findings::new();
    for diagnostic in result
        .diagnostics
        .into_iter()
        .filter(|d| d.rule.starts_with("WS-"))
    {
        let file = diagnostic
            .file
            .strip_prefix(&canonical)
            .or_else(|_| diagnostic.file.strip_prefix(root))
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| diagnostic.file.display().to_string());
        out.entry(file)
            .or_default()
            .push((diagnostic.rule, diagnostic.level));
    }
    for findings in out.values_mut() {
        findings.sort();
    }
    out
}

fn ws_findings(dir: &str) -> Findings {
    ws_findings_with(&fixtures(dir), &LintConfig::default())
}

fn expect(pairs: &[(&str, &[(&str, DiagnosticLevel)])]) -> Findings {
    pairs
        .iter()
        .map(|(file, findings)| {
            let mut findings: Vec<_> = findings
                .iter()
                .map(|(rule, level)| ((*rule).to_string(), *level))
                .collect();
            findings.sort();
            ((*file).to_string(), findings)
        })
        .collect()
}

use DiagnosticLevel::{Error, Info, Warning};

#[test]
fn windsurf_files_are_detected() {
    let config = LintConfig::default();
    for (path, expected) in [
        (
            "devin/valid/.devin/rules/testing.md",
            FileType::WindsurfRule,
        ),
        (
            "devin/invalid/.devin/workflows/address-pr-comments.md",
            FileType::WindsurfWorkflow,
        ),
        ("devin/valid/.devin/hooks.json", FileType::WindsurfHooks),
        (
            "devin/invalid/.windsurf/hooks.json",
            FileType::WindsurfHooks,
        ),
        (
            "devin/valid/.codeium/windsurf/memories/global_rules.md",
            FileType::WindsurfGlobalRules,
        ),
        (
            "devin/valid/.devin/skills/deploy-to-production/SKILL.md",
            FileType::Skill,
        ),
        // Devin CLI files in `.devin/` are not Cascade files.
        ("devin/valid/.devin/hooks.v1.json", FileType::Unknown),
        ("devin/valid/.devin/config.json", FileType::Unknown),
        (
            "windsurf/.windsurf/rules/valid-rule.md",
            FileType::WindsurfRule,
        ),
        (
            "windsurf-legacy/.windsurfrules",
            FileType::WindsurfRulesLegacy,
        ),
    ] {
        assert_eq!(
            resolve_file_type(&fixtures(path), &config),
            expected,
            "{path}"
        );
    }
}

#[test]
fn devin_valid_pack_is_clean() {
    // The vendor examples: a glob rule with `globs: **/*.test.ts` (not valid
    // YAML), always_on, model_decision with a description, manual, the
    // documented hooks.json, a short global_rules.md and AGENTS.md.
    assert_eq!(ws_findings("devin/valid"), Findings::new());
}

#[test]
fn devin_invalid_pack() {
    assert_eq!(
        ws_findings("devin/invalid"),
        expect(&[
            (
                ".codeium/windsurf/memories/global_rules.md",
                &[("WS-010", Error)]
            ),
            (
                ".devin/hooks.json",
                &[
                    // `{ "show_output": true }` without a command, and
                    // `"show_output": "yes"`.
                    ("WS-HK-001", Error),
                    ("WS-HK-001", Error),
                    // pre_read_file
                    ("WS-HK-002", Warning),
                    // powershell-only hook, `~/project`, show_output on
                    // post_cascade_response.
                    ("WS-HK-003", Warning),
                    ("WS-HK-003", Warning),
                    ("WS-HK-003", Info),
                    // python3 hooks/format_code.py
                    ("WS-HK-004", Info),
                    // --token hk_...
                    ("WS-HK-005", Warning),
                ],
            ),
            (
                // "Write good code" only: "Be concise." is vendor-endorsed advice
                // and no longer in PE-005's phrase list, which WS-008 reuses.
                ".devin/rules/generic.md",
                &[("WS-008", Info)]
            ),
            (".devin/rules/no-description.md", &[("WS-005", Warning)]),
            (".devin/rules/no-trigger.md", &[("WS-006", Info)]),
            (".devin/rules/unknown-trigger.md", &[("WS-006", Warning)]),
            (
                ".devin/workflows/address-pr-comments.md",
                &[("WS-009", Info)]
            ),
            // Legacy file, ignored because .devin/hooks.json defines hooks.
            (
                ".windsurf/hooks.json",
                &[("WS-007", Info), ("WS-HK-003", Warning)],
            ),
            ("AGENTS.md", &[("WS-011", Warning)]),
        ])
    );
}

#[test]
fn legacy_windsurf_pack() {
    assert_eq!(
        ws_findings("windsurf"),
        expect(&[
            // First file of .windsurf/rules carries the WS-007 note.
            (
                ".windsurf/rules/cursor-style.md",
                &[("WS-006", Info), ("WS-007", Info)],
            ),
            (".windsurf/rules/empty-rule.md", &[("WS-001", Warning)]),
            (
                ".windsurf/rules/glob-without-globs.md",
                &[("WS-006", Warning)]
            ),
            (
                ".windsurf/rules/model-decision-no-description.md",
                &[("WS-005", Warning)],
            ),
            (
                ".windsurf/workflows/empty-workflow.md",
                &[("WS-003", Warning), ("WS-007", Info), ("WS-009", Info)],
            ),
            (".windsurf/workflows/valid-workflow.md", &[("WS-009", Info)]),
        ])
    );
    assert_eq!(
        ws_findings("windsurf-legacy"),
        expect(&[(".windsurfrules", &[("WS-004", Info)])])
    );
}

#[test]
fn size_limits_count_characters_through_the_pipeline() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for dir in [".devin/rules", ".devin/workflows"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    let frontmatter = "---\ntrigger: always_on\n---\n";
    // 12,001 characters: over the limit.
    let over = format!("{frontmatter}{}", "x".repeat(12_001 - frontmatter.len()));
    // 11,000 CJK characters (33,000 bytes): under the limit.
    let cjk = format!("{frontmatter}{}", "规".repeat(11_000));
    std::fs::write(root.join(".devin/rules/over.md"), &over).unwrap();
    std::fs::write(root.join(".devin/rules/cjk.md"), &cjk).unwrap();
    std::fs::write(root.join(".devin/workflows/over.md"), "y".repeat(12_001)).unwrap();
    std::fs::write(root.join(".devin/workflows/cjk.md"), "规".repeat(11_000)).unwrap();

    let findings = ws_findings_with(root, &LintConfig::default());
    assert_eq!(
        findings,
        expect(&[
            (".devin/rules/over.md", &[("WS-002", Error)]),
            (".devin/workflows/cjk.md", &[("WS-009", Info)]),
            (
                ".devin/workflows/over.md",
                &[("WS-003", Error), ("WS-009", Info)]
            ),
        ])
    );
}

#[test]
fn agents_md_limit_needs_windsurf_in_scope() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::write(
        root.join("AGENTS.md"),
        format!("# Rules\n{}", "a".repeat(12_100)),
    )
    .unwrap();
    // No Windsurf markers and no tools: AGENTS.md is shared by many tools.
    assert_eq!(
        ws_findings_with(root, &LintConfig::default()),
        Findings::new()
    );
    // Devin CLI configuration alone is not Devin Desktop use.
    std::fs::create_dir_all(root.join(".devin")).unwrap();
    std::fs::write(root.join(".devin/config.json"), "{}").unwrap();
    assert_eq!(
        ws_findings_with(root, &LintConfig::default()),
        Findings::new()
    );
    // Selecting windsurf turns it on.
    let mut config = LintConfig::default();
    config.set_tools(vec!["windsurf".to_string()]);
    assert_eq!(
        ws_findings_with(root, &config),
        expect(&[("AGENTS.md", &[("WS-011", Warning)])])
    );
}

#[test]
fn windsurf_rules_are_tool_gated() {
    let mut config = LintConfig::default();
    config.set_tools(vec!["claude-code".to_string()]);
    assert_eq!(
        ws_findings_with(&fixtures("devin/invalid"), &config),
        Findings::new()
    );
}
