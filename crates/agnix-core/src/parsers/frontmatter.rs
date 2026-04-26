//! YAML frontmatter parser
//!
//! ## Security: YAML Bomb Protection
//!
//! YAML bombs (deeply nested structures that expand to disproportionate memory
//! use in `serde_yaml`) are mitigated by a layered defense:
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
//! 4. **Explicit Depth Check**: `check_yaml_depth` rejects frontmatter whose
//!    structural nesting (flow-style `[`/`{` or block-style leading `- ` /
//!    leading whitespace) exceeds [`MAX_YAML_DEPTH`]. This runs before
//!    `serde_yaml::from_str` so pathological inputs are refused cheaply
//!    without building the intermediate deserialization tree.
//!
//! **Known Limitation**: `check_yaml_depth` is a conservative syntactic
//! approximation, not a full YAML parser. It uses the maximum of three
//! signals (flow bracket depth, block dash-list depth, leading-whitespace
//! indent depth in 2-space units) and rejects anything above 32. Quoted
//! scalars (single- and double-quoted) are tracked across line boundaries
//! so brackets/dashes inside multi-line quoted strings are not miscounted
//! as structural depth. Block scalars (`|` / `>`) are not modeled; because
//! their contents are indented at or below the key's indent, they at worst
//! contribute to `max_indent_units` and cannot induce false positives
//! below the 32-unit cap on realistic frontmatter. False positives remain
//! possible on extreme but legitimate inputs; in that case raise
//! `MAX_YAML_DEPTH` rather than disabling the check.

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

/// Maximum allowed YAML structural nesting depth.
///
/// Realistic agent-config frontmatter is almost never deeper than 5 - 6
/// levels; 32 leaves plenty of headroom for unusual-but-legitimate inputs
/// while still bounding pathological "YAML bomb" nesting that can cost
/// disproportionate memory in `serde_yaml`.
pub const MAX_YAML_DEPTH: usize = 32;

/// Reject YAML frontmatter whose structural nesting depth exceeds
/// [`MAX_YAML_DEPTH`]. Runs in O(n) over the input without allocating.
///
/// This is a conservative pre-parse guard: we track three independent
/// approximations of depth and reject if any of them exceeds the limit.
///
/// 1. **Flow-style bracket depth**: max concurrent `[` / `{` open.
/// 2. **Block-style dash depth**: max consecutive `- ` prefixes on one line
///    (e.g. `- - - - value` opens four list levels).
/// 3. **Indentation depth**: deepest leading-whitespace indent on any
///    non-blank line, measured in 2-space units (tabs count as one unit).
///
/// # Errors
///
/// Returns `ValidationError::Other` with a descriptive message if depth
/// exceeds the limit.
pub(crate) fn check_yaml_depth(yaml: &str) -> LintResult<()> {
    let mut flow_depth: usize = 0;
    let mut max_flow: usize = 0;
    let mut max_dash: usize = 0;
    let mut max_indent_units: usize = 0;
    // Quote state is tracked ACROSS lines: YAML single- and double-quoted
    // scalars are permitted to span multiple lines, so brackets inside a
    // multi-line quoted scalar must not be counted as structural depth.
    let mut in_single: bool = false;
    let mut in_double: bool = false;

    for line in yaml.lines() {
        // When we're mid quoted-scalar (carried over from a previous line),
        // the whole line is part of the scalar value until the closing quote;
        // indent/dash-list signals don't apply. Scan only for the quote
        // terminator (and bracket chars, which we ignore inside quotes).
        if in_single || in_double {
            let bytes = line.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                let b = bytes[i];
                if in_single {
                    if b == b'\'' {
                        // '' inside a single-quoted string is a literal apostrophe (escape).
                        if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                            i += 2;
                            continue;
                        }
                        in_single = false;
                    }
                } else if in_double {
                    if b == b'\\' {
                        // Skip escaped char in double-quoted scalar.
                        i += 2;
                        continue;
                    }
                    if b == b'"' {
                        in_double = false;
                    }
                }
                i += 1;
            }
            // Line was (wholly or partly) a scalar continuation; no
            // structural accounting for this line.
            continue;
        }

        // Count leading whitespace as indentation units (2 spaces = 1 unit,
        // tab = 1 unit). Skip blank lines.
        let leading_ws = line
            .bytes()
            .take_while(|b| *b == b' ' || *b == b'\t')
            .count();
        let trimmed = &line[leading_ws..];
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Count each leading space and each leading tab as one indent unit.
        // A previous draft used `spaces / 2 + tabs` to match YAML's typical
        // 2-space indent convention, but that lets 1-space-indented YAML
        // (still valid) hide from the cap — 63 nested maps at 1 space each
        // yield `indent_units == 63 / 2 == 31` and slip past `MAX_YAML_DEPTH`.
        // Counting raw columns guarantees pathological depth is flagged no
        // matter which indent width the attacker chose.
        let spaces = line.bytes().take_while(|b| *b == b' ').count();
        let tabs = line[spaces..].bytes().take_while(|b| *b == b'\t').count();
        let indent_units = spaces + tabs;
        if indent_units > max_indent_units {
            max_indent_units = indent_units;
        }

        // Count consecutive "- " dash-list prefixes at the start of the
        // content (after indentation). Example: "- - - value" = 3 levels.
        let mut rest = trimmed;
        let mut dashes: usize = 0;
        while let Some(after) = rest.strip_prefix("- ") {
            dashes += 1;
            rest = after;
        }
        if dashes > max_dash {
            max_dash = dashes;
        }

        // Track flow-style bracket depth, respecting single/double quoted
        // strings so brackets inside quotes don't inflate the count. Quote
        // state can persist to the next line if a scalar is unterminated.
        let bytes = rest.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if in_single {
                if b == b'\'' {
                    if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                        i += 2;
                        continue;
                    }
                    in_single = false;
                }
            } else if in_double {
                if b == b'\\' {
                    i += 2;
                    continue;
                }
                if b == b'"' {
                    in_double = false;
                }
            } else {
                match b {
                    b'\'' => in_single = true,
                    b'"' => in_double = true,
                    b'#' => break, // YAML line comment
                    b'[' | b'{' => {
                        flow_depth += 1;
                        if flow_depth > max_flow {
                            max_flow = flow_depth;
                        }
                    }
                    b']' | b'}' => {
                        flow_depth = flow_depth.saturating_sub(1);
                    }
                    _ => {}
                }
            }
            i += 1;
        }
        // Quote state intentionally persists to the next line so multi-line
        // quoted scalars are handled correctly.
    }

    let observed = max_flow.max(max_dash).max(max_indent_units);
    if observed > MAX_YAML_DEPTH {
        return Err(CoreError::Validation(ValidationError::Other(
            anyhow::anyhow!(
                "YAML frontmatter nesting depth {} exceeds maximum {} (possible YAML bomb)",
                observed,
                MAX_YAML_DEPTH
            ),
        )));
    }
    Ok(())
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
/// Protected against YAML bombs by a layered defense: the 1 MiB file size
/// cap, an explicit pre-parse depth check ([`check_yaml_depth`], limit
/// [`MAX_YAML_DEPTH`]), and `serde_yaml`'s internal protections. See module
/// documentation for details.
#[allow(dead_code)] // used in cfg(test) and __internal; not yet used by production validators
pub fn parse_frontmatter<T: DeserializeOwned>(content: &str) -> LintResult<(T, String)> {
    let parts = split_frontmatter(content);
    // Pre-parse depth check to bound memory use on pathological inputs.
    check_yaml_depth(&parts.frontmatter)?;
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

    #[test]
    fn test_check_yaml_depth_accepts_typical_frontmatter() {
        let yaml = "name: foo\ndescription: bar\ntags: [a, b, c]\n";
        assert!(check_yaml_depth(yaml).is_ok());
    }

    #[test]
    fn test_check_yaml_depth_accepts_realistic_nesting() {
        // Five levels of nesting, well under the 32 cap.
        let yaml = "a:\n  b:\n    c:\n      d:\n        e: value\n";
        assert!(check_yaml_depth(yaml).is_ok());
    }

    #[test]
    fn test_check_yaml_depth_rejects_deep_flow_brackets() {
        // 100 nested flow-style lists: [[[[...]]]]
        let depth = 100;
        let open = "[".repeat(depth);
        let close = "]".repeat(depth);
        let yaml = format!("data: {}v{}\n", open, close);
        let err = check_yaml_depth(&yaml).expect_err("deep nesting must be rejected");
        let msg = format!("{}", err);
        assert!(
            msg.contains("nesting depth") && msg.contains("exceeds maximum"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn test_check_yaml_depth_rejects_deep_indent() {
        // Build 100 levels of indented mappings (2 spaces per level).
        let mut yaml = String::new();
        for i in 0..100 {
            for _ in 0..i {
                yaml.push_str("  ");
            }
            yaml.push_str(&format!("k{}:\n", i));
        }
        assert!(check_yaml_depth(&yaml).is_err());
    }

    #[test]
    fn test_check_yaml_depth_rejects_deep_one_space_indent() {
        // Regression: previously `spaces / 2 + tabs` let 1-space-indented YAML
        // bypass the cap (63 levels would yield units=31 <= MAX_YAML_DEPTH).
        // Now each leading space counts as one indent unit, so pathological
        // 1-space nesting is flagged.
        let mut yaml = String::new();
        for i in 0..60 {
            for _ in 0..i {
                yaml.push(' ');
            }
            yaml.push_str(&format!("k{}:\n", i));
        }
        assert!(check_yaml_depth(&yaml).is_err());
    }

    #[test]
    fn test_check_yaml_depth_ignores_brackets_in_quotes() {
        // The flow-depth counter must not be tricked by brackets in strings.
        let yaml = "note: \"[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[not real]]]]\"\n";
        assert!(check_yaml_depth(yaml).is_ok());
    }

    #[test]
    fn test_check_yaml_depth_multiline_double_quoted_scalar_with_brackets() {
        // A double-quoted scalar that spans multiple lines and contains many
        // '[' / '{' characters inside its value. Brackets inside the scalar
        // must NOT count as structural depth. Real structural depth here is
        // 1 (top-level mapping), well under MAX_YAML_DEPTH.
        let yaml = "description: \"line one with [[[[ brackets\n\
                    still quoted [[[[ on line two\n\
                    and [[[[ line three ending here\"\n\
                    name: ok\n";
        assert!(
            check_yaml_depth(yaml).is_ok(),
            "multi-line double-quoted scalar containing '[' must not be rejected"
        );
    }

    #[test]
    fn test_check_yaml_depth_multiline_single_quoted_scalar_with_brackets() {
        // Single-quoted multi-line scalar; '' is the escape for a literal
        // apostrophe and must not prematurely close the quote.
        let yaml = "description: 'line one with [[[[ brackets and it''s fine\n\
                    still quoted [[[[ on line two\n\
                    and [[[[ line three'\n\
                    name: ok\n";
        assert!(
            check_yaml_depth(yaml).is_ok(),
            "multi-line single-quoted scalar containing '[' must not be rejected"
        );
    }

    #[test]
    fn test_check_yaml_depth_multiline_scalar_ignores_dash_list_prefix() {
        // Inside a quoted scalar continuation, a line starting with "- " is
        // literal text, not a block-list entry, so max_dash should stay 0.
        let yaml = "note: \"start\n\
                    - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - end\"\n\
                    name: ok\n";
        assert!(
            check_yaml_depth(yaml).is_ok(),
            "dashes inside a multi-line quoted scalar must not count as list depth"
        );
    }

    #[test]
    fn test_parse_frontmatter_rejects_yaml_bomb() {
        #[derive(Debug, serde::Deserialize)]
        struct Any {
            #[allow(dead_code)]
            data: serde_yaml::Value,
        }

        let open = "[".repeat(100);
        let close = "]".repeat(100);
        let content = format!("---\ndata: {}v{}\n---\nbody\n", open, close);

        let result: LintResult<(Any, String)> = parse_frontmatter(&content);
        assert!(
            result.is_err(),
            "pathologically nested YAML must be rejected before serde_yaml"
        );
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
