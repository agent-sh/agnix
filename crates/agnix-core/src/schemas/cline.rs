//! Cline rules schema helpers
//!
//! Provides parsing and validation for:
//! - `.clinerules` single file (plain text, no frontmatter)
//! - `.clinerules/*.md` and `.clinerules/*.txt` folder files, `.cline/rules/`
//!   and the workflow folders (optional YAML frontmatter)
//!
//! Rule frontmatter holds `paths` (glob patterns for scoped rule
//! application) and `name`, `disabled` and `enabled`, which the Cline SDK
//! loader reads. The keys are checked in `rules::cline`.

/// The frontmatter block of a Cline rule or workflow file.
#[derive(Debug, Clone)]
pub struct ParsedClineFrontmatter {
    /// Raw frontmatter string (between --- markers); it starts on line 2.
    pub raw: String,
    /// Line number of the closing `---` (1-indexed), or the last line when
    /// the block is not closed.
    pub end_line: usize,
    /// Body content after frontmatter
    pub body: String,
    /// False when the opening `---` has no closing `---`.
    pub closed: bool,
}

/// Result of validating a glob pattern
#[derive(Debug, Clone)]
pub struct GlobValidation {
    pub valid: bool,
    pub error: Option<String>,
}

/// Split the frontmatter from a Cline rule or workflow file.
///
/// Returns `None` when the file does not start with a `---` line.
pub fn parse_frontmatter(content: &str) -> Option<ParsedClineFrontmatter> {
    let content = content.strip_prefix('\u{FEFF}').unwrap_or(content);
    let lines: Vec<&str> = content.lines().collect();
    if lines.first()?.trim() != "---" {
        return None;
    }

    let Some(end_idx) = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, line)| line.trim() == "---")
        .map(|(i, _)| i)
    else {
        return Some(ParsedClineFrontmatter {
            raw: lines[1..].join("\n"),
            end_line: lines.len(),
            body: String::new(),
            closed: false,
        });
    };

    Some(ParsedClineFrontmatter {
        raw: lines[1..end_idx].join("\n"),
        end_line: end_idx + 1,
        body: lines[end_idx + 1..].join("\n"),
        closed: true,
    })
}

/// Validate a glob pattern
pub fn validate_glob_pattern(pattern: &str) -> GlobValidation {
    match glob::Pattern::new(pattern) {
        Ok(_) => GlobValidation {
            valid: true,
            error: None,
        },
        Err(e) => GlobValidation {
            valid: false,
            error: Some(e.to_string()),
        },
    }
}

/// Check if content is empty
pub fn is_content_empty(content: &str) -> bool {
    content
        .strip_prefix('\u{FEFF}')
        .unwrap_or(content)
        .trim()
        .is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_frontmatter() {
        let content = r#"---
paths:
  - "**/*.ts"
---
# TypeScript Rules

Use strict mode.
"#;
        let result = parse_frontmatter(content).unwrap();
        assert!(result.closed);
        assert_eq!(result.raw, "paths:\n  - \"**/*.ts\"");
        assert!(result.body.contains("TypeScript Rules"));
    }

    #[test]
    fn test_parse_frontmatter_accepts_utf8_bom() {
        let content = "\u{FEFF}---\npaths:\n  - \"**/*.ts\"\n---\n# TypeScript Rules\n";
        let result = parse_frontmatter(content).expect("BOM-prefixed frontmatter should parse");
        assert!(result.closed);
        assert_eq!(result.raw, "paths:\n  - \"**/*.ts\"");
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let content = "# Just markdown without frontmatter";
        let result = parse_frontmatter(content);
        assert!(result.is_none());
        assert!(parse_frontmatter("").is_none());
    }

    #[test]
    fn test_parse_unclosed_frontmatter() {
        let content = r#"---
paths: "**/*.ts"
# Missing closing ---
"#;
        let result = parse_frontmatter(content).unwrap();
        assert!(!result.closed);
        assert_eq!(result.end_line, 3);
    }

    #[test]
    fn test_valid_glob_patterns() {
        let patterns = vec!["**/*.ts", "*.rs", "src/**/*.js", "[abc].txt"];
        for pattern in patterns {
            let result = validate_glob_pattern(pattern);
            assert!(result.valid, "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_invalid_glob_pattern() {
        let result = validate_glob_pattern("[unclosed");
        assert!(!result.valid);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_empty_content() {
        assert!(is_content_empty(""));
        assert!(is_content_empty("   \n\t  "));
        assert!(!is_content_empty("# Instructions"));
        assert!(is_content_empty("\u{FEFF}"));
    }

    #[test]
    fn test_frontmatter_line_numbers() {
        let content = r#"---
paths: "**/*.ts"
---
# Body
"#;
        let result = parse_frontmatter(content).unwrap();
        assert_eq!(result.end_line, 3);
        assert_eq!(result.body, "# Body");
    }
}
