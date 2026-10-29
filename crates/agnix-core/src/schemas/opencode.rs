//! OpenCode configuration file schema data.
//!
//! OpenCode publishes one schema at https://opencode.ai/config.json for both
//! release lines: the native OpenCode 2 fields plus the OpenCode 1 fields,
//! which it marks deprecated but still reads ("supported V1 and native V2
//! fields may coexist at the top level"). The top-level object has
//! `additionalProperties: false`.

use crate::schemas::kiro_json::blank_jsonc;
use std::collections::HashSet;

/// Valid values for the `share` field.
pub const VALID_SHARE_MODES: &[&str] = &["manual", "auto", "disabled"];

/// Valid values for the OpenCode 2 `update` field.
pub const VALID_UPDATE_MODES: &[&str] = &["disable", "notify", "auto"];

/// Top-level keys of https://opencode.ai/config.json (fetched 2026-10-17),
/// plus the OpenCode 1 TUI keys that the schema dropped and OpenCode 1 still
/// migrates (`theme`, `keybinds`, `tui`; OC-DEP-005).
pub const KNOWN_TOP_LEVEL_KEYS: &[&str] = &[
    "$schema",
    "agent",
    "agents",
    "attachment",
    "autoshare",
    "autoupdate",
    "command",
    "commands",
    "compaction",
    "default_agent",
    "disabled_providers",
    "enabled_providers",
    "enterprise",
    "experimental",
    "formatter",
    "instructions",
    "keybinds",
    "layout",
    "logLevel",
    "lsp",
    "mcp",
    "media",
    "mode",
    "model",
    "permission",
    "permissions",
    "plugin",
    "plugins",
    "policies",
    "provider",
    "providers",
    "reference",
    "references",
    "server",
    "share",
    "shell",
    "skills",
    "small_model",
    "snapshot",
    "snapshots",
    "subagent_depth",
    "theme",
    "timezone",
    "tool_output",
    "tools",
    "tui",
    "update",
    "username",
    "warming",
    "watcher",
    "websearch",
    "worktree",
];

/// `logLevel` values (schema `$defs/LogLevel`, case-sensitive).
pub const VALID_LOG_LEVELS: &[&str] = &["DEBUG", "INFO", "WARN", "ERROR"];

/// Valid diff style values for TUI.
pub const VALID_DIFF_STYLES: &[&str] = &["auto", "stacked"];

/// Known TUI configuration keys.
pub const KNOWN_TUI_KEYS: &[&str] = &[
    "$schema",
    "theme",
    "keybinds",
    "mouse",
    "scroll_speed",
    "scroll_acceleration",
    "diff_style",
];

/// OpenCode 1 OAuth members of a remote MCP server (`McpOAuthConfig`).
pub const V1_OAUTH_KEYS: &[&str] = &[
    "clientId",
    "clientSecret",
    "scope",
    "callbackPort",
    "redirectUri",
];

/// OpenCode 2 OAuth members (`Mcp.OAuthConfigEncoded`).
pub const V2_OAUTH_KEYS: &[&str] = &[
    "client_id",
    "client_secret",
    "scope",
    "callback_port",
    "redirect_uri",
    "auth_server_metadata_url",
];

/// MCP `protocol` values (OpenCode 2).
pub const MCP_PROTOCOLS: &[&str] = &["legacy", "auto", "2026-07-28"];

/// Source spans of an unambiguous top-level JSON property. Offsets include
/// key quotes; `comma` is the optional following separator.
pub(crate) struct ConfigPropertySpan {
    pub key: String,
    pub key_start: usize,
    pub value_end: usize,
    pub comma: Option<usize>,
}

/// Use serde's token consumption to avoid matching nested keys, strings, or
/// comments. Duplicate decoded root keys suppress fixes. Comments and
/// trailing commas are blanked with spaces first, so offsets match the file.
pub(crate) fn config_property_spans(content: &str) -> Option<Vec<ConfigPropertySpan>> {
    let (stripped, _) = blank_jsonc(content);
    if stripped.len() != content.len() {
        return None;
    }
    let root: serde_json::Value = serde_json::from_str(&stripped).ok()?;
    root.as_object()?;
    let bytes = stripped.as_bytes();
    let mut pos = stripped.find('{')? + 1;
    let mut keys = HashSet::new();
    let mut properties = Vec::new();
    loop {
        while bytes.get(pos).is_some_and(u8::is_ascii_whitespace) {
            pos += 1;
        }
        if bytes.get(pos) == Some(&b'}') {
            break;
        }
        let key_start = pos;
        let mut stream = serde_json::Deserializer::from_str(&stripped[pos..]).into_iter::<String>();
        let key = stream.next()?.ok()?;
        pos += stream.byte_offset();
        if !keys.insert(key.clone()) {
            return None;
        }
        while bytes.get(pos).is_some_and(u8::is_ascii_whitespace) {
            pos += 1;
        }
        if bytes.get(pos) != Some(&b':') {
            return None;
        }
        pos += 1;
        let mut stream =
            serde_json::Deserializer::from_str(&stripped[pos..]).into_iter::<serde_json::Value>();
        stream.next()?.ok()?;
        pos += stream.byte_offset();
        let value_end = pos;
        while bytes.get(pos).is_some_and(u8::is_ascii_whitespace) {
            pos += 1;
        }
        let comma = (bytes.get(pos) == Some(&b',')).then_some(pos);
        if comma.is_some() {
            pos += 1;
        }
        properties.push(ConfigPropertySpan {
            key,
            key_start,
            value_end,
            comma,
        });
    }
    Some(properties)
}

/// Check if a path looks like a valid glob pattern (contains glob characters)
pub fn is_glob_pattern(path: &str) -> bool {
    path.contains('*') || path.contains('?') || path.contains('[')
}

/// Validate a glob pattern syntax
pub fn validate_glob_pattern(pattern: &str) -> bool {
    glob::Pattern::new(pattern).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_glob_pattern() {
        assert!(is_glob_pattern("**/*.md"));
        assert!(is_glob_pattern("docs/*.txt"));
        assert!(is_glob_pattern("file[0-9].md"));
        assert!(!is_glob_pattern("README.md"));
        assert!(!is_glob_pattern("docs/guide.md"));
    }

    #[test]
    fn test_validate_glob_pattern() {
        assert!(validate_glob_pattern("**/*.md"));
        assert!(validate_glob_pattern("docs/*.txt"));
        assert!(!validate_glob_pattern("[unclosed"));
    }

    #[test]
    fn known_keys_cover_both_release_lines() {
        for key in [
            "agent",
            "agents",
            "permission",
            "permissions",
            "mcp",
            "policies",
            "update",
        ] {
            assert!(KNOWN_TOP_LEVEL_KEYS.contains(&key), "{key}");
        }
    }

    #[test]
    fn property_spans_accept_jsonc_trailing_commas() {
        let content = "{\n  // c\n  \"theme\": \"dark\",\n  \"model\": \"a/b\",\n}";
        let spans = config_property_spans(content).unwrap();
        assert_eq!(spans.len(), 2);
        assert!(content[spans[0].key_start..].starts_with("\"theme\""));
        assert!(config_property_spans("{\"a\": 1, \"a\": 2}").is_none());
    }
}
