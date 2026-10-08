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
        "# Test\n<!-- agnix: noqa: TEST-001 -->\n",
        &LintConfig::default(),
        &registry_with_test_validator(),
    );

    assert!(diagnostics.is_empty(), "diagnostic should be suppressed");
}

#[test]
fn markdown_inline_comment_noqa_is_supported() {
    let diagnostics = validate_content(
        Path::new("CLAUDE.md"),
        "# Test\nIntentional wording. <!-- agnix: noqa: TEST-001 -->\n",
        &LintConfig::default(),
        &registry_with_test_validator(),
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn markdown_prose_and_code_examples_do_not_suppress() {
    for content in [
        "# Test\n# agnix-disable\n",
        "# Test\nagnix-disable\n",
        "# Test\n`<!-- agnix-disable -->`\n",
        "```md\n<!-- agnix-disable -->\n```\n",
        "~~~md\n<!-- agnix-disable -->\n~~~\n",
        "\n    <!-- agnix-disable -->\n",
        "\n\t<!-- agnix-disable -->\n",
    ] {
        let diagnostics = validate_content(
            Path::new("CLAUDE.md"),
            content,
            &LintConfig::default(),
            &registry_with_test_validator(),
        );
        assert_eq!(diagnostics.len(), 1, "{content}");
    }
}

#[test]
fn a_second_html_comment_can_contain_a_suppression() {
    let diagnostics = validate_content(
        Path::new("CLAUDE.md"),
        "# Test\n<!-- explanation --> <!-- agnix: noqa: TEST-001 -->\n",
        &LintConfig::default(),
        &registry_with_test_validator(),
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn quoted_toml_markers_cannot_suppress() {
    let config = LintConfig::builder()
        .files(agnix_core::config::FilesConfig {
            include_as_memory: vec!["*.toml".into()],
            ..Default::default()
        })
        .build()
        .unwrap();
    for content in [
        "model = \"\"\"\n# agnix-disable\n\"\"\"\nbogus = true\n",
        "model = '''\n# agnix-disable\n'''\nbogus = true\n",
        "model = \"demo\"\nbogus = '# agnix-disable'\n",
    ] {
        let diagnostics = validate_content(
            Path::new("demo.toml"),
            content,
            &config,
            &registry_with_test_validator(),
        );
        assert_eq!(diagnostics.len(), 1, "{content}");
    }
    let diagnostics = validate_content(
        Path::new("demo.toml"),
        "model = \"\"\"\ntext\n\"\"\"\n# agnix-disable\n",
        &config,
        &registry_with_test_validator(),
    );
    assert!(
        diagnostics.is_empty(),
        "a comment after the string must still work"
    );
}

#[test]
fn yaml_scalar_markers_are_literal_text() {
    let config = LintConfig::builder()
        .files(agnix_core::config::FilesConfig {
            include_as_memory: vec!["*.yaml".into()],
            ..Default::default()
        })
        .build()
        .unwrap();
    for content in [
        "model: |\n  # agnix-disable\nbogus: true\n",
        "model: >-\n  # agnix-disable\nbogus: true\n",
        "models:\n  - |\n    # agnix-disable\nbogus: true\n",
    ] {
        let diagnostics = validate_content(
            Path::new("demo.yaml"),
            content,
            &config,
            &registry_with_test_validator(),
        );
        assert_eq!(diagnostics.len(), 1, "{content}");
    }
}

#[test]
fn escaped_single_quotes_do_not_hide_later_comments() {
    for (extension, comment) in [("js", "//"), ("py", "#")] {
        let config = LintConfig::builder()
            .files(agnix_core::config::FilesConfig {
                include_as_memory: vec![format!("*.{extension}")],
                ..Default::default()
            })
            .build()
            .unwrap();
        let content = format!("value = 'don\\'t';\n{comment} agnix-disable\n");
        let diagnostics = validate_content(
            Path::new(&format!("demo.{extension}")),
            &content,
            &config,
            &registry_with_test_validator(),
        );
        assert!(diagnostics.is_empty(), "{content}");
    }
}

#[test]
fn extensionless_markdown_rules_support_html_suppressions() {
    for filename in [".cursorrules", ".clinerules", ".windsurfrules", ".roorules"] {
        let path = Path::new(filename);
        let config = LintConfig::default();
        let file_type = agnix_core::resolve_file_type(path, &config);
        assert_ne!(file_type, FileType::Unknown);
        let mut registry = ValidatorRegistry::new();
        registry.register(file_type, || Box::new(TestWarningValidator));
        assert_eq!(
            validate_content(
                path,
                "Don't ignore the rule.\ntrigger\n",
                &config,
                &registry
            )
            .len(),
            1
        );
        let diagnostics = validate_content(
            path,
            "Don't ignore the rule.\n<!-- agnix: noqa: TEST-001 -->\n",
            &config,
            &registry,
        );
        assert!(diagnostics.is_empty(), "{filename}: {diagnostics:?}");
    }
}

#[test]
fn block_comment_noqa_suppresses_all_rules_on_same_line() {
    let diagnostics = validate_content(
        Path::new("CLAUDE.md"),
        "# Test\n<!-- agnix: noqa -->\n",
        &LintConfig::default(),
        &registry_with_test_validator(),
    );

    assert!(diagnostics.is_empty(), "diagnostic should be suppressed");
}

#[test]
fn inline_disable_next_line_suppresses_matching_rule_on_next_line() {
    let diagnostics = validate_content(
        Path::new("CLAUDE.md"),
        "<!-- agnix-disable-next-line TEST-001 -->\ntrigger\n",
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
    rules.disabled_rules.push("CC-SK-011".to_string());
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
    let retired_skill = warnings
        .iter()
        .find(|warning| warning.message.contains("CC-SK-011"))
        .expect("Removed skill rule should warn on stale suppression");
    assert!(
        retired_skill
            .suggestion
            .as_deref()
            .is_some_and(|suggestion| suggestion.contains("no direct replacement"))
    );
    assert!(messages.iter().all(|message| message.contains("removed")));
}

#[test]
fn crate_removed_rules_mirror_matches_knowledge_base_when_available() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let crate_removed_rules = manifest_dir.join("removed-rules.json");
    let kb_removed_rules = manifest_dir.join("../../knowledge-base/removed-rules.json");
    if !kb_removed_rules.exists() {
        eprintln!("Skipping removed-rules parity test: workspace knowledge base not found");
        return;
    }

    let crate_content =
        std::fs::read_to_string(crate_removed_rules).expect("read crate removed-rules mirror");
    let kb_content =
        std::fs::read_to_string(kb_removed_rules).expect("read knowledge-base removed-rules");

    assert_eq!(
        crate_content, kb_content,
        "crates/agnix-core/removed-rules.json must mirror knowledge-base/removed-rules.json"
    );
}
