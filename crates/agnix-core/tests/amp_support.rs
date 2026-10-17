//! Amp checks, settings, AGENTS.md @-mentions and skills through the real
//! pipeline (path detection, the registry and the validators) on the
//! fixture packs under tests/fixtures/amp-checks and tests/fixtures/amp-skills.
//! Only AMP rules are asserted; other families have their own tests.
use agnix_core::{DiagnosticLevel, FileType, LintConfig, resolve_file_type, validate_project};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixtures(dir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(dir)
}

/// AMP (rule, level) pairs per file (relative path), from `validate_project`.
fn amp_findings(dir: &str) -> BTreeMap<String, Vec<(String, DiagnosticLevel)>> {
    let root = fixtures(dir);
    let canonical = std::fs::canonicalize(&root).unwrap();
    let result = validate_project(&root, &LintConfig::default()).unwrap();
    let mut out: BTreeMap<String, Vec<(String, DiagnosticLevel)>> = BTreeMap::new();
    for diagnostic in result
        .diagnostics
        .into_iter()
        .filter(|d| d.rule.starts_with("AMP-"))
    {
        let file = diagnostic
            .file
            .strip_prefix(&canonical)
            .or_else(|_| diagnostic.file.strip_prefix(&root))
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| diagnostic.file.display().to_string());
        out.entry(file)
            .or_default()
            .push((diagnostic.rule, diagnostic.level));
    }
    for findings in out.values_mut() {
        findings.sort();
        findings.dedup();
    }
    out
}

fn expect(
    pairs: &[(&str, &[(&str, DiagnosticLevel)])],
) -> BTreeMap<String, Vec<(String, DiagnosticLevel)>> {
    pairs
        .iter()
        .map(|(file, findings)| {
            (
                (*file).to_string(),
                findings
                    .iter()
                    .map(|(rule, level)| ((*rule).to_string(), *level))
                    .collect(),
            )
        })
        .collect()
}

use DiagnosticLevel::{Error, Info, Warning};

#[test]
fn amp_files_are_detected() {
    let config = LintConfig::default();
    for (path, expected) in [
        ("amp-checks/.amp/settings.json", FileType::AmpSettings),
        (
            "amp-checks/valid/.amp/settings.jsonc",
            FileType::AmpSettings,
        ),
        (
            "amp-checks/.agents/checks/empty-body.md",
            FileType::AmpCheck,
        ),
        (
            "amp-skills/valid/.agents/skills/chrome-devtools/mcp.json",
            FileType::AmpSkillMcp,
        ),
        (
            "amp-skills/valid/.claude/skills/linear/mcp.json",
            FileType::AmpSkillMcp,
        ),
        (
            "amp-skills/invalid/.agents/skills/a/b/c/d/e/f/SKILL.md",
            FileType::Skill,
        ),
        // Mentioned guidance files are not agent configs themselves.
        ("amp-checks/docs/typescript.md", FileType::Unknown),
    ] {
        assert_eq!(
            resolve_file_type(&fixtures(path), &config),
            expected,
            "{path}"
        );
    }
}

#[test]
fn amp_checks_settings_and_mentions() {
    assert_eq!(
        amp_findings("amp-checks"),
        expect(&[
            (".agents/checks/empty-body.md", &[("AMP-001", Error)]),
            (
                ".agents/checks/invalid-severity.md",
                &[("AMP-002", Warning)]
            ),
            (".agents/checks/missing-name.md", &[("AMP-001", Error)]),
            (
                ".amp/settings.json",
                &[
                    ("AMP-004", Warning),
                    ("AMP-006", Error),
                    ("AMP-007", Warning)
                ],
            ),
            // Reported on the file AGENTS.md mentions, not on AGENTS.md.
            ("docs/typescript.md", &[("AMP-003", Warning)]),
        ])
    );
}

#[test]
fn amp_skills_valid_pack_is_clean() {
    assert_eq!(amp_findings("amp-skills/valid"), BTreeMap::new());
}

#[test]
fn amp_skills_invalid_pack() {
    assert_eq!(
        amp_findings("amp-skills/invalid"),
        expect(&[
            (
                ".agents/skills/a/b/c/d/e/f/SKILL.md",
                &[("AMP-SK-003", Warning)]
            ),
            (".agents/skills/broken/mcp.json", &[("AMP-010", Error)]),
            (".agents/skills/no-filter/mcp.json", &[("AMP-008", Info)]),
            (".agents/skills/token/mcp.json", &[("AMP-006", Error)]),
            (".agents/skills/twice/SKILL.md", &[("AMP-005", Warning)]),
            (
                ".claude/skills/review-docs/SKILL.md",
                &[("AMP-SK-002", Info)]
            ),
        ])
    );
}

#[test]
fn amp_skill_mcp_json_gets_no_generic_mcp_rules() {
    // The generic MCP rules read `{"linear": {"url": ...}}` as a stdio
    // server without a command (MCP-009); Amp's flat skill shape has its
    // own file type now. Since 0.59.0 the skill's servers get the
    // server-level MCP checks that Amp settings get (MCP-018/019/027/028/029,
    // in rules/mcp_servers.rs), so the floating `@latest` in Amp's own
    // chrome-devtools example is an MCP-027 warning; no structural MCP rule
    // runs on the file.
    let root = fixtures("amp-skills/valid");
    let result = validate_project(&root, &LintConfig::default()).unwrap();
    let mcp: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("MCP-") && d.file.ends_with("mcp.json"))
        .collect();
    let rules: Vec<&str> = mcp.iter().map(|d| d.rule.as_str()).collect();
    assert_eq!(rules, ["MCP-027"], "{mcp:?}");
    assert!(mcp[0].file.ends_with("chrome-devtools/mcp.json"));
    assert_eq!(mcp[0].level, agnix_core::DiagnosticLevel::Warning);
}

#[test]
fn amp_rules_are_tool_gated() {
    // AMP rules apply only when amp is in scope.
    let root = fixtures("amp-skills/invalid");
    let mut config = LintConfig::default();
    config.set_tools(vec!["claude-code".to_string()]);
    let result = validate_project(&root, &config).unwrap();
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.rule.starts_with("AMP-")),
        "{:?}",
        result.diagnostics
    );
}
