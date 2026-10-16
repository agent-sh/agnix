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
    // Documented compatibility aliases.
    "AgentSpawn",
    "agentSpawn",
    "userPromptSubmit",
    "preToolUse",
    "postToolUse",
    "stop",
];

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
