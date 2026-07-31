//! Shared validation utilities

/// Check if a tool name is valid (either known or properly formatted MCP tool).
///
/// Two MCP forms are accepted, both documented as equivalent by Claude Code:
/// the server-only `mcp__<server>` (matches every tool from that server) and
/// the fully qualified `mcp__<server>__<tool>` (see
/// <https://code.claude.com/docs/en/permissions> "MCP" and
/// <https://code.claude.com/docs/en/sub-agents> "Available tools"). The
/// `mcp__` prefix is case-sensitive and must be lowercase.
///
/// The server segment must be glob-free - the same docs require an allow rule
/// to name a specific configured server - so `mcp__supabase-*` is rejected
/// while the tool segment may glob (`mcp__github__get_*`).
///
/// # Examples
/// ```
/// use agnix_core::validation::is_valid_mcp_tool_format;
///
/// assert!(is_valid_mcp_tool_format("mcp__filesystem__read_file", &["Read", "Write"]));
/// assert!(is_valid_mcp_tool_format("mcp__playwright", &["Read"])); // Server-only form
/// assert!(is_valid_mcp_tool_format("mcp__github__get_*", &["Read"])); // Tool-segment glob
/// assert!(is_valid_mcp_tool_format("Read", &["Read", "Write"]));
/// assert!(!is_valid_mcp_tool_format("mcp__", &["Read"])); // Empty
/// assert!(!is_valid_mcp_tool_format("mcp__supabase-*", &["Read"])); // Glob in server segment
/// assert!(!is_valid_mcp_tool_format("mcp__bad name", &["Read"])); // Whitespace in server segment
/// assert!(!is_valid_mcp_tool_format("MCP__server__tool", &["Read"])); // Uppercase
/// ```
pub fn is_valid_mcp_tool_format(tool: &str, known_tools: &[&str]) -> bool {
    // Strip parenthesized parameters from tool names like "Read(file_path)"
    // so that both "Read" and "Read(file_path)" match the known tool "Read".
    let base_name = tool.split('(').next().unwrap_or(tool);

    // Check if it's a known tool
    if known_tools.contains(&base_name) {
        return true;
    }

    // Check if it's a valid MCP tool: mcp__<server> or mcp__<server>__<tool>
    if let Some(rest) = base_name.strip_prefix("mcp__") {
        let (server, tool_name) = match rest.find("__") {
            Some(tool_start) => (&rest[..tool_start], Some(&rest[tool_start + 2..])),
            // Server-only form: `mcp__<server>` matches any tool from that server.
            None => (rest, None),
        };
        // The server segment names one configured server, so it can't be
        // empty, contain a glob, or contain whitespace. The tool segment, when
        // present, must be non-empty but may glob (`mcp__github__get_*`).
        return !server.is_empty()
            && !server.contains('*')
            && !server.chars().any(char::is_whitespace)
            && tool_name.is_none_or(|name| !name.is_empty());
    }

    false
}
