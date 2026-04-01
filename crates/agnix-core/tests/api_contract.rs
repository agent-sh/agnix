//! API contract tests for agnix-core.
//!
//! These tests catch accidental public API breakage by verifying that all
//! documented public types, functions, and trait implementations remain
//! importable and have the expected shape.
//!
//! If a test here fails after a code change, it means a public API was
//! modified. Check CONTRIBUTING.md's backward-compatibility policy before
//! proceeding.

// ============================================================================
// Public type importability
// ============================================================================

#[test]
fn public_types_are_importable() {
    // Public/Stable re-exports at crate root
    let _ = std::any::type_name::<agnix_core::LintConfig>();
    let _ = std::any::type_name::<agnix_core::Diagnostic>();
    let _ = std::any::type_name::<agnix_core::DiagnosticLevel>();
    let _ = std::any::type_name::<agnix_core::Fix>();
    let _ = std::any::type_name::<agnix_core::LintError>();
    let _ = std::any::type_name::<agnix_core::ValidationResult>();
    let _ = std::any::type_name::<agnix_core::FileType>();
    let _ = std::any::type_name::<agnix_core::ValidatorRegistry>();
    let _ = std::any::type_name::<agnix_core::FixResult>();
    let _ = std::any::type_name::<agnix_core::ConfigWarning>();
    let _ = std::any::type_name::<agnix_core::FilesConfig>();
    let _ = std::any::type_name::<agnix_core::ValidationOutcome>();

    // Error types: CoreError is the concrete enum; LintError is its public alias.
    // Both are re-exported. CoreResult was removed in #477.
    let _ = std::any::type_name::<agnix_core::CoreError>();
    // LintResult type alias - the sole public Result alias.
    let _ = std::any::type_name::<agnix_core::LintResult<()>>();

    // ValidatorFactory type alias
    let _ = std::any::type_name::<agnix_core::ValidatorFactory>();

    // ValidatorMetadata struct
    let _ = std::any::type_name::<agnix_core::ValidatorMetadata>();

    // Trait objects
    fn _assert_validator_trait(_: &dyn agnix_core::Validator) {}
    fn _assert_filesystem_trait(_: &dyn agnix_core::FileSystem) {}
    fn _assert_file_type_detector_trait(_: &dyn agnix_core::FileTypeDetector) {}

    // FileSystem implementations
    let _ = std::any::type_name::<agnix_core::MockFileSystem>();
    let _ = std::any::type_name::<agnix_core::RealFileSystem>();

    // FileTypeDetectorChain (new in file_types module extraction)
    let _ = std::any::type_name::<agnix_core::FileTypeDetectorChain>();
}

// ============================================================================
// Public function signatures
// ============================================================================

#[test]
fn public_functions_compile_with_expected_signatures() {
    use std::path::Path;

    // validate_file(path, config) -> LintResult<ValidationOutcome>
    let _: fn(
        &Path,
        &agnix_core::LintConfig,
    ) -> agnix_core::LintResult<agnix_core::ValidationOutcome> = agnix_core::validate_file;

    // validate_project(path, config) -> LintResult<ValidationResult>
    let _: fn(
        &Path,
        &agnix_core::LintConfig,
    ) -> agnix_core::LintResult<agnix_core::ValidationResult> = agnix_core::validate_project;

    // validate_project_rules(root, config) -> LintResult<Vec<Diagnostic>>
    let _: fn(
        &Path,
        &agnix_core::LintConfig,
    ) -> agnix_core::LintResult<Vec<agnix_core::Diagnostic>> = agnix_core::validate_project_rules;

    // validate_project_with_registry(path, config, registry) -> LintResult<ValidationResult>
    let _: fn(
        &Path,
        &agnix_core::LintConfig,
        &agnix_core::ValidatorRegistry,
    ) -> agnix_core::LintResult<agnix_core::ValidationResult> =
        agnix_core::validate_project_with_registry;

    // validate_file_with_registry(path, config, registry) -> LintResult<ValidationOutcome>
    let _: fn(
        &Path,
        &agnix_core::LintConfig,
        &agnix_core::ValidatorRegistry,
    ) -> agnix_core::LintResult<agnix_core::ValidationOutcome> =
        agnix_core::validate_file_with_registry;

    // detect_file_type(path) -> FileType
    let _: fn(&Path) -> agnix_core::FileType = agnix_core::detect_file_type;

    // resolve_file_type(path, config) -> FileType
    let _: fn(&Path, &agnix_core::LintConfig) -> agnix_core::FileType =
        agnix_core::resolve_file_type;

    // validate_content(path, content, config, registry) -> Vec<Diagnostic>
    let _: fn(
        &Path,
        &str,
        &agnix_core::LintConfig,
        &agnix_core::ValidatorRegistry,
    ) -> Vec<agnix_core::Diagnostic> = agnix_core::validate_content;

    // apply_fixes(diagnostics, dry_run, safe_only) -> LintResult<Vec<FixResult>>
    let _: fn(
        &[agnix_core::Diagnostic],
        bool,
        bool,
    ) -> agnix_core::LintResult<Vec<agnix_core::FixResult>> = agnix_core::apply_fixes;

    // generate_schema() -> schemars::Schema
    let _: fn() -> schemars::Schema = agnix_core::generate_schema;
}

// ============================================================================
// Key trait implementations
// ============================================================================

fn assert_serialize<T: serde::Serialize>() {}
fn assert_clone<T: Clone>() {}
fn assert_debug<T: std::fmt::Debug>() {}
fn assert_partial_eq<T: PartialEq>() {}
fn assert_eq_trait<T: Eq>() {}
fn assert_copy<T: Copy>() {}
fn assert_hash<T: std::hash::Hash>() {}
fn assert_default<T: Default>() {}
fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}

#[test]
fn diagnostic_implements_required_traits() {
    assert_serialize::<agnix_core::Diagnostic>();
    assert_clone::<agnix_core::Diagnostic>();
    assert_debug::<agnix_core::Diagnostic>();
}

#[test]
fn diagnostic_level_implements_required_traits() {
    assert_partial_eq::<agnix_core::DiagnosticLevel>();
    assert_eq_trait::<agnix_core::DiagnosticLevel>();
    assert_clone::<agnix_core::DiagnosticLevel>();
    assert_copy::<agnix_core::DiagnosticLevel>();
}

#[test]
fn file_type_implements_required_traits() {
    assert_partial_eq::<agnix_core::FileType>();
    assert_eq_trait::<agnix_core::FileType>();
    assert_hash::<agnix_core::FileType>();
    assert_clone::<agnix_core::FileType>();
    assert_copy::<agnix_core::FileType>();
}

fn assert_display<T: std::fmt::Display>() {}

#[test]
fn file_type_implements_display() {
    assert_display::<agnix_core::FileType>();

    // Spot-check a few variants
    assert_eq!(agnix_core::FileType::Skill.to_string(), "Skill");
    assert_eq!(agnix_core::FileType::Unknown.to_string(), "Unknown");
}

#[test]
fn file_type_is_validatable_contract() {
    // Unknown is not validatable; all others are
    assert!(!agnix_core::FileType::Unknown.is_validatable());
    assert!(agnix_core::FileType::Skill.is_validatable());
    assert!(agnix_core::FileType::GenericMarkdown.is_validatable());
}

#[test]
fn lint_config_implements_required_traits() {
    assert_default::<agnix_core::LintConfig>();
    assert_debug::<agnix_core::LintConfig>();
}

#[test]
fn validator_registry_implements_required_traits() {
    assert_default::<agnix_core::ValidatorRegistry>();
    assert_send::<agnix_core::ValidatorRegistry>();
    assert_sync::<agnix_core::ValidatorRegistry>();
}

// ============================================================================
// Struct field accessibility (construction by field)
// ============================================================================

#[test]
fn diagnostic_fields_are_accessible() {
    use std::path::PathBuf;

    let diag = agnix_core::Diagnostic {
        level: agnix_core::DiagnosticLevel::Warning,
        message: String::from("test message"),
        file: PathBuf::from("test.md"),
        line: 1,
        column: 0,
        rule: String::from("AS-001"),
        suggestion: Some(String::from("try this")),
        fixes: vec![],
        assumption: None,
        metadata: None,
    };

    // Read back all fields to verify accessibility
    let _: &agnix_core::DiagnosticLevel = &diag.level;
    let _: &String = &diag.message;
    let _: &PathBuf = &diag.file;
    let _: usize = diag.line;
    let _: usize = diag.column;
    let _: &String = &diag.rule;
    let _: &Option<String> = &diag.suggestion;
    let _: &Vec<agnix_core::Fix> = &diag.fixes;
    let _: &Option<String> = &diag.assumption;
}

#[test]
fn fix_fields_are_accessible() {
    let fix = agnix_core::Fix {
        start_byte: 0,
        end_byte: 10,
        replacement: String::from("new text"),
        description: String::from("replace old text"),
        safe: true,
        confidence: None,
        group: None,
        depends_on: None,
    };

    // Read back all fields
    let _: usize = fix.start_byte;
    let _: usize = fix.end_byte;
    let _: &String = &fix.replacement;
    let _: &String = &fix.description;
    let _: bool = fix.safe;
    let _: Option<f32> = fix.confidence;
    let _: Option<String> = fix.group;
    let _: Option<String> = fix.depends_on;
}

// ============================================================================
// FileType enum exhaustive match
// ============================================================================

#[test]
fn file_type_enum_covers_all_variants() {
    // This match must cover ALL variants. If a variant is added or removed,
    // this test will fail to compile.
    let variants = [
        agnix_core::FileType::Skill,
        agnix_core::FileType::ClaudeMd,
        agnix_core::FileType::Agent,
        agnix_core::FileType::AmpCheck,
        agnix_core::FileType::Hooks,
        agnix_core::FileType::Plugin,
        agnix_core::FileType::Mcp,
        agnix_core::FileType::Copilot,
        agnix_core::FileType::CopilotScoped,
        agnix_core::FileType::CopilotAgent,
        agnix_core::FileType::CopilotPrompt,
        agnix_core::FileType::CopilotHooks,
        agnix_core::FileType::ClaudeRule,
        agnix_core::FileType::CursorRule,
        agnix_core::FileType::CursorHooks,
        agnix_core::FileType::CursorAgent,
        agnix_core::FileType::CursorEnvironment,
        agnix_core::FileType::CursorRulesLegacy,
        agnix_core::FileType::ClineRules,
        agnix_core::FileType::ClineRulesFolder,
        agnix_core::FileType::OpenCodeConfig,
        agnix_core::FileType::GeminiMd,
        agnix_core::FileType::GeminiSettings,
        agnix_core::FileType::AmpSettings,
        agnix_core::FileType::GeminiExtension,
        agnix_core::FileType::GeminiIgnore,
        agnix_core::FileType::CodexConfig,
        agnix_core::FileType::CodexPlugin,
        agnix_core::FileType::RooRules,
        agnix_core::FileType::RooModes,
        agnix_core::FileType::RooIgnore,
        agnix_core::FileType::RooModeRules,
        agnix_core::FileType::RooMcp,
        agnix_core::FileType::WindsurfRule,
        agnix_core::FileType::WindsurfWorkflow,
        agnix_core::FileType::WindsurfRulesLegacy,
        agnix_core::FileType::KiroSteering,
        agnix_core::FileType::KiroPower,
        agnix_core::FileType::KiroAgent,
        agnix_core::FileType::KiroHook,
        agnix_core::FileType::KiroMcp,
        agnix_core::FileType::GenericMarkdown,
        agnix_core::FileType::Unknown,
    ];

    assert_eq!(
        variants.len(),
        43,
        "A new FileType variant may have been added or removed. Please update this test's variant list and the match statement below."
    );

    for variant in &variants {
        match variant {
            agnix_core::FileType::Skill => {}
            agnix_core::FileType::ClaudeMd => {}
            agnix_core::FileType::Agent => {}
            agnix_core::FileType::AmpCheck => {}
            agnix_core::FileType::Hooks => {}
            agnix_core::FileType::Plugin => {}
            agnix_core::FileType::Mcp => {}
            agnix_core::FileType::Copilot => {}
            agnix_core::FileType::CopilotScoped => {}
            agnix_core::FileType::CopilotAgent => {}
            agnix_core::FileType::CopilotPrompt => {}
            agnix_core::FileType::CopilotHooks => {}
            agnix_core::FileType::ClaudeRule => {}
            agnix_core::FileType::CursorRule => {}
            agnix_core::FileType::CursorHooks => {}
            agnix_core::FileType::CursorAgent => {}
            agnix_core::FileType::CursorEnvironment => {}
            agnix_core::FileType::CursorRulesLegacy => {}
            agnix_core::FileType::ClineRules => {}
            agnix_core::FileType::ClineRulesFolder => {}
            agnix_core::FileType::OpenCodeConfig => {}
            agnix_core::FileType::GeminiMd => {}
            agnix_core::FileType::GeminiSettings => {}
            agnix_core::FileType::AmpSettings => {}
            agnix_core::FileType::GeminiExtension => {}
            agnix_core::FileType::GeminiIgnore => {}
            agnix_core::FileType::CodexConfig => {}
            agnix_core::FileType::CodexPlugin => {}
            agnix_core::FileType::RooRules => {}
            agnix_core::FileType::RooModes => {}
            agnix_core::FileType::RooIgnore => {}
            agnix_core::FileType::RooModeRules => {}
            agnix_core::FileType::RooMcp => {}
            agnix_core::FileType::WindsurfRule => {}
            agnix_core::FileType::WindsurfWorkflow => {}
            agnix_core::FileType::WindsurfRulesLegacy => {}
            agnix_core::FileType::KiroSteering => {}
            agnix_core::FileType::KiroPower => {}
            agnix_core::FileType::KiroAgent => {}
            agnix_core::FileType::KiroHook => {}
            agnix_core::FileType::KiroMcp => {}
            agnix_core::FileType::GenericMarkdown => {}
            agnix_core::FileType::Unknown => {}
        }
    }
}

// ============================================================================
// Module accessibility
// ============================================================================

#[test]
fn public_modules_are_accessible() {
    // Public/Stable modules
    let _ = std::any::type_name::<agnix_core::config::LintConfig>();
    let _ = std::any::type_name::<agnix_core::diagnostics::Diagnostic>();
    let _ = std::any::type_name::<agnix_core::fixes::FixResult>();
    let _ = std::any::type_name::<agnix_core::fs::RealFileSystem>();

    // Public/Unstable modules -- file_types
    let _ = std::any::type_name::<agnix_core::file_types::FileType>();
    let _ = std::any::type_name::<agnix_core::file_types::BuiltinDetector>();
    let _ = std::any::type_name::<agnix_core::file_types::FileTypeDetectorChain>();

    // Public/Unstable modules -- eval
    let _ = std::any::type_name::<agnix_core::eval::EvalCase>();
    let _ = std::any::type_name::<agnix_core::eval::EvalFormat>();
    let _ = std::any::type_name::<agnix_core::eval::EvalResult>();
    let _ = std::any::type_name::<agnix_core::eval::EvalSummary>();
    let _ = std::any::type_name::<agnix_core::eval::EvalManifest>();
    let _ = std::any::type_name::<agnix_core::eval::EvalError>();
}

// ============================================================================
// Submodule types
// ============================================================================

#[test]
fn config_submodule_types_are_accessible() {
    let _ = std::any::type_name::<agnix_core::config::TargetTool>();
    let _ = std::any::type_name::<agnix_core::config::SeverityLevel>();
    let _ = std::any::type_name::<agnix_core::config::RuleConfig>();
    let _ = std::any::type_name::<agnix_core::config::ToolVersions>();
    let _ = std::any::type_name::<agnix_core::config::SpecRevisions>();
    let _ = std::any::type_name::<agnix_core::config::ConfigWarning>();
    let _ = std::any::type_name::<agnix_core::config::FilesConfig>();
}

#[test]
fn eval_submodule_types_are_accessible() {
    let _ = std::any::type_name::<agnix_core::eval::EvalFormat>();
    let _ = std::any::type_name::<agnix_core::eval::EvalCase>();
    let _ = std::any::type_name::<agnix_core::eval::EvalResult>();
    let _ = std::any::type_name::<agnix_core::eval::EvalSummary>();
    let _ = std::any::type_name::<agnix_core::eval::EvalManifest>();
    let _ = std::any::type_name::<agnix_core::eval::EvalError>();
}

// ============================================================================
// ValidationResult field accessibility
// ============================================================================

#[test]
fn validation_result_fields_are_accessible() {
    let result = agnix_core::ValidationResult::new(vec![], 0);

    let _: &Vec<agnix_core::Diagnostic> = &result.diagnostics;
    let _: usize = result.files_checked;

    // New metadata fields default to None/0
    assert!(result.validation_time_ms.is_none());
    assert_eq!(result.validator_factories_registered, 0);

    // Builder-style setters
    let result = agnix_core::ValidationResult::new(vec![], 5)
        .with_timing(42)
        .with_validator_factories_registered(10);
    assert_eq!(result.validation_time_ms, Some(42));
    assert_eq!(result.validator_factories_registered, 10);
    assert_eq!(result.files_checked, 5);
}

#[test]
fn validation_result_implements_required_traits() {
    assert_clone::<agnix_core::ValidationResult>();
    assert_debug::<agnix_core::ValidationResult>();
}

#[test]
fn validation_result_allows_struct_literal_construction() {
    // Guard: this integration test is an external crate, so struct literal construction would
    // fail to compile if #[non_exhaustive] is re-added. Adding a new field to ValidationResult
    // would also break this test.
    let result = agnix_core::ValidationResult {
        diagnostics: vec![],
        files_checked: 42,
        validation_time_ms: Some(100),
        validator_factories_registered: 5,
    };
    assert_eq!(result.files_checked, 42);
    assert_eq!(result.validation_time_ms, Some(100));
    assert_eq!(result.validator_factories_registered, 5);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn validation_result_allows_exhaustive_destructuring() {
    // Guard: exhaustive destructuring (no `..`) is forbidden for #[non_exhaustive] types
    // in external crates. This test fails to compile if #[non_exhaustive] is re-added.
    let result = agnix_core::ValidationResult::new(vec![], 3).with_timing(10);
    let agnix_core::ValidationResult {
        diagnostics,
        files_checked,
        validation_time_ms,
        validator_factories_registered,
    } = result;
    assert_eq!(files_checked, 3);
    assert_eq!(validation_time_ms, Some(10));
    let _ = (diagnostics, validator_factories_registered);
}

// ============================================================================
// FixResult field accessibility
// ============================================================================

#[test]
fn fix_result_fields_are_accessible() {
    use std::path::PathBuf;

    let result = agnix_core::FixResult {
        path: PathBuf::from("test.md"),
        original: String::from("old"),
        fixed: String::from("new"),
        applied: vec![String::from("applied a fix")],
    };

    let _: &PathBuf = &result.path;
    let _: &String = &result.original;
    let _: &String = &result.fixed;
    let _: &Vec<String> = &result.applied;
    let _: bool = result.has_changes();
}

// ============================================================================
// ConfigWarning field accessibility
// ============================================================================

#[test]
fn config_warning_fields_are_accessible() {
    let warning = agnix_core::ConfigWarning {
        field: String::from("rules.disabled_rules"),
        message: String::from("Unknown rule ID"),
        suggestion: Some(String::from("Did you mean AS-001?")),
    };

    let _: &String = &warning.field;
    let _: &String = &warning.message;
    let _: &Option<String> = &warning.suggestion;
}

// ============================================================================
// DiagnosticLevel enum exhaustive match
// ============================================================================

#[test]
fn diagnostic_level_covers_all_variants() {
    let levels = [
        agnix_core::DiagnosticLevel::Error,
        agnix_core::DiagnosticLevel::Warning,
        agnix_core::DiagnosticLevel::Info,
    ];

    for level in &levels {
        match level {
            agnix_core::DiagnosticLevel::Error => {}
            agnix_core::DiagnosticLevel::Warning => {}
            agnix_core::DiagnosticLevel::Info => {}
        }
    }
}

// ============================================================================
// TargetTool enum exhaustive match
// ============================================================================

#[test]
fn target_tool_covers_all_variants() {
    let tools = [
        agnix_core::config::TargetTool::Generic,
        agnix_core::config::TargetTool::ClaudeCode,
        agnix_core::config::TargetTool::Cursor,
        agnix_core::config::TargetTool::Codex,
        agnix_core::config::TargetTool::Kiro,
    ];

    for tool in &tools {
        match tool {
            agnix_core::config::TargetTool::Generic => {}
            agnix_core::config::TargetTool::ClaudeCode => {}
            agnix_core::config::TargetTool::Cursor => {}
            agnix_core::config::TargetTool::Codex => {}
            agnix_core::config::TargetTool::Kiro => {}
        }
    }
}

#[test]
fn target_tool_kiro_serde_roundtrip() {
    let value = serde_json::to_string(&agnix_core::config::TargetTool::Kiro).unwrap();
    assert_eq!(value, "\"Kiro\"");

    let parsed: agnix_core::config::TargetTool = serde_json::from_str(&value).unwrap();
    assert_eq!(parsed, agnix_core::config::TargetTool::Kiro);
}

// ============================================================================
// SeverityLevel enum exhaustive match
// ============================================================================

#[test]
fn severity_level_covers_all_variants() {
    let levels = [
        agnix_core::config::SeverityLevel::Error,
        agnix_core::config::SeverityLevel::Warning,
        agnix_core::config::SeverityLevel::Info,
    ];

    for level in &levels {
        match level {
            agnix_core::config::SeverityLevel::Error => {}
            agnix_core::config::SeverityLevel::Warning => {}
            agnix_core::config::SeverityLevel::Info => {}
        }
    }
}

// ============================================================================
// Fix constructor and helper methods
// ============================================================================

#[test]
fn fix_constructors_and_helpers() {
    // Fix::replace
    let replace = agnix_core::Fix::replace(0, 10, "new", "replace text", true);
    assert_eq!(replace.start_byte, 0);
    assert_eq!(replace.end_byte, 10);
    assert!(!replace.is_insertion());
    assert!(!replace.is_deletion());

    // Fix::insert (start_byte == end_byte)
    let insert = agnix_core::Fix::insert(5, "inserted", "insert text", true);
    assert_eq!(insert.start_byte, 5);
    assert_eq!(insert.end_byte, 5);
    assert!(insert.is_insertion());
    assert!(!insert.is_deletion());

    // Fix::delete (replacement is empty)
    let delete = agnix_core::Fix::delete(10, 20, "delete text", false);
    assert_eq!(delete.start_byte, 10);
    assert_eq!(delete.end_byte, 20);
    assert!(!delete.is_insertion());
    assert!(delete.is_deletion());
}

// ============================================================================
// Diagnostic builder methods
// ============================================================================

#[test]
fn diagnostic_builder_methods() {
    use std::path::PathBuf;

    // Constructor variants
    let _err = agnix_core::Diagnostic::error(PathBuf::from("a.md"), 1, 0, "R-001", "err");
    let _warn = agnix_core::Diagnostic::warning(PathBuf::from("b.md"), 2, 0, "R-002", "warn");
    let _info = agnix_core::Diagnostic::info(PathBuf::from("c.md"), 3, 0, "R-003", "info");

    // Builder chain
    let diag = agnix_core::Diagnostic::error(PathBuf::from("d.md"), 1, 0, "R-004", "test")
        .with_suggestion("try this")
        .with_assumption("assuming v1")
        .with_fix(agnix_core::Fix::replace(0, 5, "fixed", "auto", true))
        .with_fixes(vec![agnix_core::Fix::insert(
            10,
            "extra",
            "add extra",
            true,
        )]);

    assert!(diag.has_fixes());
    assert!(diag.has_safe_fixes());
    assert_eq!(diag.fixes.len(), 2);
    assert_eq!(diag.assumption, Some("assuming v1".to_string()));
    assert_eq!(diag.suggestion, Some("try this".to_string()));
}

// ============================================================================
// ValidatorRegistry method signatures
// ============================================================================

#[test]
fn validator_registry_methods() {
    // new() and with_defaults()
    let empty = agnix_core::ValidatorRegistry::new();
    let defaults = agnix_core::ValidatorRegistry::with_defaults();

    // validators_for returns &[Box<dyn Validator>] - lock in return type at call site
    let empty_validators = empty.validators_for(agnix_core::FileType::Skill);
    let _: &[Box<dyn agnix_core::Validator>] = empty_validators;
    assert!(empty_validators.is_empty());

    let default_validators = defaults.validators_for(agnix_core::FileType::Skill);
    assert!(!default_validators.is_empty());

    // register() signature check: accepts FileType + ValidatorFactory (fn pointer)
    let _: fn(
        &mut agnix_core::ValidatorRegistry,
        agnix_core::FileType,
        agnix_core::ValidatorFactory,
    ) = agnix_core::ValidatorRegistry::register;

    // total_validator_count()
    assert_eq!(empty.total_validator_count(), 0);
    assert!(defaults.total_validator_count() > 0);

    // disable_validator() and disabled_validator_count()
    let mut registry = agnix_core::ValidatorRegistry::with_defaults();
    assert_eq!(registry.disabled_validator_count(), 0);
    registry.disable_validator("XmlValidator");
    assert_eq!(registry.disabled_validator_count(), 1);

    // disable_validator_owned() with runtime string
    let mut registry2 = agnix_core::ValidatorRegistry::with_defaults();
    let name = String::from("PromptValidator");
    registry2.disable_validator_owned(&name);
    assert_eq!(registry2.disabled_validator_count(), 1);

    // builder()
    let _builder_registry = agnix_core::ValidatorRegistry::builder()
        .with_defaults()
        .build();
}

#[test]
#[allow(deprecated)]
fn total_factory_count_deprecated_alias_still_works() {
    let registry = agnix_core::ValidatorRegistry::with_defaults();
    // Deprecated alias must still compile and return the same value
    assert_eq!(
        registry.total_factory_count(),
        registry.total_validator_count(),
    );
}

// ============================================================================
// ValidatorProvider and ValidatorRegistryBuilder importability
// ============================================================================

#[test]
fn new_plugin_types_are_importable() {
    let _ = std::any::type_name::<agnix_core::ValidatorRegistryBuilder>();

    // ValidatorProvider is a trait - verify it can be used as trait bound
    fn _assert_provider_trait(_: &dyn agnix_core::ValidatorProvider) {}
}

#[test]
fn builder_method_signatures_compile() {
    // builder() -> ValidatorRegistryBuilder
    let mut builder = agnix_core::ValidatorRegistry::builder();

    // with_defaults() -> &mut Self
    let _: &mut agnix_core::ValidatorRegistryBuilder = builder.with_defaults();

    // without_validator() -> &mut Self
    let _: &mut agnix_core::ValidatorRegistryBuilder = builder.without_validator("XmlValidator");

    // without_validator_owned() with runtime string -> &mut Self
    let name = String::from("PromptValidator");
    let _: &mut agnix_core::ValidatorRegistryBuilder = builder.without_validator_owned(&name);

    // build() -> ValidatorRegistry
    let _: agnix_core::ValidatorRegistry = builder.build();
}

#[test]
fn builder_built_registry_matches_with_defaults_factory_count() {
    let via_builder = agnix_core::ValidatorRegistry::builder()
        .with_defaults()
        .build();
    let via_direct = agnix_core::ValidatorRegistry::with_defaults();

    assert_eq!(
        via_builder.total_validator_count(),
        via_direct.total_validator_count(),
    );
}

// ============================================================================
// ValidatorProvider::named_validators() contract
// ============================================================================

#[test]
fn named_validators_method_is_callable_on_trait_object() {
    // Verify that named_validators() is object-safe and callable through a
    // trait object reference, returning the expected tuple shape.
    struct DummyProvider;
    impl agnix_core::ValidatorProvider for DummyProvider {
        fn validators(&self) -> Vec<(agnix_core::FileType, agnix_core::ValidatorFactory)> {
            vec![]
        }
    }

    let provider: &dyn agnix_core::ValidatorProvider = &DummyProvider;
    let named = provider.named_validators();
    assert!(
        named.is_empty(),
        "Empty provider should return empty named_validators()"
    );
}

#[test]
fn named_validators_default_yields_none_names() {
    // A custom provider that only implements validators() should get None
    // names from the default named_validators() implementation.
    struct CustomProvider;
    impl agnix_core::ValidatorProvider for CustomProvider {
        fn validators(&self) -> Vec<(agnix_core::FileType, agnix_core::ValidatorFactory)> {
            // Return a dummy entry
            fn dummy_factory() -> Box<dyn agnix_core::Validator> {
                struct Dummy;
                impl agnix_core::Validator for Dummy {
                    fn validate(
                        &self,
                        _: &std::path::Path,
                        _: &str,
                        _: &agnix_core::LintConfig,
                    ) -> Vec<agnix_core::Diagnostic> {
                        vec![]
                    }
                }
                Box::new(Dummy)
            }
            vec![(agnix_core::FileType::Skill, dummy_factory)]
        }
    }

    let provider: &dyn agnix_core::ValidatorProvider = &CustomProvider;
    let named = provider.named_validators();
    assert_eq!(named.len(), 1);
    let (ft, name, _factory) = &named[0];
    assert_eq!(*ft, agnix_core::FileType::Skill);
    assert!(
        name.is_none(),
        "Default named_validators() must return None names"
    );
}

// ============================================================================
// Builder fast-path: named disabled validators skip factory call
// ============================================================================

#[test]
fn builder_with_named_provider_skips_factory_for_disabled_validator() {
    // Verifies the end-to-end builder path: a provider that overrides
    // named_validators() with Some(name) causes build() to call register_named(),
    // which skips the factory entirely when the name is disabled.
    use std::sync::atomic::{AtomicUsize, Ordering};

    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    fn counting_factory() -> Box<dyn agnix_core::Validator> {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        struct NoopValidator;
        impl agnix_core::Validator for NoopValidator {
            fn validate(
                &self,
                _: &std::path::Path,
                _: &str,
                _: &agnix_core::LintConfig,
            ) -> Vec<agnix_core::Diagnostic> {
                vec![]
            }
        }
        Box::new(NoopValidator)
    }

    struct NamedProvider;
    impl agnix_core::ValidatorProvider for NamedProvider {
        fn validators(&self) -> Vec<(agnix_core::FileType, agnix_core::ValidatorFactory)> {
            vec![(agnix_core::FileType::Skill, counting_factory)]
        }
        fn named_validators(
            &self,
        ) -> Vec<(
            agnix_core::FileType,
            Option<&'static str>,
            agnix_core::ValidatorFactory,
        )> {
            vec![(
                agnix_core::FileType::Skill,
                Some("NoopValidator"),
                counting_factory,
            )]
        }
    }

    CALL_COUNT.store(0, Ordering::SeqCst);

    let registry = agnix_core::ValidatorRegistry::builder()
        .with_provider(&NamedProvider)
        .without_validator("NoopValidator")
        .build();

    assert_eq!(
        CALL_COUNT.load(Ordering::SeqCst),
        0,
        "factory must not be called for a named disabled validator via the builder path"
    );
    assert!(
        registry
            .validators_for(agnix_core::FileType::Skill)
            .is_empty(),
        "disabled validator must not appear in registry"
    );
}

// ============================================================================
// ValidatorMetadata API contract
// ============================================================================

#[test]
fn validator_metadata_is_copy_and_eq() {
    let meta = agnix_core::ValidatorMetadata {
        name: "TestValidator",
        rule_ids: &["TEST-001"],
    };

    // ValidatorMetadata must derive Copy
    let copy = meta;
    assert_eq!(meta, copy);

    // ValidatorMetadata must derive Eq
    assert_eq!(meta, meta);
}

#[test]
fn validator_metadata_callable_on_dyn_validator() {
    // Ensure metadata() is object-safe and callable on trait objects
    let registry = agnix_core::ValidatorRegistry::with_defaults();
    let validators = registry.validators_for(agnix_core::FileType::Skill);
    assert!(!validators.is_empty());

    // Call metadata() through a trait object reference (&dyn Validator)
    let v: &dyn agnix_core::Validator = &*validators[0];
    let meta = v.metadata();
    assert!(!meta.name.is_empty());
    assert!(!meta.rule_ids.is_empty());
}

// ============================================================================
// FileTypeDetector trait contract
// ============================================================================

#[test]
fn builtin_detector_is_send_sync() {
    // Trait importability already tested in public_types_are_importable (line 42).
    // Here we check BuiltinDetector's Send + Sync bounds.
    assert_send::<agnix_core::file_types::BuiltinDetector>();
    assert_sync::<agnix_core::file_types::BuiltinDetector>();
}

#[test]
fn file_type_detector_chain_api() {
    use std::path::Path;

    // Constructors
    let empty = agnix_core::FileTypeDetectorChain::new();
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.detect(Path::new("anything")), None);

    // with_builtin()
    let builtin = agnix_core::FileTypeDetectorChain::with_builtin();
    assert_eq!(builtin.len(), 1);
    assert_eq!(
        builtin.detect(Path::new("SKILL.md")),
        Some(agnix_core::FileType::Skill)
    );
}

#[test]
fn file_type_detector_chain_is_send_sync() {
    assert_send::<agnix_core::FileTypeDetectorChain>();
    assert_sync::<agnix_core::FileTypeDetectorChain>();
}

#[test]
fn file_types_submodule_constants_are_accessible() {
    // Named constants exported from file_types module
    assert!(!agnix_core::file_types::DOCUMENTATION_DIRECTORIES.is_empty());
    assert!(!agnix_core::file_types::EXCLUDED_FILENAMES.is_empty());
    assert!(!agnix_core::file_types::EXCLUDED_PARENT_DIRECTORIES.is_empty());
}

// ============================================================================
// ValidationOutcome exhaustive match and trait implementations
// ============================================================================

#[test]
fn validation_outcome_is_importable() {
    let _ = std::any::type_name::<agnix_core::ValidationOutcome>();
}

#[cfg(feature = "filesystem")]
#[test]
fn validation_outcome_exhaustive_match() {
    // This match must cover ALL variants. If a variant is added,
    // this test will fail to compile.
    let outcomes = [
        agnix_core::ValidationOutcome::Success(vec![]),
        agnix_core::ValidationOutcome::IoError(agnix_core::FileError::Symlink {
            path: std::path::PathBuf::from("dummy"),
        }),
        agnix_core::ValidationOutcome::Skipped,
    ];

    for outcome in outcomes {
        match &outcome {
            agnix_core::ValidationOutcome::Success(_diags) => {}
            agnix_core::ValidationOutcome::IoError(_err) => {}
            agnix_core::ValidationOutcome::Skipped => {}
            // #[non_exhaustive] requires a wildcard arm. When a new variant is added,
            // the explicit matches above should be updated to catch it early in
            // internal tests (before the wildcard acts as a catch-all).
            _ => panic!("Unknown ValidationOutcome variant - update this test"),
        }
    }
}

#[test]
fn validation_outcome_implements_debug() {
    assert_debug::<agnix_core::ValidationOutcome>();
}

#[cfg(feature = "filesystem")]
#[test]
fn validation_outcome_convenience_methods() {
    // Success variant
    let success = agnix_core::ValidationOutcome::Success(vec![]);
    assert!(success.is_success());
    assert!(!success.is_skipped());
    assert!(!success.is_io_error());
    assert!(success.diagnostics().is_empty());
    assert!(success.io_error().is_none());

    // Skipped variant
    let skipped = agnix_core::ValidationOutcome::Skipped;
    assert!(skipped.is_skipped());
    assert!(!skipped.is_success());
    assert!(!skipped.is_io_error());
    assert!(skipped.diagnostics().is_empty());
    assert!(skipped.io_error().is_none());

    // IoError variant
    let io_err = agnix_core::ValidationOutcome::IoError(agnix_core::FileError::Symlink {
        path: std::path::PathBuf::from("test"),
    });
    assert!(io_err.is_io_error());
    assert!(!io_err.is_success());
    assert!(!io_err.is_skipped());
    assert!(io_err.diagnostics().is_empty());
    assert!(io_err.io_error().is_some());
}

#[cfg(feature = "filesystem")]
#[test]
fn validation_outcome_into_diagnostics_for_io_error() {
    let io_err = agnix_core::ValidationOutcome::IoError(agnix_core::FileError::Symlink {
        path: std::path::PathBuf::from("test.md"),
    });
    let diags = io_err.into_diagnostics();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "file::read");
}

#[test]
fn file_types_detect_file_type_accessible_via_submodule() {
    use std::path::Path;

    // detect_file_type is re-exported both at crate root and in file_types submodule
    let via_root = agnix_core::detect_file_type(Path::new("SKILL.md"));
    let via_submodule = agnix_core::file_types::detect_file_type(Path::new("SKILL.md"));
    assert_eq!(via_root, via_submodule);
}

// ============================================================================
// named_validators() name invariant - matching names register successfully
// ============================================================================

#[test]
fn named_validators_matching_name_registers_successfully() {
    // A provider that returns Some("TestValidator") where factory().name()
    // also returns "TestValidator" must register the validator without
    // triggering the debug_assert_eq! in register_named().
    struct MatchingValidator;
    impl agnix_core::Validator for MatchingValidator {
        fn validate(
            &self,
            _: &std::path::Path,
            _: &str,
            _: &agnix_core::LintConfig,
        ) -> Vec<agnix_core::Diagnostic> {
            vec![]
        }
        fn name(&self) -> &'static str {
            "TestValidator"
        }
    }

    fn test_factory() -> Box<dyn agnix_core::Validator> {
        Box::new(MatchingValidator)
    }

    struct MatchingProvider;
    impl agnix_core::ValidatorProvider for MatchingProvider {
        fn validators(&self) -> Vec<(agnix_core::FileType, agnix_core::ValidatorFactory)> {
            // Required by the trait; not exercised by this test path.
            vec![(agnix_core::FileType::Skill, test_factory)]
        }
        fn named_validators(
            &self,
        ) -> Vec<(
            agnix_core::FileType,
            Option<&'static str>,
            agnix_core::ValidatorFactory,
        )> {
            vec![(
                agnix_core::FileType::Skill,
                Some("TestValidator"),
                test_factory,
            )]
        }
    }

    let registry = agnix_core::ValidatorRegistry::builder()
        .with_provider(&MatchingProvider)
        .build();

    let validators = registry.validators_for(agnix_core::FileType::Skill);
    assert_eq!(
        validators.len(),
        1,
        "Matching named validator must be registered"
    );
    assert_eq!(
        validators[0].name(),
        "TestValidator",
        "Registered validator must have the expected name"
    );
}

// ============================================================================
// normalize_line_endings is a stable public API
// ============================================================================

#[test]
fn normalize_line_endings_crlf_is_owned() {
    use std::borrow::Cow;

    use agnix_core::normalize_line_endings;

    let crlf = normalize_line_endings("foo\r\nbar");
    assert_eq!(crlf, "foo\nbar");
    assert!(
        matches!(crlf, Cow::Owned(_)),
        "CRLF input must return Cow::Owned"
    );

    let crlf_trail = normalize_line_endings("foo\r\nbar\r\n");
    assert_eq!(crlf_trail, "foo\nbar\n");
    assert!(
        matches!(crlf_trail, Cow::Owned(_)),
        "CRLF input must return Cow::Owned"
    );
}

#[test]
fn normalize_line_endings_lone_cr_is_owned() {
    use std::borrow::Cow;

    use agnix_core::normalize_line_endings;

    let lone_cr = normalize_line_endings("foo\rbar");
    assert_eq!(lone_cr, "foo\nbar");
    assert!(
        matches!(lone_cr, Cow::Owned(_)),
        "Lone CR input must return Cow::Owned"
    );
}

#[test]
fn normalize_line_endings_mixed_is_owned() {
    use std::borrow::Cow;

    use agnix_core::normalize_line_endings;

    let mixed = normalize_line_endings("a\r\nb\rc\n");
    assert_eq!(mixed, "a\nb\nc\n");
    assert!(
        matches!(mixed, Cow::Owned(_)),
        "Mixed line endings must return Cow::Owned"
    );
}

#[test]
fn normalize_line_endings_empty_is_borrowed() {
    use std::borrow::Cow;

    use agnix_core::normalize_line_endings;

    let empty = normalize_line_endings("");
    assert_eq!(empty, "");
    assert!(
        matches!(empty, Cow::Borrowed(_)),
        "Empty string must return Cow::Borrowed"
    );
}

#[test]
fn normalize_line_endings_lf_only_is_borrowed_and_zero_copy() {
    use std::borrow::Cow;

    use agnix_core::normalize_line_endings;

    let lf_only = "foo\nbar";
    let result = normalize_line_endings(lf_only);
    assert_eq!(result, "foo\nbar");
    assert!(
        matches!(result, Cow::Borrowed(_)),
        "LF-only input must return Cow::Borrowed (zero allocation)"
    );
    // Verify the borrow points to the exact same memory (truly zero-copy)
    assert!(
        std::ptr::eq(result.as_ptr(), lf_only.as_ptr()),
        "Cow::Borrowed must point to the original allocation"
    );
}

// ============================================================================
// LintResult is the sole public Result alias (#477)
// ============================================================================

/// Verify that `LintResult<T>` is the sole public Result alias in agnix-core.
///
/// `CoreResult<T>` was removed in #477 because it was dead code - defined and
/// re-exported but never used anywhere in the codebase. `LintResult<T>` is the
/// established convention used across 40+ call sites.
#[test]
fn lint_result_is_sole_result_alias() {
    use agnix_core::{CoreError, LintError, LintResult, ValidationError};

    // LintResult<T> must accept Ok values.
    let ok: LintResult<u32> = Ok(42);
    assert_eq!(ok.unwrap(), 42);

    // LintResult<T> must accept Err values constructed from a concrete CoreError variant.
    // This verifies LintResult<T> = Result<T, LintError> = Result<T, CoreError> end-to-end.
    let err: LintResult<u32> = Err(CoreError::Validation(ValidationError::TooManyFiles {
        count: 9999,
        limit: 1000,
    }));
    assert!(err.is_err());

    // LintError and CoreError are type aliases for the same enum - constructing
    // one variant via CoreError and matching through LintError must work.
    let lint_err: LintError =
        CoreError::Validation(ValidationError::TooManyFiles { count: 1, limit: 0 });
    // Exhaustively match all three CoreError variants through the LintError alias.
    // If a new CoreError variant is added, this match will fail to compile.
    match lint_err {
        LintError::File(_) => {}
        LintError::Validation(_) => {}
        LintError::Config(_) => {}
    }
}
