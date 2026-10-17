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
