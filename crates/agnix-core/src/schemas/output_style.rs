//! `.claude/output-styles/*.md` frontmatter schema helpers
//!
//! Provides parsing for Claude Code output-style files. Output styles let users
//! customise the tone/format Claude Code responds in. Frontmatter has 3 known
//! optional fields: `name`, `description`, `keep-coding-instructions`.
//!
//! Spec: https://code.claude.com/docs/en/output-styles (verified 2026-04-22)

use serde::{Deserialize, Deserializer};
use std::collections::HashSet;

/// Known valid keys for `.claude/output-styles/*.md` frontmatter
pub(crate) const KNOWN_KEYS: &[&str] = &["name", "description", "keep-coding-instructions"];

/// Frontmatter schema for Claude output-style files
///
/// `keep_coding_instructions` is kept as a raw `serde_yaml::Value` so the
/// validator can distinguish "missing" from "present but wrong type"
/// (CC-OS-002 requires rejecting non-bool values like `"yes"`, `1`, `null`).
/// The hyphenated YAML key is mapped to a Rust-snake-case field via
/// `#[serde(rename)]`.
///
/// A custom deserializer wraps the value in `Some(_)` even when YAML is `null`,
/// so CC-OS-002 can detect `keep-coding-instructions: null` (which serde would
/// otherwise collapse into `None`, indistinguishable from "field absent").
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct OutputStyleSchema {
    pub name: Option<String>,
    pub description: Option<String>,
    #[serde(
        rename = "keep-coding-instructions",
        deserialize_with = "deserialize_present_value"
    )]
    pub keep_coding_instructions: Option<serde_yaml::Value>,
}

/// Deserialize a value while preserving `null` as `Some(Value::Null)` (absent fields
/// get `None` via `#[serde(default)]`, never via this function).
fn deserialize_present_value<'de, D>(d: D) -> Result<Option<serde_yaml::Value>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_yaml::Value::deserialize(d)?;
    Ok(Some(v))
}

/// Result of parsing output-style frontmatter
#[derive(Debug, Clone)]
pub struct ParsedOutputStyleFrontmatter {
    /// The parsed schema (if valid YAML)
    pub schema: Option<OutputStyleSchema>,
    /// Raw frontmatter string (between --- markers)
    #[allow(dead_code)] // parsed but not yet consumed by validators
    pub raw: String,
    /// Line number where frontmatter starts (1-indexed)
    pub start_line: usize,
    /// Line number where frontmatter ends (1-indexed)
    #[allow(dead_code)] // parsed but not yet consumed by validators
    pub end_line: usize,
    /// Unknown keys found in frontmatter
    pub unknown_keys: Vec<UnknownKey>,
    /// Parse error if YAML is invalid
    pub parse_error: Option<String>,
    /// True when the body after the closing `---` is empty/whitespace-only.
    /// Pre-computed during parse so CC-OS-004 doesn't re-scan the file.
    pub body_is_empty: bool,
}

/// An unknown key found in frontmatter
#[derive(Debug, Clone)]
pub struct UnknownKey {
    pub key: String,
    pub line: usize,
    pub column: usize,
}

/// Parse frontmatter from a `.claude/output-styles/*.md` file.
///
/// Returns parsed frontmatter if a `---` opening delimiter is present, or
/// `None` if no frontmatter exists.
pub fn parse_frontmatter(content: &str) -> Option<ParsedOutputStyleFrontmatter> {
    if !content.starts_with("---") {
        return None;
    }

    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return None;
    }

    // Find closing ---
    let mut end_idx = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end_idx = Some(i);
            break;
        }
    }

    // Opening --- but no closing ---: surface as parse error.
    if end_idx.is_none() {
        let frontmatter_lines: Vec<&str> = lines[1..].to_vec();
        let raw = frontmatter_lines.join("\n");

        return Some(ParsedOutputStyleFrontmatter {
            schema: None,
            raw,
            start_line: 1,
            end_line: lines.len(),
            unknown_keys: Vec::new(),
            parse_error: Some("missing closing ---".to_string()),
            body_is_empty: true,
        });
    }

    let end_idx = end_idx.unwrap();

    // Extract frontmatter content (between --- markers)
    let frontmatter_lines: Vec<&str> = lines[1..end_idx].to_vec();
    let raw = frontmatter_lines.join("\n");

    // Body lives after the closing ---
    let body_lines: &[&str] = if end_idx + 1 < lines.len() {
        &lines[end_idx + 1..]
    } else {
        &[]
    };
    let body_is_empty = body_lines.iter().all(|l| l.trim().is_empty());

    // Detect unknown keys via line scanning (independent of YAML parse success).
    let unknown_keys = find_unknown_keys(&raw, 2); // line 1 is the opening ---

    // Try to parse the schema fields from YAML.
    let (schema, parse_error) = parse_schema(&raw);

    Some(ParsedOutputStyleFrontmatter {
        schema,
        raw,
        start_line: 1,
        end_line: end_idx + 1,
        unknown_keys,
        parse_error,
        body_is_empty,
    })
}

/// Parse the schema from raw YAML frontmatter.
///
/// Uses `serde_yaml` deserialization directly into [`OutputStyleSchema`].
/// The hyphenated YAML key `keep-coding-instructions` maps to
/// `keep_coding_instructions` via `#[serde(rename)]` on the struct field.
/// `keep_coding_instructions` deserializes as `serde_yaml::Value` so the
/// validator can detect non-bool values (string `"yes"`, number `1`, `null`)
/// in CC-OS-002 — using `Option<bool>` would silently coerce or fail the
/// whole struct.
fn parse_schema(raw: &str) -> (Option<OutputStyleSchema>, Option<String>) {
    if raw.trim().is_empty() {
        return (Some(OutputStyleSchema::default()), None);
    }

    match serde_yaml::from_str::<OutputStyleSchema>(raw) {
        Ok(schema) => (Some(schema), None),
        Err(e) => (None, Some(e.to_string())),
    }
}

/// Find unknown keys in frontmatter YAML by line scanning.
///
/// Top-level keys in YAML frontmatter are not indented; this matches that
/// heuristic so nested mapping keys are not flagged.
fn find_unknown_keys(yaml: &str, start_line: usize) -> Vec<UnknownKey> {
    let known: HashSet<&str> = KNOWN_KEYS.iter().copied().collect();
    let mut unknown = Vec::new();

    for (i, line) in yaml.lines().enumerate() {
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }
        if line.trim_start().starts_with('#') {
            continue;
        }
        if let Some(colon_idx) = line.find(':') {
            let key_raw = &line[..colon_idx];
            let key = key_raw.trim().trim_matches(|c| c == '\'' || c == '\"');
            if !key.is_empty() && !known.contains(key) {
                unknown.push(UnknownKey {
                    key: key.to_string(),
                    line: start_line + i,
                    column: key_raw.len() - key_raw.trim_start().len(),
                });
            }
        }
    }

    unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_no_frontmatter() {
        let content = "# Just markdown without frontmatter";
        let result = parse_frontmatter(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_empty_frontmatter() {
        let content = "---\n---\nbody";
        let result = parse_frontmatter(content).unwrap();
        let schema = result.schema.unwrap();
        assert!(schema.name.is_none());
        assert!(schema.description.is_none());
        assert!(schema.keep_coding_instructions.is_none());
        assert!(result.parse_error.is_none());
        assert!(!result.body_is_empty);
    }

    #[test]
    fn test_parse_with_keep_coding_instructions_true() {
        let content = "---\nname: Concise\ndescription: short\nkeep-coding-instructions: true\n---\nBe brief.";
        let result = parse_frontmatter(content).unwrap();
        let schema = result.schema.unwrap();
        assert_eq!(schema.name.as_deref(), Some("Concise"));
        assert_eq!(schema.description.as_deref(), Some("short"));
        assert_eq!(
            schema.keep_coding_instructions,
            Some(serde_yaml::Value::Bool(true))
        );
        assert!(result.unknown_keys.is_empty());
    }

    #[test]
    fn test_parse_with_keep_coding_instructions_non_bool() {
        let content = "---\nname: Concise\nkeep-coding-instructions: \"yes\"\n---\nBody";
        let result = parse_frontmatter(content).unwrap();
        let schema = result.schema.unwrap();
        let v = schema.keep_coding_instructions.expect("present");
        assert!(v.as_bool().is_none(), "value must NOT be bool");
        assert_eq!(v.as_str(), Some("yes"));
    }

    #[test]
    fn test_detect_unknown_keys() {
        let content = "---\nname: X\ndescription: y\nfoo: bar\nalwaysApply: true\n---\nbody";
        let result = parse_frontmatter(content).unwrap();
        assert_eq!(result.unknown_keys.len(), 2);
        assert!(result.unknown_keys.iter().any(|k| k.key == "foo"));
        assert!(result.unknown_keys.iter().any(|k| k.key == "alwaysApply"));
    }

    #[test]
    fn test_detect_empty_body() {
        let content = "---\nname: X\n---\n   \n\n";
        let result = parse_frontmatter(content).unwrap();
        assert!(result.body_is_empty);
    }

    #[test]
    fn test_detect_non_empty_body() {
        let content = "---\nname: X\n---\nReal instructions.";
        let result = parse_frontmatter(content).unwrap();
        assert!(!result.body_is_empty);
    }

    #[test]
    fn test_known_keys_not_flagged() {
        let content = "---\nname: X\ndescription: y\nkeep-coding-instructions: false\n---\nbody";
        let result = parse_frontmatter(content).unwrap();
        assert!(result.unknown_keys.is_empty());
    }

    #[test]
    fn test_unclosed_frontmatter_is_parse_error() {
        let content = "---\nname: X";
        let result = parse_frontmatter(content).unwrap();
        assert!(result.parse_error.is_some());
        assert_eq!(result.parse_error.as_deref(), Some("missing closing ---"));
    }
}
