//! Tests extracted from lib.rs during module split (task #354).
//!
//! These tests exercise the public API surface of agnix-core:
//! file type detection, single-file validation, project validation,
//! and the validator registry.

// Allow common test patterns that clippy flags but are intentional in tests
#![allow(
    clippy::field_reassign_with_default,
    clippy::len_zero,
    clippy::useless_vec
)]

use std::path::{Path, PathBuf};

use agnix_core::*;

/// Extract diagnostics from a successful `ValidationOutcome`, panicking on
/// `IoError` or `Skipped` variants.
fn expect_success(outcome: ValidationOutcome) -> Vec<Diagnostic> {
    match outcome {
        ValidationOutcome::Success(diags) => diags,
        other => panic!("expected ValidationOutcome::Success, got: {:?}", other),
    }
}

fn workspace_root() -> &'static Path {
    use std::sync::OnceLock;

    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for ancestor in manifest_dir.ancestors() {
            let cargo_toml = ancestor.join("Cargo.toml");
            if let Ok(content) = std::fs::read_to_string(&cargo_toml)
                && (content.contains("[workspace]") || content.contains("[workspace."))
            {
                return ancestor.to_path_buf();
            }
        }
        panic!(
            "Failed to locate workspace root from CARGO_MANIFEST_DIR={}",
            manifest_dir.display()
        );
    })
    .as_path()
}

#[test]
fn test_detect_skill_file() {
    assert_eq!(detect_file_type(Path::new("SKILL.md")), FileType::Skill);
    assert_eq!(
        detect_file_type(Path::new(".claude/skills/my-skill/SKILL.md")),
        FileType::Skill
    );
}

#[test]
fn test_detect_claude_md() {
    assert_eq!(detect_file_type(Path::new("CLAUDE.md")), FileType::ClaudeMd);
    assert_eq!(detect_file_type(Path::new("AGENTS.md")), FileType::ClaudeMd);
    assert_eq!(
        detect_file_type(Path::new("project/CLAUDE.md")),
        FileType::ClaudeMd
    );
}

#[test]
fn test_detect_instruction_variants() {
    // CLAUDE.local.md variant
    assert_eq!(
        detect_file_type(Path::new("CLAUDE.local.md")),
        FileType::ClaudeMd
    );
    assert_eq!(
        detect_file_type(Path::new("project/CLAUDE.local.md")),
        FileType::ClaudeMd
    );

    // AGENTS.local.md variant
    assert_eq!(
        detect_file_type(Path::new("AGENTS.local.md")),
        FileType::ClaudeMd
    );
    assert_eq!(
        detect_file_type(Path::new("subdir/AGENTS.local.md")),
        FileType::ClaudeMd
    );

    // AGENTS.override.md variant
    assert_eq!(
        detect_file_type(Path::new("AGENTS.override.md")),
        FileType::ClaudeMd
    );
    assert_eq!(
        detect_file_type(Path::new("deep/nested/AGENTS.override.md")),
        FileType::ClaudeMd
    );
}

#[test]
fn test_repo_agents_md_matches_claude_md() {
    let repo_root = workspace_root();

    let claude_path = repo_root.join("CLAUDE.md");
    let claude = std::fs::read_to_string(&claude_path).unwrap_or_else(|e| {
        panic!("Failed to read CLAUDE.md at {}: {e}", claude_path.display());
    });
    let agents_path = repo_root.join("AGENTS.md");
    let agents = std::fs::read_to_string(&agents_path).unwrap_or_else(|e| {
        panic!("Failed to read AGENTS.md at {}: {e}", agents_path.display());
    });

    assert_eq!(agents, claude, "AGENTS.md must match CLAUDE.md");
}

#[test]
fn test_detect_agents() {
    assert_eq!(
        detect_file_type(Path::new("agents/my-agent.md")),
        FileType::Agent
    );
    assert_eq!(
        detect_file_type(Path::new(".claude/agents/helper.md")),
        FileType::Agent
    );
}

#[test]
fn test_detect_hooks() {
    assert_eq!(
        detect_file_type(Path::new("settings.json")),
        FileType::Hooks
    );
    assert_eq!(
        detect_file_type(Path::new(".claude/settings.local.json")),
        FileType::Hooks
    );
}

#[test]
fn test_detect_plugin() {
    // plugin.json in .claude-plugin/ directory
    assert_eq!(
        detect_file_type(Path::new("my-plugin.claude-plugin/plugin.json")),
        FileType::Plugin
    );
    // plugin.json outside .claude-plugin/ is still classified as Plugin
    // (validator checks location constraint CC-PL-001)
    assert_eq!(
        detect_file_type(Path::new("some/plugin.json")),
        FileType::Plugin
    );
    assert_eq!(detect_file_type(Path::new("plugin.json")), FileType::Plugin);
}

#[test]
fn test_detect_generic_markdown() {
    // Generic markdown in non-excluded directories
    assert_eq!(
        detect_file_type(Path::new("notes/setup.md")),
        FileType::GenericMarkdown
    );
    assert_eq!(
        detect_file_type(Path::new("plans/feature.md")),
        FileType::GenericMarkdown
    );
    assert_eq!(
        detect_file_type(Path::new("research/analysis.md")),
        FileType::GenericMarkdown
    );
}

#[test]
fn test_detect_excluded_project_files() {
    // Common project files should be Unknown, not GenericMarkdown
    assert_eq!(detect_file_type(Path::new("README.md")), FileType::Unknown);
    assert_eq!(
        detect_file_type(Path::new("CONTRIBUTING.md")),
        FileType::Unknown
    );
    assert_eq!(detect_file_type(Path::new("LICENSE.md")), FileType::Unknown);
    assert_eq!(
        detect_file_type(Path::new("CODE_OF_CONDUCT.md")),
        FileType::Unknown
    );
    assert_eq!(
        detect_file_type(Path::new("SECURITY.md")),
        FileType::Unknown
    );
    // Case insensitive
    assert_eq!(detect_file_type(Path::new("readme.md")), FileType::Unknown);
    assert_eq!(detect_file_type(Path::new("Readme.md")), FileType::Unknown);
}

#[test]
fn test_detect_excluded_documentation_directories() {
    // Files in docs/ directories should be Unknown
    assert_eq!(
        detect_file_type(Path::new("docs/guide.md")),
        FileType::Unknown
    );
    assert_eq!(detect_file_type(Path::new("doc/api.md")), FileType::Unknown);
    assert_eq!(
        detect_file_type(Path::new("documentation/setup.md")),
        FileType::Unknown
    );
    assert_eq!(
        detect_file_type(Path::new("docs/descriptors/some-linter.md")),
        FileType::Unknown
    );
    assert_eq!(
        detect_file_type(Path::new("wiki/getting-started.md")),
        FileType::Unknown
    );
    assert_eq!(
        detect_file_type(Path::new("examples/basic.md")),
        FileType::Unknown
    );
}

#[test]
fn test_agent_directory_takes_precedence_over_filename_exclusion() {
    // agents/README.md should be detected as Agent, not Unknown
    assert_eq!(
        detect_file_type(Path::new("agents/README.md")),
        FileType::Agent,
        "agents/README.md should be Agent, not excluded as README"
    );
    assert_eq!(
        detect_file_type(Path::new(".claude/agents/README.md")),
        FileType::Agent,
        ".claude/agents/README.md should be Agent"
    );
    assert_eq!(
        detect_file_type(Path::new("agents/CONTRIBUTING.md")),
        FileType::Agent,
        "agents/CONTRIBUTING.md should be Agent"
    );
}

#[test]
fn test_detect_mcp() {
    assert_eq!(detect_file_type(Path::new("mcp.json")), FileType::Mcp);
    assert_eq!(detect_file_type(Path::new("tools.mcp.json")), FileType::Mcp);
    assert_eq!(
        detect_file_type(Path::new("my-server.mcp.json")),
        FileType::Mcp
    );
    assert_eq!(detect_file_type(Path::new("mcp-tools.json")), FileType::Mcp);
    assert_eq!(
        detect_file_type(Path::new("mcp-servers.json")),
        FileType::Mcp
    );
    assert_eq!(
        detect_file_type(Path::new(".claude/mcp.json")),
        FileType::Mcp
    );
}

#[test]
fn test_detect_codex() {
    assert_eq!(
        detect_file_type(Path::new(".codex/config.toml")),
        FileType::CodexConfig
    );
    assert_eq!(
        detect_file_type(Path::new("project/.codex/config.toml")),
        FileType::CodexConfig
    );
    // config.toml outside .codex should be Unknown
    assert_eq!(
        detect_file_type(Path::new("config.toml")),
        FileType::Unknown
    );
    assert_eq!(
        detect_file_type(Path::new("other/config.toml")),
        FileType::Unknown
    );
}

#[test]
fn test_detect_unknown() {
    assert_eq!(detect_file_type(Path::new("main.rs")), FileType::Unknown);
    assert_eq!(
        detect_file_type(Path::new("package.json")),
        FileType::Unknown
    );
}

#[test]
fn test_detect_gemini_md() {
    assert_eq!(detect_file_type(Path::new("GEMINI.md")), FileType::GeminiMd);
    assert_eq!(
        detect_file_type(Path::new("GEMINI.local.md")),
        FileType::GeminiMd
    );
    assert_eq!(
        detect_file_type(Path::new("project/GEMINI.md")),
        FileType::GeminiMd
    );
    assert_eq!(
        detect_file_type(Path::new("subdir/GEMINI.local.md")),
        FileType::GeminiMd
    );
}

#[test]
fn test_validators_for_gemini_md() {
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(FileType::GeminiMd);
    assert_eq!(validators.len(), 5);
}

#[test]
fn test_validators_for_gemini_settings() {
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(FileType::GeminiSettings);
    assert_eq!(validators.len(), 1);
    assert_eq!(validators[0].name(), "GeminiSettingsValidator");
}

#[test]
fn test_validators_for_gemini_extension() {
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(FileType::GeminiExtension);
    assert_eq!(validators.len(), 1);
    assert_eq!(validators[0].name(), "GeminiExtensionValidator");
}

#[test]
fn test_validators_for_gemini_ignore() {
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(FileType::GeminiIgnore);
    assert_eq!(validators.len(), 1);
    assert_eq!(validators[0].name(), "GeminiIgnoreValidator");
}

#[test]
fn test_validators_for_skill() {
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(FileType::Skill);
    assert_eq!(validators.len(), 4);
}

#[test]
fn test_validators_for_claude_md() {
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(FileType::ClaudeMd);
    assert_eq!(validators.len(), 8);
    assert!(validators.iter().any(|v| v.name() == "AmpValidator"));
}

#[test]
fn test_validators_for_amp_check() {
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(FileType::AmpCheck);
    assert!(!validators.is_empty());
    assert!(validators.iter().any(|v| v.name() == "AmpValidator"));
}

#[test]
fn test_validators_for_amp_settings() {
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(FileType::AmpSettings);
    assert!(!validators.is_empty());
    assert!(validators.iter().any(|v| v.name() == "AmpValidator"));
}

#[test]
fn test_validators_for_mcp() {
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(FileType::Mcp);
    assert_eq!(validators.len(), 1);
}

#[test]
fn test_validators_for_unknown() {
    let registry = ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(FileType::Unknown);
    assert_eq!(validators.len(), 0);
}

#[test]
fn test_validate_file_with_custom_registry() {
    struct DummyValidator;

    impl Validator for DummyValidator {
        fn validate(&self, path: &Path, _content: &str, _config: &LintConfig) -> Vec<Diagnostic> {
            vec![Diagnostic::error(
                path.to_path_buf(),
                1,
                1,
                "TEST-001",
                "Registry override".to_string(),
            )]
        }
    }

    let temp = tempfile::TempDir::new().unwrap();
    let skill_path = temp.path().join("SKILL.md");
    std::fs::write(&skill_path, "---\nname: test\n---\nBody").unwrap();

    let mut registry = ValidatorRegistry::new();
    registry.register(FileType::Skill, || Box::new(DummyValidator));

    let diagnostics = expect_success(
        validate_file_with_registry(&skill_path, &LintConfig::default(), &registry).unwrap(),
    );

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule, "TEST-001");
}

#[test]
fn test_validate_file_unknown_type() {
    let temp = tempfile::TempDir::new().unwrap();
    let unknown_path = temp.path().join("test.rs");
    std::fs::write(&unknown_path, "fn main() {}").unwrap();

    let config = LintConfig::default();
    let outcome = validate_file(&unknown_path, &config).unwrap();
    assert!(
        outcome.is_skipped(),
        "Unknown file type should return ValidationOutcome::Skipped, got: {:?}",
        outcome
    );
}

#[test]
fn test_validate_file_skill() {
    let temp = tempfile::TempDir::new().unwrap();
    let skill_dir = temp.path().join("test-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: test-skill\ndescription: Use when testing\n---\nBody",
    )
    .unwrap();

    let config = LintConfig::default();
    let diagnostics = expect_success(validate_file(&skill_path, &config).unwrap());

    assert!(diagnostics.is_empty());
}

#[test]
fn test_validate_file_invalid_skill() {
    let temp = tempfile::TempDir::new().unwrap();
    let skill_path = temp.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: deploy-prod\ndescription: Deploys\n---\nBody",
    )
    .unwrap();

    let config = LintConfig::default();
    let diagnostics = expect_success(validate_file(&skill_path, &config).unwrap());

    assert!(!diagnostics.is_empty());
    assert!(diagnostics.iter().any(|d| d.rule == "CC-SK-006"));
}

#[test]
fn test_validate_project_finds_issues() {
    let temp = tempfile::TempDir::new().unwrap();
    let skill_dir = temp.path().join("skills").join("deploy");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: deploy-prod\ndescription: Deploys\n---\nBody",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    assert!(!result.diagnostics.is_empty());
}

#[test]
fn test_validate_project_empty_dir() {
    let temp = tempfile::TempDir::new().unwrap();

    // Disable VER-001 since we're testing an empty project
    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = vec!["VER-001".to_string()];
    let result = validate_project(temp.path(), &config).unwrap();

    assert!(result.diagnostics.is_empty());
}

#[test]
fn test_validate_project_sorts_by_severity() {
    let temp = tempfile::TempDir::new().unwrap();

    let skill_dir = temp.path().join("skill1");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: deploy-prod\ndescription: Deploys\n---\nBody",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    for i in 1..result.diagnostics.len() {
        assert!(result.diagnostics[i - 1].level <= result.diagnostics[i].level);
    }
}

#[test]
fn test_validate_invalid_skill_triggers_both_rules() {
    let temp = tempfile::TempDir::new().unwrap();
    let skill_path = temp.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: deploy-prod\ndescription: Deploys\nallowed-tools: Bash Read Write\n---\nBody",
    )
    .unwrap();

    let config = LintConfig::default();
    let diagnostics = expect_success(validate_file(&skill_path, &config).unwrap());

    assert!(diagnostics.iter().any(|d| d.rule == "CC-SK-006"));
    assert!(diagnostics.iter().any(|d| d.rule == "CC-SK-007"));
}

#[test]
fn test_validate_valid_skill_produces_no_errors() {
    let temp = tempfile::TempDir::new().unwrap();
    let skill_dir = temp.path().join("code-review");
    std::fs::create_dir_all(&skill_dir).unwrap();
    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: code-review\ndescription: Use when reviewing code\n---\nBody",
    )
    .unwrap();

    let config = LintConfig::default();
    let diagnostics = expect_success(validate_file(&skill_path, &config).unwrap());

    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Error)
        .collect();
    assert!(errors.is_empty());
}

#[test]
fn test_parallel_validation_deterministic_output() {
    // Create a project structure with multiple files that will generate diagnostics
    let temp = tempfile::TempDir::new().unwrap();

    // Create multiple skill files with issues to ensure non-trivial parallel work
    for i in 0..5 {
        let skill_dir = temp.path().join(format!("skill-{}", i));
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!(
                "---\nname: deploy-prod-{}\ndescription: Deploys things\n---\nBody",
                i
            ),
        )
        .unwrap();
    }

    // Create some CLAUDE.md files too
    for i in 0..3 {
        let dir = temp.path().join(format!("project-{}", i));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("CLAUDE.md"),
            "# Project\n\nBe helpful and concise.\n",
        )
        .unwrap();
    }

    let config = LintConfig::default();

    // Run validation multiple times and verify identical output
    let first_result = validate_project(temp.path(), &config).unwrap();

    for run in 1..=10 {
        let result = validate_project(temp.path(), &config).unwrap();

        assert_eq!(
            first_result.diagnostics.len(),
            result.diagnostics.len(),
            "Run {} produced different number of diagnostics",
            run
        );

        for (i, (a, b)) in first_result
            .diagnostics
            .iter()
            .zip(result.diagnostics.iter())
            .enumerate()
        {
            assert_eq!(
                a.file, b.file,
                "Run {} diagnostic {} has different file",
                run, i
            );
            assert_eq!(
                a.rule, b.rule,
                "Run {} diagnostic {} has different rule",
                run, i
            );
            assert_eq!(
                a.level, b.level,
                "Run {} diagnostic {} has different level",
                run, i
            );
        }
    }

    // Verify we actually got some diagnostics (the dangerous name rule should fire)
    assert!(
        !first_result.diagnostics.is_empty(),
        "Expected diagnostics for deploy-prod-* skill names"
    );
}

#[test]
fn test_parallel_validation_single_file() {
    // Edge case: verify parallel code works correctly with just one file
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::write(
        temp.path().join("SKILL.md"),
        "---\nname: deploy-prod\ndescription: Deploys\n---\nBody",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // Should have at least one diagnostic for the dangerous name (CC-SK-006)
    assert!(
        result.diagnostics.iter().any(|d| d.rule == "CC-SK-006"),
        "Expected CC-SK-006 diagnostic for dangerous deploy-prod name"
    );
}

#[test]
fn test_parallel_validation_mixed_results() {
    // Test mix of valid and invalid files processed in parallel
    let temp = tempfile::TempDir::new().unwrap();

    // Valid skill (no diagnostics expected)
    let valid_dir = temp.path().join("valid");
    std::fs::create_dir_all(&valid_dir).unwrap();
    std::fs::write(
        valid_dir.join("SKILL.md"),
        "---\nname: valid\ndescription: Use when reviewing code\n---\nBody",
    )
    .unwrap();

    // Invalid skill (diagnostics expected)
    let invalid_dir = temp.path().join("invalid");
    std::fs::create_dir_all(&invalid_dir).unwrap();
    std::fs::write(
        invalid_dir.join("SKILL.md"),
        "---\nname: deploy-prod\ndescription: Deploys\n---\nBody",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // Should have diagnostics only from the invalid skill
    let error_diagnostics: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Error)
        .collect();

    assert!(
        error_diagnostics
            .iter()
            .all(|d| d.file.to_string_lossy().contains("invalid")),
        "Errors should only come from the invalid skill"
    );
}

#[test]
fn test_validate_project_agents_md_collection() {
    // Verify that validation correctly collects AGENTS.md paths for AGM-006
    let temp = tempfile::TempDir::new().unwrap();

    // Create multiple AGENTS.md files in different directories
    std::fs::write(temp.path().join("AGENTS.md"), "# Root agents").unwrap();

    let subdir = temp.path().join("subproject");
    std::fs::create_dir_all(&subdir).unwrap();
    std::fs::write(subdir.join("AGENTS.md"), "# Subproject agents").unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // Should have AGM-006 warnings for both AGENTS.md files
    let agm006_diagnostics: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "AGM-006")
        .collect();

    assert_eq!(
        agm006_diagnostics.len(),
        2,
        "Expected AGM-006 diagnostic for each AGENTS.md file, got: {:?}",
        agm006_diagnostics
    );
}

#[test]
fn test_validate_project_files_checked_count() {
    // Verify that validation correctly counts recognized file types
    let temp = tempfile::TempDir::new().unwrap();

    // Create recognized file types
    std::fs::write(
        temp.path().join("SKILL.md"),
        "---\nname: test-skill\ndescription: Test skill\n---\nBody",
    )
    .unwrap();
    std::fs::write(temp.path().join("CLAUDE.md"), "# Project memory").unwrap();

    // Create unrecognized file types (should not be counted)
    // Note: .md files are GenericMarkdown (recognized), so use non-markdown extensions
    std::fs::write(temp.path().join("notes.txt"), "Some notes").unwrap();
    std::fs::write(temp.path().join("data.json"), "{}").unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // files_checked should only count recognized types (SKILL.md + CLAUDE.md = 2)
    // .txt and .json (not matching MCP patterns) are FileType::Unknown
    assert_eq!(
        result.files_checked, 2,
        "files_checked should count only recognized file types, got {}",
        result.files_checked
    );
}

#[test]
fn test_validate_project_plugin_detection() {
    let temp = tempfile::TempDir::new().unwrap();
    let plugin_dir = temp.path().join("my-plugin.claude-plugin");
    std::fs::create_dir_all(&plugin_dir).unwrap();

    // Create plugin.json with a validation issue (missing recommended description - CC-PL-004 warning)
    std::fs::write(
        plugin_dir.join("plugin.json"),
        r#"{"name": "test-plugin", "version": "1.0.0"}"#,
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // Should detect the plugin.json and report CC-PL-004 warning for missing description
    let plugin_diagnostics: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("CC-PL-"))
        .collect();

    assert!(
        !plugin_diagnostics.is_empty(),
        "validate_project() should detect and validate plugin.json files"
    );

    assert!(
        plugin_diagnostics.iter().any(|d| d.rule == "CC-PL-004"),
        "Should report CC-PL-004 for missing recommended description field"
    );

    assert!(
        plugin_diagnostics.iter().any(|d| d.rule == "CC-PL-004"
            && d.level == agnix_core::diagnostics::DiagnosticLevel::Warning),
        "CC-PL-004 for missing description should be a warning, not an error"
    );
}

// ===== MCP Validation Integration Tests =====

#[test]
fn test_validate_file_mcp() {
    let temp = tempfile::TempDir::new().unwrap();
    let mcp_path = temp.path().join("tools.mcp.json");
    std::fs::write(
        &mcp_path,
        r#"{"name": "test-tool", "description": "A test tool for testing purposes", "inputSchema": {"type": "object"}}"#,
    )
    .unwrap();

    let config = LintConfig::default();
    let diagnostics = expect_success(validate_file(&mcp_path, &config).unwrap());

    // Tool without consent field should trigger MCP-005 warning
    assert!(diagnostics.iter().any(|d| d.rule == "MCP-005"));
}

#[test]
fn test_validate_file_mcp_invalid_schema() {
    let temp = tempfile::TempDir::new().unwrap();
    let mcp_path = temp.path().join("mcp.json");
    std::fs::write(
        &mcp_path,
        r#"{"name": "test-tool", "description": "A test tool for testing purposes", "inputSchema": "not an object"}"#,
    )
    .unwrap();

    let config = LintConfig::default();
    let diagnostics = expect_success(validate_file(&mcp_path, &config).unwrap());

    // Invalid schema should trigger MCP-003
    assert!(diagnostics.iter().any(|d| d.rule == "MCP-003"));
}

#[test]
fn test_validate_project_mcp_detection() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create an MCP file with issues
    std::fs::write(
        temp.path().join("tools.mcp.json"),
        r#"{"name": "", "description": "Short", "inputSchema": {"type": "object"}}"#,
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // Should detect the MCP file and report issues
    let mcp_diagnostics: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("MCP-"))
        .collect();

    assert!(
        !mcp_diagnostics.is_empty(),
        "validate_project() should detect and validate MCP files"
    );

    // Empty name should trigger MCP-002
    assert!(
        mcp_diagnostics.iter().any(|d| d.rule == "MCP-002"),
        "Should report MCP-002 for empty name"
    );
}

// ===== Cross-Platform Validation Integration Tests =====

#[test]
fn test_validate_agents_md_with_claude_features() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create AGENTS.md with Claude-specific features
    std::fs::write(
        temp.path().join("AGENTS.md"),
        r#"# Agent Config
- type: PreToolExecution
  command: echo "test"
"#,
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // Should detect XP-001 error for Claude-specific hooks in AGENTS.md
    let xp_001: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-001")
        .collect();
    assert!(
        !xp_001.is_empty(),
        "Expected XP-001 error for hooks in AGENTS.md"
    );
}

#[test]
fn test_validate_agents_md_with_context_fork() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create AGENTS.md with context: fork
    std::fs::write(
        temp.path().join("AGENTS.md"),
        r#"---
name: test
context: fork
agent: Explore
---
# Test Agent
"#,
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // Should detect XP-001 errors for Claude-specific features
    let xp_001: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-001")
        .collect();
    assert!(
        !xp_001.is_empty(),
        "Expected XP-001 errors for context:fork and agent in AGENTS.md"
    );
}

#[test]
fn test_validate_agents_md_no_headers() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create AGENTS.md with no headers
    std::fs::write(
        temp.path().join("AGENTS.md"),
        "Just plain text without any markdown headers.",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // Should detect XP-002 warning for missing headers
    let xp_002: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-002")
        .collect();
    assert!(
        !xp_002.is_empty(),
        "Expected XP-002 warning for missing headers in AGENTS.md"
    );
}

#[test]
fn test_validate_agents_md_hard_coded_paths() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create AGENTS.md with hard-coded platform paths
    std::fs::write(
        temp.path().join("AGENTS.md"),
        r#"# Config
Check .claude/settings.json and .cursor/rules/ for configuration.
"#,
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // Should detect XP-003 warnings for hard-coded paths
    let xp_003: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-003")
        .collect();
    assert_eq!(
        xp_003.len(),
        2,
        "Expected 2 XP-003 warnings for hard-coded paths"
    );
}

#[test]
fn test_validate_valid_agents_md() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create valid AGENTS.md without any issues
    std::fs::write(
        temp.path().join("AGENTS.md"),
        r#"# Project Guidelines

Follow the coding style guide.

## Commands
- npm run build
- npm run test
"#,
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // Should have no XP-* diagnostics
    let xp_rules: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("XP-"))
        .collect();
    assert!(
        xp_rules.is_empty(),
        "Valid AGENTS.md should have no XP-* diagnostics"
    );
}

#[test]
fn test_validate_claude_md_allows_claude_features() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create CLAUDE.md with Claude-specific features (allowed)
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        r#"---
name: test
context: fork
agent: Explore
allowed-tools: Read Write
---
# Claude Agent
"#,
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // XP-001 should NOT fire for CLAUDE.md (Claude features are allowed there)
    let xp_001: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-001")
        .collect();
    assert!(
        xp_001.is_empty(),
        "CLAUDE.md should be allowed to have Claude-specific features"
    );
}

// ===== AGM-006: Multiple AGENTS.md Tests =====

#[test]
fn test_agm_006_nested_agents_md() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create nested AGENTS.md files
    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\nThis project does something.",
    )
    .unwrap();

    let subdir = temp.path().join("subdir");
    std::fs::create_dir_all(&subdir).unwrap();
    std::fs::write(
        subdir.join("AGENTS.md"),
        "# Subproject\n\nThis is a nested AGENTS.md.",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // Should detect AGM-006 for both AGENTS.md files
    let agm_006: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "AGM-006")
        .collect();
    assert_eq!(
        agm_006.len(),
        2,
        "Should detect both AGENTS.md files, got {:?}",
        agm_006
    );
    assert!(
        agm_006
            .iter()
            .any(|d| d.file.to_string_lossy().contains("subdir"))
    );
    assert!(
        agm_006
            .iter()
            .any(|d| d.message.contains("Nested AGENTS.md"))
    );
    assert!(
        agm_006
            .iter()
            .any(|d| d.message.contains("Multiple AGENTS.md files"))
    );
}

#[test]
fn test_agm_006_no_nesting() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create single AGENTS.md file
    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\nThis project does something.",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // Should not detect AGM-006 for a single AGENTS.md
    let agm_006: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "AGM-006")
        .collect();
    assert!(
        agm_006.is_empty(),
        "Single AGENTS.md should not trigger AGM-006"
    );
}

#[test]
fn test_agm_006_multiple_agents_md() {
    let temp = tempfile::TempDir::new().unwrap();

    let app_a = temp.path().join("app-a");
    let app_b = temp.path().join("app-b");
    std::fs::create_dir_all(&app_a).unwrap();
    std::fs::create_dir_all(&app_b).unwrap();

    std::fs::write(
        app_a.join("AGENTS.md"),
        "# App A\n\nThis project does something.",
    )
    .unwrap();
    std::fs::write(
        app_b.join("AGENTS.md"),
        "# App B\n\nThis project does something.",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    let agm_006: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "AGM-006")
        .collect();
    assert_eq!(
        agm_006.len(),
        2,
        "Should detect both AGENTS.md files, got {:?}",
        agm_006
    );
    assert!(
        agm_006
            .iter()
            .all(|d| d.message.contains("Multiple AGENTS.md files"))
    );
}

#[test]
fn test_agm_006_disabled() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create nested AGENTS.md files
    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\nThis project does something.",
    )
    .unwrap();

    let subdir = temp.path().join("subdir");
    std::fs::create_dir_all(&subdir).unwrap();
    std::fs::write(
        subdir.join("AGENTS.md"),
        "# Subproject\n\nThis is a nested AGENTS.md.",
    )
    .unwrap();

    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = vec!["AGM-006".to_string()];
    let result = validate_project(temp.path(), &config).unwrap();

    // Should not detect AGM-006 when disabled
    let agm_006: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "AGM-006")
        .collect();
    assert!(agm_006.is_empty(), "AGM-006 should not fire when disabled");
}

// ===== XP-004: Conflicting Build Commands =====

#[test]
fn test_xp_004_conflicting_package_managers() {
    let temp = tempfile::TempDir::new().unwrap();

    // CLAUDE.md uses npm
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\nUse `npm install` for dependencies.",
    )
    .unwrap();

    // AGENTS.md uses pnpm
    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\nUse `pnpm install` for dependencies.",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    let xp_004: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-004")
        .collect();
    assert!(
        !xp_004.is_empty(),
        "Should detect conflicting package managers"
    );
    assert!(xp_004.iter().any(|d| d.message.contains("npm")));
    assert!(xp_004.iter().any(|d| d.message.contains("pnpm")));
}

#[test]
fn test_xp_004_no_conflict_same_manager() {
    let temp = tempfile::TempDir::new().unwrap();

    // Both files use npm
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\nUse `npm install` for dependencies.",
    )
    .unwrap();

    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\nUse `npm run build` for building.",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    let xp_004: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-004")
        .collect();
    assert!(
        xp_004.is_empty(),
        "Should not detect conflict when same package manager is used"
    );
}

// ===== XP-005: Conflicting Tool Constraints =====

#[test]
fn test_xp_005_conflicting_tool_constraints() {
    let temp = tempfile::TempDir::new().unwrap();

    // CLAUDE.md allows Bash
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\nallowed-tools: Read Write Bash",
    )
    .unwrap();

    // AGENTS.md disallows Bash
    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\nNever use Bash for operations.",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    let xp_005: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-005")
        .collect();
    assert!(
        !xp_005.is_empty(),
        "Should detect conflicting tool constraints"
    );
    assert!(xp_005.iter().any(|d| d.message.contains("Bash")));
}

#[test]
fn test_xp_005_no_conflict_consistent_constraints() {
    let temp = tempfile::TempDir::new().unwrap();

    // Both files allow Read
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\nallowed-tools: Read Write",
    )
    .unwrap();

    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\nYou can use Read for file access.",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    let xp_005: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-005")
        .collect();
    assert!(
        xp_005.is_empty(),
        "Should not detect conflict when constraints are consistent"
    );
}

// ===== XP-006: Layer Precedence =====

#[test]
fn test_xp_006_no_precedence_documentation() {
    let temp = tempfile::TempDir::new().unwrap();

    // Both files exist but neither documents precedence
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\nThis is Claude.md.",
    )
    .unwrap();

    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\nThis is Agents.md.",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    let xp_006: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-006")
        .collect();
    assert!(
        !xp_006.is_empty(),
        "Should detect missing precedence documentation"
    );
}

#[test]
fn test_xp_006_with_precedence_documentation() {
    let temp = tempfile::TempDir::new().unwrap();

    // CLAUDE.md documents precedence
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\nCLAUDE.md takes precedence over AGENTS.md.",
    )
    .unwrap();

    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\nThis is Agents.md.",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    let xp_006: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-006")
        .collect();
    assert!(
        xp_006.is_empty(),
        "Should not trigger XP-006 when precedence is documented"
    );
}

#[test]
fn test_xp_006_single_layer_no_issue() {
    let temp = tempfile::TempDir::new().unwrap();

    // Only CLAUDE.md exists
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\nThis is Claude.md.",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    let xp_006: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-006")
        .collect();
    assert!(
        xp_006.is_empty(),
        "Should not trigger XP-006 with single instruction layer"
    );
}

// ===== XP-004/005/006 Edge Case Tests (review findings) =====

#[test]
fn test_xp_004_three_files_conflicting_managers() {
    let temp = tempfile::TempDir::new().unwrap();

    // CLAUDE.md uses npm
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\nUse `npm install` for dependencies.",
    )
    .unwrap();

    // AGENTS.md uses pnpm
    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\nUse `pnpm install` for dependencies.",
    )
    .unwrap();

    // Add .cursor rules directory with yarn
    let cursor_dir = temp.path().join(".cursor").join("rules");
    std::fs::create_dir_all(&cursor_dir).unwrap();
    std::fs::write(
        cursor_dir.join("dev.mdc"),
        "# Rules\n\nUse `yarn install` for dependencies.",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    let xp_004: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-004")
        .collect();

    // Should detect conflicts between all three different package managers
    assert!(
        xp_004.len() >= 2,
        "Should detect multiple conflicts with 3 different package managers, got {}",
        xp_004.len()
    );
}

#[test]
fn test_xp_004_disabled_rule() {
    let temp = tempfile::TempDir::new().unwrap();

    // CLAUDE.md uses npm
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\nUse `npm install` for dependencies.",
    )
    .unwrap();

    // AGENTS.md uses pnpm
    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\nUse `pnpm install` for dependencies.",
    )
    .unwrap();

    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = vec!["XP-004".to_string()];
    let result = validate_project(temp.path(), &config).unwrap();

    let xp_004: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-004")
        .collect();
    assert!(xp_004.is_empty(), "XP-004 should not fire when disabled");
}

#[test]
fn test_xp_005_disabled_rule() {
    let temp = tempfile::TempDir::new().unwrap();

    // CLAUDE.md allows Bash
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\nallowed-tools: Read Write Bash",
    )
    .unwrap();

    // AGENTS.md disallows Bash
    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\nNever use Bash for operations.",
    )
    .unwrap();

    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = vec!["XP-005".to_string()];
    let result = validate_project(temp.path(), &config).unwrap();

    let xp_005: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-005")
        .collect();
    assert!(xp_005.is_empty(), "XP-005 should not fire when disabled");
}

#[test]
fn test_xp_006_disabled_rule() {
    let temp = tempfile::TempDir::new().unwrap();

    // Both files exist but neither documents precedence
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\nThis is Claude.md.",
    )
    .unwrap();

    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\nThis is Agents.md.",
    )
    .unwrap();

    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = vec!["XP-006".to_string()];
    let result = validate_project(temp.path(), &config).unwrap();

    let xp_006: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-006")
        .collect();
    assert!(xp_006.is_empty(), "XP-006 should not fire when disabled");
}

#[test]
fn test_xp_empty_instruction_files() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create empty CLAUDE.md and AGENTS.md
    std::fs::write(temp.path().join("CLAUDE.md"), "").unwrap();
    std::fs::write(temp.path().join("AGENTS.md"), "").unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // XP-004 should not fire for empty files (no commands)
    let xp_004: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-004")
        .collect();
    assert!(xp_004.is_empty(), "Empty files should not trigger XP-004");

    // XP-005 should not fire for empty files (no constraints)
    let xp_005: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-005")
        .collect();
    assert!(xp_005.is_empty(), "Empty files should not trigger XP-005");
}

#[test]
fn test_xp_005_case_insensitive_tool_matching() {
    let temp = tempfile::TempDir::new().unwrap();

    // CLAUDE.md allows BASH (uppercase)
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\nallowed-tools: Read Write BASH",
    )
    .unwrap();

    // AGENTS.md disallows bash (lowercase)
    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\nNever use bash for operations.",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    let xp_005: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-005")
        .collect();
    assert!(
        !xp_005.is_empty(),
        "Should detect conflict between BASH and bash (case-insensitive)"
    );
}

#[test]
fn test_xp_005_word_boundary_no_false_positive() {
    let temp = tempfile::TempDir::new().unwrap();

    // CLAUDE.md allows Bash
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\nallowed-tools: Read Write Bash",
    )
    .unwrap();

    // AGENTS.md mentions "subash" (not "Bash")
    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\nNever use subash command.",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    let xp_005: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XP-005")
        .collect();
    assert!(
        xp_005.is_empty(),
        "Should NOT detect conflict - 'subash' is not 'Bash'"
    );
}

// ===== VER-001 Version Awareness Tests =====

#[test]
fn test_ver_001_warns_when_no_versions_pinned() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create minimal project
    std::fs::write(temp.path().join("CLAUDE.md"), "# Project\n\nInstructions.").unwrap();

    // Default config has no versions pinned
    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    let ver_001: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "VER-001")
        .collect();
    assert!(
        !ver_001.is_empty(),
        "Should warn when no tool/spec versions are pinned"
    );
    // Should be Info level
    assert_eq!(ver_001[0].level, DiagnosticLevel::Info);
}

#[test]
fn test_ver_001_no_warning_when_tool_version_pinned() {
    let temp = tempfile::TempDir::new().unwrap();

    std::fs::write(temp.path().join("CLAUDE.md"), "# Project\n\nInstructions.").unwrap();

    let mut config = LintConfig::default();
    config.tool_versions_mut().claude_code = Some("2.1.3".to_string());
    let result = validate_project(temp.path(), &config).unwrap();

    let ver_001: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "VER-001")
        .collect();
    assert!(
        ver_001.is_empty(),
        "Should NOT warn when a tool version is pinned"
    );
}

#[test]
fn test_ver_001_no_warning_when_spec_revision_pinned() {
    let temp = tempfile::TempDir::new().unwrap();

    std::fs::write(temp.path().join("CLAUDE.md"), "# Project\n\nInstructions.").unwrap();

    let mut config = LintConfig::default();
    config.spec_revisions_mut().mcp_protocol = Some("2025-11-25".to_string());
    let result = validate_project(temp.path(), &config).unwrap();

    let ver_001: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "VER-001")
        .collect();
    assert!(
        ver_001.is_empty(),
        "Should NOT warn when a spec revision is pinned"
    );
}

#[test]
fn test_ver_001_disabled_rule() {
    let temp = tempfile::TempDir::new().unwrap();

    std::fs::write(temp.path().join("CLAUDE.md"), "# Project\n\nInstructions.").unwrap();

    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = vec!["VER-001".to_string()];
    let result = validate_project(temp.path(), &config).unwrap();

    let ver_001: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "VER-001")
        .collect();
    assert!(ver_001.is_empty(), "VER-001 should not fire when disabled");
}

// ===== AGM Validation Integration Tests =====

#[test]
fn test_agm_001_unclosed_code_block() {
    let temp = tempfile::TempDir::new().unwrap();

    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\n```rust\nfn main() {}",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    let agm_001: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "AGM-001")
        .collect();
    assert!(!agm_001.is_empty(), "Should detect unclosed code block");
}

#[test]
fn test_agm_003_over_char_limit() {
    let temp = tempfile::TempDir::new().unwrap();

    let content = format!("# Project\n\n{}", "x".repeat(13000));
    std::fs::write(temp.path().join("AGENTS.md"), content).unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    let agm_003: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "AGM-003")
        .collect();
    assert!(
        !agm_003.is_empty(),
        "Should detect character limit exceeded"
    );
}

#[test]
fn test_agm_005_unguarded_platform_features() {
    let temp = tempfile::TempDir::new().unwrap();

    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\n- type: PreToolExecution\n  command: echo test",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    let agm_005: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "AGM-005")
        .collect();
    assert!(
        !agm_005.is_empty(),
        "Should detect unguarded platform features"
    );
}

#[test]
fn test_valid_agents_md_no_agm_errors() {
    let temp = tempfile::TempDir::new().unwrap();

    std::fs::write(
        temp.path().join("AGENTS.md"),
        r#"# Project

This project is a linter for agent configurations.

## Build Commands

Run npm install and npm build.

## Claude Code Specific

- type: PreToolExecution
  command: echo "test"
"#,
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    let agm_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("AGM-") && d.level == DiagnosticLevel::Error)
        .collect();
    assert!(
        agm_errors.is_empty(),
        "Valid AGENTS.md should have no AGM-* errors, got: {:?}",
        agm_errors
    );
}
// ===== Fixture Directory Regression Tests =====

/// Helper to locate the fixtures directory for testing
fn get_fixtures_dir() -> PathBuf {
    workspace_root().join("tests").join("fixtures")
}

#[test]
fn test_validate_fixtures_directory() {
    // Run validate_project() over tests/fixtures/ to verify detect_file_type() works
    // This is a regression guard for fixture layout (issue #74)
    let fixtures_dir = get_fixtures_dir();

    let config = LintConfig::default();
    let result = validate_project(&fixtures_dir, &config).unwrap();

    // Verify skill fixtures trigger expected AS-* rules
    let skill_diagnostics: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("AS-"))
        .collect();

    // deep-reference/SKILL.md should trigger AS-013 (reference too deep)
    assert!(
        skill_diagnostics
            .iter()
            .any(|d| d.rule == "AS-013" && d.file.to_string_lossy().contains("deep-reference")),
        "Expected AS-013 from deep-reference/SKILL.md fixture"
    );

    // missing-frontmatter/SKILL.md should trigger AS-001 (missing frontmatter)
    assert!(
        skill_diagnostics
            .iter()
            .any(|d| d.rule == "AS-001"
                && d.file.to_string_lossy().contains("missing-frontmatter")),
        "Expected AS-001 from missing-frontmatter/SKILL.md fixture"
    );

    // windows-path/SKILL.md should trigger AS-014 (windows path separator)
    assert!(
        skill_diagnostics
            .iter()
            .any(|d| d.rule == "AS-014" && d.file.to_string_lossy().contains("windows-path")),
        "Expected AS-014 from windows-path/SKILL.md fixture"
    );

    // Verify MCP fixtures trigger expected MCP-* rules
    let mcp_diagnostics: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("MCP-"))
        .collect();

    // At least some MCP diagnostics should be present
    assert!(
        !mcp_diagnostics.is_empty(),
        "Expected MCP diagnostics from tests/fixtures/mcp/*.mcp.json files"
    );

    // missing-required-fields.mcp.json should trigger MCP-002 (missing description)
    assert!(
        mcp_diagnostics
            .iter()
            .any(|d| d.rule == "MCP-002"
                && d.file.to_string_lossy().contains("missing-required-fields")),
        "Expected MCP-002 from missing-required-fields.mcp.json fixture"
    );

    // empty-description.mcp.json should trigger MCP-004 (short description)
    assert!(
        mcp_diagnostics
            .iter()
            .any(|d| d.rule == "MCP-004" && d.file.to_string_lossy().contains("empty-description")),
        "Expected MCP-004 from empty-description.mcp.json fixture"
    );

    // invalid-input-schema.mcp.json should trigger MCP-003 (invalid schema)
    assert!(
        mcp_diagnostics
            .iter()
            .any(|d| d.rule == "MCP-003"
                && d.file.to_string_lossy().contains("invalid-input-schema")),
        "Expected MCP-003 from invalid-input-schema.mcp.json fixture"
    );

    // invalid-jsonrpc-version.mcp.json should trigger MCP-001 (invalid jsonrpc)
    assert!(
        mcp_diagnostics
            .iter()
            .any(|d| d.rule == "MCP-001"
                && d.file.to_string_lossy().contains("invalid-jsonrpc-version")),
        "Expected MCP-001 from invalid-jsonrpc-version.mcp.json fixture"
    );

    // missing-consent.mcp.json should trigger MCP-005 (missing consent)
    assert!(
        mcp_diagnostics
            .iter()
            .any(|d| d.rule == "MCP-005" && d.file.to_string_lossy().contains("missing-consent")),
        "Expected MCP-005 from missing-consent.mcp.json fixture"
    );

    // untrusted-annotations.mcp.json should trigger MCP-006 (untrusted annotations)
    assert!(
        mcp_diagnostics
            .iter()
            .any(|d| d.rule == "MCP-006"
                && d.file.to_string_lossy().contains("untrusted-annotations")),
        "Expected MCP-006 from untrusted-annotations.mcp.json fixture"
    );

    // New MCP expansion fixtures (MCP-013..MCP-024)
    let new_mcp_expectations = [
        ("MCP-013", "invalid-tool-name"),
        ("MCP-014", "invalid-output-schema"),
        ("MCP-015", "missing-resource-required-fields"),
        ("MCP-016", "missing-prompt-name"),
        ("MCP-017", "insecure-http-server"),
        ("MCP-018", "plaintext-env-secret"),
        ("MCP-019", "dangerous-stdio-command"),
        ("MCP-020", "invalid-capability-key"),
        ("MCP-021", "wildcard-http-binding"),
        ("MCP-022", "invalid-args-type"),
        ("MCP-023", "duplicate-server-names"),
        ("MCP-024", "empty-server-config"),
    ];

    for (rule, file_part) in new_mcp_expectations {
        assert!(
            mcp_diagnostics
                .iter()
                .any(|d| d.rule == rule && d.file.to_string_lossy().contains(file_part)),
            "Expected {} from {}.mcp.json fixture",
            rule,
            file_part
        );
    }

    // Verify AGM, XP, REF, and XML fixtures trigger expected rules
    let expectations = [
        (
            "AGM-002",
            "no-headers",
            "Expected AGM-002 from agents_md/no-headers/AGENTS.md fixture",
        ),
        (
            "XP-003",
            "hard-coded",
            "Expected XP-003 from cross_platform/hard-coded/AGENTS.md fixture",
        ),
        (
            "REF-001",
            "missing-import",
            "Expected REF-001 from refs/missing-import.md fixture",
        ),
        (
            "REF-002",
            "broken-link",
            "Expected REF-002 from refs/broken-link.md fixture",
        ),
        (
            "XML-001",
            "xml-001-unclosed",
            "Expected XML-001 from xml/xml-001-unclosed.md fixture",
        ),
        (
            "XML-002",
            "xml-002-mismatch",
            "Expected XML-002 from xml/xml-002-mismatch.md fixture",
        ),
        (
            "XML-003",
            "xml-003-unmatched",
            "Expected XML-003 from xml/xml-003-unmatched.md fixture",
        ),
    ];

    for (rule, file_part, message) in expectations {
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| { d.rule == rule && d.file.to_string_lossy().contains(file_part) }),
            "{}",
            message
        );
    }

    let amp_diagnostics: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("AMP-"))
        .collect();
    assert!(
        amp_diagnostics.iter().any(|d| {
            d.rule == "AMP-001"
                && d.file
                    .to_string_lossy()
                    .replace('\\', "/")
                    .contains("amp-checks/.agents/checks/missing-name.md")
        }),
        "Expected AMP-001 from amp-checks/.agents/checks/missing-name.md fixture"
    );
    assert!(
        amp_diagnostics.iter().any(|d| {
            d.rule == "AMP-002"
                && d.file
                    .to_string_lossy()
                    .replace('\\', "/")
                    .contains("amp-checks/.agents/checks/invalid-severity.md")
        }),
        "Expected AMP-002 from amp-checks/.agents/checks/invalid-severity.md fixture"
    );
    assert!(
        amp_diagnostics.iter().any(|d| {
            d.rule == "AMP-003"
                && d.file
                    .to_string_lossy()
                    .replace('\\', "/")
                    .contains("amp-checks/AGENTS.md")
        }),
        "Expected AMP-003 from amp-checks/AGENTS.md fixture"
    );
    assert!(
        amp_diagnostics.iter().any(|d| {
            d.rule == "AMP-004"
                && d.file
                    .to_string_lossy()
                    .replace('\\', "/")
                    .contains("amp-checks/.amp/settings.json")
        }),
        "Expected AMP-004 from amp-checks/.amp/settings.json fixture"
    );
    assert!(
        !amp_diagnostics.iter().any(|d| {
            d.file
                .to_string_lossy()
                .replace('\\', "/")
                .contains("amp-checks/.agents/checks/valid.md")
        }),
        "Expected no AMP diagnostics for amp-checks/.agents/checks/valid.md fixture"
    );
}

#[test]
fn test_fixture_positive_cases_by_family() {
    let fixtures_dir = get_fixtures_dir();
    let config = LintConfig::default();

    let temp = tempfile::TempDir::new().unwrap();
    let pe_source = fixtures_dir.join("valid/pe/prompt-complete-valid.md");
    let pe_content = std::fs::read_to_string(&pe_source)
        .unwrap_or_else(|_| panic!("Failed to read {}", pe_source.display()));
    let pe_path = temp.path().join("CLAUDE.md");
    std::fs::write(&pe_path, pe_content).unwrap();

    let mut cases = vec![
        ("AGM-", fixtures_dir.join("agents_md/valid/AGENTS.md")),
        ("XP-", fixtures_dir.join("cross_platform/valid/AGENTS.md")),
        ("MCP-", fixtures_dir.join("mcp/valid-tool.mcp.json")),
        ("REF-", fixtures_dir.join("refs/valid-links.md")),
        ("XML-", fixtures_dir.join("xml/xml-valid.md")),
        (
            "AMP-",
            fixtures_dir.join("amp-checks/.agents/checks/valid.md"),
        ),
    ];
    cases.push(("PE-", pe_path));

    for (prefix, path) in cases {
        let diagnostics = expect_success(validate_file(&path, &config).unwrap());
        let family_diagnostics: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with(prefix))
            .collect();

        assert!(
            family_diagnostics.is_empty(),
            "Expected no {} diagnostics for fixture {}",
            prefix,
            path.display()
        );
    }
}

#[test]
fn test_fixture_file_type_detection() {
    // Verify that fixture files are detected as correct FileType
    let fixtures_dir = get_fixtures_dir();

    // Skill fixtures should be detected as FileType::Skill
    assert_eq!(
        detect_file_type(&fixtures_dir.join("skills/deep-reference/SKILL.md")),
        FileType::Skill,
        "deep-reference/SKILL.md should be detected as Skill"
    );
    assert_eq!(
        detect_file_type(&fixtures_dir.join("skills/missing-frontmatter/SKILL.md")),
        FileType::Skill,
        "missing-frontmatter/SKILL.md should be detected as Skill"
    );
    assert_eq!(
        detect_file_type(&fixtures_dir.join("skills/windows-path/SKILL.md")),
        FileType::Skill,
        "windows-path/SKILL.md should be detected as Skill"
    );

    // MCP fixtures should be detected as FileType::Mcp
    assert_eq!(
        detect_file_type(&fixtures_dir.join("mcp/valid-tool.mcp.json")),
        FileType::Mcp,
        "valid-tool.mcp.json should be detected as Mcp"
    );
    assert_eq!(
        detect_file_type(&fixtures_dir.join("mcp/empty-description.mcp.json")),
        FileType::Mcp,
        "empty-description.mcp.json should be detected as Mcp"
    );

    // Copilot fixtures should be detected as FileType::Copilot or CopilotScoped
    assert_eq!(
        detect_file_type(&fixtures_dir.join("copilot/.github/copilot-instructions.md")),
        FileType::Copilot,
        "copilot-instructions.md should be detected as Copilot"
    );
    assert_eq!(
        detect_file_type(
            &fixtures_dir.join("copilot/.github/instructions/typescript.instructions.md")
        ),
        FileType::CopilotScoped,
        "typescript.instructions.md should be detected as CopilotScoped"
    );

    // Cline .txt fixtures should be detected as FileType::ClineRulesFolder
    assert_eq!(
        detect_file_type(&fixtures_dir.join("cline/.clinerules/03-python.txt")),
        FileType::ClineRulesFolder,
        "03-python.txt should be detected as ClineRulesFolder"
    );
    assert_eq!(
        detect_file_type(&fixtures_dir.join("cline-invalid/.clinerules/bad-glob.txt")),
        FileType::ClineRulesFolder,
        "bad-glob.txt should be detected as ClineRulesFolder"
    );
    assert_eq!(
        detect_file_type(&fixtures_dir.join("cline-invalid/.clinerules/scalar-paths.txt")),
        FileType::ClineRulesFolder,
        "scalar-paths.txt should be detected as ClineRulesFolder"
    );
    assert_eq!(
        detect_file_type(&fixtures_dir.join("cline-invalid/.clinerules/unknown-keys.txt")),
        FileType::ClineRulesFolder,
        "unknown-keys.txt should be detected as ClineRulesFolder"
    );
}

// ===== Cline Validation Integration Tests =====

#[test]
fn test_validate_cline_fixtures() {
    let fixtures_dir = get_fixtures_dir();
    let config = LintConfig::default();

    // Valid .txt file should produce no CLN-* diagnostics
    let valid_txt = fixtures_dir.join("cline/.clinerules/03-python.txt");
    let diagnostics = expect_success(validate_file(&valid_txt, &config).unwrap());
    let cln_diagnostics: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("CLN-"))
        .collect();
    assert!(
        cln_diagnostics.is_empty(),
        "Expected no CLN-* diagnostics for valid 03-python.txt, got: {:?}",
        cln_diagnostics
    );
}

#[test]
fn test_validate_cline_invalid_bad_glob_txt() {
    let fixtures_dir = get_fixtures_dir();
    let config = LintConfig::default();
    let bad_glob = fixtures_dir.join("cline-invalid/.clinerules/bad-glob.txt");
    let diagnostics = expect_success(validate_file(&bad_glob, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "CLN-002"),
        "Expected CLN-002 from bad-glob.txt fixture"
    );
}

#[test]
fn test_validate_cline_invalid_scalar_paths_txt() {
    let fixtures_dir = get_fixtures_dir();
    let config = LintConfig::default();
    let scalar_paths = fixtures_dir.join("cline-invalid/.clinerules/scalar-paths.txt");
    let diagnostics = expect_success(validate_file(&scalar_paths, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "CLN-004"),
        "Expected CLN-004 from scalar-paths.txt fixture"
    );
}

#[test]
fn test_validate_cline_invalid_unknown_keys_txt() {
    let fixtures_dir = get_fixtures_dir();
    let config = LintConfig::default();
    let unknown_keys = fixtures_dir.join("cline-invalid/.clinerules/unknown-keys.txt");
    let diagnostics = expect_success(validate_file(&unknown_keys, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "CLN-003"),
        "Expected CLN-003 from unknown-keys.txt fixture"
    );
}

// ===== GitHub Copilot Validation Integration Tests =====

#[test]
fn test_detect_copilot_global() {
    assert_eq!(
        detect_file_type(Path::new(".github/copilot-instructions.md")),
        FileType::Copilot
    );
    assert_eq!(
        detect_file_type(Path::new("project/.github/copilot-instructions.md")),
        FileType::Copilot
    );
}

#[test]
fn test_detect_copilot_scoped() {
    assert_eq!(
        detect_file_type(Path::new(".github/instructions/typescript.instructions.md")),
        FileType::CopilotScoped
    );
    assert_eq!(
        detect_file_type(Path::new(
            "project/.github/instructions/rust.instructions.md"
        )),
        FileType::CopilotScoped
    );
}

#[test]
fn test_copilot_not_detected_outside_github() {
    // Files outside .github/ should not be detected as Copilot
    assert_ne!(
        detect_file_type(Path::new("copilot-instructions.md")),
        FileType::Copilot
    );
    assert_ne!(
        detect_file_type(Path::new("instructions/typescript.instructions.md")),
        FileType::CopilotScoped
    );
}

#[test]
fn test_validators_for_copilot() {
    let registry = ValidatorRegistry::with_defaults();

    let copilot_validators = registry.validators_for(FileType::Copilot);
    assert_eq!(copilot_validators.len(), 2); // copilot + xml

    let scoped_validators = registry.validators_for(FileType::CopilotScoped);
    assert_eq!(scoped_validators.len(), 2); // copilot + xml
}

#[test]
fn test_validate_copilot_fixtures() {
    // Use validate_file directly since .github is a hidden directory
    // that ignore::WalkBuilder skips by default
    let fixtures_dir = get_fixtures_dir();
    let copilot_dir = fixtures_dir.join("copilot");

    let config = LintConfig::default();

    // Validate global instructions
    let global_path = copilot_dir.join(".github/copilot-instructions.md");
    let diagnostics = expect_success(validate_file(&global_path, &config).unwrap());
    let cop_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("COP-") && d.level == DiagnosticLevel::Error)
        .collect();
    assert!(
        cop_errors.is_empty(),
        "Valid global file should have no COP errors, got: {:?}",
        cop_errors
    );

    // Validate scoped instructions
    let scoped_path = copilot_dir.join(".github/instructions/typescript.instructions.md");
    let diagnostics = expect_success(validate_file(&scoped_path, &config).unwrap());
    let cop_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("COP-") && d.level == DiagnosticLevel::Error)
        .collect();
    assert!(
        cop_errors.is_empty(),
        "Valid scoped file should have no COP errors, got: {:?}",
        cop_errors
    );

    // Validate custom agent
    let agent_path = copilot_dir.join(".github/agents/reviewer.agent.md");
    let diagnostics = expect_success(validate_file(&agent_path, &config).unwrap());
    let cop_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("COP-") && d.level == DiagnosticLevel::Error)
        .collect();
    assert!(
        cop_errors.is_empty(),
        "Valid custom agent file should have no COP errors, got: {:?}",
        cop_errors
    );
    assert!(
        diagnostics.iter().all(|d| d.rule != "COP-010"),
        "Valid custom agent should not trigger COP-010 warnings, got: {:?}",
        diagnostics
            .iter()
            .filter(|d| d.rule == "COP-010")
            .collect::<Vec<_>>()
    );

    // Validate reusable prompt
    let prompt_path = copilot_dir.join(".github/prompts/refactor.prompt.md");
    let diagnostics = expect_success(validate_file(&prompt_path, &config).unwrap());
    let cop_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("COP-") && d.level == DiagnosticLevel::Error)
        .collect();
    assert!(
        cop_errors.is_empty(),
        "Valid prompt file should have no COP errors, got: {:?}",
        cop_errors
    );

    // Validate hooks.json
    let hooks_path = copilot_dir.join(".github/hooks/hooks.json");
    let diagnostics = expect_success(validate_file(&hooks_path, &config).unwrap());
    assert!(
        diagnostics.iter().all(|d| d.rule != "COP-017"),
        "Valid hooks.json should not trigger COP-017, got: {:?}",
        diagnostics.iter().map(|d| &d.rule).collect::<Vec<_>>()
    );

    // Validate setup workflow
    let setup_steps_path = copilot_dir.join(".github/workflows/copilot-setup-steps.yml");
    let diagnostics = expect_success(validate_file(&setup_steps_path, &config).unwrap());
    assert!(
        diagnostics.iter().all(|d| d.rule != "COP-018"),
        "Valid setup workflow should not trigger COP-018, got: {:?}",
        diagnostics.iter().map(|d| &d.rule).collect::<Vec<_>>()
    );
}

#[test]
fn test_validate_copilot_invalid_fixtures() {
    // Use validate_file directly since .github is a hidden directory
    let fixtures_dir = get_fixtures_dir();
    let copilot_invalid_dir = fixtures_dir.join("copilot-invalid");
    let config = LintConfig::default();

    // COP-001: Empty global file
    let empty_global = copilot_invalid_dir.join(".github/copilot-instructions.md");
    let diagnostics = expect_success(validate_file(&empty_global, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-001"),
        "Expected COP-001 from empty copilot-instructions.md fixture"
    );

    // COP-002: Invalid YAML in bad-frontmatter
    let bad_frontmatter =
        copilot_invalid_dir.join(".github/instructions/bad-frontmatter.instructions.md");
    let diagnostics = expect_success(validate_file(&bad_frontmatter, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-002"),
        "Expected COP-002 from bad-frontmatter.instructions.md fixture"
    );

    // COP-003: Invalid glob in bad-glob
    let bad_glob = copilot_invalid_dir.join(".github/instructions/bad-glob.instructions.md");
    let diagnostics = expect_success(validate_file(&bad_glob, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-003"),
        "Expected COP-003 from bad-glob.instructions.md fixture"
    );

    // COP-004: Unknown keys in unknown-keys
    let unknown_keys =
        copilot_invalid_dir.join(".github/instructions/unknown-keys.instructions.md");
    let diagnostics = expect_success(validate_file(&unknown_keys, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-004"),
        "Expected COP-004 from unknown-keys.instructions.md fixture"
    );

    // COP-005: Invalid excludeAgent value
    let bad_exclude_agent =
        copilot_invalid_dir.join(".github/instructions/bad-exclude-agent.instructions.md");
    let diagnostics = expect_success(validate_file(&bad_exclude_agent, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-005"),
        "Expected COP-005 from bad-exclude-agent.instructions.md fixture"
    );

    // COP-007: Custom agent missing description
    let missing_description =
        copilot_invalid_dir.join(".github/agents/missing-description.agent.md");
    let diagnostics = expect_success(validate_file(&missing_description, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-007"),
        "Expected COP-007 from missing-description.agent.md fixture"
    );

    // COP-008: Unknown custom-agent field
    let unknown_agent_field = copilot_invalid_dir.join(".github/agents/unknown-field.agent.md");
    let diagnostics = expect_success(validate_file(&unknown_agent_field, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-008"),
        "Expected COP-008 from unknown-field.agent.md fixture"
    );

    // COP-009: Invalid custom-agent target
    let invalid_target = copilot_invalid_dir.join(".github/agents/invalid-target.agent.md");
    let diagnostics = expect_success(validate_file(&invalid_target, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-009"),
        "Expected COP-009 from invalid-target.agent.md fixture"
    );

    // COP-010: Invalid infer type
    let invalid_infer = copilot_invalid_dir.join(".github/agents/invalid-infer-type.agent.md");
    let diagnostics = expect_success(validate_file(&invalid_infer, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-010"),
        "Expected COP-010 from invalid-infer-type.agent.md fixture"
    );

    let invalid_infer_null = copilot_invalid_dir.join(".github/agents/invalid-infer-null.agent.md");
    let diagnostics = expect_success(validate_file(&invalid_infer_null, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-010"),
        "Expected COP-010 from invalid-infer-null.agent.md fixture"
    );

    // COP-012: Unsupported GitHub.com fields
    let unsupported_fields = copilot_invalid_dir.join(".github/agents/unsupported-fields.agent.md");
    let diagnostics = expect_success(validate_file(&unsupported_fields, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-012"),
        "Expected COP-012 from unsupported-fields.agent.md fixture"
    );

    // COP-013: Empty prompt body
    let empty_prompt = copilot_invalid_dir.join(".github/prompts/empty.prompt.md");
    let diagnostics = expect_success(validate_file(&empty_prompt, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-013"),
        "Expected COP-013 from empty.prompt.md fixture"
    );

    // COP-014: Unknown prompt field
    let unknown_prompt_field = copilot_invalid_dir.join(".github/prompts/unknown-field.prompt.md");
    let diagnostics = expect_success(validate_file(&unknown_prompt_field, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-014"),
        "Expected COP-014 from unknown-field.prompt.md fixture"
    );

    // COP-015: Invalid prompt agent mode
    let invalid_prompt_agent = copilot_invalid_dir.join(".github/prompts/invalid-agent.prompt.md");
    let diagnostics = expect_success(validate_file(&invalid_prompt_agent, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-015"),
        "Expected COP-015 from invalid-agent.prompt.md fixture"
    );

    // COP-017: Hooks schema violations
    let invalid_hooks = copilot_invalid_dir.join(".github/hooks/hooks.json");
    let diagnostics = expect_success(validate_file(&invalid_hooks, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-017"),
        "Expected COP-017 from hooks.json fixture"
    );

    // COP-018: Missing jobs.copilot-setup-steps in workflow
    let invalid_setup_workflow =
        copilot_invalid_dir.join(".github/workflows/copilot-setup-steps.yml");
    let diagnostics = expect_success(validate_file(&invalid_setup_workflow, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-018"),
        "Expected COP-018 from copilot-setup-steps.yml fixture"
    );
}

#[test]
fn test_validate_copilot_006_too_long() {
    let fixtures_dir = get_fixtures_dir();
    let copilot_too_long_dir = fixtures_dir.join("copilot-too-long");
    let config = LintConfig::default();

    let long_global = copilot_too_long_dir.join(".github/copilot-instructions.md");
    let diagnostics = expect_success(validate_file(&long_global, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-006"),
        "Expected COP-006 from copilot-too-long fixture, got: {:?}",
        diagnostics.iter().map(|d| &d.rule).collect::<Vec<_>>()
    );

    let long_agent = copilot_too_long_dir.join(".github/agents/too-long.agent.md");
    let diagnostics = expect_success(validate_file(&long_agent, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "COP-011"),
        "Expected COP-011 from too-long.agent.md fixture, got: {:?}",
        diagnostics.iter().map(|d| &d.rule).collect::<Vec<_>>()
    );
}

#[test]
fn test_validate_copilot_file_empty() {
    // Test validate_file directly (not validate_project which skips hidden dirs)
    let temp = tempfile::TempDir::new().unwrap();
    let github_dir = temp.path().join(".github");
    std::fs::create_dir_all(&github_dir).unwrap();
    let file_path = github_dir.join("copilot-instructions.md");
    std::fs::write(&file_path, "").unwrap();

    let config = LintConfig::default();
    let diagnostics = expect_success(validate_file(&file_path, &config).unwrap());

    let cop_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-001").collect();
    assert_eq!(cop_001.len(), 1, "Expected COP-001 for empty file");
}

#[test]
fn test_validate_copilot_scoped_missing_frontmatter() {
    // Test validate_file directly
    let temp = tempfile::TempDir::new().unwrap();
    let instructions_dir = temp.path().join(".github").join("instructions");
    std::fs::create_dir_all(&instructions_dir).unwrap();
    let file_path = instructions_dir.join("test.instructions.md");
    std::fs::write(&file_path, "# Instructions without frontmatter").unwrap();

    let config = LintConfig::default();
    let diagnostics = expect_success(validate_file(&file_path, &config).unwrap());

    let cop_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "COP-002").collect();
    assert_eq!(cop_002.len(), 1, "Expected COP-002 for missing frontmatter");
}

#[test]
fn test_validate_copilot_valid_scoped() {
    // Test validate_file directly
    let temp = tempfile::TempDir::new().unwrap();
    let instructions_dir = temp.path().join(".github").join("instructions");
    std::fs::create_dir_all(&instructions_dir).unwrap();
    let file_path = instructions_dir.join("rust.instructions.md");
    std::fs::write(
        &file_path,
        r#"---
applyTo: "**/*.rs"
---
# Rust Instructions

Use idiomatic Rust patterns.
"#,
    )
    .unwrap();

    let config = LintConfig::default();
    let diagnostics = expect_success(validate_file(&file_path, &config).unwrap());

    let cop_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("COP-") && d.level == DiagnosticLevel::Error)
        .collect();
    assert!(
        cop_errors.is_empty(),
        "Valid scoped file should have no COP errors"
    );
}

#[test]
fn test_validate_project_finds_github_hidden_dir() {
    // Test validate_project walks .github directory (not just validate_file)
    let temp = tempfile::TempDir::new().unwrap();
    let github_dir = temp.path().join(".github");
    std::fs::create_dir_all(&github_dir).unwrap();

    // Create an empty copilot-instructions.md file (should trigger COP-001)
    let file_path = github_dir.join("copilot-instructions.md");
    std::fs::write(&file_path, "").unwrap();

    let config = LintConfig::default();
    // Use validate_project (directory walk) instead of validate_file
    let result = validate_project(temp.path(), &config).unwrap();

    assert!(
        result.diagnostics.iter().any(|d| d.rule == "COP-001"),
        "validate_project should find .github/copilot-instructions.md and report COP-001. Found: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.rule)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_validate_project_finds_codex_hidden_dir() {
    // Test validate_project walks .codex directory (hidden dot-directory)
    let temp = tempfile::TempDir::new().unwrap();
    let codex_dir = temp.path().join(".codex");
    std::fs::create_dir_all(&codex_dir).unwrap();

    // Create config.toml with invalid approvalMode (should trigger CDX-001)
    let file_path = codex_dir.join("config.toml");
    std::fs::write(&file_path, "approvalMode = \"yolo\"").unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    assert!(
        result.diagnostics.iter().any(|d| d.rule == "CDX-001"),
        "validate_project should find .codex/config.toml and report CDX-001. Found: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.rule)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_validate_project_finds_codex_invalid_fixtures() {
    // Test validate_project on the actual codex-invalid fixture directory
    let fixtures_dir = get_fixtures_dir();
    let codex_invalid_dir = fixtures_dir.join("codex-invalid");

    let config = LintConfig::default();
    let result = validate_project(&codex_invalid_dir, &config).unwrap();

    // Should find CDX-001 and CDX-002 from .codex/config.toml
    assert!(
        result.diagnostics.iter().any(|d| d.rule == "CDX-001"),
        "Should report CDX-001 from .codex/config.toml. Rules found: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.rule)
            .collect::<Vec<_>>()
    );
    assert!(
        result.diagnostics.iter().any(|d| d.rule == "CDX-002"),
        "Should report CDX-002 from .codex/config.toml. Rules found: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.rule)
            .collect::<Vec<_>>()
    );
    // Should find CDX-003 from AGENTS.override.md
    assert!(
        result.diagnostics.iter().any(|d| d.rule == "CDX-003"),
        "Should report CDX-003 from AGENTS.override.md. Rules found: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.rule)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_validate_project_finds_copilot_invalid_fixtures() {
    // Test validate_project on the actual fixture directory
    let fixtures_dir = get_fixtures_dir();
    let copilot_invalid_dir = fixtures_dir.join("copilot-invalid");

    let config = LintConfig::default();
    let result = validate_project(&copilot_invalid_dir, &config).unwrap();

    // Should find COP-001 from empty copilot-instructions.md
    assert!(
        result.diagnostics.iter().any(|d| d.rule == "COP-001"),
        "validate_project should find COP-001 in copilot-invalid fixtures. Found rules: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.rule)
            .collect::<Vec<_>>()
    );

    // Should find COP-002 from bad-frontmatter.instructions.md
    assert!(
        result.diagnostics.iter().any(|d| d.rule == "COP-002"),
        "validate_project should find COP-002 in copilot-invalid fixtures. Found rules: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.rule)
            .collect::<Vec<_>>()
    );
}

// ===== Cursor Project Rules Validation Integration Tests =====

#[test]
fn test_detect_cursor_rule() {
    assert_eq!(
        detect_file_type(Path::new(".cursor/rules/typescript.mdc")),
        FileType::CursorRule
    );
    assert_eq!(
        detect_file_type(Path::new(".cursor/rules/typescript.md")),
        FileType::CursorRule
    );
    assert_eq!(
        detect_file_type(Path::new("project/.cursor/rules/rust.mdc")),
        FileType::CursorRule
    );
    assert_eq!(
        detect_file_type(Path::new("project/.cursor/rules/frontend/rust.md")),
        FileType::CursorRule
    );
}

#[test]
fn test_detect_cursor_hooks_agent_environment() {
    assert_eq!(
        detect_file_type(Path::new(".cursor/hooks.json")),
        FileType::CursorHooks
    );
    assert_eq!(
        detect_file_type(Path::new(".cursor/environment.json")),
        FileType::CursorEnvironment
    );
    assert_eq!(
        detect_file_type(Path::new(".cursor/agents/reviewer.md")),
        FileType::CursorAgent
    );
    assert_eq!(
        detect_file_type(Path::new("project/.cursor/agents/nested/reviewer.md")),
        FileType::CursorAgent
    );
    assert_eq!(
        detect_file_type(Path::new("project/.cursor/agents/AGENTS.md")),
        FileType::CursorAgent
    );
    assert_eq!(
        detect_file_type(Path::new("project/.cursor/agents/CLAUDE.md")),
        FileType::CursorAgent
    );
}

#[test]
fn test_detect_cursor_legacy() {
    assert_eq!(
        detect_file_type(Path::new(".cursorrules")),
        FileType::CursorRulesLegacy
    );
    assert_eq!(
        detect_file_type(Path::new("project/.cursorrules")),
        FileType::CursorRulesLegacy
    );
    // Also test .cursorrules.md variant
    assert_eq!(
        detect_file_type(Path::new(".cursorrules.md")),
        FileType::CursorRulesLegacy
    );
    assert_eq!(
        detect_file_type(Path::new("project/.cursorrules.md")),
        FileType::CursorRulesLegacy
    );
}

#[test]
fn test_cursor_not_detected_outside_cursor_dir() {
    // .md/.mdc files outside .cursor/rules/ should not be detected as CursorRule
    assert_ne!(
        detect_file_type(Path::new("rules/typescript.mdc")),
        FileType::CursorRule
    );
    assert_ne!(
        detect_file_type(Path::new("rules/typescript.md")),
        FileType::CursorRule
    );
    assert_ne!(
        detect_file_type(Path::new(".cursor/typescript.mdc")),
        FileType::CursorRule
    );
    assert_ne!(
        detect_file_type(Path::new(".cursor/notes.md")),
        FileType::CursorRule
    );
}

#[test]
fn test_validators_for_cursor() {
    let registry = ValidatorRegistry::with_defaults();

    let cursor_validators = registry.validators_for(FileType::CursorRule);
    assert_eq!(cursor_validators.len(), 3); // cursor + prompt + claude_md

    let hooks_validators = registry.validators_for(FileType::CursorHooks);
    assert_eq!(hooks_validators.len(), 1); // cursor
    assert_eq!(hooks_validators[0].name(), "CursorValidator");

    let agent_validators = registry.validators_for(FileType::CursorAgent);
    assert_eq!(agent_validators.len(), 1); // cursor
    assert_eq!(agent_validators[0].name(), "CursorValidator");

    let environment_validators = registry.validators_for(FileType::CursorEnvironment);
    assert_eq!(environment_validators.len(), 1); // cursor
    assert_eq!(environment_validators[0].name(), "CursorValidator");

    let legacy_validators = registry.validators_for(FileType::CursorRulesLegacy);
    assert_eq!(legacy_validators.len(), 3); // cursor + prompt + claude_md
}

#[test]
fn test_validate_cursor_fixtures() {
    // Use validate_file directly since .cursor is a hidden directory
    let fixtures_dir = get_fixtures_dir();
    let cursor_dir = fixtures_dir.join("cursor");

    let config = LintConfig::default();

    // Validate valid .mdc file
    let valid_path = cursor_dir.join(".cursor/rules/valid.mdc");
    let diagnostics = expect_success(validate_file(&valid_path, &config).unwrap());
    let cur_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("CUR-") && d.level == DiagnosticLevel::Error)
        .collect();
    assert!(
        cur_errors.is_empty(),
        "Valid .mdc file should have no CUR errors, got: {:?}",
        cur_errors
    );

    // Validate .mdc file with multiple globs
    let multiple_globs_path = cursor_dir.join(".cursor/rules/multiple-globs.mdc");
    let diagnostics = expect_success(validate_file(&multiple_globs_path, &config).unwrap());
    let cur_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("CUR-") && d.level == DiagnosticLevel::Error)
        .collect();
    assert!(
        cur_errors.is_empty(),
        "Valid .mdc file with multiple globs should have no CUR errors, got: {:?}",
        cur_errors
    );

    let hooks_path = cursor_dir.join(".cursor/hooks.json");
    let diagnostics = expect_success(validate_file(&hooks_path, &config).unwrap());
    assert!(
        diagnostics.iter().all(|d| !matches!(
            d.rule.as_str(),
            "CUR-010" | "CUR-011" | "CUR-012" | "CUR-013"
        )),
        "Valid hooks fixture should have no CUR-010..CUR-013 diagnostics, got: {:?}",
        diagnostics
            .iter()
            .map(|d| (&d.rule, &d.message))
            .collect::<Vec<_>>()
    );

    let agent_path = cursor_dir.join(".cursor/agents/reviewer.md");
    let diagnostics = expect_success(validate_file(&agent_path, &config).unwrap());
    assert!(
        diagnostics
            .iter()
            .all(|d| !matches!(d.rule.as_str(), "CUR-014" | "CUR-015")),
        "Valid agent fixture should have no CUR-014/CUR-015 diagnostics, got: {:?}",
        diagnostics
            .iter()
            .map(|d| (&d.rule, &d.message))
            .collect::<Vec<_>>()
    );

    let environment_path = cursor_dir.join(".cursor/environment.json");
    let diagnostics = expect_success(validate_file(&environment_path, &config).unwrap());
    assert!(
        diagnostics.iter().all(|d| d.rule != "CUR-016"),
        "Valid environment fixture should have no CUR-016 diagnostics, got: {:?}",
        diagnostics
            .iter()
            .map(|d| (&d.rule, &d.message))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_validate_cursor_invalid_fixtures() {
    // Use validate_file directly since .cursor is a hidden directory
    let fixtures_dir = get_fixtures_dir();
    let cursor_invalid_dir = fixtures_dir.join("cursor-invalid");
    let config = LintConfig::default();

    // CUR-001: Empty .mdc file
    let empty_mdc = cursor_invalid_dir.join(".cursor/rules/empty.mdc");
    let diagnostics = expect_success(validate_file(&empty_mdc, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "CUR-001"),
        "Expected CUR-001 from empty.mdc fixture"
    );

    // CUR-002: Missing frontmatter
    let no_frontmatter = cursor_invalid_dir.join(".cursor/rules/no-frontmatter.mdc");
    let diagnostics = expect_success(validate_file(&no_frontmatter, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "CUR-002"),
        "Expected CUR-002 from no-frontmatter.mdc fixture"
    );

    // CUR-003: Invalid YAML
    let bad_yaml = cursor_invalid_dir.join(".cursor/rules/bad-yaml.mdc");
    let diagnostics = expect_success(validate_file(&bad_yaml, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "CUR-003"),
        "Expected CUR-003 from bad-yaml.mdc fixture"
    );

    // CUR-004: Invalid glob pattern
    let bad_glob = cursor_invalid_dir.join(".cursor/rules/bad-glob.mdc");
    let diagnostics = expect_success(validate_file(&bad_glob, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "CUR-004"),
        "Expected CUR-004 from bad-glob.mdc fixture"
    );

    // CUR-005: Unknown keys
    let unknown_keys = cursor_invalid_dir.join(".cursor/rules/unknown-keys.mdc");
    let diagnostics = expect_success(validate_file(&unknown_keys, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "CUR-005"),
        "Expected CUR-005 from unknown-keys.mdc fixture"
    );

    // CUR-010: Invalid hooks schema
    let cur_010_hooks = cursor_invalid_dir.join("hooks-cur010/.cursor/hooks.json");
    let diagnostics = expect_success(validate_file(&cur_010_hooks, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "CUR-010"),
        "Expected CUR-010 from hooks-cur010 fixture"
    );

    // CUR-011/CUR-012/CUR-013 from malformed hook entry
    let cur_011_to_013 = cursor_invalid_dir.join("hooks-cur011-013/.cursor/hooks.json");
    let diagnostics = expect_success(validate_file(&cur_011_to_013, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "CUR-011"),
        "Expected CUR-011 from hooks-cur011-013 fixture"
    );
    assert!(
        diagnostics.iter().any(|d| d.rule == "CUR-012"),
        "Expected CUR-012 from hooks-cur011-013 fixture"
    );
    assert!(
        diagnostics.iter().any(|d| d.rule == "CUR-013"),
        "Expected CUR-013 from hooks-cur011-013 fixture"
    );

    // CUR-014: Invalid Cursor agent frontmatter
    let cur_014_agent = cursor_invalid_dir.join("agent-cur014/.cursor/agents/reviewer.md");
    let diagnostics = expect_success(validate_file(&cur_014_agent, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "CUR-014"),
        "Expected CUR-014 from agent-cur014 fixture"
    );

    // CUR-015: Empty Cursor agent body
    let cur_015_agent = cursor_invalid_dir.join("agent-cur015/.cursor/agents/reviewer.md");
    let diagnostics = expect_success(validate_file(&cur_015_agent, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "CUR-015"),
        "Expected CUR-015 from agent-cur015 fixture"
    );

    // CUR-016: Invalid environment schema
    let cur_016_environment =
        cursor_invalid_dir.join("environment-cur016/.cursor/environment.json");
    let diagnostics = expect_success(validate_file(&cur_016_environment, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "CUR-016"),
        "Expected CUR-016 from environment-cur016 fixture"
    );
}

#[test]
fn test_validate_cursor_legacy_fixture() {
    let fixtures_dir = get_fixtures_dir();
    let legacy_path = fixtures_dir.join("cursor-legacy/.cursorrules");
    let config = LintConfig::default();

    let diagnostics = expect_success(validate_file(&legacy_path, &config).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "CUR-006"),
        "Expected CUR-006 from .cursorrules fixture"
    );
}

#[test]
fn test_validate_cursor_file_empty() {
    let temp = tempfile::TempDir::new().unwrap();
    let cursor_dir = temp.path().join(".cursor").join("rules");
    std::fs::create_dir_all(&cursor_dir).unwrap();
    let file_path = cursor_dir.join("empty.mdc");
    std::fs::write(&file_path, "").unwrap();

    let config = LintConfig::default();
    let diagnostics = expect_success(validate_file(&file_path, &config).unwrap());

    let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
    assert_eq!(cur_001.len(), 1, "Expected CUR-001 for empty file");
}

#[test]
fn test_validate_cursor_mdc_missing_frontmatter() {
    let temp = tempfile::TempDir::new().unwrap();
    let cursor_dir = temp.path().join(".cursor").join("rules");
    std::fs::create_dir_all(&cursor_dir).unwrap();
    let file_path = cursor_dir.join("test.mdc");
    std::fs::write(&file_path, "# Rules without frontmatter").unwrap();

    let config = LintConfig::default();
    let diagnostics = expect_success(validate_file(&file_path, &config).unwrap());

    let cur_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-002").collect();
    assert_eq!(cur_002.len(), 1, "Expected CUR-002 for missing frontmatter");
}

#[test]
fn test_validate_cursor_valid_mdc() {
    let temp = tempfile::TempDir::new().unwrap();
    let cursor_dir = temp.path().join(".cursor").join("rules");
    std::fs::create_dir_all(&cursor_dir).unwrap();
    let file_path = cursor_dir.join("rust.mdc");
    std::fs::write(
        &file_path,
        r#"---
description: Rust rules
globs: "**/*.rs"
---
# Rust Rules

Use idiomatic Rust patterns.
"#,
    )
    .unwrap();

    let config = LintConfig::default();
    let diagnostics = expect_success(validate_file(&file_path, &config).unwrap());

    let cur_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("CUR-") && d.level == DiagnosticLevel::Error)
        .collect();
    assert!(
        cur_errors.is_empty(),
        "Valid .mdc file should have no CUR errors"
    );
}

#[test]
fn test_validate_project_finds_cursor_hidden_dir() {
    // Test validate_project walks .cursor directory
    let temp = tempfile::TempDir::new().unwrap();
    let cursor_dir = temp.path().join(".cursor").join("rules");
    std::fs::create_dir_all(&cursor_dir).unwrap();

    // Create an empty .mdc file (should trigger CUR-001)
    let file_path = cursor_dir.join("empty.mdc");
    std::fs::write(&file_path, "").unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    assert!(
        result.diagnostics.iter().any(|d| d.rule == "CUR-001"),
        "validate_project should find .cursor/rules/empty.mdc and report CUR-001. Found: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.rule)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_validate_project_finds_cursor_invalid_fixtures() {
    // Test validate_project on the actual fixture directory
    let fixtures_dir = get_fixtures_dir();
    let cursor_invalid_dir = fixtures_dir.join("cursor-invalid");

    let config = LintConfig::default();
    let result = validate_project(&cursor_invalid_dir, &config).unwrap();

    // Should find CUR-001 from empty.mdc
    assert!(
        result.diagnostics.iter().any(|d| d.rule == "CUR-001"),
        "validate_project should find CUR-001 in cursor-invalid fixtures. Found rules: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.rule)
            .collect::<Vec<_>>()
    );

    // Should find CUR-002 from no-frontmatter.mdc
    assert!(
        result.diagnostics.iter().any(|d| d.rule == "CUR-002"),
        "validate_project should find CUR-002 in cursor-invalid fixtures. Found rules: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.rule)
            .collect::<Vec<_>>()
    );
}

// ===== PE Rules Dispatch Integration Tests =====

#[test]
fn test_pe_rules_dispatched() {
    // Verify PE-* rules are dispatched when validating ClaudeMd file type.
    // Per SPEC.md, PE rules apply to CLAUDE.md and AGENTS.md only (not SKILL.md).
    let fixtures_dir = get_fixtures_dir().join("prompt");
    let config = LintConfig::default();
    let registry = ValidatorRegistry::with_defaults();
    let temp = tempfile::TempDir::new().unwrap();
    let claude_path = temp.path().join("CLAUDE.md");

    // Test cases: (fixture_file, expected_rule)
    let test_cases = [
        ("pe-001-critical-in-middle.md", "PE-001"),
        ("pe-002-cot-on-simple.md", "PE-002"),
        ("pe-003-weak-language.md", "PE-003"),
        ("pe-004-ambiguous.md", "PE-004"),
    ];

    for (fixture, expected_rule) in test_cases {
        let content = std::fs::read_to_string(fixtures_dir.join(fixture))
            .unwrap_or_else(|_| panic!("Failed to read fixture: {}", fixture));
        std::fs::write(&claude_path, &content).unwrap();
        let diagnostics =
            expect_success(validate_file_with_registry(&claude_path, &config, &registry).unwrap());
        assert!(
            diagnostics.iter().any(|d| d.rule == expected_rule),
            "Expected {} from {} content",
            expected_rule,
            fixture
        );
    }

    // Also verify PE rules dispatch on AGENTS.md file type
    let agents_path = temp.path().join("AGENTS.md");
    let pe_003_content =
        std::fs::read_to_string(fixtures_dir.join("pe-003-weak-language.md")).unwrap();
    std::fs::write(&agents_path, &pe_003_content).unwrap();
    let diagnostics =
        expect_success(validate_file_with_registry(&agents_path, &config, &registry).unwrap());
    assert!(
        diagnostics.iter().any(|d| d.rule == "PE-003"),
        "Expected PE-003 from AGENTS.md with weak language content"
    );
}

#[test]
fn test_exclude_patterns_with_absolute_path() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create a structure that should be partially excluded
    let target_dir = temp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();
    std::fs::write(
        target_dir.join("SKILL.md"),
        "---\nname: build-artifact\ndescription: Should be excluded\n---\nBody",
    )
    .unwrap();

    // Create a file that should NOT be excluded
    std::fs::write(
        temp.path().join("SKILL.md"),
        "---\nname: valid-skill\ndescription: Should be validated\n---\nBody",
    )
    .unwrap();

    let mut config = LintConfig::default();
    config.set_exclude(vec!["target/**".to_string()]);

    // Use absolute path (canonicalize returns absolute path)
    let abs_path = std::fs::canonicalize(temp.path()).unwrap();
    let result = validate_project(&abs_path, &config).unwrap();

    // Should NOT have diagnostics from target/SKILL.md (excluded)
    let target_diags: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.file.to_string_lossy().contains("target"))
        .collect();
    assert!(
        target_diags.is_empty(),
        "Files in target/ should be excluded when using absolute path, got: {:?}",
        target_diags
    );
}

#[test]
fn test_exclude_patterns_with_relative_path() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create a structure that should be partially excluded
    let node_modules = temp.path().join("node_modules");
    std::fs::create_dir_all(&node_modules).unwrap();
    std::fs::write(
        node_modules.join("SKILL.md"),
        "---\nname: npm-artifact\ndescription: Should be excluded\n---\nBody",
    )
    .unwrap();

    // Create a file that should NOT be excluded
    std::fs::write(
        temp.path().join("AGENTS.md"),
        "# Project\n\nThis should be validated.",
    )
    .unwrap();

    let mut config = LintConfig::default();
    config.set_exclude(vec!["node_modules/**".to_string()]);

    // Use temp.path() directly to validate exclude pattern handling
    let result = validate_project(temp.path(), &config).unwrap();

    // Should NOT have diagnostics from node_modules/
    let nm_diags: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.file.to_string_lossy().contains("node_modules"))
        .collect();
    assert!(
        nm_diags.is_empty(),
        "Files in node_modules/ should be excluded, got: {:?}",
        nm_diags
    );
}

#[test]
fn test_exclude_patterns_nested_directories() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create deeply nested target directory
    let deep_target = temp.path().join("subproject").join("target").join("debug");
    std::fs::create_dir_all(&deep_target).unwrap();
    std::fs::write(
        deep_target.join("SKILL.md"),
        "---\nname: deep-artifact\ndescription: Deep exclude test\n---\nBody",
    )
    .unwrap();

    let mut config = LintConfig::default();
    // Use ** prefix to match at any level
    config.set_exclude(vec!["**/target/**".to_string()]);

    let abs_path = std::fs::canonicalize(temp.path()).unwrap();
    let result = validate_project(&abs_path, &config).unwrap();

    let target_diags: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.file.to_string_lossy().contains("target"))
        .collect();
    assert!(
        target_diags.is_empty(),
        "Deeply nested target/ files should be excluded, got: {:?}",
        target_diags
    );
}

// ===== ValidationResult files_checked Tests =====

#[test]
fn test_files_checked_with_no_diagnostics() {
    // Test that files_checked is accurate even when there are no diagnostics
    let temp = tempfile::TempDir::new().unwrap();

    // Create valid skill files that produce no diagnostics
    let skill_dir = temp.path().join("skills").join("code-review");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: code-review\ndescription: Use when reviewing code\n---\nBody",
    )
    .unwrap();

    // Create another valid skill
    let skill_dir2 = temp.path().join("skills").join("test-runner");
    std::fs::create_dir_all(&skill_dir2).unwrap();
    std::fs::write(
        skill_dir2.join("SKILL.md"),
        "---\nname: test-runner\ndescription: Use when running tests\n---\nBody",
    )
    .unwrap();

    // Disable VER-001 since we're testing for zero diagnostics on valid files
    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = vec!["VER-001".to_string()];
    let result = validate_project(temp.path(), &config).unwrap();

    // Should have counted exactly the two valid skill files
    assert_eq!(
        result.files_checked, 2,
        "files_checked should count exactly the validated skill files, got {}",
        result.files_checked
    );
    assert!(
        result.diagnostics.is_empty(),
        "Valid skill files should have no diagnostics"
    );
}

#[test]
fn test_files_checked_excludes_unknown_file_types() {
    // Test that files_checked only counts recognized file types
    let temp = tempfile::TempDir::new().unwrap();

    // Create files of unknown type
    std::fs::write(temp.path().join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();

    // Create one recognized file
    std::fs::write(
        temp.path().join("SKILL.md"),
        "---\nname: code-review\ndescription: Use when reviewing code\n---\nBody",
    )
    .unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    // Should only count the SKILL.md file, not .rs or package.json
    assert_eq!(
        result.files_checked, 1,
        "files_checked should only count recognized file types"
    );
}

// ===== Concurrent Access Tests =====

#[test]
fn test_validator_registry_concurrent_access() {
    use std::sync::Arc;
    use std::thread;

    let registry = Arc::new(ValidatorRegistry::with_defaults());

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let registry = Arc::clone(&registry);
            thread::spawn(move || {
                // Multiple threads accessing validators_for concurrently
                for _ in 0..100 {
                    let _ = registry.validators_for(FileType::Skill);
                    let _ = registry.validators_for(FileType::ClaudeMd);
                    let _ = registry.validators_for(FileType::Mcp);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[test]
fn test_concurrent_file_validation() {
    use std::sync::Arc;
    use std::thread;
    let temp = tempfile::TempDir::new().unwrap();

    // Create multiple files
    for i in 0..5 {
        let skill_dir = temp.path().join(format!("skill-{}", i));
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!(
                "---\nname: skill-{}\ndescription: Skill number {}\n---\nBody",
                i, i
            ),
        )
        .unwrap();
    }

    let config = Arc::new(LintConfig::default());
    let registry = Arc::new(ValidatorRegistry::with_defaults());
    let temp_path = temp.path().to_path_buf();

    let handles: Vec<_> = (0..5)
        .map(|i| {
            let config = Arc::clone(&config);
            let registry = Arc::clone(&registry);
            let path = temp_path.join(format!("skill-{}", i)).join("SKILL.md");
            thread::spawn(move || validate_file_with_registry(&path, &config, &registry))
        })
        .collect();

    for handle in handles {
        let result = handle.join().expect("Thread panicked");
        assert!(result.is_ok(), "Concurrent validation should succeed");
    }
}

#[test]
fn test_concurrent_project_validation() {
    use std::sync::Arc;
    use std::thread;
    let temp = tempfile::TempDir::new().unwrap();

    // Create a project structure
    std::fs::write(
        temp.path().join("SKILL.md"),
        "---\nname: test-skill\ndescription: Test description\n---\nBody",
    )
    .unwrap();
    std::fs::write(temp.path().join("CLAUDE.md"), "# Project memory").unwrap();

    let config = Arc::new(LintConfig::default());
    let temp_path = temp.path().to_path_buf();

    // Run multiple validate_project calls concurrently
    let handles: Vec<_> = (0..5)
        .map(|_| {
            let config = Arc::clone(&config);
            let path = temp_path.clone();
            thread::spawn(move || validate_project(&path, &config))
        })
        .collect();

    let mut results: Vec<_> = handles
        .into_iter()
        .map(|h| {
            h.join()
                .expect("Thread panicked")
                .expect("Validation failed")
        })
        .collect();

    // All results should be identical
    let first = results.pop().unwrap();
    for result in results {
        assert_eq!(
            first.diagnostics.len(),
            result.diagnostics.len(),
            "Concurrent validations should produce identical results"
        );
    }
}

#[test]
fn test_validate_project_with_poisoned_import_cache_does_not_panic() {
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    let temp = tempfile::TempDir::new().unwrap();
    std::fs::write(temp.path().join("notes.md"), "See @missing.md").unwrap();

    // Pre-poison the shared cache before validation starts
    let cache: agnix_core::__internal::ImportCache = Arc::new(RwLock::new(HashMap::new()));
    let cache_for_poison = cache.clone();
    let _ = std::thread::spawn(move || {
        let _guard = cache_for_poison.write().unwrap();
        panic!("poison import cache lock");
    })
    .join();
    assert!(cache.read().is_err(), "Cache lock should be poisoned");

    let mut config = LintConfig::default();
    config.set_import_cache(cache);

    let result = validate_project(temp.path(), &config);
    assert!(
        result.is_ok(),
        "Project validation should continue with a poisoned import cache lock"
    );
    let outcome = result.unwrap();
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|d| d.rule == "REF-001" && d.message.contains("@missing.md")),
        "Imports validation should still run and report missing imports after cache poisoning"
    );
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|d| d.rule == "lint::cache-poison"),
        "Expected lint::cache-poison warning to surface through validation pipeline"
    );
}

// ===== Security: File Count Limit Tests =====

#[test]
fn test_file_count_limit_enforced() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create 15 markdown files
    for i in 0..15 {
        std::fs::write(temp.path().join(format!("file{}.md", i)), "# Content").unwrap();
    }

    // Set a limit of 10 files
    let mut config = LintConfig::default();
    config.set_max_files_to_validate(Some(10));

    let result = validate_project(temp.path(), &config);

    // Should return TooManyFiles error
    assert!(result.is_err(), "Should error when file limit exceeded");
    match result.unwrap_err() {
        CoreError::Validation(ValidationError::TooManyFiles { count, limit }) => {
            assert!(count > 10, "Count should exceed limit");
            assert_eq!(limit, 10);
        }
        e => panic!("Expected TooManyFiles error, got: {:?}", e),
    }
}

#[test]
fn test_file_count_limit_not_exceeded() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create 5 markdown files
    for i in 0..5 {
        std::fs::write(temp.path().join(format!("file{}.md", i)), "# Content").unwrap();
    }

    // Set a limit of 10 files
    let mut config = LintConfig::default();
    config.set_max_files_to_validate(Some(10));

    let result = validate_project(temp.path(), &config);

    // Should succeed
    assert!(
        result.is_ok(),
        "Should succeed when under file limit: {:?}",
        result
    );
}

#[test]
fn test_file_count_limit_disabled() {
    let temp = tempfile::TempDir::new().unwrap();

    // Create 15 markdown files
    for i in 0..15 {
        std::fs::write(temp.path().join(format!("file{}.md", i)), "# Content").unwrap();
    }

    // Disable the limit
    let mut config = LintConfig::default();
    config.set_max_files_to_validate(None);

    let result = validate_project(temp.path(), &config);

    // Should succeed even with many files
    assert!(
        result.is_ok(),
        "Should succeed when file limit disabled: {:?}",
        result
    );
}

#[test]
fn test_default_file_count_limit() {
    let config = LintConfig::default();
    assert_eq!(
        config.max_files_to_validate(),
        Some(config::DEFAULT_MAX_FILES)
    );
    assert_eq!(config::DEFAULT_MAX_FILES, 10_000);
}

#[test]
fn test_file_count_concurrent_validation() {
    // Test that file counting is thread-safe during parallel validation
    let temp = tempfile::TempDir::new().unwrap();

    // Create enough files to trigger parallel validation (rayon will use multiple threads)
    for i in 0..20 {
        std::fs::write(temp.path().join(format!("file{}.md", i)), "# Content").unwrap();
    }

    // Set a limit that allows all files
    let mut config = LintConfig::default();
    config.set_max_files_to_validate(Some(25));

    let result = validate_project(temp.path(), &config);

    // Should succeed - no race condition in file counting
    assert!(
        result.is_ok(),
        "Concurrent validation should handle file counting correctly"
    );

    // Verify the count is accurate
    let validation_result = result.unwrap();
    assert_eq!(
        validation_result.files_checked, 20,
        "Should count all validated files"
    );
}

// ===== Performance Tests =====

#[test]
#[ignore] // Run with: cargo test --release -- --ignored test_validation_scales_to_10k_files
fn test_validation_scales_to_10k_files() {
    // This test verifies that validation can handle 10,000 files (the default limit)
    // in reasonable time. It's marked #[ignore] because it's slow.
    use std::time::Instant;

    let temp = tempfile::TempDir::new().unwrap();

    // Create 10,000 small markdown files
    for i in 0..10_000 {
        std::fs::write(
            temp.path().join(format!("file{:05}.md", i)),
            format!("# File {}\n\nContent here.", i),
        )
        .unwrap();
    }

    let config = LintConfig::default();
    let start = Instant::now();
    let result = validate_project(temp.path(), &config);
    let duration = start.elapsed();

    // Should succeed
    assert!(
        result.is_ok(),
        "Should handle 10,000 files: {:?}",
        result.err()
    );

    // Should complete in reasonable time (adjust threshold based on CI performance)
    // On typical hardware: ~2-10 seconds for 10k files
    assert!(
        duration.as_secs() < 60,
        "10,000 file validation took too long: {:?}",
        duration
    );

    let validation_result = result.unwrap();
    assert_eq!(
        validation_result.files_checked, 10_000,
        "Should have checked all 10,000 files"
    );

    eprintln!(
        "Performance: Validated 10,000 files in {:?} ({:.0} files/sec)",
        duration,
        10_000.0 / duration.as_secs_f64()
    );
}

// =========================================================================
// resolve_file_type tests
// =========================================================================

#[test]
fn test_resolve_file_type_no_config_falls_through() {
    let config = LintConfig::default();
    // No files config patterns -> same as detect_file_type
    assert_eq!(
        resolve_file_type(Path::new("CLAUDE.md"), &config),
        FileType::ClaudeMd
    );
    assert_eq!(
        resolve_file_type(Path::new("main.rs"), &config),
        FileType::Unknown
    );
    assert_eq!(
        resolve_file_type(Path::new("notes/setup.md"), &config),
        FileType::GenericMarkdown
    );
}

#[test]
fn test_resolve_file_type_include_as_memory() {
    let mut config = LintConfig::default();
    config.files_mut().include_as_memory = vec!["docs/ai-rules/*.md".to_string()];
    config.set_root_dir(PathBuf::from("/project"));

    // File matching the pattern -> ClaudeMd
    assert_eq!(
        resolve_file_type(Path::new("/project/docs/ai-rules/coding.md"), &config),
        FileType::ClaudeMd
    );

    // File NOT matching -> falls through to detect_file_type
    assert_eq!(
        resolve_file_type(Path::new("/project/docs/other/coding.md"), &config),
        FileType::Unknown // docs/ is a documentation directory
    );
}

#[test]
fn test_resolve_file_type_include_as_generic() {
    let mut config = LintConfig::default();
    config.files_mut().include_as_generic = vec!["internal/*.md".to_string()];
    config.set_root_dir(PathBuf::from("/project"));

    assert_eq!(
        resolve_file_type(Path::new("/project/internal/notes.md"), &config),
        FileType::GenericMarkdown
    );
}

#[test]
fn test_resolve_file_type_exclude() {
    let mut config = LintConfig::default();
    config.files_mut().exclude = vec!["generated/**".to_string()];
    config.set_root_dir(PathBuf::from("/project"));

    // CLAUDE.md in generated/ -> excluded (Unknown)
    assert_eq!(
        resolve_file_type(Path::new("/project/generated/CLAUDE.md"), &config),
        FileType::Unknown
    );

    // CLAUDE.md outside generated/ -> still ClaudeMd
    assert_eq!(
        resolve_file_type(Path::new("/project/CLAUDE.md"), &config),
        FileType::ClaudeMd
    );
}

#[test]
fn test_resolve_file_type_priority_exclude_over_include() {
    let mut config = LintConfig::default();
    config.files_mut().include_as_memory = vec!["docs/**/*.md".to_string()];
    config.files_mut().exclude = vec!["docs/drafts/**".to_string()];
    config.set_root_dir(PathBuf::from("/project"));

    // In docs/ but also in drafts/ -> exclude wins
    assert_eq!(
        resolve_file_type(Path::new("/project/docs/drafts/wip.md"), &config),
        FileType::Unknown
    );

    // In docs/ but not in drafts/ -> include_as_memory wins
    assert_eq!(
        resolve_file_type(Path::new("/project/docs/rules/coding.md"), &config),
        FileType::ClaudeMd
    );
}

#[test]
fn test_resolve_file_type_priority_memory_over_generic() {
    let mut config = LintConfig::default();
    config.files_mut().include_as_memory = vec!["rules/*.md".to_string()];
    config.files_mut().include_as_generic = vec!["rules/*.md".to_string()]; // overlapping
    config.set_root_dir(PathBuf::from("/project"));

    // When both match, memory takes priority
    assert_eq!(
        resolve_file_type(Path::new("/project/rules/coding.md"), &config),
        FileType::ClaudeMd
    );
}

#[test]
fn test_resolve_file_type_no_root_dir_uses_filename() {
    let mut config = LintConfig::default();
    config.files_mut().include_as_memory = vec!["INSTRUCTIONS.md".to_string()];
    // No root_dir set

    assert_eq!(
        resolve_file_type(Path::new("some/path/INSTRUCTIONS.md"), &config),
        FileType::ClaudeMd
    );
}

#[test]
fn test_resolve_file_type_non_matching_files_fall_through() {
    let mut config = LintConfig::default();
    config.files_mut().include_as_memory = vec!["custom/*.md".to_string()];
    config.set_root_dir(PathBuf::from("/project"));

    // Regular SKILL.md still detected normally
    assert_eq!(
        resolve_file_type(Path::new("/project/SKILL.md"), &config),
        FileType::Skill
    );

    // Regular CLAUDE.md still detected normally
    assert_eq!(
        resolve_file_type(Path::new("/project/CLAUDE.md"), &config),
        FileType::ClaudeMd
    );
}

#[test]
fn test_resolve_file_type_exclude_overrides_builtin() {
    let mut config = LintConfig::default();
    config.files_mut().exclude = vec!["vendor/CLAUDE.md".to_string()];
    config.set_root_dir(PathBuf::from("/project"));

    // CLAUDE.md in vendor/ is excluded even though it would normally be ClaudeMd
    assert_eq!(
        resolve_file_type(Path::new("/project/vendor/CLAUDE.md"), &config),
        FileType::Unknown
    );
}

#[test]
fn test_resolve_file_type_backslash_normalization() {
    let mut config = LintConfig::default();
    config.files_mut().include_as_memory = vec!["docs\\ai-rules\\*.md".to_string()];
    config.set_root_dir(PathBuf::from("/project"));

    // Backslashes in patterns are normalized to forward slashes
    assert_eq!(
        resolve_file_type(Path::new("/project/docs/ai-rules/coding.md"), &config),
        FileType::ClaudeMd
    );
}

#[test]
fn test_resolve_file_type_invalid_pattern_falls_back() {
    let mut config = LintConfig::default();
    config.files_mut().include_as_memory = vec!["[invalid".to_string()];

    // Invalid pattern should fall back to detect_file_type
    assert_eq!(
        resolve_file_type(Path::new("CLAUDE.md"), &config),
        FileType::ClaudeMd
    );
}

// =========================================================================
// Integration tests with tempdir
// =========================================================================

#[test]
fn test_validate_project_with_files_config_include() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path();

    // Create a custom instruction file that would normally be GenericMarkdown.
    // Content includes "Usually" which triggers PE-004 (ambiguous terms) via
    // the PromptValidator. PromptValidator runs for ClaudeMd but NOT for
    // GenericMarkdown, proving the include_as_memory override works correctly.
    let custom_dir = root.join("custom-rules");
    std::fs::create_dir_all(&custom_dir).unwrap();
    let custom_file = custom_dir.join("coding-standards.md");
    std::fs::write(
        &custom_file,
        "# Coding Standards\n\nUsually prefer TypeScript over JavaScript.\n",
    )
    .unwrap();

    // Without config, this file would be GenericMarkdown (in non-doc dir)
    assert_eq!(detect_file_type(&custom_file), FileType::GenericMarkdown);

    // With include_as_memory config, it should be validated as ClaudeMd
    let mut config = LintConfig::default();
    config.files_mut().include_as_memory = vec!["custom-rules/*.md".to_string()];

    let result = validate_project(root, &config).unwrap();
    // Should have checked the file (it's now ClaudeMd, not just GenericMarkdown)
    assert!(result.files_checked > 0);

    // Verify that ClaudeMd-specific validators ran by checking for PE-004
    // (ambiguous instructions). The PromptValidator is registered for ClaudeMd
    // but NOT for GenericMarkdown, so PE-004 firing confirms the file was
    // routed through ClaudeMd validation.
    let pe_diags: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("PE-"))
        .collect();
    assert!(
        !pe_diags.is_empty(),
        "Expected PE-* diagnostics (from PromptValidator, ClaudeMd-only) but found none. \
         This means the file was not validated as ClaudeMd despite include_as_memory config. \
         All diagnostics: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| (&d.rule, &d.message))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_validate_project_with_files_config_exclude() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path();

    // Create a CLAUDE.md that would normally be validated
    std::fs::write(root.join("CLAUDE.md"), "# Project\n\nInstructions here.\n").unwrap();

    // Create a CLAUDE.md in a vendor dir that should be excluded
    let vendor_dir = root.join("vendor");
    std::fs::create_dir_all(&vendor_dir).unwrap();
    std::fs::write(
        vendor_dir.join("CLAUDE.md"),
        "# Vendor instructions\n\nDo not validate this.\n",
    )
    .unwrap();

    // With exclude config
    let mut config = LintConfig::default();
    config.files_mut().exclude = vec!["vendor/**".to_string()];

    let result = validate_project(root, &config).unwrap();
    // Only the root CLAUDE.md should be checked, not vendor/CLAUDE.md
    assert_eq!(
        result.files_checked, 1,
        "Only root CLAUDE.md should be checked, got {}",
        result.files_checked
    );
}

#[test]
fn test_validate_project_with_invalid_files_pattern() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path();

    // Create a file so the project is not empty
    std::fs::write(root.join("CLAUDE.md"), "# Project\n").unwrap();

    let mut config = LintConfig::default();
    config.files_mut().include_as_memory = vec!["[invalid".to_string()];

    // Invalid patterns degrade gracefully: validation proceeds with no
    // file overrides applied (consistent with LintConfig::validate() which
    // only produces warnings for invalid patterns).
    let result = validate_project(root, &config);
    assert!(
        result.is_ok(),
        "Expected graceful degradation for invalid file pattern, got error: {:?}",
        result.unwrap_err()
    );
}

#[test]
fn test_validate_file_respects_files_config_exclude() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path();

    // Create a CLAUDE.md that would normally produce diagnostics
    let claude_file = root.join("CLAUDE.md");
    std::fs::write(&claude_file, "# Project\n\nNever use var.\n").unwrap();

    // With exclude config, the file should be skipped entirely
    let mut config = LintConfig::default();
    config.files_mut().exclude = vec!["CLAUDE.md".to_string()];
    config.set_root_dir(root.to_path_buf());

    let registry = ValidatorRegistry::with_defaults();
    let outcome = validate_file_with_registry(&claude_file, &config, &registry).unwrap();
    assert!(
        outcome.is_skipped(),
        "Expected Skipped for excluded file, got: {:?}",
        outcome
    );
}

#[test]
fn test_resolve_file_type_glob_separator_behavior() {
    // With require_literal_separator=true, `*` should NOT match path separators.
    // `dir/*.md` should match `dir/file.md` but NOT `dir/sub/file.md`.
    // `dir/**/*.md` should match `dir/sub/file.md`.
    let mut config = LintConfig::default();
    config.files_mut().include_as_memory = vec!["dir/*.md".to_string()];
    config.set_root_dir(PathBuf::from("/project"));

    // Single-level match: dir/*.md matches dir/file.md
    assert_eq!(
        resolve_file_type(Path::new("/project/dir/file.md"), &config),
        FileType::ClaudeMd,
        "dir/*.md should match dir/file.md"
    );

    // Multi-level: dir/*.md should NOT match dir/sub/file.md
    assert_ne!(
        resolve_file_type(Path::new("/project/dir/sub/file.md"), &config),
        FileType::ClaudeMd,
        "dir/*.md should NOT match dir/sub/file.md (require_literal_separator)"
    );

    // With ** pattern, multi-level should match
    let mut config2 = LintConfig::default();
    config2.files_mut().include_as_memory = vec!["dir/**/*.md".to_string()];
    config2.set_root_dir(PathBuf::from("/project"));

    assert_eq!(
        resolve_file_type(Path::new("/project/dir/sub/file.md"), &config2),
        FileType::ClaudeMd,
        "dir/**/*.md should match dir/sub/file.md"
    );
}

#[test]
fn test_resolve_file_type_case_sensitive() {
    // Patterns are case-sensitive (FILES_MATCH_OPTIONS.case_sensitive = true).
    // "DEVELOPER.md" should match "DEVELOPER.md" but NOT "developer.md".
    let mut config = LintConfig::default();
    config.files_mut().include_as_memory = vec!["DEVELOPER.md".to_string()];
    config.set_root_dir(PathBuf::from("/project"));

    assert_eq!(
        resolve_file_type(Path::new("/project/DEVELOPER.md"), &config),
        FileType::ClaudeMd,
        "DEVELOPER.md pattern should match DEVELOPER.md"
    );
    assert_ne!(
        resolve_file_type(Path::new("/project/developer.md"), &config),
        FileType::ClaudeMd,
        "DEVELOPER.md pattern should NOT match developer.md (case-sensitive)"
    );
}

#[test]
fn test_resolve_file_type_double_star_recursive() {
    // "instructions/**/*.md" should match files at arbitrary nesting depth.
    let mut config = LintConfig::default();
    config.files_mut().include_as_memory = vec!["instructions/**/*.md".to_string()];
    config.set_root_dir(PathBuf::from("/project"));

    assert_eq!(
        resolve_file_type(Path::new("/project/instructions/sub/deep/file.md"), &config),
        FileType::ClaudeMd,
        "instructions/**/*.md should match instructions/sub/deep/file.md"
    );
    assert_eq!(
        resolve_file_type(Path::new("/project/instructions/file.md"), &config),
        FileType::ClaudeMd,
        "instructions/**/*.md should match instructions/file.md"
    );
    // Should not match files outside the instructions directory
    assert_ne!(
        resolve_file_type(Path::new("/project/other/file.md"), &config),
        FileType::ClaudeMd,
        "instructions/**/*.md should NOT match other/file.md"
    );
}

// ===== validate_project_rules() Tests =====

#[test]
fn test_validate_project_rules_agm006() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Create two AGENTS.md files at different levels
    std::fs::write(temp_dir.path().join("AGENTS.md"), "# Root AGENTS").unwrap();
    let sub_dir = temp_dir.path().join("sub");
    std::fs::create_dir(&sub_dir).unwrap();
    std::fs::write(sub_dir.join("AGENTS.md"), "# Sub AGENTS").unwrap();

    let config = LintConfig::default();
    let diagnostics = validate_project_rules(temp_dir.path(), &config).unwrap();
    let agm006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-006").collect();
    assert!(
        agm006.len() >= 2,
        "Expected AGM-006 for both AGENTS.md files, got {} diagnostics",
        agm006.len()
    );
}

#[test]
fn test_validate_project_rules_empty_dir() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = LintConfig::default();
    let diagnostics = validate_project_rules(temp_dir.path(), &config).unwrap();
    // Only VER-001 should fire (no version pins)
    let non_ver = diagnostics.iter().filter(|d| d.rule != "VER-001").count();
    assert_eq!(
        non_ver, 0,
        "Empty dir should produce no non-VER diagnostics"
    );
}

#[test]
fn test_validate_project_rules_ver001() {
    let temp_dir = tempfile::tempdir().unwrap();
    // No .agnix.toml, no version pins
    let config = LintConfig::default();
    let diagnostics = validate_project_rules(temp_dir.path(), &config).unwrap();
    assert!(
        diagnostics.iter().any(|d| d.rule == "VER-001"),
        "Expected VER-001 when no versions are pinned"
    );
}

#[test]
fn test_validate_project_rules_disabled_rules() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Create two AGENTS.md files
    std::fs::write(temp_dir.path().join("AGENTS.md"), "# Root").unwrap();
    let sub = temp_dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("AGENTS.md"), "# Sub").unwrap();

    let mut config = LintConfig::default();
    config
        .rules_mut()
        .disabled_rules
        .push("AGM-006".to_string());
    config
        .rules_mut()
        .disabled_rules
        .push("VER-001".to_string());

    let diagnostics = validate_project_rules(temp_dir.path(), &config).unwrap();
    assert!(
        !diagnostics.iter().any(|d| d.rule == "AGM-006"),
        "AGM-006 should be disabled"
    );
    assert!(
        !diagnostics.iter().any(|d| d.rule == "VER-001"),
        "VER-001 should be disabled"
    );
}

#[test]
fn test_validate_project_rules_xp004() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Create conflicting instruction files
    std::fs::write(
        temp_dir.path().join("CLAUDE.md"),
        "# Setup\n\nRun `npm install` to install deps.\n`npm test` to run tests.\n",
    )
    .unwrap();
    std::fs::write(
        temp_dir.path().join("AGENTS.md"),
        "# Setup\n\nRun `yarn install` to install deps.\n`yarn test` to run tests.\n",
    )
    .unwrap();

    let config = LintConfig::default();
    let diagnostics = validate_project_rules(temp_dir.path(), &config).unwrap();
    let xp004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-004").collect();
    assert!(
        !xp004.is_empty(),
        "Expected XP-004 for conflicting package managers"
    );
}

#[test]
fn test_validate_project_rules_xp005() {
    let temp_dir = tempfile::tempdir().unwrap();

    // CLAUDE.md allows Bash
    std::fs::write(
        temp_dir.path().join("CLAUDE.md"),
        "# Project\n\nallowed-tools: Read Write Bash\n",
    )
    .unwrap();

    // AGENTS.md disallows Bash
    std::fs::write(
        temp_dir.path().join("AGENTS.md"),
        "# Project\n\nNever use Bash for operations.\n",
    )
    .unwrap();

    let config = LintConfig::default();
    let diagnostics = validate_project_rules(temp_dir.path(), &config).unwrap();
    let xp005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-005").collect();
    assert!(
        !xp005.is_empty(),
        "Expected XP-005 for conflicting tool constraints (Bash allowed in one, disallowed in other)"
    );
    assert!(
        xp005.iter().any(|d| d.message.contains("Bash")),
        "XP-005 diagnostic should mention the conflicting tool 'Bash'"
    );
}

#[test]
fn test_validate_project_rules_xp006() {
    let temp_dir = tempfile::tempdir().unwrap();

    // CLAUDE.md with commands section (no precedence documentation)
    std::fs::write(
        temp_dir.path().join("CLAUDE.md"),
        "# Project\n\n## Commands\n- npm test\n",
    )
    .unwrap();

    // AGENTS.md with commands section (no precedence documentation)
    std::fs::write(
        temp_dir.path().join("AGENTS.md"),
        "# Project\n\n## Commands\n- npm build\n",
    )
    .unwrap();

    let config = LintConfig::default();
    let diagnostics = validate_project_rules(temp_dir.path(), &config).unwrap();
    let xp006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "XP-006").collect();
    assert!(
        !xp006.is_empty(),
        "Expected XP-006 for multiple instruction layers without precedence documentation"
    );
}

// ===== resolve_validation_root file-input Tests =====

#[test]
fn test_validate_project_file_input_single_file() {
    // When a file path is passed to validate_project(), only that single file
    // should be validated - sibling files in other directories are ignored.
    let temp = tempfile::TempDir::new().unwrap();

    let alpha_dir = temp.path().join("skills").join("alpha");
    std::fs::create_dir_all(&alpha_dir).unwrap();
    std::fs::write(
        alpha_dir.join("SKILL.md"),
        "---\nname: deploy-prod\ndescription: Deploys\n---\nBody",
    )
    .unwrap();

    let beta_dir = temp.path().join("skills").join("beta");
    std::fs::create_dir_all(&beta_dir).unwrap();
    std::fs::write(
        beta_dir.join("SKILL.md"),
        "---\nname: deploy-staging\ndescription: Deploys staging\n---\nBody",
    )
    .unwrap();

    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = vec!["VER-001".to_string()];

    // Pass the file path for alpha/SKILL.md, not the directory
    let target_file = alpha_dir.join("SKILL.md");
    let result = validate_project(&target_file, &config).unwrap();

    assert_eq!(
        result.files_checked, 1,
        "Only the targeted file should be checked, got {}",
        result.files_checked
    );

    // All diagnostics should reference the target file, not the beta sibling
    for d in &result.diagnostics {
        assert!(
            d.file.ends_with("alpha/SKILL.md") || d.file.ends_with("alpha\\SKILL.md"),
            "Diagnostic should reference alpha/SKILL.md, got: {}",
            d.file.display()
        );
    }
}

#[test]
fn test_validate_project_file_input_produces_diagnostics() {
    // Passing a single SKILL.md with a known violation should produce diagnostics.
    let temp = tempfile::TempDir::new().unwrap();
    let skill_path = temp.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: deploy-prod\ndescription: Deploys\n---\nBody",
    )
    .unwrap();

    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = vec!["VER-001".to_string()];

    let result = validate_project(&skill_path, &config).unwrap();

    assert_eq!(
        result.files_checked, 1,
        "Exactly one file should be checked, got {}",
        result.files_checked
    );
    assert!(
        result.diagnostics.iter().any(|d| d.rule == "CC-SK-006"),
        "Expected CC-SK-006 for dangerous deploy-prod name, got rules: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.rule)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_validate_project_file_input_valid_file_no_errors() {
    // Passing a valid CLAUDE.md file should produce no diagnostics,
    // even when a sibling SKILL.md has violations.
    let temp = tempfile::TempDir::new().unwrap();

    // Valid CLAUDE.md
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\nInstructions here.",
    )
    .unwrap();

    // Sibling with violations (should not be scanned)
    std::fs::write(
        temp.path().join("SKILL.md"),
        "---\nname: deploy-prod\ndescription: Deploys\n---\nBody",
    )
    .unwrap();

    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = vec!["VER-001".to_string()];

    let target_file = temp.path().join("CLAUDE.md");
    let result = validate_project(&target_file, &config).unwrap();

    assert_eq!(
        result.files_checked, 1,
        "Only the targeted CLAUDE.md should be checked, got {}",
        result.files_checked
    );
    assert!(
        result.diagnostics.is_empty(),
        "Valid CLAUDE.md should produce no diagnostics, got: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.rule)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_validate_project_rules_file_input() {
    // When a file path is passed to validate_project_rules(), the walk is
    // scoped to that single file. AGM-006 requires multiple AGENTS.md files,
    // so it should NOT fire when only one file is walked.
    let temp = tempfile::TempDir::new().unwrap();

    std::fs::write(temp.path().join("AGENTS.md"), "# Root agents").unwrap();

    let sub_dir = temp.path().join("sub");
    std::fs::create_dir_all(&sub_dir).unwrap();
    std::fs::write(sub_dir.join("AGENTS.md"), "# Sub agents").unwrap();

    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = vec!["VER-001".to_string()];

    // Pass the root AGENTS.md file path, not the directory
    let target_file = temp.path().join("AGENTS.md");
    let diagnostics = validate_project_rules(&target_file, &config).unwrap();

    let agm006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-006").collect();
    assert!(
        agm006.is_empty(),
        "AGM-006 should not fire when walk is scoped to a single file, got {} diagnostics",
        agm006.len()
    );
}

#[test]
fn test_validate_project_file_input_unknown_type_skipped() {
    // Passing an unrecognized file type should result in zero files checked
    // and no diagnostics, even when a sibling recognized file has violations.
    let temp = tempfile::TempDir::new().unwrap();

    std::fs::write(temp.path().join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(
        temp.path().join("SKILL.md"),
        "---\nname: deploy-prod\ndescription: Deploys\n---\nBody",
    )
    .unwrap();

    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = vec!["VER-001".to_string()];

    let target_file = temp.path().join("main.rs");
    let result = validate_project(&target_file, &config).unwrap();

    assert_eq!(
        result.files_checked, 0,
        "Unrecognized file type should not be counted, got {}",
        result.files_checked
    );
    assert!(
        result.diagnostics.is_empty(),
        "Unrecognized file type should produce no diagnostics, got: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.rule)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_validate_project_with_registry_file_input() {
    // validate_project_with_registry() should also respect file-input paths,
    // validating only the targeted file.
    let temp = tempfile::TempDir::new().unwrap();
    let skill_path = temp.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: deploy-prod\ndescription: Deploys\n---\nBody",
    )
    .unwrap();

    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = vec!["VER-001".to_string()];

    let registry = ValidatorRegistry::with_defaults();
    let result = validate_project_with_registry(&skill_path, &config, &registry).unwrap();

    assert_eq!(
        result.files_checked, 1,
        "Exactly one file should be checked via registry path, got {}",
        result.files_checked
    );
    assert!(
        !result.diagnostics.is_empty(),
        "Expected diagnostics for deploy-prod skill via registry path"
    );
    assert!(
        result.diagnostics.iter().any(|d| d.rule == "CC-SK-006"),
        "Expected CC-SK-006 for dangerous deploy-prod name via registry path, got rules: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.rule)
            .collect::<Vec<_>>()
    );
}

/// Passing a nonexistent file path should return an error, not silently succeed
/// with 0 files checked.
#[test]
fn test_validate_project_file_input_nonexistent_path() {
    let temp = tempfile::TempDir::new().unwrap();
    let config = LintConfig::builder().build_lenient().unwrap();

    // Pass a nonexistent file - should return Err, not silently succeed
    let nonexistent = temp.path().join("nonexistent.md");
    let result = validate_project(&nonexistent, &config);

    assert!(
        result.is_err(),
        "Nonexistent file path should return Err, got: {:?}",
        result
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Validation root not found"),
        "Error message should contain 'Validation root not found': {err_msg}"
    );
    assert!(
        err_msg.contains(nonexistent.to_str().unwrap()),
        "Error message should contain the path: {err_msg}"
    );
}

#[test]
fn test_validate_project_nonexistent_dir_returns_error() {
    let config = LintConfig::builder().build_lenient().unwrap();
    let nonexistent = Path::new("/nonexistent/path/that/does/not/exist");
    let result = validate_project(nonexistent, &config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Validation root not found"),
        "Error message should contain 'Validation root not found': {err_msg}"
    );
    assert!(
        err_msg.contains(nonexistent.to_str().unwrap()),
        "Error message should contain the path: {err_msg}"
    );
}

#[test]
fn test_validate_project_rules_nonexistent_returns_error() {
    let config = LintConfig::builder().build_lenient().unwrap();
    let nonexistent = Path::new("/nonexistent/path/rules");
    let result = validate_project_rules(nonexistent, &config);
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Validation root not found"),
        "Error message should contain 'Validation root not found': {err_msg}"
    );
    assert!(
        err_msg.contains(nonexistent.to_str().unwrap()),
        "Error message should contain the path: {err_msg}"
    );
}

#[test]
fn test_validate_project_with_registry_nonexistent_returns_error() {
    let registry = ValidatorRegistry::with_defaults();
    let config = LintConfig::builder().build_lenient().unwrap();
    let nonexistent = Path::new("/nonexistent/path/registry");
    let result = validate_project_with_registry(nonexistent, &config, &registry);
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Validation root not found"),
        "Error message should contain 'Validation root not found': {err_msg}"
    );
    assert!(
        err_msg.contains(nonexistent.to_str().unwrap()),
        "Error message should contain the path: {err_msg}"
    );
}

// ============================================================================
// Validator name() tests
// ============================================================================

#[test]
fn test_validator_name_returns_expected_values() {
    let registry = ValidatorRegistry::with_defaults();

    // Skill validators should include known names
    let skill_validators = registry.validators_for(FileType::Skill);
    let names: Vec<&str> = skill_validators.iter().map(|v| v.name()).collect();
    assert!(names.contains(&"SkillValidator"));
    assert!(names.contains(&"PerClientSkillValidator"));
    assert!(names.contains(&"XmlValidator"));
    assert!(names.contains(&"ImportsValidator"));

    // ClaudeMd validators should include known names
    let claude_validators = registry.validators_for(FileType::ClaudeMd);
    let claude_names: Vec<&str> = claude_validators.iter().map(|v| v.name()).collect();
    assert!(claude_names.contains(&"ClaudeMdValidator"));
    assert!(claude_names.contains(&"CrossPlatformValidator"));
    assert!(claude_names.contains(&"AgentsMdValidator"));
    assert!(claude_names.contains(&"PromptValidator"));
}

#[test]
fn test_validator_names_are_ascii_and_nonempty() {
    let registry = ValidatorRegistry::with_defaults();

    // Check all file types that have validators
    let file_types = [
        FileType::Skill,
        FileType::ClaudeMd,
        FileType::Agent,
        FileType::AmpCheck,
        FileType::Hooks,
        FileType::Plugin,
        FileType::Mcp,
        FileType::Copilot,
        FileType::CopilotScoped,
        FileType::ClaudeRule,
        FileType::CursorRule,
        FileType::CursorHooks,
        FileType::CursorAgent,
        FileType::CursorEnvironment,
        FileType::CursorRulesLegacy,
        FileType::ClineRules,
        FileType::ClineRulesFolder,
        FileType::OpenCodeConfig,
        FileType::GeminiMd,
        FileType::GeminiSettings,
        FileType::AmpSettings,
        FileType::GeminiExtension,
        FileType::GeminiIgnore,
        FileType::CodexConfig,
        FileType::GenericMarkdown,
    ];

    for file_type in file_types {
        let validators = registry.validators_for(file_type);
        for v in validators {
            let name = v.name();
            assert!(!name.is_empty(), "Validator name should not be empty");
            assert!(name.is_ascii(), "Validator name should be ASCII: {}", name);
            assert!(
                name.ends_with("Validator"),
                "Validator name should end with 'Validator': {}",
                name
            );
        }
    }
}

// ============================================================================
// Validator metadata() tests
// ============================================================================

const ALL_VALIDATED_FILE_TYPES: &[FileType] = &[
    FileType::Skill,
    FileType::ClaudeMd,
    FileType::Agent,
    FileType::AmpCheck,
    FileType::Hooks,
    FileType::Plugin,
    FileType::Mcp,
    FileType::Copilot,
    FileType::CopilotScoped,
    FileType::ClaudeRule,
    FileType::CursorRule,
    FileType::CursorHooks,
    FileType::CursorAgent,
    FileType::CursorEnvironment,
    FileType::CursorRulesLegacy,
    FileType::ClineRules,
    FileType::ClineRulesFolder,
    FileType::OpenCodeConfig,
    FileType::GeminiMd,
    FileType::GeminiSettings,
    FileType::AmpSettings,
    FileType::GeminiExtension,
    FileType::GeminiIgnore,
    FileType::CodexConfig,
    FileType::GenericMarkdown,
];

#[test]
fn test_all_validators_have_nonempty_rule_ids() {
    let registry = ValidatorRegistry::with_defaults();

    for file_type in ALL_VALIDATED_FILE_TYPES {
        let validators = registry.validators_for(*file_type);
        for v in validators {
            let meta = v.metadata();
            assert!(
                !meta.rule_ids.is_empty(),
                "Validator '{}' (file_type={:?}) should have at least one rule ID",
                meta.name,
                file_type,
            );
        }
    }
}

#[test]
fn test_metadata_name_matches_name_method() {
    let registry = ValidatorRegistry::with_defaults();

    for file_type in ALL_VALIDATED_FILE_TYPES {
        let validators = registry.validators_for(*file_type);
        for v in validators {
            let meta = v.metadata();
            assert_eq!(
                meta.name,
                v.name(),
                "metadata().name should match name() for validator '{}'",
                v.name(),
            );
        }
    }
}

#[test]
fn test_metadata_rule_ids_are_well_formed() {
    let registry = ValidatorRegistry::with_defaults();

    let rule_id_pattern = regex::Regex::new(r"^[A-Z]{1,6}-[A-Z]{0,4}-?\d{1,3}$").unwrap();

    for file_type in ALL_VALIDATED_FILE_TYPES {
        let validators = registry.validators_for(*file_type);
        for v in validators {
            let meta = v.metadata();
            for rule_id in meta.rule_ids {
                assert!(
                    rule_id_pattern.is_match(rule_id),
                    "Rule ID '{}' from validator '{}' does not match expected pattern",
                    rule_id,
                    meta.name,
                );
            }
        }
    }
}

#[test]
fn test_no_duplicate_rule_ids_across_validators() {
    use std::collections::HashMap;

    let registry = ValidatorRegistry::with_defaults();

    // Collect all rule_id -> validator_name mappings
    let mut rule_owners: HashMap<&str, &str> = HashMap::new();

    for file_type in ALL_VALIDATED_FILE_TYPES {
        let validators = registry.validators_for(*file_type);
        for v in validators {
            let meta = v.metadata();
            for rule_id in meta.rule_ids {
                if let Some(existing_owner) = rule_owners.get(rule_id) {
                    // Same validator registered for multiple file types is OK
                    assert_eq!(
                        *existing_owner, meta.name,
                        "Rule ID '{}' claimed by both '{}' and '{}'",
                        rule_id, existing_owner, meta.name,
                    );
                } else {
                    rule_owners.insert(rule_id, meta.name);
                }
            }
        }
    }
}

// ============================================================================
// disabled_validators config integration tests
// ============================================================================

#[test]
fn test_disabled_validators_config_filters_in_validate_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let claude_md = temp_dir.path().join("CLAUDE.md");
    // Content with an unclosed XML tag to trigger XmlValidator (XML-001)
    // Pattern: <example>text (opening tag with body, no closing tag)
    std::fs::write(&claude_md, "# Project\n\n<example>some content here\n").unwrap();

    // Without disabling, XmlValidator should fire
    let config = LintConfig::default();
    let diags = expect_success(validate_file(&claude_md, &config).unwrap());
    let xml_diags: Vec<_> = diags.iter().filter(|d| d.rule == "XML-001").collect();
    assert!(
        !xml_diags.is_empty(),
        "Expected XML-001 diagnostic without disabled_validators, got rules: {:?}",
        diags.iter().map(|d| &d.rule).collect::<Vec<_>>()
    );

    // With XmlValidator disabled, XML-001 should not appear
    let mut config_disabled = LintConfig::default();
    config_disabled.rules_mut().disabled_validators = vec!["XmlValidator".to_string()];
    let diags_disabled = expect_success(validate_file(&claude_md, &config_disabled).unwrap());
    let xml_diags_disabled: Vec<_> = diags_disabled
        .iter()
        .filter(|d| d.rule == "XML-001")
        .collect();
    assert!(
        xml_diags_disabled.is_empty(),
        "Expected no XML-001 with XmlValidator disabled, got: {:?}",
        xml_diags_disabled
    );
}

#[test]
fn test_disabled_validators_config_filters_in_validate_project() {
    let temp_dir = tempfile::tempdir().unwrap();
    let claude_md = temp_dir.path().join("CLAUDE.md");
    std::fs::write(&claude_md, "# Project\n\n<example>some content here\n").unwrap();

    // Without disabling
    let config = LintConfig::default();
    let result = validate_project(temp_dir.path(), &config).unwrap();
    let xml_diags: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XML-001")
        .collect();
    assert!(
        !xml_diags.is_empty(),
        "Expected XML-001 in project validation, got rules: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.rule)
            .collect::<Vec<_>>()
    );

    // With XmlValidator disabled
    let mut config_disabled = LintConfig::default();
    config_disabled.rules_mut().disabled_validators = vec!["XmlValidator".to_string()];
    let result_disabled = validate_project(temp_dir.path(), &config_disabled).unwrap();
    let xml_diags_disabled: Vec<_> = result_disabled
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XML-001")
        .collect();
    assert!(
        xml_diags_disabled.is_empty(),
        "Expected no XML-001 with XmlValidator disabled"
    );
}

#[test]
fn test_disabled_validators_respected_in_validate_file_with_registry() {
    let temp_dir = tempfile::tempdir().unwrap();
    let claude_md = temp_dir.path().join("CLAUDE.md");
    // Content with an unclosed XML tag to trigger XmlValidator (XML-001)
    std::fs::write(&claude_md, "# Project\n\n<example>some content here\n").unwrap();

    // Shared registry without any pre-applied disables
    let registry = ValidatorRegistry::with_defaults();

    // Without disabling, XmlValidator should fire via validate_file_with_registry
    let config = LintConfig::default();
    let diags =
        expect_success(validate_file_with_registry(&claude_md, &config, &registry).unwrap());
    let xml_diags: Vec<_> = diags.iter().filter(|d| d.rule == "XML-001").collect();
    assert!(
        !xml_diags.is_empty(),
        "Expected XML-001 diagnostic from validate_file_with_registry without disabled_validators, got rules: {:?}",
        diags.iter().map(|d| &d.rule).collect::<Vec<_>>()
    );

    // With XmlValidator disabled via config, XML-001 should be filtered at runtime
    let mut config_disabled = LintConfig::default();
    config_disabled.rules_mut().disabled_validators = vec!["XmlValidator".to_string()];
    let diags_disabled = expect_success(
        validate_file_with_registry(&claude_md, &config_disabled, &registry).unwrap(),
    );
    let xml_diags_disabled: Vec<_> = diags_disabled
        .iter()
        .filter(|d| d.rule == "XML-001")
        .collect();
    assert!(
        xml_diags_disabled.is_empty(),
        "Expected no XML-001 from validate_file_with_registry with XmlValidator disabled, got: {:?}",
        xml_diags_disabled
    );
}

#[test]
fn test_validate_file_with_registry_consistent_with_validate_content() {
    let temp_dir = tempfile::tempdir().unwrap();
    let claude_md = temp_dir.path().join("CLAUDE.md");
    // Content with an unclosed XML tag to trigger XmlValidator (XML-001)
    std::fs::write(&claude_md, "# Project\n\n<example>some content here\n").unwrap();

    // Shared registry, config with XmlValidator disabled
    let registry = ValidatorRegistry::with_defaults();
    let mut config = LintConfig::default();
    config.rules_mut().disabled_validators = vec!["XmlValidator".to_string()];

    // validate_file_with_registry path
    let file_diags =
        expect_success(validate_file_with_registry(&claude_md, &config, &registry).unwrap());
    let file_rules: std::collections::HashSet<&str> =
        file_diags.iter().map(|d| d.rule.as_str()).collect();

    // validate_content path (read file content manually)
    let content = std::fs::read_to_string(&claude_md).unwrap();
    let content_diags = validate_content(&claude_md, &content, &config, &registry);
    let content_rules: std::collections::HashSet<&str> =
        content_diags.iter().map(|d| d.rule.as_str()).collect();

    assert_eq!(
        file_rules, content_rules,
        "validate_file_with_registry and validate_content should produce the same rule set \
         when using the same registry and config. \
         file_rules={:?}, content_rules={:?}",
        file_rules, content_rules
    );

    // Verify XML-001 is absent from both (sanity check)
    assert!(
        !file_rules.contains("XML-001"),
        "XML-001 should be filtered out by disabled_validators in validate_file_with_registry"
    );
    assert!(
        !content_rules.contains("XML-001"),
        "XML-001 should be filtered out by disabled_validators in validate_content"
    );

    // Also verify both paths agree when disabled_validators is empty
    let config_empty = LintConfig::default();
    let file_diags_enabled =
        expect_success(validate_file_with_registry(&claude_md, &config_empty, &registry).unwrap());
    let content = std::fs::read_to_string(&claude_md).unwrap();
    let content_diags_enabled = validate_content(&claude_md, &content, &config_empty, &registry);
    let file_rules_enabled: std::collections::HashSet<&str> =
        file_diags_enabled.iter().map(|d| d.rule.as_str()).collect();
    let content_rules_enabled: std::collections::HashSet<&str> = content_diags_enabled
        .iter()
        .map(|d| d.rule.as_str())
        .collect();
    assert_eq!(
        file_rules_enabled, content_rules_enabled,
        "validate_file_with_registry and validate_content should agree with empty disabled list"
    );
}

#[test]
fn test_validate_project_with_registry_respects_disabled_validators() {
    let temp_dir = tempfile::tempdir().unwrap();
    let claude_md = temp_dir.path().join("CLAUDE.md");
    std::fs::write(&claude_md, "# Project\n\n<example>some content here\n").unwrap();

    // Shared registry without pre-applied disables (mirrors the LSP shared-registry pattern)
    let registry = ValidatorRegistry::with_defaults();

    // Without disabling, XmlValidator should fire
    let config = LintConfig::default();
    let result = validate_project_with_registry(temp_dir.path(), &config, &registry).unwrap();
    let xml_diags: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XML-001")
        .collect();
    assert!(
        !xml_diags.is_empty(),
        "Expected XML-001 from validate_project_with_registry without disabled_validators, got rules: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.rule)
            .collect::<Vec<_>>()
    );

    // With XmlValidator disabled via config, XML-001 should be filtered at runtime
    let mut config_disabled = LintConfig::default();
    config_disabled.rules_mut().disabled_validators = vec!["XmlValidator".to_string()];
    let result_disabled =
        validate_project_with_registry(temp_dir.path(), &config_disabled, &registry).unwrap();
    let xml_diags_disabled: Vec<_> = result_disabled
        .diagnostics
        .iter()
        .filter(|d| d.rule == "XML-001")
        .collect();
    assert!(
        xml_diags_disabled.is_empty(),
        "Expected no XML-001 from validate_project_with_registry with XmlValidator disabled, got: {:?}",
        xml_diags_disabled
    );
}

#[test]
fn test_disabled_validators_multi_validator_validate_file_with_registry() {
    let temp_dir = tempfile::tempdir().unwrap();
    let claude_md = temp_dir.path().join("CLAUDE.md");
    // Unclosed XML tag triggers XmlValidator (XML-001).
    // "Never use var" (negative-only instruction) triggers ClaudeMdValidator (CC-MEM-006).
    std::fs::write(
        &claude_md,
        "Never use var in JavaScript.\n\n<example>some content here\n",
    )
    .unwrap();

    let registry = ValidatorRegistry::with_defaults();

    // Confirm both rules fire with default config before disabling anything
    let config = LintConfig::default();
    let diags =
        expect_success(validate_file_with_registry(&claude_md, &config, &registry).unwrap());
    assert!(
        diags.iter().any(|d| d.rule == "XML-001"),
        "Expected XML-001 to fire with default config, got rules: {:?}",
        diags.iter().map(|d| &d.rule).collect::<Vec<_>>()
    );
    assert!(
        diags.iter().any(|d| d.rule == "CC-MEM-006"),
        "Expected CC-MEM-006 to fire with default config, got rules: {:?}",
        diags.iter().map(|d| &d.rule).collect::<Vec<_>>()
    );

    // Disable both validators simultaneously - both rule sets should be absent
    let mut config_multi = LintConfig::default();
    config_multi.rules_mut().disabled_validators =
        vec!["XmlValidator".to_string(), "ClaudeMdValidator".to_string()];
    let diags_multi =
        expect_success(validate_file_with_registry(&claude_md, &config_multi, &registry).unwrap());
    assert!(
        !diags_multi.iter().any(|d| d.rule == "XML-001"),
        "Expected XML-001 absent when XmlValidator is disabled, got: {:?}",
        diags_multi
            .iter()
            .filter(|d| d.rule == "XML-001")
            .collect::<Vec<_>>()
    );
    assert!(
        !diags_multi.iter().any(|d| d.rule == "CC-MEM-006"),
        "Expected CC-MEM-006 absent when ClaudeMdValidator is disabled, got: {:?}",
        diags_multi
            .iter()
            .filter(|d| d.rule == "CC-MEM-006")
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_validate_file_with_registry_no_state_leakage_between_configs() {
    // Verify a shared registry produces correct results when called with alternating
    // configs (one disabling XmlValidator, one not), i.e. no state leakage between calls.
    let temp_dir = tempfile::tempdir().unwrap();
    let claude_md = temp_dir.path().join("CLAUDE.md");
    std::fs::write(&claude_md, "# Project\n\n<example>some content here\n").unwrap();

    let registry = ValidatorRegistry::with_defaults();
    let config_enabled = LintConfig::default();
    let mut config_disabled = LintConfig::default();
    config_disabled.rules_mut().disabled_validators = vec!["XmlValidator".to_string()];

    // Call 1: enabled - XML-001 should fire
    let diags1 = expect_success(
        validate_file_with_registry(&claude_md, &config_enabled, &registry).unwrap(),
    );
    assert!(
        diags1.iter().any(|d| d.rule == "XML-001"),
        "Call 1 (enabled): expected XML-001"
    );

    // Call 2: disabled - XML-001 should be absent
    let diags2 = expect_success(
        validate_file_with_registry(&claude_md, &config_disabled, &registry).unwrap(),
    );
    assert!(
        !diags2.iter().any(|d| d.rule == "XML-001"),
        "Call 2 (disabled): expected no XML-001"
    );

    // Call 3: enabled again - XML-001 must return (no state leakage from call 2)
    let diags3 = expect_success(
        validate_file_with_registry(&claude_md, &config_enabled, &registry).unwrap(),
    );
    assert!(
        diags3.iter().any(|d| d.rule == "XML-001"),
        "Call 3 (re-enabled): expected XML-001 to return after disabled call"
    );
}

// ============================================================================
// Custom provider end-to-end test
// ============================================================================

#[test]
fn test_custom_provider_end_to_end() {
    use agnix_core::{ValidatorFactory, ValidatorProvider};

    struct NoOpProvider;
    impl ValidatorProvider for NoOpProvider {
        fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
            vec![]
        }
    }

    // Build registry with defaults + empty provider
    let registry = ValidatorRegistry::builder()
        .with_defaults()
        .with_provider(&NoOpProvider)
        .build();

    // Should have the same count as defaults (empty provider adds nothing)
    let defaults = ValidatorRegistry::with_defaults();
    assert_eq!(
        registry.total_validator_count(),
        defaults.total_validator_count()
    );
}

// ============================================================================
// ValidationOutcome integration tests
// ============================================================================

#[test]
fn test_validation_outcome_io_error_for_nonexistent_file() {
    let config = LintConfig::default();
    let temp = tempfile::TempDir::new().unwrap();
    // Create a path to a file that doesn't exist within a temp directory
    let nonexistent_file = temp.path().join("CLAUDE.md");
    let outcome = validate_file(&nonexistent_file, &config).unwrap();
    assert!(
        outcome.is_io_error(),
        "Nonexistent file with known type should return IoError, got: {:?}",
        outcome
    );
    // into_diagnostics should produce a file::read diagnostic
    let diags = outcome.into_diagnostics();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "file::read");
}

#[test]
fn test_validation_outcome_skipped_for_unknown_type() {
    let temp = tempfile::TempDir::new().unwrap();
    let rs_file = temp.path().join("main.rs");
    std::fs::write(&rs_file, "fn main() {}").unwrap();

    let config = LintConfig::default();
    let outcome = validate_file(&rs_file, &config).unwrap();
    assert!(
        outcome.is_skipped(),
        "Unknown file type should return Skipped, got: {:?}",
        outcome
    );
    assert!(outcome.diagnostics().is_empty());
}

#[test]
fn test_validation_outcome_success_for_valid_file() {
    let temp = tempfile::TempDir::new().unwrap();
    let skill_path = temp.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: code-review\ndescription: Use when reviewing code\n---\nBody",
    )
    .unwrap();

    let config = LintConfig::default();
    let outcome = validate_file(&skill_path, &config).unwrap();
    assert!(
        outcome.is_success(),
        "Valid file should return Success, got: {:?}",
        outcome
    );
}

#[test]
fn test_validation_outcome_into_diagnostics_preserves_all() {
    let temp = tempfile::TempDir::new().unwrap();
    let claude_path = temp.path().join("CLAUDE.md");
    std::fs::write(&claude_path, "<unclosed>").unwrap();

    let config = LintConfig::default();
    let outcome = validate_file(&claude_path, &config).unwrap();
    assert!(outcome.is_success());

    let diag_count = outcome.diagnostics().len();
    let into_diags = outcome.into_diagnostics();
    assert_eq!(
        into_diags.len(),
        diag_count,
        "into_diagnostics should preserve all diagnostics"
    );
}

#[cfg(unix)]
#[test]
fn test_validate_project_collects_file_read_error_as_diagnostic() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::TempDir::new().unwrap();
    let skill_path = dir.path().join("SKILL.md");
    std::fs::write(&skill_path, "# Test\n").unwrap();

    // Make file unreadable so safe_read_file returns an IoError
    let original_mode = std::fs::metadata(&skill_path).unwrap().permissions().mode();
    std::fs::set_permissions(&skill_path, std::fs::Permissions::from_mode(0o000)).unwrap();

    // Probe whether the permission change took effect. On systems where the
    // process runs as root, chmod(0o000) does not prevent reads, so we skip
    // rather than produce a false failure.
    let probe_readable = std::fs::read(&skill_path).is_ok();
    if probe_readable {
        // Running as root or on a filesystem that ignores permission bits.
        // Restore and skip.
        std::fs::set_permissions(&skill_path, std::fs::Permissions::from_mode(original_mode))
            .unwrap();
        return;
    }

    let config = LintConfig::builder().build().unwrap();
    let result = validate_project(dir.path(), &config).unwrap();

    // Restore permissions before cleanup so the tempdir can be deleted
    std::fs::set_permissions(&skill_path, std::fs::Permissions::from_mode(0o644)).unwrap();

    let has_file_read_error = result.diagnostics.iter().any(|d| d.rule == "file::read");
    assert!(
        has_file_read_error,
        "Expected file::read diagnostic for unreadable file, got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_validate_project_skipped_files_not_counted() {
    // Verify that files with unknown types (Skipped outcome) are not counted
    // in files_checked, while recognized types are.
    let temp = tempfile::TempDir::new().unwrap();

    // Create one recognized file (SKILL.md)
    std::fs::write(
        temp.path().join("SKILL.md"),
        "---\nname: test-skill\ndescription: Test skill\n---\nBody",
    )
    .unwrap();

    // Create several unrecognized files that should be skipped
    std::fs::write(temp.path().join("helper.rs"), "fn main() {}").unwrap();
    std::fs::write(temp.path().join("data.csv"), "a,b,c").unwrap();
    std::fs::write(temp.path().join("notes.txt"), "some notes").unwrap();

    let config = LintConfig::default();
    let result = validate_project(temp.path(), &config).unwrap();

    assert_eq!(
        result.files_checked, 1,
        "files_checked should count only the recognized SKILL.md, not the skipped .rs/.csv/.txt files, got {}",
        result.files_checked
    );
}

/// Regression test for #459: invalid glob patterns in [files] config should
/// produce config::glob Warning diagnostics in validate_project output, not
/// eprintln! to stderr.
#[test]
fn test_invalid_glob_in_files_config_produces_diagnostic() {
    let temp = tempfile::TempDir::new().unwrap();

    // Write a valid CLAUDE.md so the project has at least one recognized file
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\nSome instructions.\n",
    )
    .unwrap();

    // Build a config with an invalid glob pattern in include_as_memory
    let mut config = LintConfig::default();
    config
        .files_mut()
        .include_as_memory
        .push("[invalid-glob".to_string());

    let result = validate_project(temp.path(), &config).unwrap();

    let glob_diags: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "config::glob")
        .collect();

    assert_eq!(
        glob_diags.len(),
        1,
        "Expected exactly 1 config::glob diagnostic for the invalid pattern, got: {glob_diags:?}"
    );

    assert_eq!(
        glob_diags[0].level,
        DiagnosticLevel::Warning,
        "config::glob diagnostic should be Warning level"
    );

    assert!(
        glob_diags[0].message.contains("[invalid-glob"),
        "Diagnostic message should mention the invalid pattern, got: {}",
        glob_diags[0].message
    );

    assert!(
        glob_diags[0].suggestion.is_some(),
        "config::glob diagnostic should include a suggestion"
    );

    assert_eq!(
        glob_diags[0].file,
        std::fs::canonicalize(temp.path())
            .unwrap()
            .join(".agnix.toml"),
        "diagnostic file should be absolute path"
    );
}

/// Regression test: invalid patterns across all three [files] lists each produce a diagnostic.
#[test]
fn test_invalid_glob_in_all_files_config_lists_produces_diagnostics() {
    let temp = tempfile::TempDir::new().unwrap();

    // Write a valid CLAUDE.md so the project has at least one recognized file
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\nSome instructions.\n",
    )
    .unwrap();

    // Build a config with an invalid glob pattern in each of the three lists
    let mut config = LintConfig::default();
    config
        .files_mut()
        .include_as_memory
        .push("[bad-memory".to_string());
    config
        .files_mut()
        .include_as_generic
        .push("[bad-generic".to_string());
    config.files_mut().exclude.push("[bad-exclude".to_string());

    let result = validate_project(temp.path(), &config).unwrap();

    let glob_diags: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "config::glob")
        .collect();

    assert_eq!(
        glob_diags.len(),
        3,
        "Expected exactly 3 config::glob diagnostics (one per list), got: {glob_diags:?}"
    );

    for d in &glob_diags {
        assert_eq!(d.level, DiagnosticLevel::Warning);
        assert_eq!(
            d.file,
            std::fs::canonicalize(temp.path())
                .unwrap()
                .join(".agnix.toml"),
            "diagnostic file should be absolute path"
        );
        assert!(d.suggestion.is_some());
    }
}
