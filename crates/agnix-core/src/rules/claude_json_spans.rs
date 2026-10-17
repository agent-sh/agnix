//! Locate keys and values in Claude Code JSON files (settings, hooks,
//! plugin manifests, marketplaces, `.mcp.json`) so diagnostics point at the
//! offending key or value instead of line 1.
//!
//! Built on the span-preserving tree in `schemas::kiro_json`, which is a
//! generic serde_json-backed JSON tree; only strict JSON is accepted here,
//! because Claude Code reads these files with a strict JSON parser.

use crate::schemas::kiro_json::{JsonNode, LineIndex, parse_json_document};

/// One step of a path into a JSON document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Seg<'a> {
    Key(&'a str),
    Index(usize),
}

/// Byte-offset to (line, column) resolver for a parsed JSON document.
pub(crate) struct JsonLocator<'a> {
    root: Option<JsonNode>,
    lines: LineIndex<'a>,
}

impl<'a> JsonLocator<'a> {
    pub(crate) fn new(content: &'a str) -> Self {
        Self {
            root: parse_json_document(content, false)
                .ok()
                .map(|document| document.root),
            lines: LineIndex::new(content),
        }
    }

    fn node(&self, path: &[Seg<'_>]) -> Option<&JsonNode> {
        let mut node = self.root.as_ref()?;
        for seg in path {
            node = match seg {
                Seg::Key(key) => &node.value.get(key)?.value,
                Seg::Index(index) => node.value.as_array()?.get(*index)?,
            };
        }
        Some(node)
    }

    /// Byte span of the value at `path`.
    pub(crate) fn value_span(&self, path: &[Seg<'_>]) -> Option<(usize, usize)> {
        self.node(path).map(|node| (node.start, node.end))
    }

    /// Byte span of the key of the member at `path` (the last segment must be
    /// a key); for an array element, the element itself.
    pub(crate) fn key_span(&self, path: &[Seg<'_>]) -> Option<(usize, usize)> {
        match path.split_last()? {
            (Seg::Key(key), parent) => {
                let member = self.node(parent)?.value.get(key)?;
                Some((member.key_start, member.key_end))
            }
            (Seg::Index(_), _) => self.value_span(path),
        }
    }

    /// (line, column) of the value at `path`, falling back to the nearest
    /// ancestor that exists and then to line 1.
    pub(crate) fn value_pos(&self, path: &[Seg<'_>]) -> (usize, usize) {
        for len in (0..=path.len()).rev() {
            if let Some((start, _)) = self.value_span(&path[..len]) {
                return self.lines.line_col(start);
            }
        }
        (1, 1)
    }

    /// (line, column) of the key at `path`, falling back like [`Self::value_pos`].
    pub(crate) fn key_pos(&self, path: &[Seg<'_>]) -> (usize, usize) {
        match self.key_span(path) {
            Some((start, _)) => self.lines.line_col(start),
            None => self.value_pos(path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_at_keys_and_values() {
        let content = "{\n  \"hooks\": {\n    \"Stop\": [\n      {\"hooks\": [{\"type\": \"command\"}]}\n    ]\n  }\n}\n";
        let locator = JsonLocator::new(content);
        let path = [
            Seg::Key("hooks"),
            Seg::Key("Stop"),
            Seg::Index(0),
            Seg::Key("hooks"),
            Seg::Index(0),
            Seg::Key("type"),
        ];
        assert_eq!(locator.key_pos(&path), (4, 19));
        assert_eq!(locator.value_pos(&path), (4, 27));
        assert_eq!(locator.key_pos(&[Seg::Key("hooks")]), (2, 3));
    }

    #[test]
    fn falls_back_to_nearest_ancestor() {
        let content = "{\n  \"a\": {\"b\": 1}\n}";
        let locator = JsonLocator::new(content);
        assert_eq!(
            locator.value_pos(&[Seg::Key("a"), Seg::Key("missing")]),
            (2, 8)
        );
        assert_eq!(
            JsonLocator::new("not json").value_pos(&[Seg::Key("a")]),
            (1, 1)
        );
    }
}
