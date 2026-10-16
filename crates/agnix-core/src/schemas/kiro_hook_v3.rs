//! Kiro CLI V3 standalone hook file schema helpers.
//!
//! Covers `.kiro/hooks/*.json` documents introduced by CLI V3 / IDE 1.0:
//! `{ "version": "v1", "hooks": [ { name, trigger, matcher?, action, timeout?,
//! enabled?, description?, confirm? } ] }`.
//!
//! Legacy `.kiro/hooks/*.kiro.hook` payloads stay in [`crate::schemas::kiro_hook`].

use crate::schemas::common::{ParseError, parse_json_with_raw};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Valid V3 standalone-hook trigger names (PascalCase canonical form).
///
/// The camelCase CLI 2.x names (`agentSpawn`, `userPromptSubmit`, `preToolUse`,
/// `postToolUse`, `stop`) remain accepted as compatibility aliases inside
/// agent profiles; `/upgrade-agent` keeps emitting them. `AgentSpawn` is an
/// explicitly documented alias for `SessionStart`.
#[allow(dead_code)] // schema constant used by downstream validators
pub const VALID_KIRO_V3_HOOK_TRIGGERS: &[&str] = &[
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "PostFileCreate",
    "PostFileSave",
    "PostFileDelete",
    "PreTaskExec",
    "PostTaskExec",
    "Stop",
    "SessionEnd",
    "Manual",
    // Documented compatibility aliases.
    "AgentSpawn",
    "agentSpawn",
    "userPromptSubmit",
    "preToolUse",
    "postToolUse",
    "stop",
];

/// Compatibility names explicitly accepted by V3, not arbitrary IDE migration
/// spellings. Keep migration-only aliases out until runtime support is documented.
/// Sources: hook types and custom-agent configuration reference.
pub fn canonical_trigger(trigger: &str) -> Option<&'static str> {
    match trigger {
        "AgentSpawn" | "agentSpawn" => Some("SessionStart"),
        "userPromptSubmit" => Some("UserPromptSubmit"),
        "preToolUse" => Some("PreToolUse"),
        "postToolUse" => Some("PostToolUse"),
        "stop" => Some("Stop"),
        _ => None,
    }
}

/// An unambiguous value span with a cached 1-based character location.
#[derive(Debug, Clone, Copy)]
pub(crate) struct JsonValueSpan {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

/// All editable V3 values, indexed once per document rather than once per hook.
#[derive(Default)]
pub(crate) struct KiroV3HookSpans {
    pub version: Option<JsonValueSpan>,
    pub triggers: Vec<Option<JsonValueSpan>>,
}

#[cfg(test)]
thread_local! {
    pub(crate) static SPAN_INDEX_BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl KiroV3HookSpans {
    /// Input has already passed JSON parsing. Walk siblings using serde's value
    /// parser, preserving decoded duplicate keys (unlike a Value object map).
    /// Only root -> hooks -> entry is descended into, so each byte is parsed at
    /// most three times regardless of the number of hooks. Location accounting
    /// advances monotonically too; it never rescans the prefix for each hint.
    pub fn new(content: &str) -> Self {
        #[cfg(test)]
        SPAN_INDEX_BUILDS.with(|builds| builds.set(builds.get() + 1));

        let mut spans = Self::default();
        let mut version_seen = false;
        let mut hooks_seen = false;
        let mut location_offset = 0;
        let mut line = 1;
        let mut column = 1;
        let mut located = |start, end| {
            for ch in content[location_offset..start].chars() {
                if ch == '\n' {
                    line += 1;
                    column = 1;
                } else {
                    column += 1;
                }
            }
            location_offset = start;
            JsonValueSpan {
                start,
                end,
                line,
                column,
            }
        };
        let parsed = visit_json_children(content, 0, &mut |key, start, end| {
            match key {
                Some("version") => {
                    spans.version = if version_seen {
                        None
                    } else {
                        Some(located(start, end))
                    };
                    version_seen = true;
                }
                Some("hooks") => {
                    if hooks_seen {
                        // A repeated container invalidates every descendant,
                        // including escaped spellings such as "ho\u006fks".
                        spans.triggers.clear();
                        return;
                    }
                    hooks_seen = true;
                    if !content[start..end].starts_with('[') {
                        return;
                    }
                    visit_json_children(&content[start..end], start, &mut |_, start, end| {
                        let mut trigger = None;
                        let mut trigger_seen = false;
                        if content[start..end].starts_with('{') {
                            visit_json_children(
                                &content[start..end],
                                start,
                                &mut |key, start, end| {
                                    if key == Some("trigger") {
                                        trigger = if trigger_seen {
                                            None
                                        } else {
                                            Some(located(start, end))
                                        };
                                        trigger_seen = true;
                                    }
                                },
                            );
                        }
                        spans.triggers.push(trigger);
                    });
                }
                _ => {}
            }
        });
        if parsed.is_none() {
            return Self::default();
        }
        spans
    }
}

/// Visit immediate children using the same serde-backed sibling parsing as the
/// former single-path locator. Do not search raw text or implement a second
/// string/escape parser. Absolute offsets remain valid for UTF-8 and escapes.
fn visit_json_children(
    content: &str,
    base: usize,
    visitor: &mut impl FnMut(Option<&str>, usize, usize),
) -> Option<()> {
    fn first_value<'de, T: Deserialize<'de>>(content: &'de str) -> Option<(T, usize)> {
        let mut stream = serde_json::Deserializer::from_str(content).into_iter::<T>();
        let value = stream.next()?.ok()?;
        Some((value, stream.byte_offset()))
    }
    let trimmed = content.trim_start();
    let base = base + content.len() - trimmed.len();
    let object = trimmed.starts_with('{');
    if !object && !trimmed.starts_with('[') {
        return None;
    }
    let mut pos = 1;
    loop {
        while trimmed
            .as_bytes()
            .get(pos)
            .is_some_and(u8::is_ascii_whitespace)
        {
            pos += 1;
        }
        if matches!(trimmed.as_bytes().get(pos), Some(b'}' | b']')) {
            break;
        }
        let key = if object {
            let (key, length) = first_value::<String>(trimmed.get(pos..)?)?;
            pos += length;
            while trimmed
                .as_bytes()
                .get(pos)
                .is_some_and(u8::is_ascii_whitespace)
            {
                pos += 1;
            }
            if trimmed.as_bytes().get(pos) != Some(&b':') {
                return None;
            }
            pos += 1;
            Some(key)
        } else {
            None
        };
        while trimmed
            .as_bytes()
            .get(pos)
            .is_some_and(u8::is_ascii_whitespace)
        {
            pos += 1;
        }
        let (_, length) = first_value::<serde::de::IgnoredAny>(trimmed.get(pos..)?)?;
        visitor(key.as_deref(), base + pos, base + pos + length);
        pos += length;
        while trimmed
            .as_bytes()
            .get(pos)
            .is_some_and(u8::is_ascii_whitespace)
        {
            pos += 1;
        }
        if trimmed.as_bytes().get(pos) == Some(&b',') {
            pos += 1;
        } else {
            break;
        }
    }
    Some(())
}

/// Documented `version` values for the standalone hook document.
#[allow(dead_code)] // schema constant used by downstream validators
pub const VALID_KIRO_V3_HOOK_DOC_VERSIONS: &[&str] = &["v1"];

/// Parsed Kiro V3 standalone hook document.
#[allow(dead_code)] // schema-level API; consumed by validator layer
#[derive(Debug, Clone)]
pub struct ParsedKiroV3HookDoc {
    pub doc: Option<KiroV3HookDoc>,
    pub parse_error: Option<ParseError>,
    pub raw_value: Option<Value>,
}

/// `.kiro/hooks/*.json` document schema.
#[allow(dead_code)] // schema-level API; consumed by validator layer
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KiroV3HookDoc {
    pub version: Option<String>,
    pub hooks: Option<Vec<KiroV3HookEntry>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// One hook definition inside a standalone document or an agent profile
/// `hooks` array.
#[allow(dead_code)] // schema-level API; consumed by validator layer
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroV3HookEntry {
    pub name: Option<String>,
    pub description: Option<String>,
    pub trigger: Option<String>,
    pub matcher: Option<String>,
    pub action: Option<KiroV3HookAction>,
    pub timeout: Option<Value>,
    pub enabled: Option<bool>,
    pub confirm: Option<Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// `action` block: `command` shell actions and `agent` prompt actions.
#[allow(dead_code)] // schema-level API; consumed by validator layer
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroV3HookAction {
    pub r#type: Option<String>,
    pub command: Option<String>,
    pub prompt: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Parse a Kiro V3 standalone hook JSON document.
#[allow(dead_code)] // schema-level API; consumed by validator layer
pub fn parse_kiro_v3_hook_doc(content: &str) -> ParsedKiroV3HookDoc {
    let (doc, parse_error, raw_value) = parse_json_with_raw::<KiroV3HookDoc>(content);
    ParsedKiroV3HookDoc {
        doc,
        parse_error,
        raw_value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_decode_keys_and_ignore_nested_lookalikes() {
        let content = r#"{
          "note": "é🙂 \\\"trigger\\\": \\\"stop\\\"",
          "nested": {"hooks": [{"trigger": "stop"}], "version": "1"},
          "ho\u006fks": [
            {"name": "é🙂", "trig\u0067er": "agent\u0053pawn", "action": {"trigger": "stop"}},
            {"name": "no trigger"},
            {"trigger": "stop"}
          ],
          "ver\u0073ion": "1.0"
        }"#;
        let _: Value = serde_json::from_str(content).unwrap();
        let spans = KiroV3HookSpans::new(content);
        assert_eq!(spans.triggers.len(), 3);
        assert!(spans.triggers[1].is_none());
        for (span, expected) in [
            (spans.version.unwrap(), r#""1.0""#),
            (spans.triggers[0].unwrap(), r#""agent\u0053pawn""#),
            (spans.triggers[2].unwrap(), r#""stop""#),
        ] {
            assert_eq!(&content[span.start..span.end], expected);
            assert_eq!(
                (span.line, span.column),
                crate::rules::line_col_at_offset(content, span.start)
            );
        }
    }

    #[test]
    fn spans_reject_duplicate_decoded_keys_and_containers() {
        for content in [
            r#"{"hooks":[],"hooks":[{"trigger":"stop"}]}"#,
            r#"{"hooks":[{"trigger":"stop"}],"ho\u006fks":[]}"#,
            r#"{"ho\u006fks":[],"hooks":[],"hooks":[{"trigger":"stop"}]}"#,
            r#"{"hooks":null,"hooks":[{"trigger":"stop"}]}"#,
        ] {
            assert!(
                KiroV3HookSpans::new(content).triggers.is_empty(),
                "{content}"
            );
        }
        let spans = KiroV3HookSpans::new(
            r#"{
            "version":"1", "ver\u0073ion":"1.0", "version":"1",
            "hooks":[
                {"trigger":"stop", "trig\u0067er":"agentSpawn", "trigger":"stop"},
                {"trigger":"stop", "note":1, "note":2}
            ]
        }"#,
        );
        assert!(spans.version.is_none());
        assert!(spans.triggers[0].is_none());
        assert!(
            spans.triggers[1].is_some(),
            "unrelated duplicate keys are not ambiguous"
        );
    }

    #[test]
    fn parse_valid_standalone_hook_doc() {
        let parsed = parse_kiro_v3_hook_doc(
            r#"{
  "version": "v1",
  "hooks": [
    {
      "name": "lint-on-save",
      "trigger": "PostFileSave",
      "matcher": "\\.(ts|tsx)$",
      "action": { "type": "command", "command": "npx eslint --fix" },
      "timeout": 30,
      "enabled": true
    }
  ]
}"#,
        );

        assert!(parsed.parse_error.is_none());
        let doc = parsed.doc.expect("doc should parse");
        assert_eq!(doc.version.as_deref(), Some("v1"));
        let hooks = doc.hooks.expect("hooks array");
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].trigger.as_deref(), Some("PostFileSave"));
        assert_eq!(
            hooks[0].action.as_ref().and_then(|a| a.command.as_deref()),
            Some("npx eslint --fix")
        );
    }

    #[test]
    fn parse_agent_prompt_action() {
        let parsed = parse_kiro_v3_hook_doc(
            r#"{
  "version": "v1",
  "hooks": [
    { "name": "ctx", "trigger": "UserPromptSubmit", "action": { "type": "agent", "prompt": "Add context" } }
  ]
}"#,
        );

        assert!(parsed.parse_error.is_none());
        let doc = parsed.doc.expect("doc should parse");
        let hooks = doc.hooks.expect("hooks array");
        let action = hooks[0].action.as_ref().expect("action");
        assert_eq!(action.r#type.as_deref(), Some("agent"));
        assert_eq!(action.prompt.as_deref(), Some("Add context"));
    }

    #[test]
    fn parse_invalid_doc_reports_error_location() {
        let parsed = parse_kiro_v3_hook_doc(r#"{"version":"v1","hooks":[}"#);
        let error = parsed.parse_error.expect("expected parse error");
        assert!(error.line > 0);
        assert!(error.column > 0);
    }

    #[test]
    fn parse_confirm_block_with_confirm_command() {
        let parsed = parse_kiro_v3_hook_doc(
            r#"{
  "version": "v1",
  "hooks": [
    {
      "name": "submit",
      "trigger": "Stop",
      "action": { "type": "command", "command": "./submit.sh" },
      "confirm": {
        "question": "Submit?",
        "confirmCommand": "./confirm-options.sh",
        "options": [
          { "id": "submit", "label": "Yes", "run": true },
          { "id": "dismiss", "label": "No", "run": false }
        ]
      }
    }
  ]
}"#,
        );

        assert!(parsed.parse_error.is_none());
        let doc = parsed.doc.expect("doc should parse");
        assert!(doc.hooks.expect("hooks array")[0].confirm.is_some());
    }
}
