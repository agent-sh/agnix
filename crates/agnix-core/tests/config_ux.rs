use agnix_core::config::{LintConfig, RuleConfig, SeverityLevel};
use agnix_core::{
    Diagnostic, DiagnosticLevel, FileType, Validator, ValidatorRegistry, validate_content,
    validate_project,
};
use std::collections::BTreeMap;
use std::path::Path;

struct TestWarningValidator;

impl Validator for TestWarningValidator {
    fn validate(&self, path: &Path, _content: &str, _config: &LintConfig) -> Vec<Diagnostic> {
        vec![Diagnostic::warning(
            path.to_path_buf(),
            2,
            1,
            "TEST-001",
            "test warning",
        )]
    }
}

fn registry_with_test_validator() -> ValidatorRegistry {
    let mut registry = ValidatorRegistry::new();
    registry.register(FileType::ClaudeMd, || Box::new(TestWarningValidator));
    registry
}

#[test]
fn per_rule_severity_override_remaps_diagnostic_level() {
    let mut rules = RuleConfig::default();
    rules
        .severity
        .insert("TEST-001".to_string(), SeverityLevel::Error);
    let config = LintConfig::builder()
        .rules(rules)
        .build_lenient()
        .expect("lenient config accepts test rule ID");
    let diagnostics = validate_content(
        Path::new("CLAUDE.md"),
        "# Test\ntrigger\n",
        &config,
        &registry_with_test_validator(),
    );

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].level, DiagnosticLevel::Error);
}

#[test]
fn inline_noqa_suppresses_matching_rule_on_same_line() {
    let diagnostics = validate_content(
        Path::new("CLAUDE.md"),
        "# Test\n# agnix: noqa: TEST-001\n",
        &LintConfig::default(),
        &registry_with_test_validator(),
    );

    assert!(diagnostics.is_empty(), "diagnostic should be suppressed");
}

#[test]
fn inline_disable_next_line_suppresses_matching_rule_on_next_line() {
    let diagnostics = validate_content(
        Path::new("CLAUDE.md"),
        "# agnix-disable-next-line TEST-001\ntrigger\n",
        &LintConfig::default(),
        &registry_with_test_validator(),
    );

    assert!(diagnostics.is_empty(), "diagnostic should be suppressed");
}

#[test]
fn config_extends_merges_base_file_before_child() {
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::write(
        temp.path().join("base.toml"),
        r#"
[rules]
disabled_rules = ["CC-MEM-005"]
"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join(".agnix.toml"),
        r#"
extend = "base.toml"

[rules.severity]
CC-MEM-005 = "Info"
"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Test\n\n- make sure to verify the input is valid\n",
    )
    .unwrap();

    let config = LintConfig::load(temp.path().join(".agnix.toml")).expect("load config");
    let result = validate_project(temp.path(), &config).expect("validate project");
    assert!(
        !result.diagnostics.iter().any(|d| d.rule == "CC-MEM-005"),
        "base disabled_rules should apply after extend merge"
    );
}

#[test]
fn removed_rules_emit_config_warnings() {
    let mut rules = RuleConfig {
        disabled_rules: vec!["AS-010".to_string()],
        severity: BTreeMap::from([("AS-014".to_string(), SeverityLevel::Warning)]),
        ..RuleConfig::default()
    };
    rules.disabled_rules.push("AS-007".to_string());
    let mut config = LintConfig::default();
    *config.rules_mut() = rules;

    let warnings = config.validate();
    let messages = warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect::<Vec<_>>();

    assert!(messages.iter().any(|message| message.contains("AS-007")));
    assert!(messages.iter().any(|message| message.contains("AS-010")));
    assert!(messages.iter().any(|message| message.contains("AS-014")));
    assert!(messages.iter().all(|message| message.contains("removed")));
}
