//! Cline rules validation rules (CLN-001 to CLN-009, CL-SK-002/003)
//!
//! Validates:
//! - CLN-001: Empty clinerules file (HIGH) - files must have content
//! - CLN-002: Invalid paths glob in clinerules (HIGH) - glob patterns must be valid
//! - CLN-003: Unknown frontmatter key in clinerules (MEDIUM) - only `paths` is recognized
//! - CLN-004: Scalar paths in clinerules (HIGH) - must be array, not scalar
//! - CLN-005: Empty Cline workflow file (HIGH) - workflow files must have content
//! - CLN-006: Cline workflow with frontmatter (MEDIUM) - workflows should be plain markdown
//! - CLN-009: Cline hook uses unknown event name (MEDIUM) - hook filenames must match valid events
//! - CL-SK-002: Cline skill missing required `name` field (HIGH)
//! - CL-SK-003: Cline skill missing required `description` field (HIGH)

use crate::{
    FileType,
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    parsers::frontmatter::split_frontmatter,
    rules::{Validator, ValidatorMetadata},
    schemas::cline::{is_body_empty, is_content_empty, parse_frontmatter, validate_glob_pattern},
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &[
    "CLN-001", "CLN-002", "CLN-003", "CLN-004", "CLN-005", "CLN-006", "CLN-009",
];

/// Valid Cline hook event names.
const VALID_HOOK_EVENTS: &[&str] = &[
    "TaskStart",
    "TaskResume",
    "TaskCancel",
    "TaskComplete",
    "PreToolUse",
    "PostToolUse",
    "UserPromptSubmit",
    "PreCompact",
];

/// Returns true if the path is under `.clinerules/workflows/`.
fn is_workflow_path(path: &Path) -> bool {
    let components: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    components
        .windows(2)
        .any(|w| w[0] == ".clinerules" && w[1] == "workflows")
}

/// Returns true if the path is under `.clinerules/hooks/`.
fn is_hook_path(path: &Path) -> bool {
    let components: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    components
        .windows(2)
        .any(|w| w[0] == ".clinerules" && w[1] == "hooks")
}

/// Returns true if the path is a Cline skill SKILL.md
/// (under `.cline/skills/` or `.clinerules/skills/`).
fn is_cline_skill_path(path: &Path) -> bool {
    let components: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    let has_cline_skills = components
        .windows(2)
        .any(|w| (w[0] == ".cline" || w[0] == ".clinerules") && w[1] == "skills");
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    has_cline_skills && filename == "SKILL.md"
}

pub struct ClineValidator;

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

impl Validator for ClineValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let file_type = crate::detect_file_type(path);
        let is_folder = file_type == FileType::ClineRulesFolder;

        // CLN-001: Empty clinerules file (ERROR)
        if config.is_rule_enabled("CLN-001") {
            if is_folder {
                // For folder files (.md/.txt), check body after frontmatter if present
                if let Some(parsed) = parse_frontmatter(content) {
                    // Only check body emptiness when frontmatter parsed successfully;
                    // parse errors (e.g. missing closing ---) produce empty body by default
                    if parsed.parse_error.is_none() && is_body_empty(&parsed.body) {
                        let total_lines = content.lines().count().max(1);
                        let report_line = (parsed.end_line + 1).min(total_lines);
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                report_line,
                                0,
                                "CLN-001",
                                t!("rules.cln_001.message_no_content"),
                            )
                            .with_suggestion(t!("rules.cln_001.suggestion_no_content")),
                        );
                    }
                } else if is_content_empty(content) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CLN-001",
                            t!("rules.cln_001.message_empty"),
                        )
                        .with_suggestion(t!("rules.cln_001.suggestion_empty")),
                    );
                }
            } else {
                // Single .clinerules file - just check entire content
                if is_content_empty(content) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CLN-001",
                            t!("rules.cln_001.message_empty"),
                        )
                        .with_suggestion(t!("rules.cln_001.suggestion_empty")),
                    );
                }
            }
        }

        // Workflow and hook files get their own specialized rules
        if is_folder && is_workflow_path(path) {
            self.validate_workflow(path, content, config, &mut diagnostics);
            return diagnostics;
        }
        if is_folder && is_hook_path(path) {
            self.validate_hook(path, config, &mut diagnostics);
            return diagnostics;
        }

        // CLN-002, CLN-003, and CLN-004 only apply to folder files (.md/.txt);
        // frontmatter is optional but these rules check frontmatter content when present
        if !is_folder {
            return diagnostics;
        }

        // Parse frontmatter for folder files
        let parsed = match parse_frontmatter(content) {
            Some(p) => p,
            None => {
                // No frontmatter in folder files is fine - paths field is optional
                return diagnostics;
            }
        };

        // If frontmatter has a parse error, skip CLN-002/003
        if parsed.parse_error.is_some() {
            return diagnostics;
        }

        // CLN-002: Invalid paths glob (ERROR)
        if config.is_rule_enabled("CLN-002") {
            if let Some(ref schema) = parsed.schema {
                if let Some(ref paths_field) = schema.paths {
                    for pattern in paths_field.patterns() {
                        let validation = validate_glob_pattern(pattern);
                        if !validation.valid {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    parsed.paths_line.unwrap_or(parsed.start_line + 1),
                                    0,
                                    "CLN-002",
                                    t!(
                                        "rules.cln_002.message",
                                        pattern = pattern,
                                        error = validation.error.unwrap_or_default()
                                    ),
                                )
                                .with_suggestion(t!("rules.cln_002.suggestion")),
                            );
                        }
                    }
                }
            }
        }

        // CLN-004: Scalar paths value (ERROR) - Cline ignores scalar strings
        if config.is_rule_enabled("CLN-004") {
            if let Some(ref schema) = parsed.schema {
                if let Some(ref paths_field) = schema.paths {
                    if let Some(pattern) = paths_field.as_scalar() {
                        let line = parsed.paths_line.unwrap_or(parsed.start_line + 1);
                        let mut diagnostic = Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            0,
                            "CLN-004",
                            t!("rules.cln_004.message"),
                        )
                        .with_suggestion(t!("rules.cln_004.suggestion", pattern = pattern));

                        // Auto-fix: convert scalar to array
                        if let Some((start, end)) = line_byte_range(content, line) {
                            let escaped = pattern.replace('\\', "\\\\").replace('"', "\\\"");
                            let fix_text = format!("paths:\n  - \"{}\"\n", escaped);
                            diagnostic = diagnostic.with_fix(Fix::replace(
                                start,
                                end,
                                fix_text,
                                t!("rules.cln_004.fix"),
                                true,
                            ));
                        }

                        diagnostics.push(diagnostic);
                    }
                }
            }
        }

        // CLN-003: Unknown frontmatter keys (WARNING)
        if config.is_rule_enabled("CLN-003") {
            for unknown in &parsed.unknown_keys {
                let mut diagnostic = Diagnostic::warning(
                    path.to_path_buf(),
                    unknown.line,
                    unknown.column,
                    "CLN-003",
                    t!("rules.cln_003.message", key = unknown.key.as_str()),
                )
                .with_suggestion(t!("rules.cln_003.suggestion", key = unknown.key.as_str()));

                // Auto-fix: remove unknown top-level frontmatter key line.
                // Marked unsafe because multi-line YAML values would leave orphaned lines.
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

// =============================================================================
// CLN-005, CLN-006 (workflow rules) and CLN-009 (hook rules)
// =============================================================================

impl ClineValidator {
    /// Workflow-specific rules (CLN-005, CLN-006).
    fn validate_workflow(
        &self,
        path: &Path,
        content: &str,
        config: &LintConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // CLN-005: Empty workflow file (ERROR)
        if config.is_rule_enabled("CLN-005") && is_content_empty(content) {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CLN-005",
                    t!("rules.cln_005.message"),
                )
                .with_suggestion(t!("rules.cln_005.suggestion")),
            );
        }

        // CLN-006: Workflow with frontmatter (WARNING)
        // Use split_frontmatter to distinguish real frontmatter (opening + closing ---)
        // from a standalone --- which is a valid markdown horizontal rule.
        if config.is_rule_enabled("CLN-006") {
            let parts = split_frontmatter(content);
            if parts.has_frontmatter && parts.has_closing {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "CLN-006",
                        t!("rules.cln_006.message"),
                    )
                    .with_suggestion(t!("rules.cln_006.suggestion")),
                );
            }
        }
    }

    /// Hook-specific rules (CLN-009).
    fn validate_hook(&self, path: &Path, config: &LintConfig, diagnostics: &mut Vec<Diagnostic>) {
        // CLN-009: Unknown hook event name (WARNING)
        if config.is_rule_enabled("CLN-009") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if !VALID_HOOK_EVENTS.contains(&stem) {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            1,
                            0,
                            "CLN-009",
                            t!("rules.cln_009.message", event = stem),
                        )
                        .with_suggestion(t!("rules.cln_009.suggestion")),
                    );
                }
            }
        }
    }
}

// =============================================================================
// CL-SK-002, CL-SK-003 (Cline skill rules)
// =============================================================================

const CLINE_SKILL_RULE_IDS: &[&str] = &["CL-SK-002", "CL-SK-003"];

pub struct ClineSkillValidator;

impl Validator for ClineSkillValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: CLINE_SKILL_RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Only applies to Cline skill SKILL.md files
        if !is_cline_skill_path(path) {
            return diagnostics;
        }

        let parts = split_frontmatter(content);
        if !parts.has_frontmatter || !parts.has_closing {
            // No valid frontmatter - both name and description are missing
            if config.is_rule_enabled("CL-SK-002") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CL-SK-002",
                        t!("rules.cl_sk_002.message"),
                    )
                    .with_suggestion(t!("rules.cl_sk_002.suggestion")),
                );
            }
            if config.is_rule_enabled("CL-SK-003") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CL-SK-003",
                        t!("rules.cl_sk_003.message"),
                    )
                    .with_suggestion(t!("rules.cl_sk_003.suggestion")),
                );
            }
            return diagnostics;
        }

        let fm = &parts.frontmatter;

        // Check for top-level name field
        let has_name = fm.lines().any(|line| {
            !line.starts_with(' ') && !line.starts_with('\t') && line.starts_with("name:")
        });

        // Check for top-level description field
        let has_description = fm.lines().any(|line| {
            !line.starts_with(' ') && !line.starts_with('\t') && line.starts_with("description:")
        });

        if !has_name && config.is_rule_enabled("CL-SK-002") {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CL-SK-002",
                    t!("rules.cl_sk_002.message"),
                )
                .with_suggestion(t!("rules.cl_sk_002.suggestion")),
            );
        }

        if !has_description && config.is_rule_enabled("CL-SK-003") {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CL-SK-003",
                    t!("rules.cl_sk_003.message"),
                )
                .with_suggestion(t!("rules.cl_sk_003.suggestion")),
            );
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate_single(content: &str) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(Path::new(".clinerules"), content, &LintConfig::default())
    }

    fn validate_folder(content: &str) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(
            Path::new(".clinerules/typescript.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_folder_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(Path::new(".clinerules/typescript.md"), content, config)
    }

    fn validate_folder_txt(content: &str) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(
            Path::new(".clinerules/python.txt"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_workflow(content: &str) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(
            Path::new(".clinerules/workflows/deploy.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_workflow_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(
            Path::new(".clinerules/workflows/deploy.md"),
            content,
            config,
        )
    }

    fn validate_hook(path: &str, content: &str) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn validate_hook_with_config(
        path: &str,
        content: &str,
        config: &LintConfig,
    ) -> Vec<Diagnostic> {
        let validator = ClineValidator;
        validator.validate(Path::new(path), content, config)
    }

    fn validate_cline_skill(path: &str, content: &str) -> Vec<Diagnostic> {
        let validator = ClineSkillValidator;
        validator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn validate_cline_skill_with_config(
        path: &str,
        content: &str,
        config: &LintConfig,
    ) -> Vec<Diagnostic> {
        let validator = ClineSkillValidator;
        validator.validate(Path::new(path), content, config)
    }

    // ===== CLN-001: Empty Clinerules File =====

    #[test]
    fn test_cln_001_empty_single_file() {
        let diagnostics = validate_single("");
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
        assert_eq!(cln_001[0].level, DiagnosticLevel::Error);
        assert!(cln_001[0].message.contains("empty"));
    }

    #[test]
    fn test_cln_001_whitespace_only_single() {
        let diagnostics = validate_single("   \n\n\t  ");
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
    }

    #[test]
    fn test_cln_001_valid_single_file() {
        let content = "# Project Rules\n\nAlways follow the coding style guide.";
        let diagnostics = validate_single(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert!(cln_001.is_empty());
    }

    #[test]
    fn test_cln_001_empty_folder_file() {
        let diagnostics = validate_folder("");
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
        assert_eq!(cln_001[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_cln_001_empty_body_after_frontmatter() {
        let content = "---\npaths:\n  - \"**/*.ts\"\n---\n";
        let diagnostics = validate_folder(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
        assert!(cln_001[0].message.contains("no content after frontmatter"));
    }

    #[test]
    fn test_cln_001_valid_folder_file() {
        let content = "---\npaths:\n  - \"**/*.ts\"\n---\n# TypeScript Rules\n\nUse strict mode.\n";
        let diagnostics = validate_folder(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert!(cln_001.is_empty());
    }

    #[test]
    fn test_cln_001_folder_no_frontmatter_with_content() {
        let content = "# Rules without frontmatter\n\nSome instructions.";
        let diagnostics = validate_folder(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert!(cln_001.is_empty());
    }

    #[test]
    fn test_cln_001_newlines_only() {
        let content = "\n\n\n";
        let diagnostics = validate_single(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CLN-001"));
    }

    // ===== CLN-002: Invalid Paths Glob =====

    #[test]
    fn test_cln_002_invalid_glob() {
        let content = "---\npaths:\n  - \"[unclosed\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
        assert_eq!(cln_002.len(), 1);
        assert_eq!(cln_002[0].level, DiagnosticLevel::Error);
        assert!(cln_002[0].message.contains("Invalid glob pattern"));
    }

    #[test]
    fn test_cln_002_valid_glob_patterns() {
        let patterns = vec!["**/*.ts", "*.rs", "src/**/*.js", "tests/**/*.test.ts"];

        for pattern in patterns {
            let content = format!("---\npaths:\n  - \"{}\"\n---\n# Instructions\n", pattern);
            let diagnostics = validate_folder(&content);
            let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
            assert!(cln_002.is_empty(), "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_cln_002_invalid_patterns() {
        let invalid_patterns = ["[invalid", "***", "**["];

        for pattern in invalid_patterns {
            let content = format!("---\npaths:\n  - \"{}\"\n---\nBody", pattern);
            let diagnostics = validate_folder(&content);
            let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
            assert!(
                !cln_002.is_empty(),
                "Pattern '{}' should be invalid",
                pattern
            );
        }
    }

    #[test]
    fn test_cln_002_multiple_patterns_mixed() {
        let content = "---\npaths:\n  - \"**/*.ts\"\n  - \"[invalid\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
        assert_eq!(
            cln_002.len(),
            1,
            "Only the invalid pattern should trigger CLN-002"
        );
        assert!(cln_002[0].message.contains("[invalid"));
    }

    #[test]
    fn test_cln_002_multiple_invalid_patterns() {
        let content = "---\npaths:\n  - \"[bad1\"\n  - \"**[bad2\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
        assert_eq!(
            cln_002.len(),
            2,
            "Both invalid patterns should trigger CLN-002"
        );
    }

    #[test]
    fn test_cln_002_no_paths_field() {
        // No paths field should not trigger CLN-002
        let content = r#"---
---
# Instructions
"#;
        let diagnostics = validate_folder(content);
        let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
        assert!(cln_002.is_empty());
    }

    #[test]
    fn test_cln_002_not_triggered_on_single_file() {
        // Single .clinerules file should not trigger CLN-002
        let content = "# Rules";
        let diagnostics = validate_single(content);
        let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
        assert!(cln_002.is_empty());
    }

    // ===== CLN-003: Unknown Frontmatter Keys =====

    #[test]
    fn test_cln_003_unknown_keys() {
        let content = "---\npaths:\n  - \"**/*.ts\"\nunknownKey: value\nanotherBadKey: 123\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-003").collect();
        assert_eq!(cln_003.len(), 2);
        assert_eq!(cln_003[0].level, DiagnosticLevel::Warning);
        assert!(cln_003.iter().any(|d| d.message.contains("unknownKey")));
        assert!(cln_003.iter().any(|d| d.message.contains("anotherBadKey")));
        assert!(
            cln_003.iter().all(|d| d.has_fixes()),
            "All unknown key diagnostics should include deletion fixes"
        );
        // Fix is marked unsafe because multi-line YAML values would leave orphaned lines
        assert!(cln_003.iter().all(|d| !d.fixes[0].safe));
    }

    #[test]
    fn test_cln_003_no_unknown_keys() {
        let content = "---\npaths:\n  - \"**/*.rs\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-003").collect();
        assert!(cln_003.is_empty());
    }

    #[test]
    fn test_cln_003_not_triggered_on_single_file() {
        let content = "# Rules";
        let diagnostics = validate_single(content);
        let cln_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-003").collect();
        assert!(cln_003.is_empty());
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_cline_category() {
        let mut config = LintConfig::default();
        config.rules_mut().cline = false;

        let content = "";
        let diagnostics = validate_folder_with_config(content, &config);

        let cln_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CLN-"))
            .collect();
        assert!(cln_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CLN-001".to_string()];

        let content = "";
        let diagnostics = validate_folder_with_config(content, &config);

        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert!(cln_001.is_empty());
    }

    // ===== Combined Issues =====

    #[test]
    fn test_multiple_issues() {
        let content = r#"---
unknownKey: value
---
"#;
        let diagnostics = validate_folder(content);

        // Should have CLN-001 (empty body) and CLN-003 (unknown key)
        assert!(
            diagnostics.iter().any(|d| d.rule == "CLN-001"),
            "Expected CLN-001"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "CLN-003"),
            "Expected CLN-003"
        );
    }

    #[test]
    fn test_valid_folder_no_issues() {
        let content = "---\npaths:\n  - \"**/*.ts\"\n---\n# TypeScript Guidelines\n\nAlways use strict mode and explicit types.\n";
        let diagnostics = validate_folder(content);
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got: {:?}",
            diagnostics
        );
    }

    // ===== All Rules Can Be Disabled =====

    #[test]
    fn test_all_cln_rules_can_be_disabled() {
        let rules = ["CLN-001", "CLN-002", "CLN-003", "CLN-004"];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];

            let (content, path): (&str, &str) = match rule {
                "CLN-001" => ("", ".clinerules"),
                "CLN-002" => (
                    "---\npaths:\n  - \"[invalid\"\n---\nBody",
                    ".clinerules/test.md",
                ),
                "CLN-003" => ("---\nunknown: value\n---\nBody", ".clinerules/test.md"),
                "CLN-004" => ("---\npaths: \"**/*.ts\"\n---\nBody", ".clinerules/test.md"),
                _ => unreachable!("Unknown rule: {rule}"),
            };

            let validator = ClineValidator;
            let diagnostics = validator.validate(Path::new(path), content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    // ===== CLN-004: Scalar Paths Error =====

    #[test]
    fn test_cln_004_scalar_paths_warns() {
        let content = "---\npaths: \"**/*.ts\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-004").collect();
        assert_eq!(cln_004.len(), 1);
        assert_eq!(cln_004[0].level, DiagnosticLevel::Error);
        assert!(cln_004[0].message.contains("scalar"));
    }

    #[test]
    fn test_cln_004_array_paths_no_warning() {
        let content = "---\npaths:\n  - \"**/*.ts\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-004").collect();
        assert!(cln_004.is_empty());
    }

    #[test]
    fn test_cln_004_has_autofix() {
        let content = "---\npaths: \"**/*.ts\"\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-004").collect();
        assert_eq!(cln_004.len(), 1);
        assert!(cln_004[0].has_fixes(), "CLN-004 should have an auto-fix");
        assert!(cln_004[0].fixes[0].safe, "CLN-004 fix should be safe");
        assert!(
            cln_004[0].fixes[0].replacement.contains("- \"**/*.ts\""),
            "Fix should convert scalar to array format, got: {}",
            cln_004[0].fixes[0].replacement
        );
    }

    #[test]
    fn test_cln_004_empty_array_no_warning() {
        let content = "---\npaths: []\n---\n# Instructions\n";
        let diagnostics = validate_folder(content);
        let cln_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-004").collect();
        assert!(cln_004.is_empty(), "Empty array should not trigger CLN-004");
    }

    // ===== File Type Detection =====

    #[test]
    fn test_single_file_detection() {
        assert_eq!(
            crate::detect_file_type(Path::new(".clinerules")),
            FileType::ClineRules
        );
    }

    #[test]
    fn test_folder_file_detection() {
        assert_eq!(
            crate::detect_file_type(Path::new(".clinerules/typescript.md")),
            FileType::ClineRulesFolder
        );
    }

    #[test]
    fn test_folder_file_with_numeric_prefix() {
        assert_eq!(
            crate::detect_file_type(Path::new(".clinerules/01-coding.md")),
            FileType::ClineRulesFolder
        );
    }

    // ===== .txt file validation (mirrors .md tests) =====

    #[test]
    fn test_cln_001_empty_txt_file() {
        let diagnostics = validate_folder_txt("");
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
        assert_eq!(cln_001[0].level, DiagnosticLevel::Error);
        assert!(cln_001[0].message.contains("empty"));
    }

    #[test]
    fn test_cln_001_valid_txt_file() {
        let content = "---\npaths:\n  - \"**/*.py\"\n---\n# Python Rules\n\nFollow PEP 8.\n";
        let diagnostics = validate_folder_txt(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert!(cln_001.is_empty());
    }

    #[test]
    fn test_cln_002_bad_glob_in_txt() {
        let content = "---\npaths:\n  - \"[unclosed\"\n---\n# Instructions\n";
        let diagnostics = validate_folder_txt(content);
        let cln_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-002").collect();
        assert_eq!(cln_002.len(), 1);
        assert_eq!(cln_002[0].level, DiagnosticLevel::Error);
        assert!(cln_002[0].message.contains("Invalid glob pattern"));
    }

    #[test]
    fn test_cln_003_unknown_keys_in_txt() {
        let content = "---\npaths:\n  - \"**/*.ts\"\nunknownKey: value\nanotherBadKey: 123\n---\n# Instructions\n";
        let diagnostics = validate_folder_txt(content);
        let cln_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-003").collect();
        assert_eq!(cln_003.len(), 2);
        assert_eq!(cln_003[0].level, DiagnosticLevel::Warning);
        assert!(cln_003.iter().any(|d| d.message.contains("unknownKey")));
        assert!(cln_003.iter().any(|d| d.message.contains("anotherBadKey")));
        assert!(
            cln_003.iter().all(|d| d.has_fixes()),
            "All unknown key diagnostics should include deletion fixes"
        );
        assert!(cln_003.iter().all(|d| !d.fixes[0].safe));
    }

    #[test]
    fn test_cln_004_scalar_paths_in_txt() {
        let content = "---\npaths: \"**/*.ts\"\n---\n# Instructions\n";
        let diagnostics = validate_folder_txt(content);
        let cln_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-004").collect();
        assert_eq!(cln_004.len(), 1);
        assert_eq!(cln_004[0].level, DiagnosticLevel::Error);
        assert!(cln_004[0].message.contains("scalar"));
        assert!(cln_004[0].has_fixes(), "CLN-004 should have an auto-fix");
        assert!(cln_004[0].fixes[0].safe, "CLN-004 fix should be safe");
        assert!(
            cln_004[0].fixes[0].replacement.contains("- \"**/*.ts\""),
            "Fix should convert scalar to array format, got: {}",
            cln_004[0].fixes[0].replacement
        );
    }

    #[test]
    fn test_cln_001_whitespace_only_txt() {
        let diagnostics = validate_folder_txt("   \n\n\t  ");
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
        assert_eq!(cln_001[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_cln_001_empty_body_after_frontmatter_txt() {
        let content = "---\npaths:\n  - \"**/*.py\"\n---\n";
        let diagnostics = validate_folder_txt(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert_eq!(cln_001.len(), 1);
        assert!(cln_001[0].message.contains("no content after frontmatter"));
    }

    #[test]
    fn test_cln_001_folder_no_frontmatter_with_content_txt() {
        let content = "# Rules without frontmatter\n\nSome instructions.";
        let diagnostics = validate_folder_txt(content);
        let cln_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-001").collect();
        assert!(cln_001.is_empty());
    }

    #[test]
    fn test_valid_txt_no_diagnostics() {
        let content =
            "---\npaths:\n  - \"**/*.py\"\n---\n# Python Guidelines\n\nAlways use type hints.\n";
        let diagnostics = validate_folder_txt(content);
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics for valid .txt file, got: {:?}",
            diagnostics
        );
    }

    // ===== CLN-005: Empty Workflow File =====

    #[test]
    fn test_cln_005_empty_workflow() {
        let diagnostics = validate_workflow("");
        let cln_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-005").collect();
        assert_eq!(cln_005.len(), 1);
        assert_eq!(cln_005[0].level, DiagnosticLevel::Error);
        assert!(cln_005[0].message.contains("empty"));
    }

    #[test]
    fn test_cln_005_whitespace_only_workflow() {
        let diagnostics = validate_workflow("   \n\n\t  ");
        let cln_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-005").collect();
        assert_eq!(cln_005.len(), 1);
    }

    #[test]
    fn test_cln_005_valid_workflow() {
        let content = "# Deploy Workflow\n\n1. Build the project\n2. Run tests\n";
        let diagnostics = validate_workflow(content);
        let cln_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-005").collect();
        assert!(cln_005.is_empty());
    }

    #[test]
    fn test_cln_005_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CLN-005".to_string()];
        let diagnostics = validate_workflow_with_config("", &config);
        let cln_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-005").collect();
        assert!(cln_005.is_empty());
    }

    // ===== CLN-006: Workflow with Frontmatter =====

    #[test]
    fn test_cln_006_workflow_with_frontmatter() {
        let content = "---\ntitle: Deploy\n---\n# Deploy steps\n";
        let diagnostics = validate_workflow(content);
        let cln_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-006").collect();
        assert_eq!(cln_006.len(), 1);
        assert_eq!(cln_006[0].level, DiagnosticLevel::Warning);
        assert!(cln_006[0].message.contains("frontmatter"));
    }

    #[test]
    fn test_cln_006_workflow_plain_markdown() {
        let content = "# Deploy Workflow\n\nStep 1: build\n";
        let diagnostics = validate_workflow(content);
        let cln_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-006").collect();
        assert!(cln_006.is_empty());
    }

    #[test]
    fn test_cln_006_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CLN-006".to_string()];
        let content = "---\ntitle: Deploy\n---\n# Deploy steps\n";
        let diagnostics = validate_workflow_with_config(content, &config);
        let cln_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-006").collect();
        assert!(cln_006.is_empty());
    }

    #[test]
    fn test_cln_006_not_triggered_on_regular_folder_file() {
        // Regular .clinerules/*.md files can have frontmatter
        let content = "---\npaths:\n  - \"**/*.ts\"\n---\n# TypeScript Rules\n";
        let diagnostics = validate_folder(content);
        let cln_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-006").collect();
        assert!(cln_006.is_empty());
    }

    // ===== CLN-009: Unknown Hook Event Name =====

    #[test]
    fn test_cln_009_unknown_event() {
        let diagnostics = validate_hook(
            ".clinerules/hooks/InvalidEvent.sh",
            "#!/bin/bash\necho hello",
        );
        let cln_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-009").collect();
        assert_eq!(cln_009.len(), 1);
        assert_eq!(cln_009[0].level, DiagnosticLevel::Warning);
        assert!(cln_009[0].message.contains("InvalidEvent"));
    }

    #[test]
    fn test_cln_009_valid_events() {
        let events = [
            "TaskStart",
            "TaskResume",
            "TaskCancel",
            "TaskComplete",
            "PreToolUse",
            "PostToolUse",
            "UserPromptSubmit",
            "PreCompact",
        ];
        for event in events {
            let path = format!(".clinerules/hooks/{}.sh", event);
            let diagnostics = validate_hook(&path, "#!/bin/bash\necho hello");
            let cln_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-009").collect();
            assert!(
                cln_009.is_empty(),
                "Event '{}' should be valid but triggered CLN-009",
                event
            );
        }
    }

    #[test]
    fn test_cln_009_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CLN-009".to_string()];
        let diagnostics =
            validate_hook_with_config(".clinerules/hooks/BadEvent.sh", "#!/bin/bash", &config);
        let cln_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-009").collect();
        assert!(cln_009.is_empty());
    }

    #[test]
    fn test_cln_009_not_triggered_on_non_hook_path() {
        // A file not under hooks/ should not trigger CLN-009
        let diagnostics = validate_folder("#!/bin/bash\necho hello");
        let cln_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CLN-009").collect();
        assert!(cln_009.is_empty());
    }

    // ===== CL-SK-002: Missing name Field =====

    #[test]
    fn test_cl_sk_002_missing_name() {
        let content = "---\ndescription: A test skill\n---\n# Skill body\n";
        let diagnostics = validate_cline_skill(".cline/skills/my-skill/SKILL.md", content);
        let cl_sk_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CL-SK-002")
            .collect();
        assert_eq!(cl_sk_002.len(), 1);
        assert_eq!(cl_sk_002[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_cl_sk_002_has_name() {
        let content = "---\nname: my-skill\ndescription: A test skill\n---\n# Skill body\n";
        let diagnostics = validate_cline_skill(".cline/skills/my-skill/SKILL.md", content);
        let cl_sk_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CL-SK-002")
            .collect();
        assert!(cl_sk_002.is_empty());
    }

    #[test]
    fn test_cl_sk_002_no_frontmatter() {
        let content = "# Skill without frontmatter";
        let diagnostics = validate_cline_skill(".cline/skills/my-skill/SKILL.md", content);
        let cl_sk_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CL-SK-002")
            .collect();
        assert_eq!(cl_sk_002.len(), 1, "Missing frontmatter means missing name");
    }

    #[test]
    fn test_cl_sk_002_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CL-SK-002".to_string()];
        let content = "---\ndescription: A skill\n---\n# Body\n";
        let diagnostics =
            validate_cline_skill_with_config(".cline/skills/my-skill/SKILL.md", content, &config);
        let cl_sk_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CL-SK-002")
            .collect();
        assert!(cl_sk_002.is_empty());
    }

    // ===== CL-SK-003: Missing description Field =====

    #[test]
    fn test_cl_sk_003_missing_description() {
        let content = "---\nname: my-skill\n---\n# Skill body\n";
        let diagnostics = validate_cline_skill(".cline/skills/my-skill/SKILL.md", content);
        let cl_sk_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CL-SK-003")
            .collect();
        assert_eq!(cl_sk_003.len(), 1);
        assert_eq!(cl_sk_003[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_cl_sk_003_has_description() {
        let content = "---\nname: my-skill\ndescription: A test skill\n---\n# Skill body\n";
        let diagnostics = validate_cline_skill(".cline/skills/my-skill/SKILL.md", content);
        let cl_sk_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CL-SK-003")
            .collect();
        assert!(cl_sk_003.is_empty());
    }

    #[test]
    fn test_cl_sk_003_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CL-SK-003".to_string()];
        let content = "---\nname: my-skill\n---\n# Body\n";
        let diagnostics =
            validate_cline_skill_with_config(".cline/skills/my-skill/SKILL.md", content, &config);
        let cl_sk_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CL-SK-003")
            .collect();
        assert!(cl_sk_003.is_empty());
    }

    // ===== CL-SK-002/003: Non-Cline skill paths should be skipped =====

    #[test]
    fn test_cl_sk_not_triggered_for_claude_skill() {
        let content = "---\nlicense: MIT\n---\n# Body\n";
        let diagnostics = validate_cline_skill(".claude/skills/my-skill/SKILL.md", content);
        assert!(
            diagnostics.is_empty(),
            "CL-SK rules should not fire for non-Cline skill paths"
        );
    }

    #[test]
    fn test_cl_sk_works_for_clinerules_skills_path() {
        let content = "---\nlicense: MIT\n---\n# Body\n";
        let diagnostics = validate_cline_skill(".clinerules/skills/my-skill/SKILL.md", content);
        let cl_sk_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CL-SK-002")
            .collect();
        let cl_sk_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CL-SK-003")
            .collect();
        assert_eq!(
            cl_sk_002.len(),
            1,
            "CL-SK-002 should fire for .clinerules/skills/ path"
        );
        assert_eq!(
            cl_sk_003.len(),
            1,
            "CL-SK-003 should fire for .clinerules/skills/ path"
        );
    }

    // ===== Path detection tests =====

    #[test]
    fn test_is_workflow_path() {
        assert!(is_workflow_path(Path::new(
            ".clinerules/workflows/deploy.md"
        )));
        assert!(!is_workflow_path(Path::new(".clinerules/typescript.md")));
        assert!(!is_workflow_path(Path::new(
            ".clinerules/hooks/TaskStart.sh"
        )));
    }

    #[test]
    fn test_is_hook_path() {
        assert!(is_hook_path(Path::new(".clinerules/hooks/TaskStart.sh")));
        assert!(!is_hook_path(Path::new(".clinerules/typescript.md")));
        assert!(!is_hook_path(Path::new(".clinerules/workflows/deploy.md")));
    }

    #[test]
    fn test_is_cline_skill_path() {
        assert!(is_cline_skill_path(Path::new(
            ".cline/skills/my-skill/SKILL.md"
        )));
        assert!(is_cline_skill_path(Path::new(
            ".clinerules/skills/my-skill/SKILL.md"
        )));
        assert!(!is_cline_skill_path(Path::new(
            ".claude/skills/my-skill/SKILL.md"
        )));
        assert!(!is_cline_skill_path(Path::new(
            ".cline/skills/my-skill/README.md"
        )));
    }

    // ===== All New Rules Can Be Disabled =====

    #[test]
    fn test_all_new_rules_can_be_disabled() {
        let cases: Vec<(&str, &str, &str)> = vec![
            ("CLN-005", ".clinerules/workflows/test.md", ""),
            (
                "CLN-006",
                ".clinerules/workflows/test.md",
                "---\ntitle: x\n---\n# Body",
            ),
            ("CLN-009", ".clinerules/hooks/BadEvent.sh", "#!/bin/bash"),
        ];

        for (rule, path, content) in cases {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];
            let validator = ClineValidator;
            let diagnostics = validator.validate(Path::new(path), content, &config);
            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    // ===== Workflow file type detection =====

    #[test]
    fn test_workflow_file_detected_as_cline_rules_folder() {
        assert_eq!(
            crate::detect_file_type(Path::new(".clinerules/workflows/deploy.md")),
            FileType::ClineRulesFolder
        );
    }

    #[test]
    fn test_hook_file_detected_as_cline_rules_folder() {
        assert_eq!(
            crate::detect_file_type(Path::new(".clinerules/hooks/TaskStart.md")),
            FileType::ClineRulesFolder
        );
    }
}
