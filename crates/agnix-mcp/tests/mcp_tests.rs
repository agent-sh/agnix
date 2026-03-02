//! Integration tests for agnix MCP server
//!
//! Tests the MCP server tools against real fixtures.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Get the path to the fixtures directory
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
}

/// Create a temp directory with test files
fn create_temp_project() -> TempDir {
    let temp = TempDir::new().unwrap();

    // Create a valid SKILL.md in a matching directory for AS-017
    let skill_content = r#"---
name: test-skill
description: A test skill for MCP testing
tools:
  - Read
  - Grep
---

# Test Skill

This is a test skill.
"#;
    fs::create_dir_all(temp.path().join("test-skill")).unwrap();
    fs::write(
        temp.path().join("test-skill").join("SKILL.md"),
        skill_content,
    )
    .unwrap();

    // Create an invalid SKILL.md in a subdirectory
    fs::create_dir_all(temp.path().join("subdir")).unwrap();
    let invalid_skill = r#"---
name: Invalid-Name
description: Invalid skill name
---

# Invalid Skill
"#;
    fs::write(temp.path().join("subdir").join("SKILL.md"), invalid_skill).unwrap();

    temp
}

mod parse_target_tests {
    use agnix_core::config::TargetTool;

    fn parse_target(target: Option<String>) -> TargetTool {
        match target.as_deref() {
            Some("claude-code") | Some("claudecode") => TargetTool::ClaudeCode,
            Some("cursor") => TargetTool::Cursor,
            Some("codex") => TargetTool::Codex,
            Some("kiro") => TargetTool::Kiro,
            _ => TargetTool::Generic,
        }
    }

    #[test]
    fn test_parse_target_none() {
        assert_eq!(parse_target(None), TargetTool::Generic);
    }

    #[test]
    fn test_parse_target_generic() {
        assert_eq!(
            parse_target(Some("generic".to_string())),
            TargetTool::Generic
        );
    }

    #[test]
    fn test_parse_target_claude_code() {
        assert_eq!(
            parse_target(Some("claude-code".to_string())),
            TargetTool::ClaudeCode
        );
    }

    #[test]
    fn test_parse_target_claudecode() {
        assert_eq!(
            parse_target(Some("claudecode".to_string())),
            TargetTool::ClaudeCode
        );
    }

    #[test]
    fn test_parse_target_cursor() {
        assert_eq!(parse_target(Some("cursor".to_string())), TargetTool::Cursor);
    }

    #[test]
    fn test_parse_target_codex() {
        assert_eq!(parse_target(Some("codex".to_string())), TargetTool::Codex);
    }

    #[test]
    fn test_parse_target_kiro() {
        assert_eq!(parse_target(Some("kiro".to_string())), TargetTool::Kiro);
    }

    #[test]
    fn test_parse_target_unknown() {
        assert_eq!(
            parse_target(Some("unknown".to_string())),
            TargetTool::Generic
        );
    }
}

mod validation_tests {
    use super::*;
    use agnix_core::{config::LintConfig, validate_file, validate_project};

    #[test]
    fn test_validate_valid_skill_file() {
        let temp = create_temp_project();
        let skill_path = temp.path().join("test-skill").join("SKILL.md");

        let config = LintConfig::default();
        let result = validate_file(&skill_path, &config);

        assert!(result.is_ok());
        let diagnostics = result.unwrap().into_diagnostics();
        // Valid skill should have no errors (may have warnings)
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| matches!(d.level, agnix_core::diagnostics::DiagnosticLevel::Error))
            .collect();
        assert!(errors.is_empty(), "Valid SKILL.md should have no errors");
    }

    #[test]
    fn test_validate_invalid_skill_file() {
        let temp = create_temp_project();
        let skill_path = temp.path().join("subdir").join("SKILL.md");

        let config = LintConfig::default();
        let result = validate_file(&skill_path, &config);

        assert!(result.is_ok());
        let diagnostics = result.unwrap().into_diagnostics();
        // Invalid skill name should produce error
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| matches!(d.level, agnix_core::diagnostics::DiagnosticLevel::Error))
            .collect();
        assert!(
            !errors.is_empty(),
            "Invalid skill name should produce error"
        );
    }

    #[test]
    fn test_validate_project() {
        let temp = create_temp_project();

        let config = LintConfig::default();
        let result = validate_project(temp.path(), &config);

        assert!(result.is_ok());
        let validation_result = result.unwrap();
        // Should find at least the SKILL.md files
        assert!(
            validation_result.files_checked >= 2,
            "Should check at least 2 files"
        );
    }

    #[test]
    fn test_validate_file_nonexistent_path_returns_io_error() {
        let config = LintConfig::default();
        let result = validate_file(std::path::Path::new("/nonexistent/path/file.md"), &config);
        let outcome = result.unwrap();
        assert!(
            outcome.is_io_error(),
            "nonexistent path should produce ValidationOutcome::IoError, got: {:?}",
            outcome
        );
    }

    /// Verify the `into_diagnostics()` contract for `ValidationOutcome::IoError`
    /// when `validate_file` is called on a nonexistent path with a known file
    /// type extension (SKILL.md).
    ///
    /// This pins the diagnostic fields that the MCP handler serialises to JSON:
    /// - `rule` must be `"file::read"`
    /// - `level` must be `Error`
    /// - `file` must match the input path
    ///
    /// MCP layer JSON serialisation of these fields is covered by the
    /// `test_diagnostic_json_serialization` test in `output_format_tests`.
    #[test]
    fn test_validate_file_io_error_into_diagnostics_fields() {
        use agnix_core::diagnostics::DiagnosticLevel;

        let temp = tempfile::TempDir::new().unwrap();
        let input_path = temp.path().join("SKILL.md");
        let config = LintConfig::default();
        let outcome = validate_file(&input_path, &config).unwrap();

        assert!(
            outcome.is_io_error(),
            "Expected IoError for nonexistent SKILL.md path, got: {:?}",
            outcome
        );

        let diags = outcome.into_diagnostics();
        assert_eq!(
            diags.len(),
            1,
            "IoError should produce exactly one diagnostic via into_diagnostics()"
        );

        let diag = &diags[0];
        assert_eq!(
            diag.rule, "file::read",
            "IoError diagnostic rule must be 'file::read', got: {}",
            diag.rule
        );
        assert_eq!(
            diag.level,
            DiagnosticLevel::Error,
            "IoError diagnostic must have Error level"
        );
        assert_eq!(
            diag.file, input_path,
            "IoError diagnostic file path must match the input path"
        );
    }

    #[test]
    fn test_validate_nonexistent_project_path() {
        let config = LintConfig::default();
        let temp = tempfile::TempDir::new().unwrap();
        let missing = temp.path().join("nonexistent_subdir");
        let result = validate_project(&missing, &config);

        assert!(
            result.is_err(),
            "Non-existent project path should return Err, got: {:?}",
            result
        );
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("Validation root not found"),
            "Error message should contain 'Validation root not found': {err_msg}"
        );
        assert!(
            err_msg.contains(missing.to_str().unwrap()),
            "Error message should contain the path: {err_msg}"
        );
    }

    #[test]
    fn test_validate_empty_path_string() {
        let config = LintConfig::default();
        let result = validate_file(std::path::Path::new(""), &config);

        // Empty path resolves to FileType::Unknown, and validate_file returns
        // Ok(Skipped) for unknown file types without reading the file.
        assert!(result.is_ok(), "Empty path should not panic: {:?}", result);
        assert!(result.unwrap().is_skipped());
    }

    #[test]
    fn test_validate_with_target_claude_code() {
        let temp = create_temp_project();
        let skill_path = temp.path().join("test-skill").join("SKILL.md");

        let mut config = LintConfig::default();
        config.set_target(agnix_core::config::TargetTool::ClaudeCode);
        let result = validate_file(&skill_path, &config);

        assert!(result.is_ok());
    }

    #[test]
    fn test_tools_array_overrides_target_semantics_in_core_filtering() {
        let mut config = LintConfig::default();
        config.set_target(agnix_core::config::TargetTool::Codex);
        config.set_tools(vec!["claude-code".to_string(), "cursor".to_string()]);

        // CC-* rules do not apply to codex target alone, but they do apply when tools include claude-code.
        assert!(config.is_rule_enabled("CC-AG-001"));
    }
}

mod rules_tests {
    use std::path::PathBuf;

    fn expected_rule_count_from_source() -> usize {
        let rules_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("knowledge-base")
            .join("rules.json");
        let content = std::fs::read_to_string(&rules_path).unwrap_or_else(|e| {
            panic!("Failed to read {}: {}", rules_path.display(), e);
        });
        let json: serde_json::Value = serde_json::from_str(&content).unwrap_or_else(|e| {
            panic!("Failed to parse {}: {}", rules_path.display(), e);
        });
        json["total_rules"]
            .as_u64()
            .unwrap_or_else(|| panic!("Missing numeric total_rules in {}", rules_path.display()))
            as usize
    }

    #[test]
    fn test_rules_data_not_empty() {
        assert!(!agnix_rules::RULES_DATA.is_empty());
    }

    #[test]
    fn test_rules_count() {
        // Should match the current source-of-truth total in knowledge-base/rules.json.
        assert_eq!(agnix_rules::rule_count(), expected_rule_count_from_source());
    }

    #[test]
    fn test_get_rule_name_exists() {
        let name = agnix_rules::get_rule_name("AS-001");
        assert!(name.is_some());
    }

    #[test]
    fn test_get_rule_name_not_exists() {
        let name = agnix_rules::get_rule_name("NONEXISTENT-999");
        assert!(name.is_none());
    }

    #[test]
    fn test_common_rule_ids() {
        // Test that common rule IDs exist
        assert!(agnix_rules::get_rule_name("AS-001").is_some());
        assert!(agnix_rules::get_rule_name("AS-004").is_some());
        assert!(agnix_rules::get_rule_name("CC-SK-001").is_some());
        assert!(agnix_rules::get_rule_name("PE-001").is_some());
        assert!(agnix_rules::get_rule_name("MCP-001").is_some());
    }
}

mod output_format_tests {
    use agnix_core::diagnostics::{Diagnostic, DiagnosticLevel, Fix};
    use serde_json::Value;
    use std::path::PathBuf;

    #[test]
    fn test_diagnostic_json_serialization() {
        let diagnostic = Diagnostic {
            level: DiagnosticLevel::Error,
            message: "Test error".to_string(),
            file: PathBuf::from("test.md"),
            line: 1,
            column: 1,
            rule: "AS-001".to_string(),
            suggestion: Some("Fix this".to_string()),
            fixes: vec![],
            assumption: None,
            metadata: None,
        };

        let json = serde_json::to_string(&diagnostic);
        assert!(json.is_ok());

        let parsed: Value = serde_json::from_str(&json.unwrap()).unwrap();
        assert_eq!(parsed["rule"], "AS-001");
        // Level is serialized as "Error" (enum variant name)
        assert!(
            parsed["level"] == "error" || parsed["level"] == "Error",
            "Level should be error/Error, got: {}",
            parsed["level"]
        );
    }

    #[test]
    fn test_diagnostic_with_fix() {
        let diagnostic = Diagnostic {
            level: DiagnosticLevel::Warning,
            message: "Test warning".to_string(),
            file: PathBuf::from("test.md"),
            line: 5,
            column: 10,
            rule: "PE-003".to_string(),
            suggestion: Some("Remove this".to_string()),
            fixes: vec![Fix {
                start_byte: 0,
                end_byte: 10,
                replacement: "fixed".to_string(),
                description: "Fix the issue".to_string(),
                safe: true,
                confidence: None,
                group: None,
                depends_on: None,
            }],
            assumption: None,
            metadata: None,
        };

        // Diagnostic should be fixable
        assert!(!diagnostic.fixes.is_empty());
    }
}

mod server_info_tests {
    // Note: We can't directly instantiate AgnixServer from here due to privacy,
    // but we can test the expected behavior through the binary

    #[test]
    fn test_server_version_matches_cargo() {
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
        // Version should be semver format
        assert!(version.contains('.'));
    }

    #[test]
    fn test_protocol_version() {
        // MCP protocol version should be defined
        use rmcp::model::ProtocolVersion;
        let _ = ProtocolVersion::V_2024_11_05;
    }
}

mod integration_tests {
    use super::*;

    #[test]
    fn test_binary_exists() {
        // After cargo build, binary should exist
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let target_dir = manifest_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("target")
            .join("debug");

        // Binary name varies by platform
        #[cfg(windows)]
        let binary_name = "agnix-mcp.exe";
        #[cfg(not(windows))]
        let binary_name = "agnix-mcp";

        let binary_path = target_dir.join(binary_name);

        // Note: This test may fail if build hasn't been run
        // It's informational - the binary should be built by CI
        if binary_path.exists() {
            assert!(binary_path.is_file());
        }
    }

    #[test]
    fn test_validate_fixtures_skill_valid() {
        let fixtures = fixtures_dir();
        let skill_valid = fixtures.join("skill").join("valid");

        if skill_valid.exists() {
            let config = agnix_core::config::LintConfig::default();
            let result = agnix_core::validate_project(&skill_valid, &config);

            assert!(result.is_ok());
            let validation = result.unwrap();
            // Valid fixtures should have no errors
            let errors: Vec<_> = validation
                .diagnostics
                .iter()
                .filter(|d| matches!(d.level, agnix_core::diagnostics::DiagnosticLevel::Error))
                .collect();
            assert!(
                errors.is_empty(),
                "Valid skill fixtures should have no errors"
            );
        }
    }

    #[test]
    fn test_validate_fixtures_skill_invalid() {
        let fixtures = fixtures_dir();
        let skill_invalid = fixtures.join("skill").join("invalid");

        if skill_invalid.exists() {
            let config = agnix_core::config::LintConfig::default();
            let result = agnix_core::validate_project(&skill_invalid, &config);

            assert!(result.is_ok());
            let validation = result.unwrap();
            // Invalid fixtures should have errors or warnings
            assert!(
                !validation.diagnostics.is_empty(),
                "Invalid skill fixtures should have diagnostics"
            );
        }
    }
}

mod json_schema_tests {
    use rmcp::schemars::{self, JsonSchema, SchemaGenerator};
    use serde::Deserialize;

    /// Mirror of the real `ToolsInput` enum with the same manual `JsonSchema`
    /// impl. Binary-only crates cannot export types to integration tests, so
    /// we replicate the schema here and assert its shape.
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    #[allow(dead_code)]
    enum ToolsInput {
        List(Vec<String>),
        Csv(String),
    }

    impl JsonSchema for ToolsInput {
        fn schema_name() -> std::borrow::Cow<'static, str> {
            "ToolsInput".into()
        }

        fn schema_id() -> std::borrow::Cow<'static, str> {
            "test::ToolsInput".into()
        }

        fn json_schema(_gen: &mut SchemaGenerator) -> schemars::Schema {
            schemars::json_schema!({
                "anyOf": [
                    {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Preferred: array of tool names, e.g. [\"claude-code\", \"cursor\"]"
                    },
                    {
                        "type": "string",
                        "description": "Fallback: comma-separated tool names, e.g. \"claude-code,cursor\""
                    }
                ]
            })
        }

        /// Must mirror the production impl: inline so the anyOf appears at the
        /// property site rather than behind a $ref.
        fn inline_schema() -> bool {
            true
        }
    }

    /// Mirror of `ValidateFileInput` with the updated tools field description.
    #[allow(dead_code)]
    #[derive(Debug, Deserialize, JsonSchema)]
    struct ValidateFileInput {
        path: String,
        #[schemars(
            description = "Tools to validate for. Preferred: JSON array of tool names (e.g. [\"claude-code\", \"cursor\"]). Also accepts comma-separated string (e.g. \"claude-code,cursor\") as a fallback."
        )]
        tools: Option<ToolsInput>,
        target: Option<String>,
    }

    /// Mirror of `ValidateProjectInput` with the updated tools field description.
    #[allow(dead_code)]
    #[derive(Debug, Deserialize, JsonSchema)]
    struct ValidateProjectInput {
        path: String,
        #[schemars(
            description = "Tools to validate for. Preferred: JSON array of tool names (e.g. [\"claude-code\", \"cursor\"]). Also accepts comma-separated string (e.g. \"claude-code,cursor\") as a fallback."
        )]
        tools: Option<ToolsInput>,
        target: Option<String>,
    }

    #[test]
    fn test_tools_input_schema_anyof_array_first() {
        let schema = SchemaGenerator::default().into_root_schema_for::<ToolsInput>();
        let json = serde_json::to_value(&schema).expect("schema should serialize");
        let any_of = json
            .get("anyOf")
            .and_then(|v| v.as_array())
            .expect("ToolsInput schema must have anyOf");

        assert_eq!(any_of.len(), 2, "anyOf must have exactly two entries");

        assert_eq!(
            any_of[0].get("type").and_then(|v| v.as_str()),
            Some("array"),
            "first anyOf entry must be the array variant"
        );
        assert_eq!(
            any_of[1].get("type").and_then(|v| v.as_str()),
            Some("string"),
            "second anyOf entry must be the string variant"
        );

        // Verify items constraint so MCP clients know array elements are strings
        assert_eq!(
            any_of[0]
                .get("items")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str()),
            Some("string"),
            "array variant must have items.type == 'string'"
        );
    }

    #[test]
    fn test_tools_input_schema_variant_descriptions() {
        let schema = SchemaGenerator::default().into_root_schema_for::<ToolsInput>();
        let json = serde_json::to_value(&schema).expect("schema should serialize");
        let any_of = json
            .get("anyOf")
            .and_then(|v| v.as_array())
            .expect("ToolsInput schema must have anyOf");

        let array_desc = any_of[0]
            .get("description")
            .and_then(|v| v.as_str())
            .expect("array variant must have a description");
        assert!(
            array_desc.contains("Preferred"),
            "array variant description must contain 'Preferred', got: {}",
            array_desc
        );

        let string_desc = any_of[1]
            .get("description")
            .and_then(|v| v.as_str())
            .expect("string variant must have a description");
        assert!(
            string_desc.contains("Fallback"),
            "string variant description must contain 'Fallback', got: {}",
            string_desc
        );
    }

    /// Helper: given a schema JSON object for a struct, navigate to the nested
    /// anyOf array for the inlined ToolsInput inside the `tools` property.
    ///
    /// With `inline_schema = true` on `ToolsInput`, the `tools` property schema
    /// is an `Option` wrapper (`anyOf: [{anyOf:[array,string]}, {type:null}]`).
    /// This helper finds the inner `anyOf` that belongs to `ToolsInput`.
    fn get_tools_anyof(schema_json: &serde_json::Value) -> &[serde_json::Value] {
        let props = schema_json
            .get("properties")
            .expect("schema must have properties");
        let tools_prop = props.get("tools").expect("schema must have tools property");

        // With inline_schema = true on ToolsInput, the anyOf appears directly
        // inside the Option wrapper's anyOf entries. Navigate accordingly.
        let any_of = tools_prop
            .get("anyOf")
            .and_then(|v| v.as_array())
            .expect("tools property must have anyOf");

        // The first non-null entry is the inlined ToolsInput anyOf.
        // If this expect fires, inline_schema() = true is not in effect -
        // check the ToolsInput JsonSchema impl in both main.rs and this file.
        any_of
            .iter()
            .find(|e| e.get("anyOf").is_some())
            .and_then(|e| e.get("anyOf"))
            .and_then(|v| v.as_array())
            .map(|a| a.as_slice())
            .expect("ToolsInput must be inlined as a nested anyOf within the Option wrapper anyOf")
    }

    #[test]
    fn test_validate_file_input_schema_has_tools_anyof() {
        let schema = SchemaGenerator::default().into_root_schema_for::<ValidateFileInput>();
        let json = serde_json::to_value(&schema).expect("schema should serialize");

        let props = json.get("properties").expect("schema must have properties");
        assert!(
            props.get("path").is_some(),
            "schema must include 'path' field"
        );
        assert!(
            props.get("tools").is_some(),
            "schema must include 'tools' field"
        );
        assert!(
            props.get("target").is_some(),
            "schema must include 'target' field"
        );

        // Verify the property-level description contains the Preferred/Fallback guidance
        let tools_field_desc = props
            .get("tools")
            .and_then(|v| v.get("description"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        // The field description may be on the property directly or hoisted from the Option wrapper.
        // Check the serialised JSON in case schemars places the description at a different level.
        let json_str = serde_json::to_string(&json).unwrap();
        assert!(
            tools_field_desc.contains("Preferred") || json_str.contains("Preferred"),
            "tools field description must mention 'Preferred' in ValidateFileInput schema"
        );

        // Verify the tools anyOf has array-first ordering with Preferred description
        let any_of = get_tools_anyof(&json);
        assert_eq!(
            any_of.len(),
            2,
            "tools anyOf must have exactly two entries in ValidateFileInput"
        );
        assert_eq!(
            any_of[0].get("type").and_then(|v| v.as_str()),
            Some("array"),
            "tools anyOf[0] must be array type in ValidateFileInput"
        );
        assert_eq!(
            any_of[1].get("type").and_then(|v| v.as_str()),
            Some("string"),
            "tools anyOf[1] must be string type in ValidateFileInput"
        );
        assert_eq!(
            any_of[0]
                .get("items")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str()),
            Some("string"),
            "tools array variant must have items.type == 'string' in ValidateFileInput"
        );
        let desc = any_of[0]
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            desc.contains("Preferred"),
            "tools array variant description must contain 'Preferred', got: {}",
            desc
        );
        // Verify inline_schema=true is in effect: ToolsInput must not appear in $defs
        assert!(
            json.get("$defs")
                .and_then(|d| d.get("ToolsInput"))
                .is_none(),
            "ToolsInput must be inlined (not in $defs) - check inline_schema() impl"
        );
    }

    #[test]
    fn test_validate_project_input_schema_has_tools_anyof() {
        let schema = SchemaGenerator::default().into_root_schema_for::<ValidateProjectInput>();
        let json = serde_json::to_value(&schema).expect("schema should serialize");

        let props = json.get("properties").expect("schema must have properties");
        assert!(
            props.get("path").is_some(),
            "schema must include 'path' field"
        );
        assert!(
            props.get("tools").is_some(),
            "schema must include 'tools' field"
        );
        assert!(
            props.get("target").is_some(),
            "schema must include 'target' field"
        );

        // Verify the property-level description contains the Preferred/Fallback guidance
        let tools_field_desc = props
            .get("tools")
            .and_then(|v| v.get("description"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let json_str = serde_json::to_string(&json).unwrap();
        assert!(
            tools_field_desc.contains("Preferred") || json_str.contains("Preferred"),
            "tools field description must mention 'Preferred' in ValidateProjectInput schema"
        );

        // Verify the tools anyOf has array-first ordering with Preferred description
        let any_of = get_tools_anyof(&json);
        assert_eq!(
            any_of.len(),
            2,
            "tools anyOf must have exactly two entries in ValidateProjectInput"
        );
        assert_eq!(
            any_of[0].get("type").and_then(|v| v.as_str()),
            Some("array"),
            "tools anyOf[0] must be array type in ValidateProjectInput"
        );
        assert_eq!(
            any_of[1].get("type").and_then(|v| v.as_str()),
            Some("string"),
            "tools anyOf[1] must be string type in ValidateProjectInput"
        );
        assert_eq!(
            any_of[0]
                .get("items")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str()),
            Some("string"),
            "tools array variant must have items.type == 'string' in ValidateProjectInput"
        );
        let desc = any_of[0]
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            desc.contains("Preferred"),
            "tools array variant description must contain 'Preferred', got: {}",
            desc
        );
        // Verify inline_schema=true is in effect: ToolsInput must not appear in $defs
        assert!(
            json.get("$defs")
                .and_then(|d| d.get("ToolsInput"))
                .is_none(),
            "ToolsInput must be inlined (not in $defs) - check inline_schema() impl"
        );
    }

    /// Mirror of `GetRuleDocsInput` for schema regression testing.
    #[allow(dead_code)]
    #[derive(Debug, Deserialize, JsonSchema)]
    struct GetRuleDocsInput {
        rule_id: String,
    }

    #[test]
    fn test_get_rule_docs_input_schema() {
        let schema = SchemaGenerator::default().into_root_schema_for::<GetRuleDocsInput>();
        let json = serde_json::to_value(&schema).expect("schema should serialize");
        let props = json
            .get("properties")
            .expect("GetRuleDocsInput schema must have properties");
        assert!(
            props.get("rule_id").is_some(),
            "GetRuleDocsInput schema must include 'rule_id' field"
        );
    }
}
