//! Hooks schema (Claude Code hooks)

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Full settings.json schema (for parsing hooks from settings files)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingsSchema {
    #[serde(default)]
    pub hooks: HashMap<String, Vec<HookMatcher>>,
    #[serde(flatten)]
    pub _extra: HashMap<String, Value>,
}

/// Hooks configuration schema (standalone .claude/hooks.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksSchema {
    pub hooks: HashMap<String, Vec<HookMatcher>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookMatcher {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    pub hooks: Vec<Hook>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Hook {
    #[serde(rename = "command")]
    Command {
        #[serde(skip_serializing_if = "Option::is_none")]
        command: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
        /// Conditional expression - hook only runs when this evaluates to true
        #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
        if_condition: Option<String>,
        /// Shell to use for command execution
        #[serde(skip_serializing_if = "Option::is_none")]
        shell: Option<String>,
        /// Status message displayed while hook is running
        #[serde(rename = "statusMessage", skip_serializing_if = "Option::is_none")]
        status_message: Option<String>,
        /// Run hook only once per session
        #[serde(skip_serializing_if = "Option::is_none")]
        once: Option<bool>,
        /// Run hook asynchronously (non-blocking)
        #[serde(rename = "async", skip_serializing_if = "Option::is_none")]
        is_async: Option<bool>,
    },
    #[serde(rename = "prompt")]
    Prompt {
        #[serde(skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
    #[serde(rename = "agent")]
    Agent {
        #[serde(skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
    #[serde(rename = "http")]
    Http {
        /// URL to send the HTTP request to (required)
        url: Option<String>,
        /// HTTP headers (supports `$VAR_NAME` interpolation)
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
        /// Environment variables allowed for interpolation in headers
        #[serde(rename = "allowedEnvVars", skip_serializing_if = "Option::is_none")]
        allowed_env_vars: Option<Vec<String>>,
        /// Request timeout in seconds
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
    },
}

impl SettingsSchema {
    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn from_json(content: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(content)
    }

    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn to_hooks_schema(&self) -> HooksSchema {
        HooksSchema {
            hooks: self.hooks.clone(),
        }
    }
}

impl Hook {
    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn command(&self) -> Option<&str> {
        match self {
            Hook::Command { command, .. } => command.as_deref(),
            Hook::Prompt { .. } | Hook::Agent { .. } | Hook::Http { .. } => None,
        }
    }

    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn prompt(&self) -> Option<&str> {
        match self {
            Hook::Prompt { prompt, .. } | Hook::Agent { prompt, .. } => prompt.as_deref(),
            Hook::Command { .. } | Hook::Http { .. } => None,
        }
    }

    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn is_command(&self) -> bool {
        matches!(self, Hook::Command { .. })
    }

    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn is_prompt(&self) -> bool {
        matches!(self, Hook::Prompt { .. })
    }

    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn is_agent(&self) -> bool {
        matches!(self, Hook::Agent { .. })
    }

    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn is_http(&self) -> bool {
        matches!(self, Hook::Http { .. })
    }

    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn type_name(&self) -> &'static str {
        match self {
            Hook::Command { .. } => "command",
            Hook::Prompt { .. } => "prompt",
            Hook::Agent { .. } => "agent",
            Hook::Http { .. } => "http",
        }
    }
}

impl HooksSchema {
    /// Valid hook event names (case-sensitive)
    pub const VALID_EVENTS: &'static [&'static str] = &[
        "PreToolUse",
        "PermissionRequest",
        "PostToolUse",
        "PostToolUseFailure",
        "Notification",
        "UserPromptSubmit",
        "Stop",
        "SubagentStart",
        "SubagentStop",
        "TeammateIdle",
        "TaskCompleted",
        "PreCompact",
        "PostCompact",
        "Setup",
        "SessionStart",
        "SessionEnd",
        "InstructionsLoaded",
        "ConfigChange",
        "CwdChanged",
        "FileChanged",
        "TaskCreated",
        "WorktreeCreate",
        "WorktreeRemove",
        "Elicitation",
        "ElicitationResult",
        "StopFailure",
    ];

    /// Tool-related events (matcher recommended via CC-HK-003 hint)
    pub const TOOL_EVENTS: &'static [&'static str] = &[
        "PreToolUse",
        "PermissionRequest",
        "PostToolUse",
        "PostToolUseFailure",
    ];

    /// All events that support a matcher field.
    /// Includes tool events plus lifecycle events that accept matchers.
    pub const MATCHER_EVENTS: &'static [&'static str] = &[
        // Tool events
        "PreToolUse",
        "PermissionRequest",
        "PostToolUse",
        "PostToolUseFailure",
        // Lifecycle events with matcher support
        "SessionStart",
        "SessionEnd",
        "Notification",
        "SubagentStart",
        "SubagentStop",
        "PreCompact",
        "PostCompact",
        "ConfigChange",
        "FileChanged",
        "StopFailure",
        "InstructionsLoaded",
        "Elicitation",
        "ElicitationResult",
    ];

    /// Events that support prompt/agent hooks
    pub const PROMPT_EVENTS: &'static [&'static str] = &[
        "PreToolUse",
        "PostToolUse",
        "PostToolUseFailure",
        "PermissionRequest",
        "UserPromptSubmit",
        "Stop",
        "SubagentStop",
        "TaskCompleted",
    ];

    /// Check if an event is a tool event (matcher recommended)
    pub fn is_tool_event(event: &str) -> bool {
        Self::TOOL_EVENTS.contains(&event)
    }

    /// Check if an event supports a matcher field
    pub fn supports_matcher(event: &str) -> bool {
        Self::MATCHER_EVENTS.contains(&event)
    }

    /// Check if an event supports prompt hooks
    pub fn is_prompt_event(event: &str) -> bool {
        Self::PROMPT_EVENTS.contains(&event)
    }

    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn from_json(content: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(content)
    }

    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn validate_events(&self) -> Vec<String> {
        let mut errors = Vec::new();

        for event in self.hooks.keys() {
            if !Self::VALID_EVENTS.contains(&event.as_str()) {
                errors.push(format!(
                    "Unknown hook event '{}', valid events: {:?}",
                    event,
                    Self::VALID_EVENTS
                ));
            }
        }

        errors
    }

    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        errors.extend(self.validate_events());

        for (event, matchers) in &self.hooks {
            for (i, matcher) in matchers.iter().enumerate() {
                if matcher.hooks.is_empty() {
                    errors.push(format!(
                        "Hook event '{}' matcher {} has empty hooks array",
                        event, i
                    ));
                }
            }
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_hooks_map_validates_ok() {
        let schema = HooksSchema {
            hooks: HashMap::new(),
        };
        let errors = schema.validate();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_empty_matchers_array_validates_ok() {
        let mut hooks = HashMap::new();
        hooks.insert("PreToolUse".to_string(), vec![]);
        let schema = HooksSchema { hooks };
        let errors = schema.validate();
        assert!(errors.is_empty(), "Empty matchers array is valid");
    }

    #[test]
    fn test_empty_hooks_array_in_matcher_reports_error() {
        let mut hooks = HashMap::new();
        hooks.insert(
            "PreToolUse".to_string(),
            vec![HookMatcher {
                matcher: Some("Bash".to_string()),
                hooks: vec![],
            }],
        );
        let schema = HooksSchema { hooks };
        let errors = schema.validate();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("empty hooks array"));
    }

    #[test]
    fn test_unknown_event_name_reports_error() {
        let mut hooks = HashMap::new();
        hooks.insert(
            "NonExistentEvent".to_string(),
            vec![HookMatcher {
                matcher: None,
                hooks: vec![Hook::Command {
                    command: Some("echo hi".to_string()),
                    timeout: None,
                    model: None,
                    if_condition: None,
                    shell: None,
                    status_message: None,
                    once: None,
                    is_async: None,
                }],
            }],
        );
        let schema = HooksSchema { hooks };
        let errors = schema.validate_events();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Unknown hook event"));
    }

    #[test]
    fn test_hook_type_name() {
        let cmd = Hook::Command {
            command: Some("echo".to_string()),
            timeout: None,
            model: None,
            if_condition: None,
            shell: None,
            status_message: None,
            once: None,
            is_async: None,
        };
        assert_eq!(cmd.type_name(), "command");
        assert!(cmd.is_command());
        assert!(!cmd.is_prompt());
        assert!(!cmd.is_agent());
        assert!(!cmd.is_http());

        let prompt = Hook::Prompt {
            prompt: Some("summarize".to_string()),
            timeout: None,
            model: None,
        };
        assert_eq!(prompt.type_name(), "prompt");
        assert!(prompt.is_prompt());

        let agent = Hook::Agent {
            prompt: Some("review".to_string()),
            timeout: None,
            model: None,
        };
        assert_eq!(agent.type_name(), "agent");
        assert!(agent.is_agent());

        let http = Hook::Http {
            url: Some("https://example.com".to_string()),
            headers: None,
            allowed_env_vars: None,
            timeout: None,
        };
        assert_eq!(http.type_name(), "http");
        assert!(http.is_http());
        assert!(!http.is_command());
    }

    #[test]
    fn test_is_tool_event() {
        assert!(HooksSchema::is_tool_event("PreToolUse"));
        assert!(HooksSchema::is_tool_event("PostToolUse"));
        assert!(!HooksSchema::is_tool_event("Stop"));
        assert!(!HooksSchema::is_tool_event("Notification"));
    }

    #[test]
    fn test_supports_matcher() {
        // Tool events support matchers
        assert!(HooksSchema::supports_matcher("PreToolUse"));
        assert!(HooksSchema::supports_matcher("PostToolUse"));
        assert!(HooksSchema::supports_matcher("PermissionRequest"));
        assert!(HooksSchema::supports_matcher("PostToolUseFailure"));
        // Lifecycle events that now support matchers
        assert!(HooksSchema::supports_matcher("SessionStart"));
        assert!(HooksSchema::supports_matcher("SessionEnd"));
        assert!(HooksSchema::supports_matcher("Notification"));
        assert!(HooksSchema::supports_matcher("SubagentStart"));
        assert!(HooksSchema::supports_matcher("SubagentStop"));
        assert!(HooksSchema::supports_matcher("PreCompact"));
        assert!(HooksSchema::supports_matcher("PostCompact"));
        assert!(HooksSchema::supports_matcher("ConfigChange"));
        assert!(HooksSchema::supports_matcher("FileChanged"));
        assert!(HooksSchema::supports_matcher("StopFailure"));
        assert!(HooksSchema::supports_matcher("InstructionsLoaded"));
        assert!(HooksSchema::supports_matcher("Elicitation"));
        assert!(HooksSchema::supports_matcher("ElicitationResult"));
        // Events that do NOT support matchers
        assert!(!HooksSchema::supports_matcher("Stop"));
        assert!(!HooksSchema::supports_matcher("UserPromptSubmit"));
        assert!(!HooksSchema::supports_matcher("TaskCompleted"));
        assert!(!HooksSchema::supports_matcher("TeammateIdle"));
    }

    #[test]
    fn test_is_prompt_event() {
        // All 8 events that support prompt/agent hooks
        assert!(HooksSchema::is_prompt_event("PreToolUse"));
        assert!(HooksSchema::is_prompt_event("PostToolUse"));
        assert!(HooksSchema::is_prompt_event("PostToolUseFailure"));
        assert!(HooksSchema::is_prompt_event("PermissionRequest"));
        assert!(HooksSchema::is_prompt_event("UserPromptSubmit"));
        assert!(HooksSchema::is_prompt_event("Stop"));
        assert!(HooksSchema::is_prompt_event("SubagentStop"));
        assert!(HooksSchema::is_prompt_event("TaskCompleted"));

        // Events that do NOT support prompt/agent hooks
        assert!(!HooksSchema::is_prompt_event("SessionStart"));
        assert!(!HooksSchema::is_prompt_event("SessionEnd"));
        assert!(!HooksSchema::is_prompt_event("Notification"));
        assert!(!HooksSchema::is_prompt_event("SubagentStart"));
        assert!(!HooksSchema::is_prompt_event("PreCompact"));
        assert!(!HooksSchema::is_prompt_event("TeammateIdle"));
        assert!(!HooksSchema::is_prompt_event("Setup"));
    }

    #[test]
    fn test_settings_schema_from_json_with_hooks() {
        let json = r#"{"hooks": {"PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "echo test"}]}]}}"#;
        let settings = SettingsSchema::from_json(json).unwrap();
        assert!(settings.hooks.contains_key("PreToolUse"));
        let hooks_schema = settings.to_hooks_schema();
        assert!(hooks_schema.validate_events().is_empty());
    }

    #[test]
    fn test_settings_schema_from_json_no_hooks() {
        let json = r#"{"other_field": "value"}"#;
        let settings = SettingsSchema::from_json(json).unwrap();
        assert!(settings.hooks.is_empty());
    }

    #[test]
    fn test_http_hook_deserialization() {
        let json = r#"{"hooks": {"Stop": [{"hooks": [{"type": "http", "url": "https://example.com/webhook", "headers": {"Authorization": "Bearer $TOKEN"}, "allowedEnvVars": ["TOKEN"], "timeout": 10}]}]}}"#;
        let settings = SettingsSchema::from_json(json).unwrap();
        let matchers = settings.hooks.get("Stop").unwrap();
        assert_eq!(matchers.len(), 1);
        let hook = &matchers[0].hooks[0];
        assert!(hook.is_http());
        assert_eq!(hook.type_name(), "http");
    }

    #[test]
    fn test_command_hook_new_fields_deserialization() {
        let json = "{\"hooks\": {\"PreToolUse\": [{\"matcher\": \"Bash\", \"hooks\": [{\"type\": \"command\", \"command\": \"echo test\", \"if\": \"event.tool == 'Bash'\", \"shell\": \"/bin/zsh\", \"statusMessage\": \"Running check...\", \"once\": true, \"async\": false}]}]}}";
        let settings = SettingsSchema::from_json(json).unwrap();
        let matchers = settings.hooks.get("PreToolUse").unwrap();
        let hook = &matchers[0].hooks[0];
        assert!(hook.is_command());
        match hook {
            Hook::Command {
                if_condition,
                shell,
                status_message,
                once,
                is_async,
                ..
            } => {
                assert_eq!(if_condition.as_deref(), Some("event.tool == 'Bash'"));
                assert_eq!(shell.as_deref(), Some("/bin/zsh"));
                assert_eq!(status_message.as_deref(), Some("Running check..."));
                assert_eq!(*once, Some(true));
                assert_eq!(*is_async, Some(false));
            }
            _ => panic!("Expected Command hook"),
        }
    }
}
