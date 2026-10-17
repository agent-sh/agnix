//! Prompt engineering validation rules
//!
//! Each rule runs only on files whose vendor documents the advice:
//! - PE-002: chain-of-thought phrases on simple tasks. Claude memory files
//!   (Anthropic prompting best practices).
//! - PE-004: ambiguous terms (usually/sometimes/if possible). Claude memory
//!   files ("Write instructions that are concrete enough to verify"),
//!   AGENTS.md (also Codex: "a long file full of vague rules") and Cursor
//!   rules ("Avoid vague guidance").
//! - PE-005: redundant generic instructions (be helpful, write clean code).
//!   Claude memory files (best-practices Exclude column) and AGENTS.md (also
//!   Windsurf, VS Code and Copilot code review).
//! - PE-006: negative-only instructions. Claude memory files only ("Tell
//!   Claude what to do instead of what not to do"); Gemini advises the
//!   opposite ("Use negative constraints"), so GEMINI.md and AGENTS.md, which
//!   Gemini can read, are skipped.

use crate::{
    FileType,
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::{Validator, ValidatorMetadata, line_byte_range},
    schemas::prompt::{
        find_ambiguous_instructions, find_cot_on_simple_tasks, find_negative_only_instructions,
        find_redundant_instructions, is_only_redundant_instruction,
    },
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["PE-002", "PE-004", "PE-005", "PE-006"];

pub struct PromptValidator;

/// Whose instruction file this is, which decides the PE rules that apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Audience {
    /// CLAUDE.md, CLAUDE.local.md, and direct calls on other Markdown.
    Claude,
    /// AGENTS.md, AGENTS.local.md, AGENTS.override.md: read by many tools.
    AgentsMd,
    /// `.cursor/rules/**` and `.cursorrules`.
    Cursor,
    /// Files no PE source covers (GEMINI.md).
    None,
}

impl Audience {
    fn of(path: &Path) -> Self {
        match crate::detect_file_type(path) {
            FileType::ClaudeMd => {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with("AGENTS") {
                    Self::AgentsMd
                } else {
                    Self::Claude
                }
            }
            FileType::CursorRule | FileType::CursorRulesLegacy => Self::Cursor,
            FileType::GeminiMd => Self::None,
            // The registry runs this validator only on the types above; a
            // direct call on other Markdown gets the Claude rule set.
            _ => Self::Claude,
        }
    }

    fn runs(self, rule: &str) -> bool {
        match self {
            Self::Claude => true,
            Self::AgentsMd => matches!(rule, "PE-004" | "PE-005"),
            Self::Cursor => rule == "PE-004",
            Self::None => false,
        }
    }
}

impl Validator for PromptValidator {
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
        let audience = Audience::of(path);
        let enabled = |rule: &str| audience.runs(rule) && config.is_rule_enabled(rule);

        // PE-002: Chain-of-thought on simple tasks
        if enabled("PE-002") {
            let cot_issues = find_cot_on_simple_tasks(content);
            for issue in cot_issues {
                diagnostics.push(
                    Diagnostic::info(
                        path.to_path_buf(),
                        issue.line,
                        issue.column,
                        "PE-002",
                        t!(
                            "rules.pe_002.message",
                            phrase = issue.phrase.as_str(),
                            task = issue.task_indicator.as_str()
                        ),
                    )
                    .with_suggestion(t!("rules.pe_002.suggestion")),
                );
            }
        }

        // PE-004: Ambiguous instructions
        if enabled("PE-004") {
            let ambiguous = find_ambiguous_instructions(content);
            for issue in ambiguous {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        issue.line,
                        issue.column,
                        "PE-004",
                        t!("rules.pe_004.message", term = issue.term.as_str()),
                    )
                    .with_suggestion(t!("rules.pe_004.suggestion")),
                );
            }
        }

        // PE-005: Redundant generic instructions
        if enabled("PE-005") {
            let redundant = find_redundant_instructions(content);
            let lines: Vec<&str> = content.lines().collect();
            for issue in redundant {
                let mut diagnostic = Diagnostic::warning(
                    path.to_path_buf(),
                    issue.line,
                    issue.column,
                    "PE-005",
                    t!("rules.pe_005.message", phrase = issue.phrase.as_str()),
                )
                .with_suggestion(t!("rules.pe_005.suggestion"));

                // Offer to delete the line only when it holds nothing but
                // generic phrases. Deleting text the model reads changes what
                // it is told, so the fix stays opt-in (unsafe).
                let only_generic = lines
                    .get(issue.line.saturating_sub(1))
                    .is_some_and(|line| is_only_redundant_instruction(line));
                if only_generic && let Some((start, end)) = line_byte_range(content, issue.line) {
                    diagnostic = diagnostic.with_fix(Fix::delete(
                        start,
                        end,
                        format!("Remove redundant instruction '{}'", issue.phrase),
                        false,
                    ));
                }

                diagnostics.push(diagnostic);
            }
        }

        // PE-006: Negative-only instructions
        if enabled("PE-006") {
            let negative_only = find_negative_only_instructions(content);
            for issue in negative_only {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        issue.line,
                        issue.column,
                        "PE-006",
                        t!("rules.pe_006.message", text = issue.text.as_str()),
                    )
                    .with_suggestion(t!("rules.pe_006.suggestion")),
                );
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

    // ===== PE-002: Chain-of-Thought on Simple Tasks =====

    #[test]
    fn test_pe_002_cot_on_read_file() {
        let content = r#"# Read File Skill

When asked to read the file, think step by step.
"#;
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-002").collect();
        assert_eq!(pe_002.len(), 1);
        assert_eq!(pe_002[0].level, DiagnosticLevel::Info);
    }

    #[test]
    fn test_pe_002_no_cot_on_complex_task() {
        let content = r#"# Code Review Skill

Think step by step when reviewing.
"#;
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-002").collect();
        assert!(pe_002.is_empty());
    }

    #[test]
    fn test_pe_002_simple_task_without_cot_ok() {
        let content = r#"# Read File Skill

Read the file and return contents.
"#;
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-002").collect();
        assert!(pe_002.is_empty());
    }

    // ===== PE-004: Ambiguous Instructions =====

    #[test]
    fn test_pe_004_usually() {
        let content = "Usually format output as JSON.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-004").collect();
        assert_eq!(pe_004.len(), 1);
        assert_eq!(pe_004[0].level, DiagnosticLevel::Warning);
        assert!(pe_004[0].message.to_lowercase().contains("usually"));
    }

    #[test]
    fn test_pe_004_if_possible() {
        let content = "Include tests if possible.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-004").collect();
        assert_eq!(pe_004.len(), 1);
    }

    #[test]
    fn test_pe_004_clear_instruction_ok() {
        let content = "Always format output as JSON.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-004").collect();
        assert!(pe_004.is_empty());
    }

    #[test]
    fn test_pe_004_skips_code_blocks() {
        let content = r#"```
// Usually this comment is fine
```"#;
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-004").collect();
        assert!(pe_004.is_empty());
    }

    // ===== Config Integration Tests =====

    #[test]
    fn test_config_disabled_prompt_engineering_category() {
        let mut config = LintConfig::default();
        config.rules_mut().prompt_engineering = false;

        let content = r#"# Critical Rules
You should do X.
Usually do Y.
"#;

        let validator = PromptValidator;
        let diagnostics = validator.validate(Path::new("SKILL.md"), content, &config);

        // All PE-* rules should be disabled
        let pe_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("PE-"))
            .collect();
        assert!(pe_rules.is_empty());
    }

    // ===== Edge Case Tests =====

    #[test]
    fn test_empty_content_string() {
        let content = "";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        // Should not panic, return empty diagnostics
        assert!(
            diagnostics.is_empty(),
            "Empty content should produce no diagnostics"
        );
    }

    #[test]
    fn test_pe_004_inline_code_not_flagged() {
        let content = "Format output with `usually` backticks for clarity.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-004").collect();
        // Content inside inline code should still be checked (current behavior)
        // This test documents the current behavior
        assert!(
            !pe_004.is_empty(),
            "Inline code with ambiguous terms is currently flagged"
        );
    }

    // ===== Additional PE rule tests =====

    #[test]
    fn test_pe_002_cot_on_file_read() {
        // Test PE-002: CoT markers on simple tasks
        // Content must have both: CoT marker + simple task indicator within 5 lines
        let content = "# Rules\n\nLet's think step by step.\nYou need to read the file carefully.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-002").collect();
        // PE-002 triggers when CoT marker is within 5 lines of a simple task indicator
        assert!(
            !pe_002.is_empty(),
            "Chain-of-thought near 'read the file' should trigger PE-002"
        );
    }

    #[test]
    fn test_pe_004_all_ambiguous_phrases() {
        let ambiguous = ["usually", "if possible", "when appropriate", "sometimes"];

        for phrase in ambiguous {
            let content = format!("# Rules\n\n{} do this task.", phrase);
            let validator = PromptValidator;
            let diagnostics =
                validator.validate(Path::new("SKILL.md"), &content, &LintConfig::default());

            let pe_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-004").collect();
            assert!(
                !pe_004.is_empty(),
                "Ambiguous phrase '{}' should trigger PE-004",
                phrase
            );
        }
    }

    #[test]
    fn test_pe_004_clear_instructions_ok() {
        let content = "# Rules\n\nAlways run tests before committing.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-004").collect();
        assert!(
            pe_004.is_empty(),
            "Clear instructions should not trigger PE-004"
        );
    }

    // ===== PE-005: Redundant Generic Instructions =====

    #[test]
    fn test_pe_005_be_helpful() {
        let content = "Be helpful and accurate when responding.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-005").collect();
        assert_eq!(pe_005.len(), 1);
        assert_eq!(pe_005[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_pe_005_specific_instructions_ok() {
        let content = "Format all output as JSON with 2-space indentation.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-005").collect();
        assert!(pe_005.is_empty());
    }

    #[test]
    fn test_pe_005_skips_code_blocks() {
        let content = "```\nBe helpful and accurate.\n```";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-005").collect();
        assert!(pe_005.is_empty());
    }

    // ===== PE-006: Negative-Only Instructions =====

    #[test]
    fn test_pe_006_negative_only() {
        let content = "Don't use global variables.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-006").collect();
        assert_eq!(pe_006.len(), 1);
        assert_eq!(pe_006[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_pe_006_with_alternative_ok() {
        let content = "Don't use global variables. Instead, pass values as function parameters.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-006").collect();
        assert!(pe_006.is_empty());
    }

    #[test]
    fn test_pe_006_never_without_alternative() {
        let content = "Never use eval in production code.";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-006").collect();
        assert_eq!(pe_006.len(), 1);
    }

    #[test]
    fn test_pe_006_skips_code_blocks() {
        let content = "```\nDon't use global variables.\n```";
        let validator = PromptValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let pe_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-006").collect();
        assert!(pe_006.is_empty());
    }

    // ===== PE-005/PE-006 Config Disable =====

    #[test]
    fn test_pe_005_disabled_individually() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["PE-005".to_string()];

        let content = "Be helpful and accurate.";
        let validator = PromptValidator;
        let diagnostics = validator.validate(Path::new("SKILL.md"), content, &config);

        let pe_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-005").collect();
        assert!(pe_005.is_empty(), "PE-005 should be disabled");
    }

    #[test]
    fn test_pe_006_disabled_individually() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["PE-006".to_string()];

        let content = "Don't use global variables.";
        let validator = PromptValidator;
        let diagnostics = validator.validate(Path::new("SKILL.md"), content, &config);

        let pe_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-006").collect();
        assert!(pe_006.is_empty(), "PE-006 should be disabled");
    }

    // ===== Autofix Tests =====

    // ===== Retired rules =====

    #[test]
    fn test_retired_pe_001_and_pe_003_absent() {
        // PE-001 (keyword position, no vendor source) and PE-003 (pushed for
        // stronger wording, which Anthropic now advises against) are retired.
        let mut lines: Vec<String> = (0..20).map(|i| format!("Line {}", i)).collect();
        lines[0] = "# Critical Rules".to_string();
        lines[1] = "You should follow the style.".to_string();
        lines[10] = "This is critical information.".to_string();
        let diagnostics = PromptValidator.validate(
            Path::new("CLAUDE.md"),
            &lines.join("\n"),
            &LintConfig::default(),
        );
        assert!(
            !diagnostics
                .iter()
                .any(|d| d.rule == "PE-001" || d.rule == "PE-003")
        );
    }

    #[test]
    fn test_all_pe_rules_can_be_disabled() {
        let content = "# Rules\nLet's think step by step to read the file.\nUsually do it.\nBe helpful.\nDon't use global variables.\n";
        for rule in RULE_IDS {
            let baseline =
                PromptValidator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());
            assert!(baseline.iter().any(|d| d.rule == *rule), "{rule} fires");

            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];
            let diagnostics = PromptValidator.validate(Path::new("CLAUDE.md"), content, &config);
            assert!(
                !diagnostics.iter().any(|d| d.rule == *rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    // ===== Per-vendor scope =====

    fn rules_on(path: &str, content: &str) -> Vec<String> {
        let mut rules: Vec<String> = PromptValidator
            .validate(Path::new(path), content, &LintConfig::default())
            .into_iter()
            .map(|d| d.rule)
            .collect();
        rules.sort();
        rules.dedup();
        rules
    }

    const ALL_PE: &str = "# Rules\nLet's think step by step to read the file.\nUsually do it.\nBe helpful.\nDon't use global variables.\n";

    #[test]
    fn test_claude_files_get_every_pe_rule() {
        for path in ["CLAUDE.md", "CLAUDE.local.md", ".claude/CLAUDE.md"] {
            assert_eq!(
                rules_on(path, ALL_PE),
                vec!["PE-002", "PE-004", "PE-005", "PE-006"],
                "{path}"
            );
        }
    }

    #[test]
    fn test_agents_md_gets_only_multi_vendor_rules() {
        // PE-006 contradicts Gemini ("Use negative constraints"), which can
        // read AGENTS.md; PE-002 has only an Anthropic source.
        for path in ["AGENTS.md", "AGENTS.local.md", "AGENTS.override.md"] {
            assert_eq!(rules_on(path, ALL_PE), vec!["PE-004", "PE-005"], "{path}");
        }
    }

    #[test]
    fn test_cursor_rules_get_only_pe_004() {
        // Cursor: "Avoid vague guidance". No Cursor source for the others.
        for path in [".cursor/rules/style.mdc", ".cursorrules"] {
            assert_eq!(rules_on(path, ALL_PE), vec!["PE-004"], "{path}");
        }
    }

    #[test]
    fn test_gemini_md_gets_no_pe_rules() {
        // Gemini's own example "Do not use class components" and "Be concise".
        let content = "- Do not use class components.\n- Be concise.\n";
        assert!(rules_on("GEMINI.md", content).is_empty());
    }

    // ===== PE-005 fix =====

    #[test]
    fn test_pe_005_fix_only_for_generic_only_lines() {
        let content = "Line one.\n- Be helpful and accurate.\nBe helpful and accurate when responding.\nLine four.";
        let diagnostics =
            PromptValidator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());
        let pe_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "PE-005").collect();
        assert_eq!(pe_005.len(), 2);

        // Line 2 holds nothing else: an opt-in deletion is offered.
        let fix = &pe_005[0].fixes[0];
        assert!(!fix.safe, "PE-005 fix should be unsafe");
        assert!(fix.is_deletion());
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        assert_eq!(
            fixed,
            "Line one.\nBe helpful and accurate when responding.\nLine four."
        );

        // Line 3 also says when the instruction applies: no fix.
        assert!(!pe_005[1].has_fixes());
    }

    #[test]
    fn test_pe_005_write_clean_code() {
        let content = "- Write clean code.";
        let diagnostics =
            PromptValidator.validate(Path::new("CLAUDE.md"), content, &LintConfig::default());
        assert!(diagnostics.iter().any(|d| d.rule == "PE-005"));
    }
}
