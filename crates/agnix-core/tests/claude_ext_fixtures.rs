//! Every rule added or changed by the 2026-10 Claude Code survey work fires
//! through the real pipeline (path detection, registry, validator) on a
//! checked-in fixture under `tests/fixtures/claude-ext/`.

use agnix_core::{
    Diagnostic, DiagnosticLevel, FileType, LintConfig, ValidationOutcome, detect_file_type,
    validate_file, validate_project,
};
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/claude-ext")
        .canonicalize()
        .expect("claude-ext fixtures exist")
}

fn check(rel: &str) -> Vec<Diagnostic> {
    match validate_file(&fixtures().join(rel), &LintConfig::default()).unwrap() {
        ValidationOutcome::Success(diagnostics) => diagnostics,
        other => panic!("{rel}: {other:?}"),
    }
}

fn assert_fires(rel: &str, rules: &[&str]) {
    let diagnostics = check(rel);
    for rule in rules {
        assert!(
            diagnostics.iter().any(|d| d.rule == *rule),
            "{rel}: expected {rule}, got {:?}",
            diagnostics
                .iter()
                .map(|d| (d.rule.as_str(), d.line, d.message.as_str()))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn new_file_types_are_detected() {
    let root = fixtures();
    assert_eq!(
        detect_file_type(&root.join("commands/.claude/commands/deploy.md")),
        FileType::ClaudeCommand
    );
    assert_eq!(
        detect_file_type(&root.join("marketplace/.claude-plugin/marketplace.json")),
        FileType::ClaudeMarketplace
    );
    // A plain commands/ directory outside .claude is still generic Markdown.
    assert_eq!(
        detect_file_type(Path::new("docs-site/commands/deploy.md")),
        FileType::GenericMarkdown
    );
}

#[test]
fn hooks_settings_fixture() {
    assert_fires(
        "hooks/.claude/settings.json",
        &[
            "CC-HK-002",
            "CC-HK-010",
            "CC-HK-021",
            "CC-HK-025",
            "CC-HK-029",
            "CC-HK-030",
            "CC-HK-031",
            "CC-HK-032",
            "CC-HK-033",
            "CC-HK-035",
        ],
    );
    let diagnostics = check("hooks/.claude/settings.json");
    let if_on_stop = diagnostics
        .iter()
        .find(|d| d.rule == "CC-HK-021" && d.level == DiagnosticLevel::Error)
        .expect("CC-HK-021 error on Stop");
    assert_eq!(if_on_stop.line, 4, "points at the if key");
}

#[test]
fn plugin_fixture() {
    assert_fires(
        "plugin/.claude-plugin/plugin.json",
        &[
            "CC-PL-007",
            "CC-PL-010",
            "CC-PL-018",
            "CC-PL-019",
            "CC-PL-021",
            "CC-PL-022",
            "CC-PL-023",
        ],
    );
    assert_fires("plugin/hooks/hooks.json", &["CC-HK-010", "CC-HK-034"]);
    assert_fires("plugin/settings.json", &["CC-PL-021"]);
    assert_fires("plugin/agents/reviewer.md", &["CC-PL-014", "CC-PL-020"]);
    assert_fires("plugin/skills/run/SKILL.md", &["CC-PL-020"]);
    assert_fires("plugin/.mcp.json", &["CC-MCP-001"]);
    // A plugin's .mcp.json substitutes CLAUDE_PROJECT_DIR and scopes names.
    let diagnostics = check("plugin/.mcp.json");
    assert!(
        diagnostics
            .iter()
            .all(|d| d.rule != "CC-MCP-002" && d.rule != "CC-MCP-005")
    );
}

#[test]
fn agents_fixture() {
    assert_fires(
        "agents/.claude/agents/a-reviewer.md",
        &["CC-AG-021", "CC-AG-023"],
    );
    assert_fires(
        "agents/.claude/agents/builder.md",
        &["CC-AG-019", "CC-AG-024"],
    );
    assert_fires("agents/.claude/agents/open.md", &["CC-AG-005", "CC-AG-022"]);
    let readme = check("agents/.claude/agents/README.md");
    assert!(
        readme
            .iter()
            .any(|d| d.rule == "CC-AG-007" && d.level == DiagnosticLevel::Info),
        "{readme:?}"
    );
}

#[test]
fn skills_fixture() {
    assert_fires("skills/.claude/skills/a-formatter/SKILL.md", &["CC-SK-025"]);
    assert_fires(
        "skills/.claude/skills/no-description/SKILL.md",
        &["CC-SK-024"],
    );
    assert_fires("skills/.claude/skills/status/SKILL.md", &["CC-SK-028"]);
    assert_fires(
        "skills/.claude/skills/claude-pdf/SKILL.md",
        &["CC-SK-030", "CC-SK-031"],
    );
    assert_fires(
        "skills/.claude/skills/helper/SKILL.md",
        &["CC-SK-032", "CC-SK-033"],
    );
    assert_fires("skills/.claude/skills/commit/SKILL.md", &["CC-SK-006"]);
    assert_fires("skills/.claude/skills/finance/SKILL.md", &["CC-SK-034"]);
    assert_fires("commands/.claude/commands/deploy.md", &["CC-SK-027"]);
}

#[test]
fn marketplace_fixture() {
    assert_fires(
        "marketplace/.claude-plugin/marketplace.json",
        &[
            "CC-MP-001",
            "CC-MP-002",
            "CC-MP-003",
            "CC-MP-004",
            "CC-MP-005",
        ],
    );
}

#[test]
fn mcp_fixture() {
    assert_fires(
        "mcp/.mcp.json",
        &["CC-MCP-001", "CC-MCP-002", "CC-MCP-004", "CC-MCP-005"],
    );
    assert_fires("mcp/mcp-tools.json", &["CC-MCP-003"]);
}

#[test]
fn project_walk_reports_new_rules_and_skips_other_vendor_hooks() {
    let result = validate_project(&fixtures(), &LintConfig::default()).unwrap();
    for rule in [
        "CC-HK-030",
        "CC-AG-021",
        "CC-SK-027",
        "CC-MP-003",
        "CC-MCP-005",
        "CC-PL-018",
    ] {
        assert!(
            result.diagnostics.iter().any(|d| d.rule == rule),
            "project walk should report {rule}"
        );
    }
    assert!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.file.ends_with("gemini-extension/hooks/hooks.json"))
            .all(|d| !d.rule.starts_with("CC-HK")),
        "Gemini extension hooks.json gets no Claude hook rules"
    );
}
