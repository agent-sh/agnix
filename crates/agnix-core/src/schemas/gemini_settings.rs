//! Gemini CLI settings file schema helpers
//!
//! Provides parsing for .gemini/settings.json configuration files.
//!
//! Validates:
//! - JSON/JSONC syntax
//! - Top-level keys against the documented settings categories
//!
//! The `hooks` and `hooksConfig` blocks are validated by
//! `rules::gemini_hooks`, which also reads extension `hooks/hooks.json`.

use serde_json::Value;

/// Hook events Gemini CLI fires (geminicli.com/docs/hooks "Hook events";
/// the `hooks.*` properties of settingsSchema.ts).
pub const VALID_HOOK_EVENTS: &[&str] = &[
    "SessionStart",
    "SessionEnd",
    "BeforeAgent",
    "AfterAgent",
    "BeforeModel",
    "AfterModel",
    "BeforeToolSelection",
    "BeforeTool",
    "AfterTool",
    "PreCompress",
    "Notification",
];

/// Valid top-level keys in .gemini/settings.json: the 25 categories of
/// geminicli.com/docs/reference/configuration ("Available settings in
/// `settings.json`"), plus `extensions`, which settingsSchema.ts defines but
/// the reference does not list, and `$schema`.
pub const VALID_TOP_LEVEL_KEYS: &[&str] = &[
    "$schema",
    "policyPaths",
    "adminPolicyPaths",
    "general",
    "output",
    "ui",
    "ide",
    "privacy",
    "billing",
    "model",
    "modelConfigs",
    "agents",
    "context",
    "tools",
    "mcp",
    "useWriteTodos",
    "security",
    "advanced",
    "experimental",
    "extensions",
    "skills",
    "hooksConfig",
    "hooks",
    "contextManagement",
    "admin",
    "mcpServers",
    "telemetry",
];

/// A JSON parse error with location information
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

/// Result of parsing .gemini/settings.json
#[derive(Debug, Clone)]
pub struct ParsedGeminiSettings {
    /// Parse error if JSON is invalid
    pub parse_error: Option<ParseError>,
    /// The parsed schema (if valid JSON)
    pub schema: Option<GeminiSettingsSchema>,
    /// Top-level keys not in the known set
    pub unknown_top_keys: Vec<String>,
}

/// Partial schema for .gemini/settings.json (only fields we validate)
#[derive(Debug, Clone, Default)]
pub struct GeminiSettingsSchema {
    /// The `experimental` object, when present
    pub experimental: Option<GeminiExperimentalFlags>,
}

/// The `experimental.*` keys agnix tracks.
///
/// `experimental.memoryManager` existed in settingsSchema.ts up to v0.39.0
/// and is gone from v0.40.0 on (main has only `experimental.autoMemory`,
/// which drives background extraction and the /memory inbox). GM-010
/// reports the stale key whatever its value.
#[derive(Debug, Clone, Default)]
pub struct GeminiExperimentalFlags {
    /// `experimental.memoryManager` is present (any value)
    pub has_memory_manager: bool,
}

/// Strip single-line (//) and multi-line (/* */) comments from JSONC content.
///
/// Preserves newlines inside block comments so byte-line counts stay aligned
/// with the original input - useful when callers want to search the stripped
/// buffer but still report a line number matching the raw file.
pub fn strip_jsonc_comments(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_string = false;

    while i < len {
        if in_string {
            result.push(chars[i]);
            if chars[i] == '\\' && i + 1 < len {
                i += 1;
                result.push(chars[i]);
            } else if chars[i] == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if chars[i] == '"' {
            in_string = true;
            result.push(chars[i]);
            i += 1;
            continue;
        }

        if chars[i] == '/' && i + 1 < len {
            if chars[i + 1] == '/' {
                // Replace comments with spaces so removing one cannot join
                // otherwise separate JSON tokens.
                result.push_str("  ");
                i += 2;
                while i < len && !matches!(chars[i], '\n' | '\r') {
                    result.push(' ');
                    i += 1;
                }
                continue;
            } else if chars[i + 1] == '*' {
                let comment_start = i;
                i += 2;
                let body_start = i;
                while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                if i + 1 < len {
                    result.push_str("  ");
                    for ch in &chars[body_start..i] {
                        result.push(if matches!(*ch, '\n' | '\r') { *ch } else { ' ' });
                    }
                    result.push_str("  ");
                    i += 2; // skip */
                } else {
                    // Preserve malformed input so the JSON parser reports the
                    // unterminated comment instead of accepting it silently.
                    result.extend(chars[comment_start..].iter());
                    break;
                }
                continue;
            }
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Parse .gemini/settings.json content
///
/// Uses a two-pass approach: first validates JSON syntax with `serde_json::Value`,
/// then extracts the typed schema. Supports JSONC comments.
pub fn parse_gemini_settings(content: &str) -> ParsedGeminiSettings {
    let stripped = strip_jsonc_comments(content);

    // First pass: validate JSON syntax
    let value: Value = match serde_json::from_str(&stripped) {
        Ok(v) => v,
        Err(e) => {
            return ParsedGeminiSettings {
                parse_error: Some(ParseError {
                    message: e.to_string(),
                    line: e.line(),
                    column: e.column(),
                }),
                schema: None,
                unknown_top_keys: Vec::new(),
            };
        }
    };

    // Check for unknown top-level keys
    let unknown_top_keys = if let Some(obj) = value.as_object() {
        obj.keys()
            .filter(|k| !VALID_TOP_LEVEL_KEYS.contains(&k.as_str()))
            .cloned()
            .collect()
    } else {
        Vec::new()
    };

    // Extract the experimental.* keys agnix tracks.
    let experimental = value
        .get("experimental")
        .and_then(Value::as_object)
        .map(|exp| GeminiExperimentalFlags {
            has_memory_manager: exp.contains_key("memoryManager"),
        });

    ParsedGeminiSettings {
        parse_error: None,
        schema: Some(GeminiSettingsSchema { experimental }),
        unknown_top_keys,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_settings() {
        let content = r#"{
  "general": {},
  "model": {},
  "hooksConfig": {"enabled": true},
  "hooks": {
    "BeforeAgent": [
      {"hooks": [{"type": "command", "command": "echo hello"}]}
    ]
  }
}"#;
        let result = parse_gemini_settings(content);
        assert!(result.parse_error.is_none());
        assert!(result.schema.is_some());
        assert!(result.unknown_top_keys.is_empty());
    }

    #[test]
    fn test_parse_empty_object() {
        let result = parse_gemini_settings("{}");
        assert!(result.parse_error.is_none());
        assert!(result.schema.is_some());
        assert!(result.unknown_top_keys.is_empty());
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = parse_gemini_settings("{ invalid }");
        assert!(result.parse_error.is_some());
        assert!(result.schema.is_none());
    }

    #[test]
    fn test_parse_jsonc_comments() {
        let content = r#"{
  // This is a comment
  "general": {},
  /* multi-line
     comment */
  "model": {}
}"#;
        let result = parse_gemini_settings(content);
        assert!(result.parse_error.is_none());
        assert!(result.schema.is_some());
    }

    #[test]
    fn test_parse_jsonc_line_comment_with_cr_line_endings() {
        let result = parse_gemini_settings("{\r// comment\r\"general\": {}\r}");
        assert!(result.parse_error.is_none());
        assert!(result.schema.is_some());
    }

    #[test]
    fn test_parse_unterminated_jsonc_comment_is_invalid() {
        let result = parse_gemini_settings(r#"{"general": {}} /*"#);
        assert!(result.parse_error.is_some());
        assert!(result.schema.is_none());
    }

    #[test]
    fn test_jsonc_comment_removal_does_not_join_tokens() {
        let result = parse_gemini_settings(r#"{"experimental":{"autoMemory":tr/*x*/ue}}"#);
        assert!(result.parse_error.is_some());
        assert!(result.schema.is_none());
    }

    #[test]
    fn test_unknown_top_level_keys() {
        let content = r#"{
  "general": {},
  "unknownKey": true,
  "anotherBadKey": 42
}"#;
        let result = parse_gemini_settings(content);
        assert!(result.parse_error.is_none());
        assert_eq!(result.unknown_top_keys.len(), 2);
        assert!(result.unknown_top_keys.contains(&"unknownKey".to_string()));
        assert!(
            result
                .unknown_top_keys
                .contains(&"anotherBadKey".to_string())
        );
    }

    #[test]
    fn test_all_valid_top_level_keys() {
        let content = format!(
            "{{{}}}",
            VALID_TOP_LEVEL_KEYS
                .iter()
                .map(|key| format!("\"{key}\": {{}}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let result = parse_gemini_settings(&content);
        assert!(result.parse_error.is_none());
        assert!(result.unknown_top_keys.is_empty());
    }

    #[test]
    fn test_documented_hooks_and_mcp_servers_are_known() {
        // agnix 0.58 knew only 12 keys and flagged these documented ones.
        let content = r#"{"$schema": "x", "hooks": {}, "mcpServers": {}, "privacy": {}, "telemetry": {}, "extensions": {}}"#;
        let result = parse_gemini_settings(content);
        assert!(
            result.unknown_top_keys.is_empty(),
            "{:?}",
            result.unknown_top_keys
        );
    }

    #[test]
    fn test_parse_error_location() {
        let content = "{\n  \"general\": \n}";
        let result = parse_gemini_settings(content);
        assert!(result.parse_error.is_some());
        let err = result.parse_error.unwrap();
        assert!(err.line > 0);
    }

    #[test]
    fn test_valid_hook_events_count() {
        assert_eq!(VALID_HOOK_EVENTS.len(), 11);
    }

    #[test]
    fn test_valid_top_level_keys_count() {
        // 25 documented categories + `extensions` (settingsSchema.ts) + `$schema`.
        assert_eq!(VALID_TOP_LEVEL_KEYS.len(), 27);
    }

    #[test]
    fn test_experimental_flags_parsed() {
        let content = r#"{
  "experimental": {
    "memoryManager": true,
    "autoMemory": false
  }
}"#;
        let result = parse_gemini_settings(content);
        let schema = result.schema.unwrap();
        let exp = schema.experimental.expect("experimental block parsed");
        assert!(exp.has_memory_manager);
    }

    #[test]
    fn test_memory_manager_presence_ignores_value_type() {
        for value in ["false", "null", "\"yes\"", "{}"] {
            let content = format!(r#"{{"experimental": {{"memoryManager": {value}}}}}"#);
            let exp = parse_gemini_settings(&content)
                .schema
                .unwrap()
                .experimental
                .unwrap();
            assert!(exp.has_memory_manager, "{value}");
        }
    }

    #[test]
    fn test_experimental_missing_flags_are_none() {
        let content = r#"{"experimental": {}}"#;
        let result = parse_gemini_settings(content);
        let exp = result.schema.unwrap().experimental.unwrap();
        assert!(!exp.has_memory_manager);
    }

    #[test]
    fn test_experimental_absent() {
        let content = r#"{"general": {}}"#;
        let result = parse_gemini_settings(content);
        assert!(result.schema.unwrap().experimental.is_none());
    }
}
