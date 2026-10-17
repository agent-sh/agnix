//! Source positions for keys and values in TOML and JSON configuration, so
//! Codex and OpenCode diagnostics point at the offending key or value.
//!
//! TOML spans come from `toml::de::DeTable`, which keeps the byte span of
//! every key and value. JSON spans come from the span-preserving tree in
//! `schemas::kiro_json`, which also accepts JSONC (comments and trailing
//! commas) without moving any offset.

use crate::schemas::kiro_json::{JsonMember, JsonNode, JsonValue};
use std::ops::Range;
use toml::de::{DeTable, DeValue};

/// One step in a path into a configuration document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Seg<'a> {
    Key(&'a str),
    Index(usize),
}

/// A parsed TOML document with spans.
pub(crate) struct TomlSpans<'i> {
    root: DeTable<'i>,
}

impl<'i> TomlSpans<'i> {
    /// Parse `content`; `None` when it is not valid TOML.
    pub fn parse(content: &'i str) -> Option<Self> {
        let root = DeTable::parse(content).ok()?.into_inner();
        Some(Self { root })
    }

    /// Byte span of the last key in `path` (for an index, the array item).
    pub fn key_span(&self, path: &[Seg<'_>]) -> Option<Range<usize>> {
        let (last, parent) = path.split_last()?;
        match last {
            Seg::Key(key) => {
                let table = match parent {
                    [] => &self.root,
                    parent => match self.value(parent)? {
                        DeValue::Table(table) => table,
                        _ => return None,
                    },
                };
                table
                    .iter()
                    .find(|(name, _)| name.get_ref().as_ref() == *key)
                    .map(|(name, _)| name.span())
            }
            Seg::Index(index) => match self.value(parent)? {
                DeValue::Array(items) => items.get(*index).map(|item| item.span()),
                _ => None,
            },
        }
    }

    /// Start of the key at `path`, falling back to the nearest ancestor key
    /// that has a position (inline tables and arrays of tables can lose the
    /// span of an implicit parent).
    pub fn offset(&self, path: &[Seg<'_>]) -> Option<usize> {
        (1..=path.len())
            .rev()
            .find_map(|len| self.key_span(&path[..len]))
            .map(|span| span.start)
    }

    fn value(&self, path: &[Seg<'_>]) -> Option<&DeValue<'i>> {
        let mut current: Option<&DeValue<'i>> = None;
        for seg in path {
            current = Some(match (current, seg) {
                (None, Seg::Key(key)) => self
                    .root
                    .iter()
                    .find(|(name, _)| name.get_ref().as_ref() == *key)?
                    .1
                    .get_ref(),
                (Some(DeValue::Table(table)), Seg::Key(key)) => table
                    .iter()
                    .find(|(name, _)| name.get_ref().as_ref() == *key)?
                    .1
                    .get_ref(),
                (Some(DeValue::Array(items)), Seg::Index(index)) => items.get(*index)?.get_ref(),
                _ => return None,
            });
        }
        current
    }
}

/// The member at the end of `path` in a JSON tree (for a key segment).
pub(crate) fn json_member<'a>(root: &'a JsonNode, path: &[Seg<'_>]) -> Option<&'a JsonMember> {
    let (last, parent) = path.split_last()?;
    let Seg::Key(key) = last else {
        return None;
    };
    json_node(root, parent)?.value.get(key)
}

/// The node at `path` in a JSON tree.
pub(crate) fn json_node<'a>(root: &'a JsonNode, path: &[Seg<'_>]) -> Option<&'a JsonNode> {
    let mut current = root;
    for seg in path {
        current = match (seg, &current.value) {
            (Seg::Key(key), JsonValue::Object(_)) => &current.value.get(key)?.value,
            (Seg::Index(index), JsonValue::Array(items)) => items.get(*index)?,
            _ => return None,
        };
    }
    Some(current)
}

/// Start of the key at `path` (or of the array item for an index segment),
/// falling back to the nearest ancestor with a position.
pub(crate) fn json_offset(root: &JsonNode, path: &[Seg<'_>]) -> Option<usize> {
    (1..=path.len()).rev().find_map(|len| {
        let prefix = &path[..len];
        match prefix.last()? {
            Seg::Key(_) => json_member(root, prefix).map(|member| member.key_start),
            Seg::Index(_) => json_node(root, prefix).map(|node| node.start),
        }
    })
}

/// 1-based (line, column) of a byte offset; column counts characters.
pub(crate) fn line_col(content: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(content.len());
    let before = &content[..floor_char_boundary(content, offset)];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    (line, before[line_start..].chars().count() + 1)
}

/// 1-based (line, column) of the block-style YAML mapping key at `path`
/// (each segment one nesting level deeper), or `None` when it is not found.
/// Quoted keys are matched too; flow mappings are not.
pub(crate) fn yaml_key_pos(content: &str, path: &[&str]) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let mut parent_indent: Option<usize> = None;
    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.len() - trimmed.len();
        match parent_indent {
            Some(parent) if indent <= parent => return None,
            None if indent > 0 => continue,
            _ => {}
        }
        let target = *path.get(depth)?;
        let is_key = [
            target.to_string(),
            format!("\"{target}\""),
            format!("'{target}'"),
        ]
        .iter()
        .any(|key| {
            trimmed
                .strip_prefix(key.as_str())
                .is_some_and(|rest| rest.trim_start().starts_with(':'))
        });
        if is_key {
            if depth + 1 == path.len() {
                return Some((index + 1, line[..indent].chars().count() + 1));
            }
            depth += 1;
            parent_indent = Some(indent);
        }
    }
    None
}

fn floor_char_boundary(content: &str, mut offset: usize) -> usize {
    while offset > 0 && !content.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::kiro_json::parse_json_document;

    #[test]
    fn toml_spans_find_nested_keys_and_array_items() {
        let content = "model = \"x\"\n\n[[hooks.SessionEnd]]\n[[hooks.SessionEnd.hooks]]\ntype = \"command\"\ntimeout = 60\n";
        let spans = TomlSpans::parse(content).unwrap();
        let path = [
            Seg::Key("hooks"),
            Seg::Key("SessionEnd"),
            Seg::Index(0),
            Seg::Key("hooks"),
            Seg::Index(0),
            Seg::Key("timeout"),
        ];
        let offset = spans.offset(&path).unwrap();
        assert_eq!(line_col(content, offset), (6, 1));
        assert_eq!(
            line_col(content, spans.offset(&[Seg::Key("model")]).unwrap()),
            (1, 1)
        );
        assert!(TomlSpans::parse("a = [").is_none());
    }

    #[test]
    fn json_paths_follow_objects_and_arrays_in_jsonc() {
        let content = "{\n  // c\n  \"a\": [\n    {\"b\": 1,},\n  ],\n}\n";
        let doc = parse_json_document(content, true).unwrap();
        let path = [Seg::Key("a"), Seg::Index(0), Seg::Key("b")];
        let offset = json_offset(&doc.root, &path).unwrap();
        assert_eq!(line_col(content, offset), (4, 6));
        assert!(json_member(&doc.root, &[Seg::Key("missing")]).is_none());
        // Falls back to the nearest ancestor.
        let fallback = json_offset(&doc.root, &[Seg::Key("a"), Seg::Index(0), Seg::Key("zz")]);
        assert_eq!(line_col(content, fallback.unwrap()), (4, 5));
    }

    #[test]
    fn yaml_key_positions_follow_nesting() {
        let yaml = "name: x\npermission:\n  bash:\n    \"git *\": allow\nbash: top\n";
        assert_eq!(yaml_key_pos(yaml, &["name"]), Some((1, 1)));
        assert_eq!(yaml_key_pos(yaml, &["permission", "bash"]), Some((3, 3)));
        assert_eq!(
            yaml_key_pos(yaml, &["permission", "bash", "git *"]),
            Some((4, 5))
        );
        assert_eq!(yaml_key_pos(yaml, &["bash"]), Some((5, 1)));
        assert_eq!(yaml_key_pos(yaml, &["missing"]), None);
    }

    #[test]
    fn line_col_counts_characters() {
        assert_eq!(line_col("é = 1\nab", 7), (2, 1));
        assert_eq!(line_col("éx", 2), (1, 2));
    }
}
