//! Regression coverage for recommendation-only diagnostics and retired heuristics.

#[test]
fn other_recommendations_are_informational() {
    let dir = TempDir::new().unwrap();
    let mut lines = vec!["Ordinary project guidance."; 20];
    lines[1] = "Read the file and think step by step.";
    lines[10] = "This is critical information.";
    // PE-001 is retired (no vendor source); PE-002 stays informational.
    let cases = [
        ("CLAUDE.md", lines.join("\n"), vec!["PE-002"]),
        (
            "AGENTS.md",
            "# Instructions\n- type: PreToolExecution\n  command: echo test\n".to_string(),
            vec!["XP-001"],
        ),
        (
            ".claude-plugin/plugin.json",
            r#"{"name":"test-plugin","version":"release-candidate"}"#.to_string(),
            vec!["CC-PL-003"],
        ),
    ];
    for (name, content, rules) in cases {
        let path = dir.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        let result = validate_file(&path, &LintConfig::default()).unwrap();
        for rule in rules {
            let diag = result
                .diagnostics()
                .iter()
                .find(|d| d.rule == rule)
                .unwrap_or_else(|| panic!("Missing {rule}: {:?}", result.diagnostics()));
            assert_eq!(diag.level, DiagnosticLevel::Info, "{rule}");
        }
    }
}

#[test]
fn bypass_permissions_keeps_safety_warning_without_autofix() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".claude/agents/example.md");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "---\nname: example\ndescription: Example agent\npermissionMode: bypassPermissions\n---\nAgent instructions\n").unwrap();
    let result = validate_file(&path, &LintConfig::default()).unwrap();
    let diag = result
        .diagnostics()
        .iter()
        .find(|d| d.rule == "CC-AG-012")
        .unwrap();
    assert_eq!(diag.level, DiagnosticLevel::Warning);
    assert!(diag.message.contains("bypassPermissions"));
    assert!(diag.fixes.is_empty());
}
use agnix_core::{DiagnosticLevel, LintConfig, validate_file, validate_project};
use std::fs;
use tempfile::TempDir;

#[test]
fn retired_memory_heuristics_are_absent() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("CLAUDE.md");
    let mut lines = vec!["Ordinary project guidance."; 20];
    lines[0] = "Be helpful and accurate.";
    lines[1] = "Never use var.";
    lines[10] = "This is critical information.";
    fs::write(&path, lines.join("\n")).unwrap();
    let result = validate_file(&path, &LintConfig::default()).unwrap();
    for rule in ["CC-MEM-005", "CC-MEM-006", "CC-MEM-008"] {
        assert!(
            !result.diagnostics().iter().any(|d| d.rule == rule),
            "{rule} is retired"
        );
    }
}

#[test]
fn supported_agents_hierarchy_is_not_a_defect() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("nested")).unwrap();
    for name in ["AGENTS.md", "nested/AGENTS.md"] {
        fs::write(dir.path().join(name), "# Project\nLocal instructions.\n").unwrap();
    }
    let result = validate_project(dir.path(), &LintConfig::default()).unwrap();
    assert!(!result.diagnostics.iter().any(|d| d.rule == "AGM-006"));
}

#[test]
fn prose_recommendations_are_info_without_rewriting_fixes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("CLAUDE.md");
    // CC-MEM-007 and PE-003 pushed for stronger wording and are retired;
    // CC-MEM-017 now reports emphasis on many lines, as advice only.
    fs::write(
        &path,
        "# Critical Rules\nYou should follow the style guide.\n- IMPORTANT: a\n- IMPORTANT: b\n- IMPORTANT: c\n- IMPORTANT: d\n- IMPORTANT: e\n- IMPORTANT: f\n",
    )
    .unwrap();
    let result = validate_file(&path, &LintConfig::default()).unwrap();
    for rule in ["CC-MEM-007", "PE-003"] {
        assert!(
            !result.diagnostics().iter().any(|d| d.rule == rule),
            "{rule} is retired"
        );
    }
    let diag = result
        .diagnostics()
        .iter()
        .find(|d| d.rule == "CC-MEM-017")
        .unwrap();
    assert_eq!(diag.level, DiagnosticLevel::Info);
    assert!(diag.fixes.is_empty(), "CC-MEM-017 must not rewrite prose");
}
