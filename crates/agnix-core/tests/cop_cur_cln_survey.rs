//! GitHub Copilot, Cursor and Cline rules from the 2026-10-17 vendor-docs
//! survey, checked through the real pipeline: path detection, the validator
//! registry and the validators, on the fixture trees.

use agnix_core::{DiagnosticLevel, LintConfig, validate_project};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

fn diagnostics_in(dir: &str, config: &LintConfig) -> Vec<agnix_core::Diagnostic> {
    validate_project(&fixtures().join(dir), config)
        .expect("fixture project validates")
        .diagnostics
}

fn rule_ids(diagnostics: &[agnix_core::Diagnostic]) -> BTreeSet<String> {
    diagnostics.iter().map(|d| d.rule.clone()).collect()
}

#[test]
fn copilot_survey_rules_fire_through_the_pipeline() {
    let diagnostics = diagnostics_in("copilot-invalid/survey", &LintConfig::default());
    let found = rule_ids(&diagnostics);
    for id in [
        "COP-002",
        "COP-015",
        "COP-017",
        "COP-018",
        "COP-019",
        "COP-020",
        "COP-025",
        "COP-028",
        "COP-029",
        "COP-030",
        "COP-031",
        "COP-032",
        "COP-033",
        "COP-034",
        "COP-035",
        "COP-036",
        "COP-037",
        "COP-038",
        "COP-039",
        "CP-SK-002",
    ] {
        assert!(found.contains(id), "{id} not found; got {found:?}");
    }
    // The plugin manifest in .github/plugin/ is Copilot's, not a misplaced
    // Claude Code plugin.
    assert!(
        diagnostics
            .iter()
            .all(|d| !(d.rule.starts_with("CC-PL-") && d.file.ends_with("plugin.json"))),
        "{diagnostics:#?}"
    );
    // The Copilot settings file is not linted as Claude Code settings.
    assert!(
        diagnostics
            .iter()
            .filter(|d| d.file.ends_with(".github/copilot/settings.json"))
            .all(|d| !d.rule.starts_with("CC-")),
        "{diagnostics:#?}"
    );
}

#[test]
fn copilot_valid_fixtures_have_no_errors_from_survey_rules() {
    let diagnostics = diagnostics_in("copilot", &LintConfig::default());
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("COP-") && d.level == DiagnosticLevel::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn copilot_too_long_fixture_reports_line_count() {
    let diagnostics = diagnostics_in("copilot-too-long", &LintConfig::default());
    let cop_006 = diagnostics
        .iter()
        .find(|d| d.rule == "COP-006")
        .expect("COP-006 on a 1,012-line instruction file");
    assert_eq!(cop_006.line, 1001);
}

#[test]
fn cursor_survey_rules_fire_through_the_pipeline() {
    let diagnostics = diagnostics_in("cursor-invalid/survey", &LintConfig::default());
    let found = rule_ids(&diagnostics);
    for id in [
        "CUR-021", "CUR-022", "CUR-023", "CUR-024", "CUR-025", "CUR-026", "CUR-027", "CUR-028",
        "CUR-029", "CUR-030", "CUR-031", "CUR-032", "CUR-033", "CUR-034", "CUR-036", "CUR-037",
        "CUR-039", "CUR-040",
    ] {
        assert!(found.contains(id), "{id} not found; got {found:?}");
    }
    // The plugin manifest in .cursor-plugin/ is not a Claude Code plugin.
    assert!(
        diagnostics.iter().all(|d| !d.rule.starts_with("CC-PL-")),
        "{diagnostics:#?}"
    );

    let agents = diagnostics_in("cursor-invalid/survey-agents", &LintConfig::default());
    let cur_035: Vec<_> = agents.iter().filter(|d| d.rule == "CUR-035").collect();
    assert_eq!(cur_035.len(), 1, "{cur_035:#?}");
}

#[test]
fn cursor_valid_fixtures_have_no_errors() {
    let diagnostics = diagnostics_in("cursor", &LintConfig::default());
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("CUR-") && d.level == DiagnosticLevel::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn cline_survey_rules_fire_through_the_pipeline() {
    let diagnostics = diagnostics_in("cline-invalid/survey", &LintConfig::default());
    let found = rule_ids(&diagnostics);
    for id in [
        "CLN-005",
        "CLN-006",
        "CLN-009",
        "CLN-010",
        "CLN-011",
        "CLN-012",
        "CLN-013",
        "CLN-014",
        "CLN-015",
        "CLN-016",
        "CL-SK-004",
    ] {
        assert!(found.contains(id), "{id} not found; got {found:?}");
    }
    let file_rules = |suffix: &str| -> BTreeSet<String> {
        diagnostics
            .iter()
            .filter(|d| d.file.ends_with(suffix))
            .map(|d| d.rule.clone())
            .collect()
    };
    // PreCompact is in Cline's hook table but not wired for file hooks.
    assert!(file_rules(".cline/hooks/PreCompact.sh").contains("CLN-009"));
    // Both CLN-012 checks fire in the VS Code extension's folder only.
    assert!(file_rules(".clinerules/hooks/PreToolUse.sh").contains("CLN-012"));
    assert!(file_rules(".clinerules/hooks/TaskStart").contains("CLN-012"));
    assert!(!file_rules(".cline/hooks/OnSave.sh").contains("CLN-012"));
    // Workflows are not checked as rules.
    let workflow = file_rules(".cline/workflows/release.md");
    assert!(workflow.contains("CLN-005"), "{workflow:?}");
    assert!(!workflow.contains("CLN-001"), "{workflow:?}");
}

#[test]
fn cline_valid_fixtures_have_no_cline_findings() {
    let diagnostics = diagnostics_in("cline", &LintConfig::default());
    let cline: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("CLN-") || d.rule.starts_with("CL-SK-"))
        .collect();
    assert!(cline.is_empty(), "{cline:#?}");
}
