//! `.claude/output-styles/*.md` frontmatter validation rules
//!
//! Validates output-style files added in Claude Code v2.1.94. Spec:
//! https://code.claude.com/docs/en/output-styles (verified 2026-04-22)
//!
//! - CC-OS-001 (LOW)    : description absent or whitespace-only
//! - CC-OS-002 (HIGH)   : keep-coding-instructions present but not a YAML bool
//! - CC-OS-003 (MEDIUM) : unknown top-level frontmatter key
//! - CC-OS-004 (MEDIUM) : body after closing `---` is empty/whitespace-only
//! - CC-OS-005 (LOW)    : `name` value exceeds 64 characters
//! - CC-OS-006 (HIGH)   : invalid output-style frontmatter syntax (YAML parse error)
//!
//! All rules are non-autofix.

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
    schemas::output_style::parse_frontmatter,
};
use std::path::Path;

const RULE_IDS: &[&str] = &[
    "CC-OS-001",
    "CC-OS-002",
    "CC-OS-003",
    "CC-OS-004",
    "CC-OS-005",
    "CC-OS-006",
];

const NAME_MAX_LEN: usize = 64;

pub struct OutputStyleValidator;

/// Human-readable type name for a `serde_yaml::Value`.
fn yaml_type_name(v: &serde_yaml::Value) -> &'static str {
    match v {
        serde_yaml::Value::Null => "null",
        serde_yaml::Value::Bool(_) => "boolean",
        serde_yaml::Value::Number(_) => "number",
        serde_yaml::Value::String(_) => "string",
        serde_yaml::Value::Sequence(_) => "sequence",
        serde_yaml::Value::Mapping(_) => "mapping",
        serde_yaml::Value::Tagged(_) => "tagged",
    }
}

impl Validator for OutputStyleValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Path guard: only validate .claude/output-styles/*.md files
        let parent = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str());
        let grandparent = path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str());

        if parent != Some("output-styles") || grandparent != Some(".claude") {
            return diagnostics;
        }

        // Parse frontmatter; if absent there is nothing to validate.
        let parsed = match parse_frontmatter(content) {
            Some(p) => p,
            None => return diagnostics,
        };

        // CC-OS-006: surface YAML parse errors before per-rule checks.
        if let Some(ref parse_error) = parsed.parse_error {
            if config.is_rule_enabled("CC-OS-006") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        parsed.start_line,
                        0,
                        "CC-OS-006",
                        format!("Invalid output-style frontmatter: {}", parse_error),
                    )
                    .with_suggestion(
                        "Fix the YAML syntax (close frontmatter with a line containing only `---`, escape special characters, etc).",
                    ),
                );
            }
            return diagnostics;
        }

        let schema = match parsed.schema.as_ref() {
            Some(s) => s,
            None => return diagnostics,
        };

        // CC-OS-001: description absent or whitespace-only (LOW)
        if config.is_rule_enabled("CC-OS-001") {
            let missing = match schema.description.as_deref() {
                None => true,
                Some(s) => s.trim().is_empty(),
            };
            if missing {
                diagnostics.push(
                    Diagnostic::info(
                        path.to_path_buf(),
                        parsed.start_line,
                        0,
                        "CC-OS-001",
                        "Output style is missing a `description` field"
                            .to_string(),
                    )
                    .with_suggestion(
                        "Add `description: <one-sentence summary>` so the /config picker can label this style.",
                    ),
                );
            }
        }

        // CC-OS-002: keep-coding-instructions must be a YAML boolean (HIGH)
        if config.is_rule_enabled("CC-OS-002") {
            if let Some(v) = schema.keep_coding_instructions.as_ref() {
                if v.as_bool().is_none() {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            parsed.start_line,
                            0,
                            "CC-OS-002",
                            format!(
                                "`keep-coding-instructions` must be a boolean (true/false); got {}",
                                yaml_type_name(v)
                            ),
                        )
                        .with_suggestion(
                            "Use `keep-coding-instructions: true` or `keep-coding-instructions: false`.",
                        ),
                    );
                }
            }
        }

        // CC-OS-003: unknown top-level frontmatter key (MEDIUM)
        if config.is_rule_enabled("CC-OS-003") {
            for unknown in &parsed.unknown_keys {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        unknown.line,
                        unknown.column,
                        "CC-OS-003",
                        format!("Output style frontmatter has unknown key '{}'", unknown.key),
                    )
                    .with_suggestion("Allowed keys: name, description, keep-coding-instructions."),
                );
            }
        }

        // CC-OS-004: empty/whitespace-only body (MEDIUM)
        if config.is_rule_enabled("CC-OS-004") && parsed.body_is_empty {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    parsed.start_line,
                    0,
                    "CC-OS-004",
                    "Output style has no body content - the file is a dead config".to_string(),
                )
                .with_suggestion(
                    "Add the system-prompt instructions for Claude Code below the closing `---`.",
                ),
            );
        }

        // CC-OS-005: name exceeds 64 chars (LOW)
        if config.is_rule_enabled("CC-OS-005") {
            if let Some(name) = schema.name.as_deref() {
                if name.chars().count() > NAME_MAX_LEN {
                    diagnostics.push(
                        Diagnostic::info(
                            path.to_path_buf(),
                            parsed.start_line,
                            0,
                            "CC-OS-005",
                            format!(
                                "Output style `name` is {} characters; recommended maximum is {}",
                                name.chars().count(),
                                NAME_MAX_LEN
                            ),
                        )
                        .with_suggestion(
                            "Shorten the name so it fits in the /config picker without truncation.",
                        ),
                    );
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate(content: &str) -> Vec<Diagnostic> {
        OutputStyleValidator.validate(
            Path::new(".claude/output-styles/concise.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        OutputStyleValidator.validate(
            Path::new(".claude/output-styles/concise.md"),
            content,
            config,
        )
    }

    // ===== Path guard =====

    #[test]
    fn test_wrong_path_no_diagnostics() {
        let validator = OutputStyleValidator;
        let content = "---\nfoo: bar\n---\n";
        // Wrong subdirectory - .claude/rules/ not .claude/output-styles/
        let diagnostics = validator.validate(
            Path::new(".claude/rules/concise.md"),
            content,
            &LintConfig::default(),
        );
        assert!(diagnostics.is_empty());

        // Wrong grandparent
        let diagnostics = validator.validate(
            Path::new("some/output-styles/concise.md"),
            content,
            &LintConfig::default(),
        );
        assert!(diagnostics.is_empty());
    }

    // ===== CC-OS-001 =====

    #[test]
    fn test_cc_os_001_missing_description() {
        let content = "---\nname: Concise\n---\nBody";
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-OS-001")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].level, DiagnosticLevel::Info);
    }

    #[test]
    fn test_cc_os_001_whitespace_description() {
        let content = "---\nname: Concise\ndescription: \"   \"\n---\nBody";
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-OS-001"));
    }

    #[test]
    fn test_cc_os_001_present_description() {
        let content = "---\nname: Concise\ndescription: Short replies\n---\nBody";
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-OS-001"));
    }

    // ===== CC-OS-002 =====

    #[test]
    fn test_cc_os_002_string_value() {
        let content = "---\nname: X\ndescription: y\nkeep-coding-instructions: \"yes\"\n---\nBody";
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-OS-002")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].level, DiagnosticLevel::Error);
        assert!(hits[0].message.contains("string"));
    }

    #[test]
    fn test_cc_os_002_number_value() {
        let content = "---\nname: X\ndescription: y\nkeep-coding-instructions: 1\n---\nBody";
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-OS-002"));
    }

    #[test]
    fn test_cc_os_002_null_value() {
        let content = "---\nname: X\ndescription: y\nkeep-coding-instructions: null\n---\nBody";
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-OS-002"));
    }

    #[test]
    fn test_cc_os_002_bool_value_ok() {
        let content = "---\nname: X\ndescription: y\nkeep-coding-instructions: true\n---\nBody";
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-OS-002"));
    }

    #[test]
    fn test_cc_os_002_no_autofix() {
        let content = "---\nname: X\ndescription: y\nkeep-coding-instructions: \"yes\"\n---\nBody";
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-OS-002")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(!hits[0].has_fixes());
    }

    // ===== CC-OS-003 =====

    #[test]
    fn test_cc_os_003_unknown_key() {
        let content = "---\nname: X\ndescription: y\nfoo: bar\n---\nBody";
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-OS-003")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("foo"));
    }

    #[test]
    fn test_cc_os_003_known_keys_ok() {
        let content = "---\nname: X\ndescription: y\nkeep-coding-instructions: false\n---\nBody";
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-OS-003"));
    }

    #[test]
    fn test_cc_os_003_no_autofix() {
        let content = "---\nname: X\ndescription: y\nfoo: bar\n---\nBody";
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-OS-003")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(!hits[0].has_fixes(), "CC-OS-003 must not auto-fix");
    }

    // ===== CC-OS-004 =====

    #[test]
    fn test_cc_os_004_empty_body() {
        let content = "---\nname: X\ndescription: y\n---\n\n   \n";
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-OS-004"));
    }

    #[test]
    fn test_cc_os_004_no_body_lines_at_all() {
        let content = "---\nname: X\ndescription: y\n---\n";
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-OS-004"));
    }

    #[test]
    fn test_cc_os_004_non_empty_body_ok() {
        let content = "---\nname: X\ndescription: y\n---\nReal instructions.";
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-OS-004"));
    }

    // ===== CC-OS-005 =====

    #[test]
    fn test_cc_os_005_long_name() {
        let long = "a".repeat(65);
        let content = format!("---\nname: {}\ndescription: y\n---\nBody", long);
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-OS-005")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].level, DiagnosticLevel::Info);
    }

    #[test]
    fn test_cc_os_005_exact_length_ok() {
        let exactly = "a".repeat(64);
        let content = format!("---\nname: {}\ndescription: y\n---\nBody", exactly);
        let diagnostics = validate(&content);
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-OS-005"));
    }

    #[test]
    fn test_cc_os_005_short_name_ok() {
        let content = "---\nname: Concise\ndescription: y\n---\nBody";
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-OS-005"));
    }

    // ===== CC-OS-006 =====

    #[test]
    fn test_cc_os_006_unclosed_frontmatter() {
        let content = "---\nname: X\ndescription: y";
        let diagnostics = validate(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-OS-006")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].level, DiagnosticLevel::Error);
        assert!(hits[0].message.contains("missing closing ---"));
    }

    #[test]
    fn test_cc_os_006_invalid_yaml() {
        // Tab-indented sequence is invalid YAML
        let content = "---\nname: X\nkey:\n\t- bad\n---\nBody";
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CC-OS-006"));
    }

    #[test]
    fn test_cc_os_006_does_not_use_cc_os_002() {
        // Parse-error path must use CC-OS-006, not the overloaded CC-OS-002.
        let content = "---\nname: X";
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-OS-002"));
        assert!(diagnostics.iter().any(|d| d.rule == "CC-OS-006"));
    }

    // ===== Per-rule disable =====

    #[test]
    fn test_config_disabled_specific_rules() {
        let rules = [
            "CC-OS-001",
            "CC-OS-002",
            "CC-OS-003",
            "CC-OS-004",
            "CC-OS-005",
            "CC-OS-006",
        ];

        // Triggers CC-OS-001..005 (long name, non-bool keep, unknown key, empty body)
        let long = "a".repeat(65);
        let content_005 = format!(
            "---\nname: {}\nkeep-coding-instructions: \"yes\"\nfoo: bar\n---\n   \n",
            long
        );
        // Triggers CC-OS-006 only (unclosed frontmatter)
        let content_006 = "---\nname: x".to_string();

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];

            let content = if rule == "CC-OS-006" {
                &content_006
            } else {
                &content_005
            };
            let diagnostics = validate_with_config(content, &config);
            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled but was emitted",
                rule
            );
        }
    }

    // ===== Combined / valid =====

    #[test]
    fn test_valid_output_style_no_issues() {
        let content = "---\nname: Concise\ndescription: Short replies\nkeep-coding-instructions: true\n---\nBe brief and direct.";
        let diagnostics = validate(content);
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_no_frontmatter_no_diagnostics() {
        let content = "# Just markdown\n\nNo frontmatter here.";
        let diagnostics = validate(content);
        assert!(diagnostics.is_empty());
    }
}
