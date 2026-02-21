//! YAML frontmatter parser
//!
//! ## Security: YAML Bomb Protection
//!
//! While this module doesn't implement explicit depth limits, YAML bombs (deeply
//! nested structures) are mitigated by:
//!
//! 1. **File Size Limit**: DEFAULT_MAX_FILE_SIZE (1 MiB) in file_utils.rs prevents
//!    extremely large YAML payloads from being read.
//!
//! 2. **Parser Library**: `serde_yaml` has internal protections against excessive
//!    memory usage and stack overflow from deeply nested structures.
//!
//! 3. **Memory Limit**: The entire file is bounded at 1 MiB, limiting total
//!    memory consumption regardless of structure complexity.
//!
//! **Known Limitation**: Within the 1 MiB file size, deeply nested YAML (e.g.,
//! 10,000 levels of nesting) could cause high memory usage or slow parsing.
//! This is acceptable for a local linter with bounded input size.
//!
//! **Future Enhancement**: Consider adding explicit depth tracking if memory
//! profiling reveals issues with pathological YAML structures.

use std::borrow::Cow;

use crate::diagnostics::{CoreError, LintResult, ValidationError};
use serde::de::DeserializeOwned;

/// Normalize CRLF (`\r\n`) and lone CR (`\r`) line endings to LF (`\n`).
///
/// Returns `Cow::Borrowed` (zero allocation) when no `\r` is present.
/// When normalization is needed, uses a single-pass scan to avoid the double
/// allocation that would result from two sequential `replace` calls.
#[inline]
pub fn normalize_line_endings(s: &str) -> Cow<'_, str> {
    if !s.contains('\r') {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\r' {
            // Consume a following '\n' so that \r\n becomes a single \n.
            chars.next_if_eq(&'\n');
            out.push('\n');
        } else {
            out.push(ch);
        }
    }
    Cow::Owned(out)
}

/// Parse YAML frontmatter from markdown content
///
/// Expects content in format:
/// ```markdown
/// ---
/// key: value
/// ---
/// body content
/// ```
///
/// # Security
///
/// Protected against YAML bombs by file size limit (1 MiB) and serde_yaml's
/// internal protections. See module documentation for details.
#[allow(dead_code)] // used in cfg(test) and __internal; not yet used by production validators
pub fn parse_frontmatter<T: DeserializeOwned>(content: &str) -> LintResult<(T, String)> {
    let parts = split_frontmatter(content);
    let parsed: T = serde_yaml::from_str(&parts.frontmatter)
        .map_err(|e| CoreError::Validation(ValidationError::Other(e.into())))?;
    Ok((parsed, parts.body.trim_start().to_string()))
}

/// Extract frontmatter and body from content with offsets.
#[derive(Debug, Clone)]
pub struct FrontmatterParts {
    pub has_frontmatter: bool,
    pub has_closing: bool,
    pub frontmatter: String,
    pub body: String,
    pub frontmatter_start: usize,
    pub body_start: usize,
}

/// Split frontmatter and body from content.
pub fn split_frontmatter(content: &str) -> FrontmatterParts {
    let trimmed = content.trim_start();
    let trim_offset = content.len() - trimmed.len();

    // Check for opening ---
    if !trimmed.starts_with("---") {
        return FrontmatterParts {
            has_frontmatter: false,
            has_closing: false,
            frontmatter: String::new(),
            body: trimmed.to_string(),
            frontmatter_start: trim_offset,
            body_start: trim_offset,
        };
    }

    let rest = &trimmed[3..];

    // Skip the newline that follows the opening --- delimiter so that
    // the extracted frontmatter starts at the first content character.
    let newline_len = if rest.starts_with("\r\n") {
        2
    } else if rest.starts_with('\n') {
        1
    } else {
        0
    };

    let frontmatter_start = trim_offset + 3 + newline_len;

    // Find closing ---
    if let Some(end_pos) = rest.find("\n---") {
        let frontmatter = rest.get(newline_len..end_pos).unwrap_or("");
        let body = &rest[end_pos + 4..]; // Skip \n---
        FrontmatterParts {
            has_frontmatter: true,
            has_closing: true,
            frontmatter: frontmatter.to_string(),
            body: body.to_string(),
            frontmatter_start,
            // end_pos is relative to `rest` (= trimmed[trim_offset+3..]), so body_start
            // does not include newline_len - it accounts for the full \n--- (4 bytes).
            body_start: trim_offset + 3 + end_pos + 4,
        }
    } else {
        // No closing marker - treat entire file as body
        let body = &rest[newline_len..];
        FrontmatterParts {
            has_frontmatter: true,
            has_closing: false,
            frontmatter: String::new(),
            body: body.to_string(),
            frontmatter_start,
            body_start: frontmatter_start,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestFrontmatter {
        name: String,
        description: String,
    }

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
name: test-skill
description: A test skill
---
Body content here"#;

        let (fm, body): (TestFrontmatter, String) = parse_frontmatter(content).unwrap();
        assert_eq!(fm.name, "test-skill");
        assert_eq!(fm.description, "A test skill");
        assert_eq!(body, "Body content here");
    }

    #[test]
    fn test_no_frontmatter() {
        let content = "Just body content";
        let result: LintResult<(TestFrontmatter, String)> = parse_frontmatter(content);
        assert!(result.is_err()); // Should fail to deserialize empty frontmatter
    }

    #[test]
    fn test_split_frontmatter_basic() {
        let content = "---\nname: test\n---\nbody";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
        assert_eq!(parts.frontmatter, "name: test");
        assert_eq!(parts.body, "\nbody");
        // frontmatter_start points past "---\n" (4 bytes)
        assert_eq!(parts.frontmatter_start, 4);
        assert_eq!(&content[parts.body_start..], parts.body);
    }

    #[test]
    fn test_split_frontmatter_no_closing() {
        let content = "---\nname: test";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(!parts.has_closing);
        assert!(parts.frontmatter.is_empty());
        assert_eq!(parts.body, "name: test");
        assert_eq!(parts.body_start, 4); // past ---\n
        assert_eq!(&content[parts.body_start..], parts.body);
    }

    #[test]
    fn test_split_frontmatter_no_closing_crlf() {
        let content = "---\r\nname: test";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(!parts.has_closing);
        assert!(parts.frontmatter.is_empty());
        assert_eq!(parts.body, "name: test");
        assert_eq!(parts.body_start, 5); // past ---\r\n
        assert_eq!(&content[parts.body_start..], parts.body);
    }

    #[test]
    fn test_split_frontmatter_empty() {
        let content = "";
        let parts = split_frontmatter(content);
        assert!(!parts.has_frontmatter);
        assert!(!parts.has_closing);
    }

    #[test]
    fn test_split_frontmatter_empty_body_lf() {
        // --- immediately followed by closing --- with LF
        let content = "---\n---\nbody";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
        assert_eq!(parts.frontmatter, "");
        assert_eq!(parts.body, "\nbody");
        assert_eq!(&content[parts.body_start..], parts.body);
    }

    #[test]
    fn test_split_frontmatter_empty_body_crlf() {
        // --- immediately followed by closing --- with CRLF
        let content = "---\r\n---\r\nbody";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
        assert_eq!(parts.frontmatter, "");
        assert_eq!(parts.body, "\r\nbody");
        assert_eq!(&content[parts.body_start..], parts.body);
    }

    #[test]
    fn test_split_frontmatter_whitespace_prefix() {
        let content = "  \n---\nkey: val\n---\nbody";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
    }

    #[test]
    fn test_split_frontmatter_multiple_dashes() {
        let content = "---\nfirst: 1\n---\nmiddle\n---\nlast";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
        // Should split at first closing ---
        assert!(parts.body.contains("middle"));
    }

    // ===== Edge Case Tests =====

    // Note: split_frontmatter itself does not normalize CRLF line endings.
    // The pipeline normalizes content before calling it (see pipeline.rs).
    // These tests document the raw parser behavior with CRLF input.
    #[test]
    fn test_split_frontmatter_crlf() {
        let content = "---\r\nname: test\r\n---\r\nbody";
        let parts = split_frontmatter(content);
        // find("\n---") matches at "\r\n---" since \n is contained in \r\n
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
        assert!(parts.body.contains("body"));
    }

    // See comment above test_split_frontmatter_crlf for why this tests raw
    // (un-normalized) CRLF behavior.
    #[test]
    fn test_split_frontmatter_crlf_byte_offsets() {
        let content = "---\r\nname: test\r\n---\r\nbody";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);

        // Verify offsets are within bounds
        assert!(parts.frontmatter_start <= content.len());
        assert!(parts.body_start <= content.len());

        // frontmatter_start is at byte 5 (after "---\r\n")
        assert_eq!(parts.frontmatter_start, 5);

        // The leading \r\n after "---" is stripped from frontmatter.
        // Content after "---": "\r\nname: test\r\n---\r\nbody"
        // newline_len = 2 (CRLF)
        // find("\n---") matches at index 13 in rest ("\r\nname: test\r" = 13 chars)
        // frontmatter = rest[2..13] = "name: test\r"
        assert_eq!(parts.frontmatter, "name: test\r");
    }

    #[test]
    fn test_split_frontmatter_no_newline_after_opener() {
        // --- immediately followed by content (no newline), newline_len = 0
        let content = "---key: val\n---\nbody";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
        assert_eq!(parts.frontmatter_start, 3); // newline_len = 0
        assert_eq!(parts.frontmatter, "key: val");
        assert_eq!(&content[parts.body_start..], parts.body);
    }

    #[test]
    fn test_split_frontmatter_unicode_values() {
        let content = "---\nname: \u{4f60}\u{597d}\ndescription: caf\u{00e9}\n---\nbody";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
        assert!(
            parts.frontmatter.contains("\u{4f60}\u{597d}"),
            "Frontmatter should contain CJK characters"
        );
        assert!(
            parts.frontmatter.contains("caf\u{00e9}"),
            "Frontmatter should contain accented character"
        );
    }

    #[test]
    fn test_split_frontmatter_escaped_quotes() {
        let content = "---\nname: \"test\\\"skill\"\ndescription: test\n---\nbody";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
        assert!(
            parts.frontmatter.contains("test\\\"skill"),
            "Frontmatter should preserve escaped quotes"
        );
    }

    #[test]
    fn test_split_frontmatter_long_lines() {
        let long_value = "x".repeat(5000);
        let content = format!("---\nname: {}\n---\nbody", long_value);
        let parts = split_frontmatter(&content);
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
        assert!(parts.frontmatter.contains(&long_value));
    }

    #[test]
    fn test_split_frontmatter_empty_values() {
        let content = "---\nname:\ndescription: test\n---\nbody";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
        // Parser splits without validating values
        assert!(parts.frontmatter.contains("name:"));
    }

    #[test]
    fn test_split_frontmatter_nested_yaml() {
        let content = "---\nmetadata:\n  key1: val1\n  key2: val2\n---\nbody";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
        assert!(parts.frontmatter.contains("key1: val1"));
        assert!(parts.frontmatter.contains("key2: val2"));
    }

    #[test]
    fn test_split_frontmatter_mixed_line_endings() {
        let content = "---\nname: test\r\ndescription: val\n---\nbody";
        let parts = split_frontmatter(content);
        // Should not panic and should detect frontmatter
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
    }

    #[test]
    fn test_split_frontmatter_emoji_in_yaml_keys() {
        // Emoji characters (4-byte UTF-8) in YAML keys should be handled without panic
        let content = "---\n\u{1f525}fire: hot\n\u{1f680}rocket: fast\n---\nbody";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
        assert!(parts.frontmatter.contains("\u{1f525}fire"));
        assert!(parts.frontmatter.contains("\u{1f680}rocket"));
        // Verify byte offsets are valid char boundaries
        assert!(content.is_char_boundary(parts.frontmatter_start));
        assert!(content.is_char_boundary(parts.body_start));
    }

    #[test]
    fn test_split_frontmatter_emoji_in_yaml_values() {
        let content = "---\nstatus: \u{2705} done\nmood: \u{1f60a}\n---\nbody";
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter);
        assert!(parts.has_closing);
        assert!(parts.frontmatter.contains("\u{2705}"));
        assert!(parts.frontmatter.contains("\u{1f60a}"));
    }

    // ===== normalize_line_endings Tests =====

    #[test]
    fn test_normalize_lf_only_returns_borrowed() {
        let input = "hello\nworld\n";
        let result = normalize_line_endings(input);
        assert!(
            matches!(result, Cow::Borrowed(_)),
            "LF-only input should return Cow::Borrowed"
        );
        assert_eq!(&*result, input);
    }

    #[test]
    fn test_normalize_crlf_returns_owned() {
        let input = "hello\r\nworld\r\n";
        let result = normalize_line_endings(input);
        assert!(
            matches!(result, Cow::Owned(_)),
            "CRLF input should return Cow::Owned"
        );
        assert_eq!(&*result, "hello\nworld\n");
    }

    #[test]
    fn test_normalize_lone_cr() {
        let input = "hello\rworld\r";
        let result = normalize_line_endings(input);
        assert_eq!(&*result, "hello\nworld\n");
    }

    #[test]
    fn test_normalize_mixed_line_endings() {
        let input = "line1\r\nline2\rline3\nline4";
        let result = normalize_line_endings(input);
        assert_eq!(&*result, "line1\nline2\nline3\nline4");
        assert!(!result.contains('\r'));
    }

    #[test]
    fn test_normalize_empty_string() {
        let input = "";
        let result = normalize_line_endings(input);
        assert!(
            matches!(result, Cow::Borrowed(_)),
            "Empty string should return Cow::Borrowed"
        );
        assert_eq!(&*result, "");
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn split_frontmatter_never_panics(content in ".*") {
            // split_frontmatter should never panic on any input
            let _ = split_frontmatter(&content);
        }

        #[test]
        fn split_frontmatter_valid_offsets(content in ".*") {
            let parts = split_frontmatter(&content);
            // Offsets should be within content bounds
            prop_assert!(parts.frontmatter_start <= content.len());
            prop_assert!(parts.body_start <= content.len());
        }

        #[test]
        fn frontmatter_with_dashes_detected(
            yaml in "[a-z]+: [a-z]+",
        ) {
            let content = format!("---\n{}\n---\nbody", yaml);
            let parts = split_frontmatter(&content);
            prop_assert!(parts.has_frontmatter);
            prop_assert!(parts.has_closing);
        }

        #[test]
        fn no_frontmatter_without_leading_dashes(
            content in "[^-].*"
        ) {
            let parts = split_frontmatter(&content);
            prop_assert!(!parts.has_frontmatter);
        }

        #[test]
        fn unclosed_frontmatter_has_empty_frontmatter(
            yaml in "[a-z]+: [a-z]+"
        ) {
            // Content with --- but no closing ---
            let content = format!("---\n{}", yaml);
            let parts = split_frontmatter(&content);
            prop_assert!(parts.has_frontmatter);
            prop_assert!(!parts.has_closing);
            prop_assert!(parts.frontmatter.is_empty());
        }

        #[test]
        fn normalize_line_endings_never_contains_cr(content in ".*") {
            let normalized = normalize_line_endings(&content);
            prop_assert!(
                !normalized.contains('\r'),
                "Normalized output must not contain \\r"
            );
        }
    }
}
