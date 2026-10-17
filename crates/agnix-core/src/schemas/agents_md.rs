//! AGENTS.md validation schema helpers
//!
//! Provides detection functions for:
//! - AGM-001: Valid Markdown Structure (unclosed code blocks, malformed links)
//! - AGM-002: Missing Section Headers
//! - AGM-003: Character Limit (Windsurf's 12,000-character rule-file limit)
//! - AGM-004: Missing Project Context
//! - AGM-007: Root AGENTS.md lists no build or test command

use regex::Regex;

use crate::regex_util::static_regex;

static_regex!(fn link_pattern, r"\[([^\]]*)\](?:\(([^)]*)\)?|\[([^\]]*)\]?)");
static_regex!(fn markdown_header_pattern, r"^#+\s+.+");
static_regex!(fn project_context_pattern, r"(?im)^#+\s*(project|overview|about|description|introduction|summary|this\s+(project|repository|repo))\b");

// ============================================================================
// AGM-001: Valid Markdown Structure
// ============================================================================

/// Markdown validity issue found in content
#[derive(Debug, Clone)]
pub struct MarkdownValidityIssue {
    pub line: usize,
    pub column: usize,
    pub issue_type: MarkdownIssueType,
    pub description: String,
    /// For an unclosed code block, the opening fence (for example "```" or
    /// "~~~~"), so a fix can close it with a matching fence. Empty otherwise.
    pub fence: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MarkdownIssueType {
    UnclosedCodeBlock,
    MalformedLink,
}

/// A fenced code block marker per CommonMark ("Fenced code blocks"): at most
/// three spaces of indentation, then three or more backticks or tildes. A
/// backtick fence's info string may not contain a backtick.
fn fence_marker(line: &str) -> Option<(u8, usize, &str)> {
    let rest = line.trim_start_matches(' ');
    if line.len() - rest.len() > 3 {
        return None;
    }
    let ch = *rest.as_bytes().first()?;
    if ch != b'`' && ch != b'~' {
        return None;
    }
    let len = rest.bytes().take_while(|&b| b == ch).count();
    if len < 3 {
        return None;
    }
    let info = &rest[len..];
    if ch == b'`' && info.contains('`') {
        return None;
    }
    Some((ch, len, info))
}

/// Check markdown validity (for AGM-001)
///
/// Detects:
/// - Unclosed fenced code blocks. CommonMark defines them: the block runs to
///   the end of the document, so everything after the opening fence is read
///   as code. Fences follow CommonMark: backticks or tildes, at least three,
///   closed by a fence of the same character that is at least as long and has
///   no info string.
/// - Malformed links ([text]( without closing)
pub fn check_markdown_validity(content: &str) -> Vec<MarkdownValidityIssue> {
    let mut results = Vec::new();
    let link_pattern = link_pattern();

    // Open fence: character, length and 1-based opening line.
    let mut open_fence: Option<(u8, usize, usize)> = None;

    for (line_num, line) in content.lines().enumerate() {
        if let Some((ch, len, info)) = fence_marker(line) {
            match open_fence {
                Some((open_ch, open_len, _))
                    if ch == open_ch && len >= open_len && info.trim().is_empty() =>
                {
                    open_fence = None;
                    continue;
                }
                Some(_) => {}
                None => {
                    open_fence = Some((ch, len, line_num + 1));
                    continue;
                }
            }
        }

        // Check for malformed links (only outside code blocks)
        if open_fence.is_none() {
            // Look for [text]( patterns that don't close properly
            for mat in link_pattern.find_iter(line) {
                let matched_str = mat.as_str();
                // Check if link is properly closed
                if matched_str.contains("](") && !matched_str.ends_with(')') {
                    // Verify it's actually malformed (no closing paren on same line after match)
                    let after_match = &line[mat.end()..];
                    if !after_match.starts_with(')') && !matched_str.ends_with(')') {
                        // Check if this is actually incomplete
                        let has_close_paren =
                            matched_str.matches('(').count() == matched_str.matches(')').count();
                        if !has_close_paren {
                            results.push(MarkdownValidityIssue {
                                line: line_num + 1,
                                column: mat.start() + 1,
                                issue_type: MarkdownIssueType::MalformedLink,
                                description:
                                    "Malformed markdown link (missing closing parenthesis)"
                                        .to_string(),
                                fence: String::new(),
                            });
                        }
                    }
                }
                // Check for [text][ without closing bracket
                if matched_str.contains("][") && !matched_str.ends_with(']') {
                    let has_close_bracket =
                        matched_str.matches('[').count() == matched_str.matches(']').count();
                    if !has_close_bracket {
                        results.push(MarkdownValidityIssue {
                            line: line_num + 1,
                            column: mat.start() + 1,
                            issue_type: MarkdownIssueType::MalformedLink,
                            description:
                                "Malformed markdown link reference (missing closing bracket)"
                                    .to_string(),
                            fence: String::new(),
                        });
                    }
                }
            }
        }
    }

    // Check if code block was never closed
    if let Some((ch, len, start_line)) = open_fence {
        results.push(MarkdownValidityIssue {
            line: start_line,
            column: 0,
            issue_type: MarkdownIssueType::UnclosedCodeBlock,
            description: "Unclosed code block (missing closing ```)".to_string(),
            fence: String::from(ch as char).repeat(len),
        });
    }

    results
}

// ============================================================================
// AGM-002: Missing Section Headers
// ============================================================================

/// Section header issue
#[derive(Debug, Clone)]
pub struct SectionHeaderIssue {
    pub line: usize,
    pub column: usize,
    /// English text. AGM-002/AGM-004 read their messages from the locale
    /// files; kept for callers that want a plain description.
    #[allow(dead_code)]
    pub description: String,
    #[allow(dead_code)]
    pub suggestion: String,
}

/// Check for missing section headers (for AGM-002)
///
/// AGENTS.md should have clear section headers for organization
pub fn check_section_headers(content: &str) -> Option<SectionHeaderIssue> {
    let pattern = markdown_header_pattern();

    // Skip empty or whitespace-only content
    if content.trim().is_empty() {
        return None;
    }

    // Check if file has any headers at all
    let has_headers = content.lines().any(|line| pattern.is_match(line));

    if !has_headers {
        Some(SectionHeaderIssue {
            line: 1,
            column: 0,
            description: "No markdown headers found in AGENTS instruction file".to_string(),
            suggestion: "Add section headers (# Title, ## Section) for better organization"
                .to_string(),
        })
    } else {
        None
    }
}

/// Number of non-blank lines above which AGM-002 reports a file without
/// headings. agnix heuristic: agents.md makes headings optional ("Use any
/// headings you like"), and a short file needs none.
pub const AGM_002_MIN_LINES: usize = 50;

/// Count the non-blank lines of `content`.
pub fn non_blank_line_count(content: &str) -> usize {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

// ============================================================================
// AGM-003: Character Limit
// ============================================================================

/// Character limit exceeded result
#[derive(Debug, Clone)]
pub struct CharacterLimitExceeded {
    pub char_count: usize,
    pub limit: usize,
}

/// Windsurf's limit for a workspace rule file, in characters ("Limited to
/// 12,000 characters per file", docs.devin.ai/desktop/cascade/memories).
pub const WINDSURF_CHAR_LIMIT: usize = 12000;

/// Check if content exceeds character limit (for AGM-003)
///
/// Counts characters (Unicode scalar values), the unit Windsurf states, not
/// UTF-8 bytes: a 7,000-character CJK file is about 21,000 bytes.
pub fn check_character_limit(content: &str, limit: usize) -> Option<CharacterLimitExceeded> {
    let char_count = content.chars().count();

    if char_count > limit {
        Some(CharacterLimitExceeded { char_count, limit })
    } else {
        None
    }
}

// ============================================================================
// AGM-004: Missing Project Context
// ============================================================================

/// Missing project context result
#[derive(Debug, Clone)]
pub struct MissingProjectContext {
    pub line: usize,
    pub column: usize,
    /// English text. AGM-002/AGM-004 read their messages from the locale
    /// files; kept for callers that want a plain description.
    #[allow(dead_code)]
    pub description: String,
    #[allow(dead_code)]
    pub suggestion: String,
}

/// Check for missing project context (for AGM-004)
///
/// AGENTS.md should describe project purpose/stack
pub fn check_project_context(content: &str) -> Option<MissingProjectContext> {
    let pattern = project_context_pattern();

    // Skip empty content
    if content.trim().is_empty() {
        return None;
    }

    // Check for project context section
    let has_project_context = pattern.is_match(content);

    // Also check for common project description patterns in content
    let content_lower = content.to_lowercase();
    let has_project_description = content_lower.contains("this project")
        || content_lower.contains("this repository")
        || content_lower.contains("this repo")
        || content_lower.contains("the project")
        || content_lower.contains("# project");

    if !has_project_context && !has_project_description {
        Some(MissingProjectContext {
            line: 1,
            column: 0,
            description: "Missing project context section in AGENTS instruction file".to_string(),
            suggestion:
                "Add a '# Project' or '## Overview' section describing the project purpose and tech stack"
                    .to_string(),
        })
    } else {
        None
    }
}

// ============================================================================
// AGM-007: Root AGENTS.md Lists No Build or Test Command
// ============================================================================

static_regex!(fn command_heading_pattern, r"(?im)^\s{0,3}#{1,6}\s+.*\b(build|test|tests|testing|setup|commands?|development|dev\s+environment|running|lint|linting|scripts|checks|verification|ci)\b");
static_regex!(fn command_runner_pattern, r"(?i)\b(?:(?:npm|pnpm|yarn|bun|npx|bunx)\s+(?:run\s+)?(?:test|build|lint|check|typecheck|format|ci|verify)\b|cargo\s+(?:test|build|check|clippy|nextest|fmt)\b|go\s+(?:test|build|vet)\b|pytest\b|python3?\s+-m\s+(?:pytest|unittest)\b|gradlew?\s+\w|mvn\s+\w|dotnet\s+(?:test|build)\b|uv\s+run\b|bundle\s+exec\b|brazil-build\b|bazel\s+(?:test|build)\b|ctest\b|mix\s+test\b|swift\s+(?:test|build)\b|phpunit\b|composer\s+(?:test|run)\b|hatch\s+(?:test|run)\b|poetry\s+run\b|turbo\s+run\b|nx\s+(?:test|build|run)\b)");
// Runner names that are also English words count only inside code.
static_regex!(fn code_runner_pattern, r"(?m)(?:^|[\s`$])(?:make|just|tox|nox|rake|cmake|deno\s+(?:test|task))(?:\s|$|`)");
static_regex!(fn command_doc_reference_pattern, r"(?i)(?:\]\(|@)[^)\s]*(?:contributing|development|developing|commands|building|testing|makefile|justfile|package\.json)");

/// Whether an AGENTS.md lists a build, test or setup command (for AGM-007).
///
/// Satisfied by a heading about commands (Build, Test, Setup, Commands,
/// Development, ...), a common runner command anywhere in the text, or a link
/// or @-mention to a file that usually holds them (CONTRIBUTING.md,
/// DEVELOPMENT.md, a Makefile, package.json). agents.md lists "Build and test
/// commands" among the popular sections and says agents run the checks a file
/// lists.
pub fn lists_build_or_test_commands(content: &str) -> bool {
    command_heading_pattern().is_match(content)
        || command_runner_pattern().is_match(content)
        || command_doc_reference_pattern().is_match(content)
        || code_runner_pattern().is_match(&code_text(content))
}

/// The code spans and fences of `content`, everything else blanked.
fn code_text(content: &str) -> String {
    let masked = crate::parsers::markdown::mask_code_spans(content);
    let bytes: Vec<u8> = content
        .bytes()
        .zip(masked.bytes())
        .map(|(orig, mask)| {
            if orig == b'\n' || orig != mask {
                orig
            } else {
                b' '
            }
        })
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_patterns_compile() {
        let _ = link_pattern();
        let _ = markdown_header_pattern();
        let _ = project_context_pattern();
        let _ = command_heading_pattern();
        let _ = command_runner_pattern();
        let _ = code_runner_pattern();
        let _ = command_doc_reference_pattern();
    }

    // ===== AGM-001: Valid Markdown Structure =====

    #[test]
    fn test_unclosed_code_block() {
        let content = r#"# Example
```rust
fn main() {}
"#;
        let results = check_markdown_validity(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].issue_type, MarkdownIssueType::UnclosedCodeBlock);
    }

    #[test]
    fn test_closed_code_block() {
        let content = r#"# Example
```rust
fn main() {}
```
"#;
        let results = check_markdown_validity(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_multiple_code_blocks_valid() {
        let content = r#"# Examples
```rust
fn main() {}
```

```python
print("hello")
```
"#;
        let results = check_markdown_validity(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_malformed_link_missing_paren() {
        let content = r#"Check [this link](http://example.com for more info."#;
        let results = check_markdown_validity(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].issue_type, MarkdownIssueType::MalformedLink);
    }

    #[test]
    fn test_valid_link() {
        let content = r#"Check [this link](http://example.com) for more info."#;
        let results = check_markdown_validity(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_code_block_in_link_check_ignored() {
        // Links inside code blocks shouldn't be validated
        let content = r#"```markdown
[broken link](no closing
```
"#;
        let results = check_markdown_validity(content);
        // No errors because the "malformed link" is inside a code block
        assert!(results.is_empty());
    }

    // ===== AGM-002: Missing Section Headers =====

    #[test]
    fn test_no_headers() {
        let content = "Just plain text without any headers.";
        let result = check_section_headers(content);
        assert!(result.is_some());
        assert!(result.unwrap().description.contains("No markdown headers"));
    }

    #[test]
    fn test_has_headers() {
        let content = "# Main Title\n\nSome content here.";
        let result = check_section_headers(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_empty_content_no_header_issue() {
        let content = "";
        let result = check_section_headers(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_whitespace_only_no_header_issue() {
        let content = "   \n\n   ";
        let result = check_section_headers(content);
        assert!(result.is_none());
    }

    // ===== AGM-003: Character Limit =====

    #[test]
    fn test_under_char_limit() {
        let content = "x".repeat(11000);
        let result = check_character_limit(&content, WINDSURF_CHAR_LIMIT);
        assert!(result.is_none());
    }

    #[test]
    fn test_over_char_limit() {
        let content = "x".repeat(13000);
        let result = check_character_limit(&content, WINDSURF_CHAR_LIMIT);
        assert!(result.is_some());
        let exceeded = result.unwrap();
        assert_eq!(exceeded.char_count, 13000);
        assert_eq!(exceeded.limit, WINDSURF_CHAR_LIMIT);
    }

    #[test]
    fn test_exact_char_limit() {
        let content = "x".repeat(12000);
        let result = check_character_limit(&content, WINDSURF_CHAR_LIMIT);
        assert!(result.is_none());
    }

    // ===== AGM-004: Missing Project Context =====

    #[test]
    fn test_missing_project_context() {
        let content = r#"# Build Commands
Run npm install and npm build.
"#;
        let result = check_project_context(content);
        assert!(result.is_some());
        assert!(
            result
                .unwrap()
                .description
                .contains("Missing project context")
        );
    }

    #[test]
    fn test_has_project_header() {
        let content = r#"# Project
This is a linter for agent configurations.

## Commands
Run npm test.
"#;
        let result = check_project_context(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_has_overview_header() {
        let content = r#"## Overview
A comprehensive validation tool.

## Usage
Run the CLI.
"#;
        let result = check_project_context(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_has_project_mention_in_content() {
        let content = r#"# Guidelines
This project validates agent configurations.

## Build
Use cargo build.
"#;
        let result = check_project_context(content);
        assert!(result.is_none());
    }

    // ===== AGM-001: CommonMark fences =====

    #[test]
    fn test_tilde_and_long_fences_follow_commonmark() {
        // A ``` line inside a four-backtick fence does not close it, and a
        // tilde fence closes only on tildes.
        let content = "````markdown\n```bash\nls\n```\n````\n\n~~~\ncode\n~~~\n";
        assert!(check_markdown_validity(content).is_empty());

        let unclosed = "# T\n\n~~~~\ncode\n~~~\n";
        let results = check_markdown_validity(unclosed);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line, 3);
        assert_eq!(results[0].fence, "~~~~");
    }

    #[test]
    fn test_closing_fence_cannot_carry_info_string() {
        // "```rust" inside an open block is content, not a closing fence.
        let content = "```\nfirst\n```rust\nsecond\n";
        let results = check_markdown_validity(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].issue_type, MarkdownIssueType::UnclosedCodeBlock);
        assert_eq!(results[0].line, 1);
    }

    #[test]
    fn test_indented_four_spaces_is_not_a_fence() {
        let content = "    ```\nindented code\n";
        assert!(check_markdown_validity(content).is_empty());
    }

    // ===== AGM-002: size threshold =====

    #[test]
    fn test_non_blank_line_count() {
        assert_eq!(non_blank_line_count("a\n\n  \nb\n"), 2);
    }

    // ===== AGM-003: characters, not bytes =====

    #[test]
    fn test_char_limit_counts_characters_not_bytes() {
        // 7,029 CJK characters are 21,087 UTF-8 bytes; Windsurf counts characters.
        let content = "漢".repeat(7029);
        assert!(content.len() > WINDSURF_CHAR_LIMIT);
        assert!(check_character_limit(&content, WINDSURF_CHAR_LIMIT).is_none());
        let over = "漢".repeat(12_001);
        assert_eq!(
            check_character_limit(&over, WINDSURF_CHAR_LIMIT).map(|e| e.char_count),
            Some(12_001)
        );
    }

    // ===== AGM-007: build and test commands =====

    #[test]
    fn test_lists_build_or_test_commands() {
        assert!(lists_build_or_test_commands(
            "# P\n\n## Testing\n\nRun the suite.\n"
        ));
        assert!(lists_build_or_test_commands(
            "# P\n\nUse `cargo test` before pushing.\n"
        ));
        assert!(lists_build_or_test_commands(
            "# P\n\n```\nnpm run lint\n```\n"
        ));
        assert!(lists_build_or_test_commands(
            "# P\n\nSee [contributing](CONTRIBUTING.md).\n"
        ));
        assert!(lists_build_or_test_commands(
            "# P\n\n@docs/development.md\n"
        ));
        assert!(!lists_build_or_test_commands(
            "# Project\n\nA library for parsing dates.\n\n## Style\n\nUse tabs.\n"
        ));
        // `make` and `just` are commands only in code; in prose they are words.
        assert!(!lists_build_or_test_commands(
            "Just plain text, which makes it harder to read.\n"
        ));
        assert!(lists_build_or_test_commands("Run `make check` first.\n"));
        assert!(lists_build_or_test_commands("```\njust test\n```\n"));
    }
}
