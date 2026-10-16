//! Kiro IDE hook file schema helpers.
//!
//! Covers `.kiro/hooks/*.kiro.hook` JSON payloads.

use crate::schemas::common::{ParseError, parse_json_with_raw};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Valid Kiro IDE hook event names.
#[allow(dead_code)] // schema constant used by downstream validators
pub const VALID_KIRO_HOOK_EVENTS: &[&str] = &[
    "fileEdited",
    "fileCreate",
    "fileDelete",
    "promptSubmit",
    "agentStop",
    "preToolUse",
    "postToolUse",
    "manual",
    "userTriggered",
    "sessionStart",
    "agentSpawn",
    "fileCreated",
    "fileDeleted",
    "preTaskExecution",
    "postTaskExecution",
];

/// Parsed Kiro hook document.
#[allow(dead_code)] // schema-level API; consumed by validator layer
#[derive(Debug, Clone)]
pub struct ParsedKiroHook {
    pub hook: Option<KiroIdeHook>,
    pub parse_error: Option<ParseError>,
    pub raw_value: Option<Value>,
}

/// Kiro IDE hook structure.
#[allow(dead_code)] // schema-level API; consumed by validator layer
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroIdeHook {
    pub title: Option<String>,
    pub description: Option<String>,
    pub event: Option<String>,
    pub when: Option<KiroHookWhen>,
    pub patterns: Option<Vec<String>>,
    pub tool_types: Option<Vec<String>>,
    pub then: Option<KiroHookAction>,
    pub run_command: Option<String>,
    pub ask_agent: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Legacy IDE event selector (`when.type`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroHookWhen {
    pub r#type: Option<String>,
    pub patterns: Option<Vec<String>>,
    pub tool_types: Option<Vec<String>>,
}

/// Optional nested action block used by some hook representations.
#[allow(dead_code)] // schema-level API; consumed by validator layer
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroHookAction {
    pub command: Option<String>,
    pub prompt: Option<String>,
    pub run_command: Option<String>,
    pub ask_agent: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl KiroIdeHook {
    pub fn effective_event(&self) -> Option<&str> {
        self.when
            .as_ref()
            .and_then(|w| w.r#type.as_deref())
            .or(self.event.as_deref())
    }

    pub fn effective_patterns(&self) -> Option<&[String]> {
        self.when
            .as_ref()
            .and_then(|w| w.patterns.as_deref())
            .or(self.patterns.as_deref())
    }

    pub fn effective_tool_types(&self) -> Option<&[String]> {
        self.when
            .as_ref()
            .and_then(|w| w.tool_types.as_deref().or(w.patterns.as_deref()))
            .or(self.tool_types.as_deref())
    }

    /// Resolve an effective command from top-level or nested `then`.
    #[allow(dead_code)] // helper for downstream validators
    pub fn effective_run_command(&self) -> Option<&str> {
        self.run_command.as_deref().or_else(|| {
            let action = self.then.as_ref()?;
            action.run_command.as_deref().or(action.command.as_deref())
        })
    }

    /// Resolve an effective `askAgent` target from top-level or nested `then`.
    #[allow(dead_code)] // helper for downstream validators
    pub fn effective_ask_agent(&self) -> Option<&str> {
        self.ask_agent.as_deref().or_else(|| {
            let action = self.then.as_ref()?;
            action.ask_agent.as_deref().or(action.prompt.as_deref())
        })
    }

    /// True if either supported action is configured.
    #[allow(dead_code)] // helper for downstream validators
    pub fn has_action(&self) -> bool {
        self.effective_run_command()
            .is_some_and(|command| !command.trim().is_empty())
            || self
                .effective_ask_agent()
                .is_some_and(|agent| !agent.trim().is_empty())
    }
}

/// Parse Kiro IDE hook JSON into typed schema and raw value.
#[allow(dead_code)] // schema-level API; consumed by validator layer
pub fn parse_kiro_hook(content: &str) -> ParsedKiroHook {
    let (hook, parse_error, raw_value) = parse_json_with_raw::<KiroIdeHook>(content);
    ParsedKiroHook {
        hook,
        parse_error,
        raw_value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_hook_top_level_actions() {
        let parsed = parse_kiro_hook(
            r#"{
  "event": "fileEdited",
  "patterns": ["**/*.md"],
  "runCommand": "echo changed"
}"#,
        );

        assert!(parsed.parse_error.is_none());
        let hook = parsed.hook.expect("hook should parse");
        assert_eq!(hook.event.as_deref(), Some("fileEdited"));
        assert_eq!(hook.effective_run_command(), Some("echo changed"));
        assert!(hook.has_action());
    }

    #[test]
    fn parse_valid_hook_nested_then_actions() {
        let parsed = parse_kiro_hook(
            r#"{
  "event": "promptSubmit",
  "then": {
    "askAgent": "review-agent"
  }
}"#,
        );

        assert!(parsed.parse_error.is_none());
        let hook = parsed.hook.expect("hook should parse");
        assert_eq!(hook.effective_ask_agent(), Some("review-agent"));
        assert!(hook.has_action());
    }

    #[test]
    fn parse_invalid_hook_reports_error_location() {
        let parsed = parse_kiro_hook(r#"{"event":"fileEdited","patterns":[}"#);
        let error = parsed.parse_error.expect("expected parse error");
        assert!(error.line > 0);
        assert!(error.column > 0);
    }

    #[test]
    fn blank_actions_are_not_treated_as_present() {
        let parsed = parse_kiro_hook(
            r#"{
  "event": "promptSubmit",
  "runCommand": "   ",
  "then": {
    "askAgent": "   "
  }
}"#,
        );

        assert!(parsed.parse_error.is_none());
        let hook = parsed.hook.expect("hook should parse");
        assert!(!hook.has_action());
    }
}
