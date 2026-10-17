//! TOML helpers for the Gemini CLI custom command and policy validators.
//!
//! Gemini CLI reads custom commands (`FileCommandLoader.ts`, `toml.parse`)
//! and policy files from TOML. The validators need the decoded values and
//! the byte span of every key and value, to point diagnostics at them:
//! `toml::de::DeTable` keeps both. A string value's span includes its
//! quotes; the span of an `[[array-of-tables]]` entry is its header.

use std::ops::Range;
use toml::Spanned;
use toml::de::{DeString, DeTable, DeValue};

/// A TOML syntax error: the parser's message and the byte offset it points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TomlSyntaxError {
    pub message: String,
    pub offset: usize,
}

/// A parsed TOML document.
pub(crate) type TomlDocument<'i> = Spanned<DeTable<'i>>;

/// Parse a whole TOML document with spans.
pub(crate) fn parse_toml_document(content: &str) -> Result<TomlDocument<'_>, TomlSyntaxError> {
    DeTable::parse(content).map_err(|error| TomlSyntaxError {
        message: error.message().trim().to_string(),
        offset: error.span().map_or(0, |span| span.start),
    })
}

/// The key and value of `key` in `table`.
pub(crate) fn toml_entry<'a, 'i>(
    table: &'a DeTable<'i>,
    key: &str,
) -> Option<(&'a Spanned<DeString<'i>>, &'a Spanned<DeValue<'i>>)> {
    table.get_key_value(key)
}

/// The value of an integer, when it fits in an `i128`.
pub(crate) fn toml_integer(value: &DeValue<'_>) -> Option<i128> {
    match value {
        DeValue::Integer(integer) => i128::from_str_radix(integer.as_str(), integer.radix()).ok(),
        _ => None,
    }
}

/// The source offset of the `n`-th (0-based) occurrence of `needle` inside
/// the source text of a string value (`span` includes the quotes).
///
/// Used to point inside a TOML string: the `n`-th occurrence of an ASCII
/// marker such as `!{` in the decoded string is the `n`-th occurrence in the
/// source, because no escape sequence spells or hides the marker. Returns
/// `None` when the source has fewer occurrences (for example a `!{`
/// escape), so callers fall back to the value span.
pub(crate) fn nth_occurrence_in_span(
    content: &str,
    span: Range<usize>,
    needle: &str,
    n: usize,
) -> Option<usize> {
    let raw = content.get(span.clone())?;
    raw.match_indices(needle)
        .nth(n)
        .map(|(offset, _)| span.start + offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_cover_keys_values_and_quotes() {
        let content = "description = \"Hi\"\nprompt = '''\nrun !{ls}'''\n";
        let doc = parse_toml_document(content).unwrap();
        let (key, value) = toml_entry(doc.get_ref(), "prompt").unwrap();
        assert_eq!(&content[key.span()], "prompt");
        assert_eq!(&content[value.span()], "'''\nrun !{ls}'''");
        let offset = nth_occurrence_in_span(content, value.span(), "!{", 0).unwrap();
        assert_eq!(&content[offset..offset + 2], "!{");
        assert_eq!(nth_occurrence_in_span(content, value.span(), "!{", 1), None);
        assert!(toml_entry(doc.get_ref(), "missing").is_none());
    }

    #[test]
    fn syntax_errors_carry_an_offset() {
        let error = parse_toml_document("a = 1\na = 2\n").unwrap_err();
        assert_eq!(error.offset, 6);
        assert!(error.message.contains("duplicate key"), "{error:?}");
        let error = parse_toml_document("prompt = \n").unwrap_err();
        assert!(!error.message.is_empty());
    }

    #[test]
    fn integers_decode_with_radix_and_sign() {
        let doc = parse_toml_document("a = 1_000\nb = -5\nc = 0x10\nd = 1.5\n").unwrap();
        let get = |key| toml_integer(toml_entry(doc.get_ref(), key).unwrap().1.get_ref());
        assert_eq!(get("a"), Some(1000));
        assert_eq!(get("b"), Some(-5));
        assert_eq!(get("c"), Some(16));
        assert_eq!(get("d"), None);
    }
}
