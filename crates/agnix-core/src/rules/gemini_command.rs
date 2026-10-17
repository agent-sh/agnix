//! Gemini CLI custom command validation (GM-CMD-001, GM-CMD-002, GM-CMD-003).
//!
//! Custom commands are TOML files under `~/.gemini/commands/`,
//! `<project>/.gemini/commands/` and an extension's `commands/`
//! (geminicli.com/docs/cli/custom-commands, "TOML file format (v1)").
//! `FileCommandLoader.ts` parses each file with `toml.parse` and validates
//! it with
//!
//! ```ts
//! const TomlCommandDefSchema = z.object({
//!   prompt: z.string({
//!     required_error: "The 'prompt' field is required.",
//!     invalid_type_error: "The 'prompt' field must be a string.",
//!   }),
//!   description: z.string().optional(),
//! });
//! ```
//!
//! A file that does not parse or validate is skipped ("Skipping invalid
//! command file"); other keys are allowed (the object is not strict). The
//! `/help` text is `sanitizeForDisplay(description, 100)`, or "Custom command
//! from <file>" without a description.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    rules::{Validator, ValidatorMetadata},
    schemas::{
        gemini_toml::{nth_occurrence_in_span, parse_toml_document, toml_entry},
        kiro_json::LineIndex,
    },
};
use rust_i18n::t;
use std::path::Path;
use toml::de::DeValue;

const RULE_IDS: &[&str] = &["GM-CMD-001", "GM-CMD-002", "GM-CMD-003"];

/// `description = sanitizeForDisplay(description, 100);`
const DESCRIPTION_DISPLAY_LIMIT: usize = 100;

/// The injection triggers, in the order Gemini processes them: "File
/// content injection with `@{...}` is processed _before_ shell commands
/// (`!{...}`)".
const TRIGGERS: [&str; 2] = ["@{", "!{"];

pub struct GeminiCommandValidator;

impl Validator for GeminiCommandValidator {
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
        if !RULE_IDS.iter().any(|id| config.is_rule_enabled(id)) {
            return Vec::new();
        }
        let mut cx = Cx {
            path,
            content,
            lines: LineIndex::new(content),
            config,
            diagnostics: Vec::new(),
        };
        cx.check();
        cx.diagnostics
    }
}

struct Cx<'a, 'c> {
    path: &'a Path,
    content: &'a str,
    lines: LineIndex<'a>,
    config: &'a PerFileLintConfig<'c>,
    diagnostics: Vec<Diagnostic>,
}

impl Cx<'_, '_> {
    fn push(
        &mut self,
        level: DiagnosticLevel,
        rule: &str,
        span: (usize, usize),
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) {
        if !self.config.is_rule_enabled(rule) {
            return;
        }
        let (line, column) = self.lines.line_col(span.0);
        let (end_line, end_column) = self.lines.line_col(span.1.max(span.0));
        let path = self.path.to_path_buf();
        let diagnostic = match level {
            DiagnosticLevel::Error => Diagnostic::error(path, line, column, rule, message),
            DiagnosticLevel::Warning => Diagnostic::warning(path, line, column, rule, message),
            DiagnosticLevel::Info => Diagnostic::info(path, line, column, rule, message),
        };
        self.diagnostics.push(
            diagnostic
                .with_span(end_line, end_column)
                .with_suggestion(suggestion),
        );
    }

    fn invalid(&mut self, span: (usize, usize), detail: impl AsRef<str>) {
        self.push(
            DiagnosticLevel::Error,
            "GM-CMD-001",
            span,
            t!("rules.gm_cmd_001.message", detail = detail.as_ref()),
            t!("rules.gm_cmd_001.suggestion"),
        );
    }

    fn check(&mut self) {
        let doc = match parse_toml_document(self.content) {
            Ok(doc) => doc,
            Err(error) => {
                let detail = t!(
                    "rules.gm_cmd_001.parse_error",
                    error = error.message.as_str()
                );
                self.invalid((error.offset, error.offset), detail);
                return;
            }
        };
        let table = doc.get_ref();
        let mut loads = true;

        match toml_entry(table, "prompt") {
            None => {
                self.invalid((0, 0), t!("rules.gm_cmd_001.missing_prompt"));
                loads = false;
            }
            Some((_, value)) => match value.get_ref() {
                DeValue::String(prompt) => self.check_injections(prompt, value.span()),
                other => {
                    let detail = t!(
                        "rules.gm_cmd_001.prompt_not_string",
                        found = other.type_str()
                    );
                    self.invalid((value.span().start, value.span().end), detail);
                    loads = false;
                }
            },
        }

        let description = toml_entry(table, "description");
        if let Some((_, value)) = description
            && !matches!(value.get_ref(), DeValue::String(_))
        {
            let detail = t!(
                "rules.gm_cmd_001.description_not_string",
                found = value.get_ref().type_str()
            );
            self.invalid((value.span().start, value.span().end), detail);
            return;
        }
        if !loads {
            // The file is skipped; display advice does not apply.
            return;
        }

        // GM-CMD-002: "`description` (String): A brief, one-line description
        // of what the command does."
        match description {
            None => self.advise((0, 0), t!("rules.gm_cmd_002.missing")),
            Some((_, value)) => {
                let DeValue::String(text) = value.get_ref() else {
                    return;
                };
                let span = (value.span().start, value.span().end);
                if text.trim().is_empty() {
                    self.advise(span, t!("rules.gm_cmd_002.missing"));
                } else if text.contains(['\n', '\r']) {
                    self.advise(span, t!("rules.gm_cmd_002.multiline"));
                } else {
                    let length = text.chars().count();
                    if length > DESCRIPTION_DISPLAY_LIMIT {
                        self.advise(
                            span,
                            t!(
                                "rules.gm_cmd_002.too_long",
                                length = length.to_string().as_str(),
                                limit = DESCRIPTION_DISPLAY_LIMIT.to_string().as_str()
                            ),
                        );
                    }
                }
            }
        }
    }

    fn advise(&mut self, span: (usize, usize), message: impl Into<String>) {
        self.push(
            DiagnosticLevel::Info,
            "GM-CMD-002",
            span,
            message,
            t!("rules.gm_cmd_002.suggestion"),
        );
    }

    /// GM-CMD-003: "The content inside `!{...}` must have balanced braces"
    /// and "The parser requires the content inside `@{...}` (the path) to
    /// have balanced braces".
    fn check_injections(&mut self, prompt: &str, value_span: std::ops::Range<usize>) {
        for trigger in TRIGGERS {
            let Some(start) = unclosed_block(prompt, trigger) else {
                continue;
            };
            let occurrence = prompt[..start].matches(trigger).count();
            let span =
                nth_occurrence_in_span(self.content, value_span.clone(), trigger, occurrence)
                    .map_or((value_span.start, value_span.end), |offset| {
                        (offset, offset + trigger.len())
                    });
            let kind = if trigger == "!{" {
                t!("rules.gm_cmd_003.shell")
            } else {
                t!("rules.gm_cmd_003.file")
            };
            self.push(
                DiagnosticLevel::Error,
                "GM-CMD-003",
                span,
                t!(
                    "rules.gm_cmd_003.message",
                    kind = kind.as_ref(),
                    trigger = trigger
                ),
                t!("rules.gm_cmd_003.suggestion"),
            );
        }
    }
}

/// The offset of the first `trigger` block whose braces never balance, as
/// `extractInjections` (prompt-processors/injectionParser.ts) scans it:
/// after the trigger the depth is 1, each `{` adds one, each `}` removes
/// one, and the block ends at depth 0. An unclosed block throws "Unclosed
/// injection starting at index ..."; it runs to the end of the prompt, so
/// there is at most one.
fn unclosed_block(prompt: &str, trigger: &str) -> Option<usize> {
    let bytes = prompt.as_bytes();
    let mut index = 0;
    while let Some(found) = prompt.get(index..)?.find(trigger) {
        let start = index + found;
        let mut depth = 1usize;
        let mut position = start + trigger.len();
        loop {
            match bytes.get(position) {
                None => return Some(start),
                Some(b'{') => depth += 1,
                Some(b'}') => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                Some(_) => {}
            }
            position += 1;
        }
        index = position + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;

    fn validate(content: &str) -> Vec<Diagnostic> {
        GeminiCommandValidator.validate(
            Path::new(".gemini/commands/test.toml"),
            content,
            &LintConfig::default(),
        )
    }

    fn rules(diagnostics: &[Diagnostic]) -> Vec<(&str, DiagnosticLevel)> {
        diagnostics
            .iter()
            .map(|d| (d.rule.as_str(), d.level))
            .collect()
    }

    #[test]
    fn documented_examples_are_clean() {
        // The custom-commands page examples.
        for content in [
            "description = \"Generates a fix for a given issue.\"\nprompt = \"Please provide a code fix for the issue described here: {{args}}.\"\n",
            "description = \"Greps.\"\nprompt = \"\"\"\nPlease summarize the findings for the pattern `{{args}}`.\n\nSearch Results:\n!{grep -r {{args}} .}\n\"\"\"\n",
            "description = \"Generates a Git commit message based on staged changes.\"\n\n# The prompt uses !{...}\nprompt = \"\"\"\nPlease generate a Conventional Commit message based on the following git diff:\n\n```diff\n!{git diff --staged}\n```\n\n\"\"\"\n",
            "description = \"Reviews the provided context using a best practice guide.\"\nprompt = \"\"\"\nYou are an expert code reviewer.\n\nYour task is to review {{args}}.\n\n@{docs/best-practices.md}\n\"\"\"\n",
            "description = \"JSON payload\"\nprompt = '''!{curl -d '{\"a\": {\"b\": 1}}' http://x}'''\n",
            // Extra keys are allowed: the schema is not strict.
            "prompt = \"hi\"\ndescription = \"Say hi.\"\nauthor = \"me\"\n",
        ] {
            assert!(
                validate(content).is_empty(),
                "{content}: {:?}",
                validate(content)
            );
        }
    }

    #[test]
    fn gm_cmd_001_parse_error_points_at_the_error() {
        let diagnostics = validate("description = \"x\"\nprompt = \n");
        assert_eq!(
            rules(&diagnostics),
            [("GM-CMD-001", DiagnosticLevel::Error)]
        );
        assert_eq!(diagnostics[0].line, 2);
        assert!(
            diagnostics[0].message.contains("skips"),
            "{}",
            diagnostics[0].message
        );
    }

    #[test]
    fn gm_cmd_001_missing_or_mistyped_fields() {
        let diagnostics = validate("description = \"x\"\n");
        assert_eq!(
            rules(&diagnostics),
            [("GM-CMD-001", DiagnosticLevel::Error)]
        );
        assert!(diagnostics[0].message.contains("'prompt'"));
        assert_eq!((diagnostics[0].line, diagnostics[0].column), (1, 1));

        let diagnostics = validate("description = \"x\"\nprompt = 42\n");
        assert_eq!(
            rules(&diagnostics),
            [("GM-CMD-001", DiagnosticLevel::Error)]
        );
        assert_eq!((diagnostics[0].line, diagnostics[0].column), (2, 10));
        assert!(diagnostics[0].message.contains("integer"));

        let diagnostics = validate("prompt = [\"a\"]\ndescription = true\n");
        assert_eq!(
            rules(&diagnostics),
            [
                ("GM-CMD-001", DiagnosticLevel::Error),
                ("GM-CMD-001", DiagnosticLevel::Error)
            ]
        );

        // A [prompt] table is not a string either.
        let diagnostics = validate("description = \"x\"\n[prompt]\ntext = \"hi\"\n");
        assert_eq!(
            rules(&diagnostics),
            [("GM-CMD-001", DiagnosticLevel::Error)]
        );
    }

    #[test]
    fn gm_cmd_002_description_advice() {
        let diagnostics = validate("prompt = \"hi\"\n");
        assert_eq!(rules(&diagnostics), [("GM-CMD-002", DiagnosticLevel::Info)]);
        assert!(diagnostics[0].message.contains("generic description"));

        let diagnostics = validate("prompt = \"hi\"\ndescription = \"\"\n");
        assert_eq!(rules(&diagnostics), [("GM-CMD-002", DiagnosticLevel::Info)]);

        let diagnostics = validate("prompt = \"hi\"\ndescription = \"\"\"\nTwo\nlines\"\"\"\n");
        assert_eq!(rules(&diagnostics), [("GM-CMD-002", DiagnosticLevel::Info)]);
        assert!(diagnostics[0].message.contains("one-line"));
        assert_eq!(diagnostics[0].line, 2);

        // 100 characters are shown in full; 101 are truncated. Characters,
        // not bytes: 100 two-byte characters fit.
        let exact = format!("prompt = \"hi\"\ndescription = \"{}\"\n", "a".repeat(100));
        assert!(validate(&exact).is_empty());
        let wide = format!("prompt = \"hi\"\ndescription = \"{}\"\n", "é".repeat(100));
        assert!(validate(&wide).is_empty());
        let long = format!("prompt = \"hi\"\ndescription = \"{}\"\n", "a".repeat(101));
        let diagnostics = validate(&long);
        assert_eq!(rules(&diagnostics), [("GM-CMD-002", DiagnosticLevel::Info)]);
        assert!(diagnostics[0].message.contains("101"));
    }

    #[test]
    fn gm_cmd_002_is_not_reported_for_files_that_do_not_load() {
        assert_eq!(
            rules(&validate("title = \"x\"\n")),
            [("GM-CMD-001", DiagnosticLevel::Error)]
        );
    }

    #[test]
    fn gm_cmd_003_unclosed_blocks_point_at_the_trigger() {
        let content = "description = \"x\"\nprompt = \"\"\"\nStatus:\n!{git status}\nBroken:\n!{awk '{print $1' file}\n\"\"\"\n";
        let diagnostics = validate(content);
        assert_eq!(
            rules(&diagnostics),
            [("GM-CMD-003", DiagnosticLevel::Error)]
        );
        assert_eq!((diagnostics[0].line, diagnostics[0].column), (6, 1));
        assert!(diagnostics[0].message.contains("!{"));

        let content = "description = \"x\"\nprompt = \"Read @{docs/{a.md and !{ls}\"\n";
        let diagnostics = validate(content);
        assert_eq!(
            rules(&diagnostics),
            [("GM-CMD-003", DiagnosticLevel::Error)]
        );
        assert_eq!((diagnostics[0].line, diagnostics[0].column), (2, 16));
        assert!(diagnostics[0].message.contains("@{"));
    }

    #[test]
    fn gm_cmd_003_escapes_before_the_block_keep_the_position() {
        // `\t` and `\"` change offsets between source and value.
        let content = "description = \"x\"\nprompt = \"a\\tb \\\"q\\\" !{echo {\"\n";
        let diagnostics = validate(content);
        assert_eq!(
            rules(&diagnostics),
            [("GM-CMD-003", DiagnosticLevel::Error)]
        );
        assert_eq!((diagnostics[0].line, diagnostics[0].column), (2, 22));
    }

    #[test]
    fn unclosed_block_scans_like_the_injection_parser() {
        assert_eq!(unclosed_block("!{ls}", "!{"), None);
        assert_eq!(unclosed_block("!{echo {a} {b}}", "!{"), None);
        assert_eq!(unclosed_block("a !{x} b !{y", "!{"), Some(9));
        assert_eq!(unclosed_block("!{echo {{args}}", "!{"), Some(0));
        // An extra `}` ends the block early; the rest is plain text.
        assert_eq!(unclosed_block("!{echo }} done", "!{"), None);
        assert_eq!(unclosed_block("no blocks {here", "!{"), None);
        assert_eq!(unclosed_block("@{a} @{b", "@{"), Some(5));
    }

    #[test]
    fn rules_can_be_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["GM-CMD-002".to_string()];
        let diagnostics = GeminiCommandValidator.validate(
            Path::new(".gemini/commands/test.toml"),
            "prompt = \"hi\"\n",
            &config,
        );
        assert!(diagnostics.is_empty());
    }
}
