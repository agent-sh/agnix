//! Span-preserving JSON (and JSONC) trees for the Kiro config validators.
//!
//! Kiro reads `mcp.json`, `cli.json` and `permissions.json` with two different
//! parsers: the Rust host (Kiro CLI 2.x, "V2") uses strict serde_json, while the
//! V3 engine reads `mcp.json` with comment-json, which also accepts `//` and
//! `/* */` comments and trailing commas. Validators need the decoded values,
//! the exact byte span of every key and value (to point diagnostics and to edit
//! values in place), and every duplicate key in source order (a `Value` map
//! keeps only the last one, which is also what Kiro keeps).
//!
//! The tree is built with serde_json for every string and scalar token, so no
//! second string/escape decoder exists here. JSONC input is first "blanked":
//! comments and trailing commas outside strings are overwritten with spaces,
//! which keeps every byte offset and line number of the original text.

use serde::Deserialize;
use serde::de::IgnoredAny;
use serde_json::Number;

/// A JSON value with the byte span of its source text.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct JsonNode {
    pub start: usize,
    pub end: usize,
    pub value: JsonValue,
}

/// Decoded JSON value. Objects keep every member in source order, duplicates
/// included.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum JsonValue {
    Object(Vec<JsonMember>),
    Array(Vec<JsonNode>),
    String(String),
    Number(Number),
    Bool(bool),
    Null,
}

/// One `"key": value` member of an object.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct JsonMember {
    pub key: String,
    pub key_start: usize,
    pub key_end: usize,
    pub value: JsonNode,
}

/// Which dialect the document needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JsonDialect {
    /// Plain RFC 8259 JSON, accepted by every Kiro parser.
    Strict,
    /// Parses only after comments or trailing commas are removed. `offset` is
    /// the first such construct.
    Jsonc { offset: usize },
}

/// A parsed document.
#[derive(Debug, Clone)]
pub(crate) struct JsonDocument {
    pub root: JsonNode,
    pub dialect: JsonDialect,
    /// For [`JsonDialect::Jsonc`]: the error strict serde_json (Kiro CLI 2.x)
    /// reports for the original text.
    pub strict_error: Option<JsonSyntaxError>,
}

/// A syntax error with serde_json's message and 1-based location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JsonSyntaxError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl JsonSyntaxError {
    fn from_serde(error: &serde_json::Error) -> Self {
        let full = error.to_string();
        // serde_json appends " at line L column C"; the location is kept
        // separately, so strip it from the message.
        let message = match full.rfind(" at line ") {
            Some(index) => full[..index].to_string(),
            None => full,
        };
        Self {
            message,
            line: error.line().max(1),
            column: error.column(),
        }
    }
}

impl JsonValue {
    pub fn as_object(&self) -> Option<&[JsonMember]> {
        match self {
            Self::Object(members) => Some(members),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[JsonNode]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<&Number> {
        match self {
            Self::Number(value) => Some(value),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// JSON type name for messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Object(_) => "object",
            Self::Array(_) => "array",
            Self::String(_) => "string",
            Self::Number(_) => "number",
            Self::Bool(_) => "boolean",
            Self::Null => "null",
        }
    }

    /// The last member named `key`, which is the one serde_json and
    /// `JSON.parse` keep when a key repeats.
    pub fn get(&self, key: &str) -> Option<&JsonMember> {
        self.as_object()?
            .iter()
            .rev()
            .find(|member| member.key == key)
    }
}

/// Parse `content` as strict JSON, or as JSONC when `allow_jsonc` is set.
pub(crate) fn parse_json_document(
    content: &str,
    allow_jsonc: bool,
) -> Result<JsonDocument, JsonSyntaxError> {
    let strict_error = match serde_json::from_str::<IgnoredAny>(content) {
        Ok(_) => {
            let root = build_tree(content).ok_or_else(internal_error)?;
            return Ok(JsonDocument {
                root,
                dialect: JsonDialect::Strict,
                strict_error: None,
            });
        }
        Err(error) => JsonSyntaxError::from_serde(&error),
    };
    if !allow_jsonc {
        return Err(strict_error);
    }
    let (blanked, first_offset) = blank_jsonc(content);
    let Some(offset) = first_offset else {
        return Err(strict_error);
    };
    match serde_json::from_str::<IgnoredAny>(&blanked) {
        Ok(_) => {
            let root = build_tree(&blanked).ok_or_else(internal_error)?;
            Ok(JsonDocument {
                root,
                dialect: JsonDialect::Jsonc { offset },
                strict_error: Some(strict_error),
            })
        }
        // Report the real syntax error, not the first comment.
        Err(error) => Err(JsonSyntaxError::from_serde(&error)),
    }
}

fn internal_error() -> JsonSyntaxError {
    JsonSyntaxError {
        message: "unsupported JSON structure".to_string(),
        line: 1,
        column: 0,
    }
}

/// Overwrite comments and trailing commas outside strings with spaces.
/// Returns the new text and the offset of the first removed construct.
/// Newlines inside block comments are kept so line numbers do not move.
pub(crate) fn blank_jsonc(content: &str) -> (String, Option<usize>) {
    let mut bytes = content.as_bytes().to_vec();
    let mut first: Option<usize> = None;
    let mut i = 0;
    let mut in_string = false;
    let mut escape = false;
    while i < bytes.len() {
        let b = bytes[i];
        if in_string {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match b {
            b'"' => {
                in_string = true;
                i += 1;
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                first.get_or_insert(i);
                while i < bytes.len() && bytes[i] != b'\n' {
                    bytes[i] = b' ';
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                // An unterminated block comment is left in place so the
                // document fails to parse, as it does in comment-json.
                let Some(rel) = content[i + 2..].find("*/") else {
                    break;
                };
                first.get_or_insert(i);
                let start = i;
                let end = i + 2 + rel + 2;
                for byte in &mut bytes[start..end] {
                    if *byte != b'\n' && *byte != b'\r' {
                        *byte = b' ';
                    }
                }
                i = end;
            }
            b',' => {
                if next_significant(&bytes, i + 1).is_some_and(|c| c == b'}' || c == b']') {
                    first.get_or_insert(i);
                    bytes[i] = b' ';
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    // Only ASCII bytes were replaced by ASCII spaces, so the text stays UTF-8.
    let text = String::from_utf8(bytes).unwrap_or_else(|_| content.to_string());
    (text, first)
}

/// The next byte that is not whitespace or part of a comment.
fn next_significant(bytes: &[u8], mut i: usize) -> Option<u8> {
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
            }
            other => return Some(other),
        }
    }
    None
}

/// Build the tree for text serde_json already accepted. Scalars and keys are
/// decoded by serde_json; only structural characters are handled here.
fn build_tree(text: &str) -> Option<JsonNode> {
    let mut pos = skip_ws(text, 0);
    let node = parse_value(text, &mut pos)?;
    Some(node)
}

fn skip_ws(text: &str, mut pos: usize) -> usize {
    let bytes = text.as_bytes();
    while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r') {
        pos += 1;
    }
    pos
}

fn first_value<'de, T: Deserialize<'de>>(text: &'de str) -> Option<(T, usize)> {
    let mut stream = serde_json::Deserializer::from_str(text).into_iter::<T>();
    let value = stream.next()?.ok()?;
    Some((value, stream.byte_offset()))
}

fn parse_value(text: &str, pos: &mut usize) -> Option<JsonNode> {
    let start = *pos;
    match text.as_bytes().get(start)? {
        b'{' => {
            let mut members = Vec::new();
            *pos = skip_ws(text, start + 1);
            if text.as_bytes().get(*pos) == Some(&b'}') {
                *pos += 1;
                return Some(JsonNode {
                    start,
                    end: *pos,
                    value: JsonValue::Object(members),
                });
            }
            loop {
                let key_start = *pos;
                let (key, length) = first_value::<String>(text.get(key_start..)?)?;
                let key_end = key_start + length;
                *pos = skip_ws(text, key_end);
                if text.as_bytes().get(*pos) != Some(&b':') {
                    return None;
                }
                *pos = skip_ws(text, *pos + 1);
                let value = parse_value(text, pos)?;
                members.push(JsonMember {
                    key,
                    key_start,
                    key_end,
                    value,
                });
                *pos = skip_ws(text, *pos);
                match text.as_bytes().get(*pos)? {
                    b',' => *pos = skip_ws(text, *pos + 1),
                    b'}' => {
                        *pos += 1;
                        break;
                    }
                    _ => return None,
                }
            }
            Some(JsonNode {
                start,
                end: *pos,
                value: JsonValue::Object(members),
            })
        }
        b'[' => {
            let mut items = Vec::new();
            *pos = skip_ws(text, start + 1);
            if text.as_bytes().get(*pos) == Some(&b']') {
                *pos += 1;
                return Some(JsonNode {
                    start,
                    end: *pos,
                    value: JsonValue::Array(items),
                });
            }
            loop {
                items.push(parse_value(text, pos)?);
                *pos = skip_ws(text, *pos);
                match text.as_bytes().get(*pos)? {
                    b',' => *pos = skip_ws(text, *pos + 1),
                    b']' => {
                        *pos += 1;
                        break;
                    }
                    _ => return None,
                }
            }
            Some(JsonNode {
                start,
                end: *pos,
                value: JsonValue::Array(items),
            })
        }
        _ => {
            let (value, length) = first_value::<serde_json::Value>(text.get(start..)?)?;
            *pos = start + length;
            let value = match value {
                serde_json::Value::String(s) => JsonValue::String(s),
                serde_json::Value::Number(n) => JsonValue::Number(n),
                serde_json::Value::Bool(b) => JsonValue::Bool(b),
                serde_json::Value::Null => JsonValue::Null,
                // Containers start with `{` or `[` and are handled above.
                _ => return None,
            };
            Some(JsonNode {
                start,
                end: *pos,
                value,
            })
        }
    }
}

/// The candidate closest to `name` for a "did you mean" hint: a
/// case-insensitive match, or an edit distance of at most a quarter of the
/// name's length (at least 1, at most 3). Ties keep the first candidate.
pub(crate) fn closest_name<'c>(name: &str, candidates: &[&'c str]) -> Option<&'c str> {
    if let Some(exact) = candidates.iter().find(|c| c.eq_ignore_ascii_case(name)) {
        return Some(exact);
    }
    let lower = name.to_ascii_lowercase();
    let budget = (name.chars().count() / 4).clamp(1, 3);
    candidates
        .iter()
        .map(|c| (edit_distance(&lower, &c.to_ascii_lowercase()), *c))
        .filter(|(distance, _)| *distance <= budget)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, c)| c)
}

/// Optimal string alignment distance: insertions, deletions, substitutions
/// and adjacent transpositions ("evn" -> "env") each cost 1.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = best;
        }
    }
    d[a.len()][b.len()]
}

/// Maps byte offsets to 1-based (line, column) pairs; columns count chars.
pub(crate) struct LineIndex<'a> {
    content: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    pub fn new(content: &'a str) -> Self {
        let mut line_starts = vec![0];
        for (index, byte) in content.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(index + 1);
            }
        }
        Self {
            content,
            line_starts,
        }
    }

    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.content.len());
        let line = self.line_starts.partition_point(|&start| start <= offset);
        let line_start = self.line_starts[line - 1];
        let column = self
            .content
            .get(line_start..offset)
            .map_or(offset - line_start, |prefix| prefix.chars().count())
            + 1;
        (line, column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strict(content: &str) -> JsonDocument {
        parse_json_document(content, false).expect("valid JSON")
    }

    #[test]
    fn spans_point_at_keys_and_values() {
        let content = "{\n  \"a\": [1, \"x\"],\n  \"b\": {\"c\": true}\n}";
        let doc = strict(content);
        assert_eq!(doc.dialect, JsonDialect::Strict);
        let a = doc.root.value.get("a").unwrap();
        assert_eq!(&content[a.key_start..a.key_end], "\"a\"");
        assert_eq!(&content[a.value.start..a.value.end], "[1, \"x\"]");
        let b = doc.root.value.get("b").unwrap();
        let c = b.value.value.get("c").unwrap();
        assert_eq!(&content[c.value.start..c.value.end], "true");
        assert_eq!(c.value.value, JsonValue::Bool(true));
    }

    #[test]
    fn duplicate_keys_are_kept_and_last_wins() {
        let content = r#"{"k": 1, "k": "two"}"#;
        let doc = strict(content);
        let members = doc.root.value.as_object().unwrap();
        assert_eq!(members.len(), 2);
        assert_eq!(
            doc.root.value.get("k").unwrap().value.value,
            JsonValue::String("two".into())
        );
    }

    #[test]
    fn escaped_keys_decode_and_spans_stay_on_source() {
        let content = r#"{"toolSearch.enabled": "é\"x"}"#;
        let doc = strict(content);
        let member = doc.root.value.get("toolSearch.enabled").unwrap();
        assert_eq!(&content[member.value.start..member.value.end], r#""é\"x""#);
    }

    #[test]
    fn jsonc_comments_and_trailing_commas_keep_offsets() {
        let content = "// lead\n{\n  /* a */ \"a\": 1, // tail\n  \"b\": [2,],\n}\n";
        assert!(parse_json_document(content, false).is_err());
        let doc = parse_json_document(content, true).unwrap();
        assert_eq!(doc.dialect, JsonDialect::Jsonc { offset: 0 });
        let strict_error = doc.strict_error.unwrap();
        assert_eq!(strict_error.line, 1);
        let b = doc.root.value.get("b").unwrap();
        assert_eq!(&content[b.value.start..b.value.end], "[2,]");
        assert_eq!(LineIndex::new(content).line_col(b.key_start), (4, 3));
    }

    #[test]
    fn comment_markers_inside_strings_are_not_comments() {
        let content = r#"{"url": "https://x.dev/a,", "p": "/* no */"}"#;
        let doc = strict(content);
        assert_eq!(doc.dialect, JsonDialect::Strict);
        let (blanked, first) = blank_jsonc(content);
        assert_eq!(blanked, content);
        assert!(first.is_none());
    }

    #[test]
    fn real_syntax_errors_are_reported_even_with_comments() {
        let error = parse_json_document("// c\n{\"a\": }", true).unwrap_err();
        assert_eq!(error.line, 2);
        assert!(error.message.contains("expected value"), "{error:?}");
        let error = parse_json_document("{\"a\": 1,", false).unwrap_err();
        assert!(!error.message.contains(" at line "), "{error:?}");
    }

    #[test]
    fn out_of_range_numbers_are_syntax_errors_like_serde_json() {
        assert!(parse_json_document(r#"{"n": 1e400}"#, true).is_err());
    }

    #[test]
    fn line_index_counts_chars() {
        let content = "é\n\"x\"";
        let index = LineIndex::new(content);
        assert_eq!(index.line_col(0), (1, 1));
        assert_eq!(index.line_col(2), (1, 2));
        assert_eq!(index.line_col(3), (2, 1));
        assert_eq!(index.line_col(999), (2, 4));
    }

    #[test]
    fn closest_name_suggests_typos_only() {
        let known = ["command", "args", "env", "disabledTools"];
        assert_eq!(closest_name("comand", &known), Some("command"));
        assert_eq!(closest_name("evn", &known), Some("env"));
        assert_eq!(closest_name("ARGS", &known), Some("args"));
        assert_eq!(closest_name("disabledTool", &known), Some("disabledTools"));
        assert_eq!(closest_name("transport", &known), None);
        assert_eq!(closest_name("description", &known), None);
    }

    #[test]
    fn unterminated_block_comment_is_an_error() {
        assert!(parse_json_document("{} /* open", true).is_err());
    }
}
