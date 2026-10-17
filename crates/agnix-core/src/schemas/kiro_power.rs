//! Kiro power file schema helpers.
//!
//! Covers the legacy `POWER.md` (frontmatter + body) and the Agent Plugins
//! `plugin.json` manifest that Kiro powers use now.
//!
//! Kiro CLI V3 reads POWER.md frontmatter without a schema: only
//! `description`, `displayName` and `keywords` (an array, anything else counts
//! as none) are used, and a YAML error makes the power unreadable. Parsing is
//! therefore lenient here: a wrong field type is not a parse error.

use crate::parsers::frontmatter::split_frontmatter;
use crate::schemas::common::ParseError;
use serde::Deserialize;
use serde_json::Value;

/// Parsed Kiro power markdown.
#[allow(dead_code)] // schema-level API; consumed by validator layer
#[derive(Debug, Clone)]
pub struct ParsedKiroPower {
    pub frontmatter: Option<KiroPowerFrontmatter>,
    pub body: String,
    /// Byte offset of the body in the original content.
    pub body_start: usize,
    /// Byte offset of the frontmatter text in the original content.
    pub frontmatter_start: usize,
    pub has_frontmatter: bool,
    pub has_closing_frontmatter: bool,
    /// The opening `---` is not the first line, so Kiro reads no frontmatter.
    pub frontmatter_not_first: bool,
    pub parse_error: Option<ParseError>,
}

/// The POWER.md frontmatter fields agnix checks, read leniently.
#[allow(dead_code)] // schema-level API; consumed by validator layer
#[derive(Debug, Clone, Default)]
pub struct KiroPowerFrontmatter {
    /// The whole frontmatter value (a mapping, or another YAML value).
    pub raw: serde_yaml::Value,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    /// `Some` only when `keywords` is an array; non-string items are kept out.
    pub keywords: Option<Vec<String>>,
    pub author: Option<String>,
}

impl KiroPowerFrontmatter {
    fn from_value(raw: serde_yaml::Value) -> Self {
        let string = |key: &str| raw.get(key).and_then(|v| v.as_str()).map(str::to_string);
        let keywords = raw
            .get("keywords")
            .and_then(|v| v.as_sequence())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect()
            });
        Self {
            name: string("name"),
            display_name: string("displayName"),
            description: string("description"),
            keywords,
            author: string("author"),
            raw,
        }
    }
}

fn line_col_at_offset(content: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;

    for (idx, ch) in content.char_indices() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}

/// Parse POWER.md content into structured frontmatter + body.
#[allow(dead_code)] // schema-level API; consumed by validator layer
pub fn parse_kiro_power(content: &str) -> ParsedKiroPower {
    let parts = split_frontmatter(content);
    let without_bom = content.strip_prefix('\u{FEFF}').unwrap_or(content);
    let mut parsed = ParsedKiroPower {
        frontmatter: None,
        body: parts.body.clone(),
        body_start: parts.body_start,
        frontmatter_start: parts.frontmatter_start,
        has_frontmatter: parts.has_frontmatter,
        has_closing_frontmatter: parts.has_closing,
        frontmatter_not_first: parts.has_frontmatter && !without_bom.starts_with("---"),
        parse_error: None,
    };

    if !parts.has_frontmatter || parsed.frontmatter_not_first {
        if parsed.frontmatter_not_first {
            parsed.body = content.to_string();
            parsed.body_start = 0;
        }
        return parsed;
    }

    if !parts.has_closing {
        parsed.parse_error = Some(ParseError::new(
            "Missing closing frontmatter delimiter",
            1,
            1,
        ));
        return parsed;
    }

    match serde_yaml::from_str::<serde_yaml::Value>(&parts.frontmatter) {
        Ok(raw) => parsed.frontmatter = Some(KiroPowerFrontmatter::from_value(raw)),
        Err(err) => {
            let (rel_line, rel_col) = err
                .location()
                .map(|loc| (loc.line(), loc.column()))
                .unwrap_or((1, 1));
            let (fm_start_line, fm_start_col) =
                line_col_at_offset(content, parts.frontmatter_start);
            let abs_line = fm_start_line + rel_line.saturating_sub(1);
            let abs_col = if rel_line <= 1 {
                fm_start_col + rel_col.saturating_sub(1)
            } else {
                rel_col
            };
            parsed.parse_error = Some(ParseError::new(err.to_string(), abs_line, abs_col));
        }
    }
    parsed
}

// ---------------------------------------------------------------------------
// plugin.json (Agent Plugins manifest)
// ---------------------------------------------------------------------------

/// `$schema` values Kiro accepts for plugin.json (Agent Plugins 1.x).
#[allow(dead_code)] // used by KiroPowerPluginValidator once plugin.json powers are routed
pub const AGENT_PLUGINS_SCHEMA_PATTERN: &str =
    r"^https://agent-plugins\.org/schemas/1\.\d+\.\d+/plugin\.schema\.json$";

/// Top-level plugin.json keys the Kiro V3 loader accepts. `displayName` is
/// accepted by Kiro but is not in the Agent Plugins 1.0.0 schema.
#[allow(dead_code)] // used by KiroPowerPluginValidator once plugin.json powers are routed
pub const PLUGIN_MANIFEST_FIELDS: &[&str] = &[
    "$schema",
    "name",
    "displayName",
    "version",
    "description",
    "author",
    "homepage",
    "repository",
    "license",
    "keywords",
    "extensions",
];

/// Keys allowed in the plugin.json `author` object.
#[allow(dead_code)] // used by KiroPowerPluginValidator once plugin.json powers are routed
pub const PLUGIN_AUTHOR_FIELDS: &[&str] = &["name", "email", "url"];

/// True for a valid Agent Plugins `name`: 1-64 chars of `a-z`, `0-9`, `.`
/// and `-`, starting and ending with a letter or digit, without `--` or `..`.
#[allow(dead_code)] // used by KiroPowerPluginValidator once plugin.json powers are routed
pub fn is_valid_plugin_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    !name.is_empty()
        && name.len() <= 64
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'.' || *b == b'-')
        && bytes[0].is_ascii_alphanumeric()
        && bytes[bytes.len() - 1].is_ascii_alphanumeric()
        && !name.contains("--")
        && !name.contains("..")
}

/// True for a valid Agent Skills name (also required of plugin skill
/// directory names): 1-64 chars of `a-z`, `0-9` and `-`, starting and ending
/// with a letter or digit, without `--`.
#[allow(dead_code)] // used by KiroPowerPluginValidator once plugin.json powers are routed
pub fn is_valid_skill_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    !name.is_empty()
        && name.len() <= 64
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
        && bytes[0].is_ascii_alphanumeric()
        && bytes[bytes.len() - 1].is_ascii_alphanumeric()
        && !name.contains("--")
}

/// One member of a JSON object: decoded key and byte spans in the document.
#[allow(dead_code)] // used by KiroPowerPluginValidator once plugin.json powers are routed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonMember {
    pub key: String,
    pub key_start: usize,
    pub value_start: usize,
    pub value_end: usize,
}

/// List the members (objects) or items (arrays, with an empty key) of the JSON
/// value spanning `content[start..end]`, with absolute byte offsets. Uses
/// serde's own parser for keys and values, so escapes and nesting are exact.
#[allow(dead_code)] // used by KiroPowerPluginValidator once plugin.json powers are routed
pub fn json_children(content: &str, start: usize, end: usize) -> Option<Vec<JsonMember>> {
    fn first_value<'de, T: Deserialize<'de>>(text: &'de str) -> Option<(T, usize)> {
        let mut stream = serde_json::Deserializer::from_str(text).into_iter::<T>();
        let value = stream.next()?.ok()?;
        Some((value, stream.byte_offset()))
    }
    let text = content.get(start..end)?;
    let bytes = text.as_bytes();
    let object = text.starts_with('{');
    if !object && !text.starts_with('[') {
        return None;
    }
    let skip_ws = |mut pos: usize| {
        while bytes.get(pos).is_some_and(u8::is_ascii_whitespace) {
            pos += 1;
        }
        pos
    };
    let mut members = Vec::new();
    let mut pos = skip_ws(1);
    if matches!(bytes.get(pos), Some(b'}' | b']')) {
        return Some(members);
    }
    loop {
        let (key, key_start) = if object {
            let key_start = pos;
            let (key, length) = first_value::<String>(text.get(pos..)?)?;
            pos = skip_ws(pos + length);
            if bytes.get(pos) != Some(&b':') {
                return None;
            }
            pos = skip_ws(pos + 1);
            (key, key_start)
        } else {
            (String::new(), pos)
        };
        let (_, length) = first_value::<serde::de::IgnoredAny>(text.get(pos..)?)?;
        members.push(JsonMember {
            key,
            key_start: start + key_start,
            value_start: start + pos,
            value_end: start + pos + length,
        });
        pos = skip_ws(pos + length);
        match bytes.get(pos) {
            Some(b',') => pos = skip_ws(pos + 1),
            Some(b'}' | b']') => return Some(members),
            _ => return None,
        }
    }
}

/// Parsed plugin.json: the value and the top-level member spans.
#[allow(dead_code)] // used by KiroPowerPluginValidator once plugin.json powers are routed
#[derive(Debug, Clone)]
pub struct ParsedPluginManifest {
    pub value: Value,
    /// Top-level members in document order (duplicates included).
    pub members: Vec<JsonMember>,
}

/// Parse plugin.json strictly (Kiro uses `JSON.parse`: no comments, no
/// trailing commas). `Err` carries the message and 1-based line and column.
#[allow(dead_code)] // used by KiroPowerPluginValidator once plugin.json powers are routed
pub fn parse_plugin_manifest(content: &str) -> Result<ParsedPluginManifest, ParseError> {
    let value: Value = serde_json::from_str(content)
        .map_err(|e| ParseError::new(e.to_string(), e.line().max(1), e.column().max(1)))?;
    let trimmed_start = content.len() - content.trim_start().len();
    let trimmed_end = content.trim_end().len();
    let members = if value.is_object() {
        json_children(content, trimmed_start, trimmed_end).unwrap_or_default()
    } else {
        Vec::new()
    };
    Ok(ParsedPluginManifest { value, members })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_power_frontmatter_and_body() {
        let parsed = parse_kiro_power(
            r#"---
name: test-power
description: Valid power
keywords:
  - kiro
  - power
---
# Test

Body content.
"#,
        );

        assert!(parsed.parse_error.is_none());
        assert!(parsed.has_frontmatter);
        assert!(parsed.has_closing_frontmatter);
        assert!(parsed.body.contains("Body content"));
        let frontmatter = parsed.frontmatter.unwrap();
        assert_eq!(frontmatter.name.as_deref(), Some("test-power"));
        assert_eq!(frontmatter.keywords.unwrap(), ["kiro", "power"]);
    }

    #[test]
    fn parse_power_without_frontmatter_is_supported() {
        let parsed = parse_kiro_power("# No frontmatter\n\nBody");
        assert!(!parsed.has_frontmatter);
        assert!(parsed.parse_error.is_none());
    }

    #[test]
    fn parse_power_with_invalid_yaml_reports_location() {
        let parsed = parse_kiro_power(
            r#"---
name:
  - bad
description: still present
keywords: [ok
---
Body
"#,
        );

        let error = parsed.parse_error.expect("expected parse error");
        assert!(error.line > 0);
        assert!(error.column > 0);
    }

    #[test]
    fn wrong_field_types_are_not_parse_errors() {
        // Changed expectation: Kiro reads POWER.md without a schema, so a
        // string `keywords` is "no keywords", not invalid YAML.
        let parsed = parse_kiro_power("---\nname: 1\nkeywords: solo\n---\nBody\n");
        assert!(parsed.parse_error.is_none());
        let frontmatter = parsed.frontmatter.unwrap();
        assert!(frontmatter.name.is_none());
        assert!(frontmatter.keywords.is_none());
        assert!(frontmatter.raw.get("keywords").is_some());
    }

    #[test]
    fn frontmatter_after_blank_line_is_not_frontmatter() {
        let parsed = parse_kiro_power("\n---\nname: x\n---\nBody\n");
        assert!(parsed.frontmatter_not_first);
        assert!(parsed.frontmatter.is_none());
    }

    #[test]
    fn json_children_spans() {
        let content = "{\n  \"a\": 1,\n  \"b\\u0063\": [\"x\", {\"y\": 2}]\n}";
        let members = json_children(content, 0, content.len()).unwrap();
        assert_eq!(members.len(), 2);
        assert_eq!(members[1].key, "bc");
        let value = &content[members[1].value_start..members[1].value_end];
        assert_eq!(value, "[\"x\", {\"y\": 2}]");
        let items = json_children(content, members[1].value_start, members[1].value_end).unwrap();
        assert_eq!(
            &content[items[1].value_start..items[1].value_end],
            "{\"y\": 2}"
        );
    }

    #[test]
    fn plugin_names() {
        for name in ["supabase", "power-builder", "a", "aws.sam", "x1"] {
            assert!(is_valid_plugin_name(name), "{name}");
        }
        for name in [
            "",
            "Supabase",
            "-a",
            "a-",
            "a--b",
            "a..b",
            "a_b",
            &"a".repeat(65),
        ] {
            assert!(!is_valid_plugin_name(name), "{name}");
        }
        assert!(is_valid_skill_name("setup-db"));
        assert!(!is_valid_skill_name("setup.db"));
    }
}
