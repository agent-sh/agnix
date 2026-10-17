//! `.claude/rules/**/*.md` validation rules
//!
//! Validates:
//! - CC-MEM-011: Invalid paths glob in rules (HIGH) - glob patterns must be valid
//! - CC-MEM-012: Rules file unknown frontmatter key (MEDIUM) - only `paths` is known
//!
//! Rule-file length (CC-MEM-014) and emphasis (CC-MEM-017) are checked by
//! `ClaudeMdValidator`, which also runs on `FileType::ClaudeRule`.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::{Validator, ValidatorMetadata},
    schemas::claude_rules::{parse_frontmatter, validate_glob_pattern},
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["CC-MEM-011", "CC-MEM-012"];

pub struct ClaudeRulesValidator;

fn line_byte_range(content: &str, line_number: usize) -> Option<(usize, usize)> {
    if line_number == 0 {
        return None;
    }

    let mut current_line = 1usize;
    let mut line_start = 0usize;

    for (idx, ch) in content.char_indices() {
        if current_line == line_number && ch == '\n' {
            return Some((line_start, idx + 1));
        }
        if ch == '\n' {
            current_line += 1;
            line_start = idx + 1;
        }
    }

    if current_line == line_number {
        Some((line_start, content.len()))
    } else {
        None
    }
}

impl Validator for ClaudeRulesValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Only validate files under `.claude/rules/`, at any depth: "All `.md`
        // files are discovered recursively, so you can organize rules into
        // subdirectories like `frontend/` or `backend/`". This previously
        // required `rules/` to be the immediate parent, so nested rule files
        // were detected as ClaudeRule but then skipped here without a
        // diagnostic - the check has to match the recursive detection.
        if !crate::file_types::path_contains_consecutive_components(path, ".claude", "rules") {
            return diagnostics;
        }

        let parsed = parse_frontmatter(content);

        // Parse frontmatter - if no frontmatter, nothing more to validate
        let Some(parsed) = parsed else {
            return diagnostics;
        };

        // "If the YAML between the markers doesn't parse, Claude Code ignores the
        // frontmatter and loads the rule as if it had no `paths`."
        if let Some(ref parse_error) = parsed.parse_error {
            if config.is_rule_enabled("CC-MEM-011") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        parsed.start_line,
                        0,
                        "CC-MEM-011",
                        t!("rules.cc_mem_011.parse_error", error = parse_error.as_str()),
                    )
                    .with_suggestion(t!("rules.cc_mem_011.parse_error_suggestion")),
                );
            }
            return diagnostics;
        }

        // CC-MEM-011: Invalid paths glob in rules (ERROR)
        if config.is_rule_enabled("CC-MEM-011") {
            if let Some(ref schema) = parsed.schema {
                for (i, pattern) in schema.paths.iter().enumerate() {
                    let validation = validate_glob_pattern(pattern);
                    if !validation.valid {
                        // Try to find the line number of this pattern in the frontmatter
                        let line = find_pattern_line(&parsed.raw, pattern, parsed.start_line)
                            .unwrap_or(parsed.start_line + 1);

                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                line,
                                0,
                                "CC-MEM-011",
                                t!(
                                    "rules.cc_mem_011.message",
                                    pattern = pattern.as_str(),
                                    error = validation.error.unwrap_or_default(),
                                    index = i + 1
                                ),
                            )
                            .with_suggestion(t!("rules.cc_mem_011.suggestion")),
                        );
                    }
                }
            }
        }

        // CC-MEM-012: Unknown frontmatter keys (WARNING)
        if config.is_rule_enabled("CC-MEM-012") {
            for unknown in &parsed.unknown_keys {
                let mut diagnostic = Diagnostic::warning(
                    path.to_path_buf(),
                    unknown.line,
                    unknown.column,
                    "CC-MEM-012",
                    t!("rules.cc_mem_012.message", key = unknown.key.as_str()),
                )
                .with_suggestion(t!(
                    "rules.cc_mem_012.suggestion",
                    key = unknown.key.as_str()
                ));

                // Auto-fix: remove unknown top-level frontmatter key line.
                // Marked unsafe because the key's value may span multiple lines
                // (e.g. block scalars, nested maps) and we only delete the key line.
                if let Some((start, end)) = line_byte_range(content, unknown.line) {
                    diagnostic = diagnostic.with_fix(Fix::delete(
                        start,
                        end,
                        format!("Remove unknown frontmatter key '{}'", unknown.key),
                        false,
                    ));
                }

                diagnostics.push(diagnostic);
            }
        }

        diagnostics
    }
}

/// Find the line number of a pattern in the raw frontmatter
///
/// Matches YAML list items precisely by stripping the leading `- ` prefix
/// and surrounding quotes, avoiding false matches when one pattern is a
/// substring of another.
fn find_pattern_line(raw: &str, pattern: &str, start_line: usize) -> Option<usize> {
    for (i, line) in raw.lines().enumerate() {
        if let Some(value_part) = line.trim().strip_prefix('-') {
            let value = value_part.trim();
            let value_unquoted = value.trim_matches(|c| c == '\'' || c == '"');
            if value_unquoted == pattern {
                return Some(start_line + 1 + i);
            }
        }
    }
    // A comma-separated `paths` string keeps every pattern on one line.
    raw.lines()
        .position(|line| line.contains(pattern))
        .map(|i| start_line + 1 + i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate_rule(content: &str) -> Vec<Diagnostic> {
        let validator = ClaudeRulesValidator;
        validator.validate(
            Path::new(".claude/rules/my-rule.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_rule_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = ClaudeRulesValidator;
        validator.validate(Path::new(".claude/rules/my-rule.md"), content, config)
    }

    // ===== CC-MEM-011: Invalid Paths Glob =====

    #[test]
    fn test_cc_mem_011_invalid_glob() {
        let content = r#"---
paths:
  - "[unclosed"
---
# Rule content
"#;
        let diagnostics = validate_rule(content);
        let mem_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-011")
            .collect();
        assert_eq!(mem_011.len(), 1);
        assert_eq!(mem_011[0].level, DiagnosticLevel::Error);
        assert!(mem_011[0].message.contains("[unclosed"));
    }

    #[test]
    fn test_cc_mem_011_multiple_invalid_globs() {
        let content = r#"---
paths:
  - "[bad1"
  - "**/*.ts"
  - "[bad2"
---
# Rule content
"#;
        let diagnostics = validate_rule(content);
        let mem_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-011")
            .collect();
        assert_eq!(mem_011.len(), 2);
    }

    #[test]
    fn test_cc_mem_011_valid_globs() {
        let patterns = vec![
            "**/*.ts",
            "*.rs",
            "src/**/*.js",
            "tests/**/*.test.ts",
            "{src,lib}/**/*.ts",
        ];

        for pattern in patterns {
            let content = format!(
                r#"---
paths:
  - "{}"
---
# Rule content
"#,
                pattern
            );
            let diagnostics = validate_rule(&content);
            let mem_011: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-MEM-011")
                .collect();
            assert!(mem_011.is_empty(), "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_cc_mem_011_no_frontmatter() {
        let content = "# Rule without frontmatter\n\nSome instructions.";
        let diagnostics = validate_rule(content);
        let mem_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-011")
            .collect();
        assert!(mem_011.is_empty());
    }

    #[test]
    fn test_cc_mem_011_empty_paths() {
        let content = "---\npaths: []\n---\n# Content";
        let diagnostics = validate_rule(content);
        let mem_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-011")
            .collect();
        assert!(mem_011.is_empty());
    }

    #[test]
    fn test_cc_mem_011_unclosed_frontmatter() {
        let content = "---\npaths:\n  - \"src/**/*.ts\"";
        let diagnostics = validate_rule(content);
        let mem_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-011")
            .collect();
        assert_eq!(mem_011.len(), 1);
        assert!(mem_011[0].message.contains("missing closing ---"));
    }

    // ===== CC-MEM-012: Unknown Frontmatter Keys =====

    #[test]
    fn test_cc_mem_012_unknown_keys() {
        let content = r#"---
paths:
  - "src/**/*.ts"
description: "some rule"
alwaysApply: true
---
# Content
"#;
        let diagnostics = validate_rule(content);
        let mem_012: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-012")
            .collect();
        assert_eq!(mem_012.len(), 2);
        assert_eq!(mem_012[0].level, DiagnosticLevel::Warning);
        assert!(mem_012.iter().any(|d| d.message.contains("description")));
        assert!(mem_012.iter().any(|d| d.message.contains("alwaysApply")));
    }

    #[test]
    fn test_cc_mem_012_has_autofix() {
        let content = r#"---
paths:
  - "src/**/*.ts"
description: "some rule"
---
# Content
"#;
        let diagnostics = validate_rule(content);
        let mem_012: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-012")
            .collect();
        assert_eq!(mem_012.len(), 1);
        assert!(mem_012[0].has_fixes());
        assert!(!mem_012[0].fixes[0].safe); // Unsafe: single-line delete may miss multi-line values
        assert!(mem_012[0].fixes[0].is_deletion());
    }

    #[test]
    fn test_cc_mem_012_no_unknown_keys() {
        let content = r#"---
paths:
  - "src/**/*.ts"
---
# Content
"#;
        let diagnostics = validate_rule(content);
        let mem_012: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-012")
            .collect();
        assert!(mem_012.is_empty());
    }

    #[test]
    fn test_cc_mem_012_no_frontmatter() {
        let content = "# Rule without frontmatter";
        let diagnostics = validate_rule(content);
        let mem_012: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-012")
            .collect();
        assert!(mem_012.is_empty());
    }

    // ===== Path Guard =====

    #[test]
    fn test_wrong_path_no_diagnostics() {
        let validator = ClaudeRulesValidator;
        let content = r#"---
unknownKey: value
---
# Content
"#;
        // Not in .claude/rules/ path
        let diagnostics = validator.validate(
            Path::new("some/other/path.md"),
            content,
            &LintConfig::default(),
        );
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_claude_rules_path() {
        let validator = ClaudeRulesValidator;
        let content = r#"---
unknownKey: value
---
# Content
"#;
        let diagnostics = validator.validate(
            Path::new(".claude/rules/my-rule.md"),
            content,
            &LintConfig::default(),
        );
        assert!(!diagnostics.is_empty());
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_memory_category() {
        let mut config = LintConfig::default();
        config.rules_mut().memory = false;

        let content = r#"---
unknownKey: value
paths:
  - "[invalid"
---
# Content
"#;
        let diagnostics = validate_rule_with_config(content, &config);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rules() {
        let rules = ["CC-MEM-011", "CC-MEM-012"];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];

            let content = r#"---
unknownKey: value
paths:
  - "[invalid"
---
# Content
"#;
            let diagnostics = validate_rule_with_config(content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    // ===== Combined Issues =====

    #[test]
    fn test_both_rules_trigger() {
        let content = r#"---
paths:
  - "[invalid"
unknownKey: value
---
# Content
"#;
        let diagnostics = validate_rule(content);
        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-MEM-011"),
            "Expected CC-MEM-011"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-MEM-012"),
            "Expected CC-MEM-012"
        );
    }

    #[test]
    fn test_valid_rule_file_no_issues() {
        let content = r#"---
paths:
  - "src/**/*.ts"
  - "lib/**/*.js"
---
# TypeScript Guidelines

Always use strict mode and explicit types.
"#;
        let diagnostics = validate_rule(content);
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got: {:?}",
            diagnostics
        );
    }

    /// Rules are discovered recursively - "All `.md` files are discovered
    /// recursively, so you can organize rules into subdirectories like
    /// `frontend/` or `backend/`". The validator required `rules/` to be the
    /// immediate parent, so nested rule files were silently skipped and got no
    /// validation at all.
    #[test]
    fn test_nested_rule_file_is_validated() {
        let content = r#"---
paths:
  - "photos [2024/**"
---
# Nested rule with an invalid glob
"#;
        let validator = ClaudeRulesValidator;

        for path in [
            ".claude/rules/frontend/style.md",
            ".claude/rules/backend/api/handlers.md",
            "project/.claude/rules/deep/nested/rule.md",
        ] {
            let diagnostics = validator.validate(Path::new(path), content, &LintConfig::default());
            assert!(
                diagnostics.iter().any(|d| d.rule == "CC-MEM-011"),
                "nested rule file '{path}' must be validated, got: {diagnostics:?}"
            );
        }
    }

    /// A `.md` file that merely has a `rules` ancestor without `.claude` before
    /// it is not a Claude rule and must stay untouched.
    #[test]
    fn test_non_claude_rules_dir_still_skipped() {
        let content = r#"---
paths:
  - "photos [2024/**"
---
# Not a Claude rule
"#;
        let validator = ClaudeRulesValidator;

        for path in ["docs/rules/style.md", ".cursor/rules/style.md"] {
            let diagnostics = validator.validate(Path::new(path), content, &LintConfig::default());
            assert!(
                diagnostics.is_empty(),
                "'{path}' is not under .claude/rules and must be skipped, got: {diagnostics:?}"
            );
        }
    }

    // ===== CC-MEM-011: comma-separated paths and escapes =====

    #[test]
    fn test_cc_mem_011_comma_separated_string_checks_each_pattern() {
        // "Accepts a YAML list or a comma-separated string": each pattern is
        // validated on its own, so the bad one is reported by name.
        let content = "---\npaths: \"src/**/*.{ts,tsx}, photos [2024/**\"\n---\n# Rule\n";
        let diagnostics = validate_rule(content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-MEM-011")
            .collect();
        assert_eq!(hits.len(), 1, "{diagnostics:?}");
        assert!(
            hits[0].message.contains("'photos [2024/**'"),
            "{}",
            hits[0].message
        );
        assert!(hits[0].message.contains("item 2"));
        assert_eq!(hits[0].line, 2);
    }

    #[test]
    fn test_cc_mem_011_parse_error_respects_disabled_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CC-MEM-011".to_string()];
        let diagnostics = validate_rule_with_config("---\npaths: [\n---\n# Rule\n", &config);
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-MEM-011"));
    }

    #[test]
    fn test_cc_mem_011_parse_error_says_rule_loads_everywhere() {
        let diagnostics = validate_rule("---\npaths: [\n---\n# Rule\n");
        let hit = diagnostics
            .iter()
            .find(|d| d.rule == "CC-MEM-011")
            .expect("CC-MEM-011");
        assert!(
            hit.message.contains("as if it had no paths"),
            "{}",
            hit.message
        );
    }

    // ===== CC-MEM-014 / CC-MEM-017 on rule files (ClaudeMdValidator) =====

    fn validate_rule_as_memory(content: &str) -> Vec<Diagnostic> {
        crate::rules::claude_md::ClaudeMdValidator.validate(
            Path::new(".claude/rules/my-rule.md"),
            content,
            &LintConfig::default(),
        )
    }

    #[test]
    fn test_cc_mem_014_rule_file_over_limit() {
        // Frontmatter is removed before the rule loads, so it does not count.
        let body: String = (1..=200).map(|i| format!("- rule {i}\n")).collect();
        let at_limit = format!("---\npaths:\n  - src/**\n---\n{body}");
        assert!(
            !validate_rule_as_memory(&at_limit)
                .iter()
                .any(|d| d.rule == "CC-MEM-014")
        );

        let over = format!("{body}- one more\n");
        let hit = validate_rule_as_memory(&over)
            .into_iter()
            .find(|d| d.rule == "CC-MEM-014")
            .expect("CC-MEM-014");
        assert_eq!(hit.level, DiagnosticLevel::Warning);
        assert!(hit.message.contains("201 lines"));
        // An unconditional rule is told to add paths.
        assert!(
            hit.suggestion
                .as_deref()
                .unwrap_or_default()
                .contains("paths field")
        );
    }

    #[test]
    fn test_cc_mem_017_rule_file_lines_include_frontmatter_offset() {
        let body: String = (0..6).map(|i| format!("- NEVER do thing {i}\n")).collect();
        let content = format!("---\npaths:\n  - src/**\n---\n{body}");
        let hit = validate_rule_as_memory(&content)
            .into_iter()
            .find(|d| d.rule == "CC-MEM-017")
            .expect("CC-MEM-017");
        assert_eq!((hit.line, hit.column), (5, 3));
    }

    #[test]
    fn test_cc_mem_011_escaped_bracket_is_valid() {
        // memory "Path-specific rules": "To match a literal `[` in a file
        // name, escape it as `photos \\[2024/**`."
        let content = "---\npaths:\n  - \"photos \\\\[2024/**\"\n---\n# Rule\n";
        let diagnostics = validate_rule(content);
        assert!(
            !diagnostics.iter().any(|d| d.rule == "CC-MEM-011"),
            "{diagnostics:?}"
        );
    }
}
