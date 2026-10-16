//! CLAUDE.md validation rules

use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

use crate::regex_util::static_regex;

static GENERIC_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

static_regex!(fn weak_language_pattern, r"(?i)\b(should|try\s+to|consider|maybe|might\s+want\s+to|could|possibly)\b");
static_regex!(fn critical_section_pattern, r"(?i)^#+\s*.*(critical|important|required|mandatory|rules|must|essential)");
static_regex!(fn npm_run_pattern, r"npm\s+run\s+([a-zA-Z0-9_:-]+)");

/// Generic instruction patterns that Claude already knows
pub fn generic_patterns() -> &'static Vec<Regex> {
    GENERIC_PATTERNS.get_or_init(|| {
        const PATTERNS: &[&str] = &[
            r"(?i)\bbe\s+helpful",
            r"(?i)\bbe\s+accurate",
            r"(?i)\bthink\s+step\s+by\s+step",
            r"(?i)\bbe\s+concise",
            r"(?i)\bformat.*properly",
            r"(?i)\bprovide.*clear.*explanations",
            r"(?i)\bmake\s+sure\s+to",
            r"(?i)\balways\s+be",
            // Role-play identity preambles
            r"(?i)^#+\s*(?:you\s+are|your\s+role)\b",
            r"(?i)^\s*-?\s*you(?:'re|\s+are)\s+a\s+(?:helpful|expert|senior|skilled|experienced)\b",
            // Generic programming principles without project context
            r"(?i)\bfollow\s+(?:best\s+practices|coding\s+standards|clean\s+code)\b",
            r"(?i)\bwrite\s+clean\s+(?:and\s+)?(?:maintainable|readable)\s+code\b",
        ];
        PATTERNS
            .iter()
            .map(|&p| Regex::new(p).unwrap_or_else(|e| panic!("BUG: invalid regex '{}': {}", p, e)))
            .collect()
    })
}

/// Count generic instruction patterns, once per pattern per line.
///
/// Codex uses the count to assess specificity; repeated occurrences of the same
/// pattern on one line still count only once.
pub fn count_generic_instructions(content: &str) -> usize {
    let patterns = generic_patterns();
    content
        .lines()
        .map(|line| {
            patterns
                .iter()
                .filter(|pattern| pattern.is_match(line))
                .count()
        })
        .sum()
}

// ============================================================================
// CC-MEM-009: Token Count Exceeded
// ============================================================================

/// Result when token count exceeds limit
#[derive(Debug, Clone)]
pub struct TokenCountExceeded {
    #[allow(dead_code)] // parsed but not yet consumed by validators
    pub char_count: usize,
    pub estimated_tokens: usize,
    pub limit: usize,
}

/// Check if content exceeds token limit (~1500 tokens = ~6000 chars)
/// Returns Some if exceeded, None if within limit
pub fn check_token_count(content: &str) -> Option<TokenCountExceeded> {
    let char_count = content.chars().count();
    let estimated_tokens = char_count / 4; // Rough approximation: 4 chars per token
    let limit = 1500;

    if estimated_tokens > limit {
        Some(TokenCountExceeded {
            char_count,
            estimated_tokens,
            limit,
        })
    } else {
        None
    }
}

// ============================================================================
// CC-MEM-007: Weak Constraint Language
// ============================================================================

#[derive(Debug, Clone)]
pub struct WeakConstraint {
    pub line: usize,
    pub column: usize,
    pub text: String,
    pub section: String,
}

/// Find weak constraint language in critical sections
pub fn find_weak_constraints(content: &str) -> Vec<WeakConstraint> {
    let mut results = Vec::new();
    let weak_pattern = weak_language_pattern();
    let section_pattern = critical_section_pattern();

    let mut current_section: Option<String> = None;

    for (line_num, line) in content.lines().enumerate() {
        // Check if this is a header line
        if line.starts_with('#') {
            if section_pattern.is_match(line) {
                current_section = Some(line.trim_start_matches('#').trim().to_string());
            } else {
                // New non-critical header ends the critical section
                current_section = None;
            }
        }

        // Check for weak language in critical sections
        if let Some(section_name) = &current_section {
            if let Some(mat) = weak_pattern.find(line) {
                results.push(WeakConstraint {
                    line: line_num + 1,
                    column: mat.start() + 1,
                    text: mat.as_str().to_string(),
                    section: section_name.clone(),
                });
            }
        }
    }

    results
}

// ============================================================================
// CC-MEM-004: Invalid npm Script Reference
// ============================================================================

#[derive(Debug, Clone)]
pub struct NpmScriptReference {
    pub line: usize,
    pub column: usize,
    pub script_name: String,
}

/// Extract npm script references from content
pub fn extract_npm_scripts(content: &str) -> Vec<NpmScriptReference> {
    let mut results = Vec::new();
    let pattern = npm_run_pattern();

    for (line_num, line) in content.lines().enumerate() {
        for cap in pattern.captures_iter(line) {
            if let Some(script_match) = cap.get(1) {
                results.push(NpmScriptReference {
                    line: line_num + 1,
                    column: cap.get(0).map(|m| m.start() + 1).unwrap_or(1),
                    script_name: script_match.as_str().to_string(),
                });
            }
        }
    }

    results
}

// ============================================================================
// CC-MEM-010: README Duplication
// ============================================================================

/// Calculate text overlap between two texts as a percentage (0.0 - 1.0)
/// Uses word-set Jaccard similarity
pub fn calculate_text_overlap(text1: &str, text2: &str) -> f64 {
    // Normalize and extract words
    let text1_lower = text1.to_lowercase();
    let text2_lower = text2.to_lowercase();

    let words1: HashSet<&str> = text1_lower
        .split_whitespace()
        .filter(|w| w.len() > 3) // Skip short words
        .collect();

    let words2: HashSet<&str> = text2_lower
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .collect();

    if words1.is_empty() || words2.is_empty() {
        return 0.0;
    }

    // Jaccard similarity: intersection / union
    let intersection = words1.intersection(&words2).count();
    let union = words1.union(&words2).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Result when README duplication is detected
#[derive(Debug, Clone)]
pub struct ReadmeDuplication {
    pub overlap_percent: f64,
    pub threshold: f64,
}

/// Check if content duplicates README beyond threshold
pub fn check_readme_duplication(claude_md: &str, readme: &str) -> Option<ReadmeDuplication> {
    let overlap = calculate_text_overlap(claude_md, readme);
    let threshold = 0.40; // 40% overlap threshold

    if overlap > threshold {
        Some(ReadmeDuplication {
            overlap_percent: overlap * 100.0,
            threshold: threshold * 100.0,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_patterns_compile() {
        let _ = generic_patterns();
        let _ = weak_language_pattern();
        let _ = critical_section_pattern();
        let _ = npm_run_pattern();
    }

    #[test]
    fn test_count_generic_instructions() {
        assert_eq!(count_generic_instructions("Be helpful and be accurate."), 2);
        assert_eq!(count_generic_instructions("Be helpful and be helpful."), 1);
        assert_eq!(count_generic_instructions("Be helpful.\nBe helpful."), 2);
        assert_eq!(count_generic_instructions("Be helpful.\r\nBe helpful."), 2);
        assert_eq!(count_generic_instructions(""), 0);
    }

    #[test]
    fn test_detect_generic_preambles_and_principles() {
        for content in [
            "# You Are an Expert\nDo this specific task.",
            "You are a helpful assistant for this project.",
            "Follow best practices when writing code.",
            "Write clean and maintainable code.",
        ] {
            assert_eq!(count_generic_instructions(content), 1, "{content}");
        }
    }

    #[test]
    fn test_no_false_positive_specific_instructions() {
        for content in [
            "Use the coding style defined in .editorconfig\nFollow team conventions",
            "You are a developer who writes Rust code.",
            "Follow the airbnb style guide for JavaScript.",
        ] {
            assert_eq!(count_generic_instructions(content), 0, "{content}");
        }
    }

    // CC-MEM-009 tests
    #[test]
    fn test_check_token_count_under_limit() {
        let content = "Short content that is well under the limit.";
        assert!(check_token_count(content).is_none());
    }

    #[test]
    fn test_check_token_count_over_limit() {
        // Create content > 6000 chars (1500 tokens * 4 chars/token)
        let content = "x".repeat(6100);
        let result = check_token_count(&content);
        assert!(result.is_some());
        let exceeded = result.unwrap();
        assert!(exceeded.estimated_tokens > 1500);
        assert_eq!(exceeded.limit, 1500);
    }

    // CC-MEM-007 tests
    #[test]
    fn test_find_weak_constraints_in_critical() {
        let content = "# Critical Rules\n\nYou should follow the coding style.";
        let results = find_weak_constraints(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text.to_lowercase(), "should");
    }

    #[test]
    fn test_find_weak_constraints_outside_critical() {
        let content = "# General Guidelines\n\nYou should follow the coding style.";
        let results = find_weak_constraints(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_weak_constraints_section_ends() {
        let content =
            "# Critical Rules\n\nMust follow style.\n\n# Other\n\nYou should do this too.";
        let results = find_weak_constraints(content);
        // "should" is in non-critical section, so no results
        assert!(results.is_empty());
    }

    // CC-MEM-004 tests
    #[test]
    fn test_extract_npm_scripts() {
        let content = "Run tests with npm run test\nBuild with npm run build";
        let results = extract_npm_scripts(content);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].script_name, "test");
        assert_eq!(results[1].script_name, "build");
    }

    #[test]
    fn test_extract_npm_scripts_with_colon() {
        let content = "Run npm run test:unit for unit tests";
        let results = extract_npm_scripts(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].script_name, "test:unit");
    }

    #[test]
    fn test_no_npm_scripts() {
        let content = "Use cargo test for testing.";
        let results = extract_npm_scripts(content);
        assert!(results.is_empty());
    }

    // CC-MEM-010 tests
    #[test]
    fn test_calculate_text_overlap_identical() {
        let text = "This is some sample text with enough words to test overlap calculation.";
        let overlap = calculate_text_overlap(text, text);
        assert!((overlap - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_text_overlap_different() {
        let text1 = "This project uses Rust for performance.";
        let text2 = "Python is great for machine learning.";
        let overlap = calculate_text_overlap(text1, text2);
        assert!(overlap < 0.3);
    }

    #[test]
    fn test_check_readme_duplication_detected() {
        let claude_md =
            "This is a project about Rust validation. It validates agent configurations.";
        let readme = "This is a project about Rust validation. It validates agent configurations.";
        let result = check_readme_duplication(claude_md, readme);
        assert!(result.is_some());
    }

    #[test]
    fn test_check_readme_duplication_not_detected() {
        let claude_md = "Project-specific instructions for Claude. Focus on these guidelines.";
        let readme = "Welcome to the project. Installation: npm install. Usage: npm start.";
        let result = check_readme_duplication(claude_md, readme);
        assert!(result.is_none());
    }

    #[test]
    fn test_weak_constraint_locations() {
        for newline in ["\n", "\r\n"] {
            let content = [
                "# Critical Rules",
                "",
                "You should do this.",
                "Try to do that.",
            ]
            .join(newline);
            let results = find_weak_constraints(&content);
            assert_eq!(results.len(), 2);
            assert_eq!(results[0].line, 3);
            assert_eq!(results[0].column, 5);
            assert_eq!(results[0].text.to_lowercase(), "should");
            assert_eq!(results[0].section, "Critical Rules");
            assert_eq!(results[1].line, 4);
            assert_eq!(results[1].column, 1);
            assert_eq!(results[1].text.to_lowercase(), "try to");
        }
    }

    #[test]
    fn test_weak_constraint_multiple_on_same_line() {
        let results = find_weak_constraints("# Critical Rules\nYou should consider doing this.");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text.to_lowercase(), "should");
    }
}
