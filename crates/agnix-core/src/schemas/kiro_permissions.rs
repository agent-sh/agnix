//! Capability policy schema shared by Kiro's user/workspace permission files.
//!
//! Source: <https://kiro.dev/docs/permissions/>. `match` and `exclude` are
//! optional lists of glob strings; omitting `match` means unrestricted scope.

pub const VALID_PERMISSION_CAPABILITIES: &[&str] = &[
    "fs_read",
    "fs_write",
    "shell",
    "web_fetch",
    "web_search",
    "mcp",
    "subagent",
    "skill",
    "power",
    "context",
    "diagnostics",
    "sandbox_network",
    "all",
    "builtin",
    "filesystem",
];
pub const VALID_PERMISSION_EFFECTS: &[&str] = &["allow", "deny", "ask"];

/// Keep YAML values uncoerced: numeric/bool scalars are not pattern strings.
pub fn parse_kiro_permissions(content: &str) -> Result<serde_yaml::Value, serde_yaml::Error> {
    serde_yaml::from_str(content)
}
