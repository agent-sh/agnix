//! Capability policy schema shared by Kiro's permission files and agent
//! `permissions` blocks.
//!
//! Sources: <https://kiro.dev/docs/permissions/> and the V3 engine's strict
//! policy parser. `match` and `exclude` are optional lists of glob strings;
//! omitting `match` (or `[]`, or `["*"]`) means every resource.

/// Capabilities the V3 engine accepts, including the meta-capabilities `all`,
/// `builtin` and `filesystem`. `workflow` is accepted by the engine although
/// kiro.dev does not list it. An unknown capability makes Kiro skip the rule.
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
    "workflow",
];

/// Effects a rule may use in user, workspace and agent scope. `evaluate` is
/// the weakest effect, which the `approve-for-me` preset adds: "Both scopes
/// support all effects (`deny`, `ask`, `allow`, and `evaluate`)" (permissions
/// reference, verified 2026-11-04).
pub const VALID_PERMISSION_EFFECTS: &[&str] = &["allow", "deny", "ask", "evaluate"];

/// Effects allowed in an administrator `managed-settings.json`.
pub const ADMIN_PERMISSION_EFFECTS: &[&str] = &["deny", "ask"];

/// Rule fields the strict policy parser accepts; any other key in a rule
/// rejects the whole file.
pub const VALID_PERMISSION_RULE_FIELDS: &[&str] = &["capability", "effect", "match", "exclude"];

/// Preset ids accepted in a policy file's `policies` list. Every preset
/// except `approve-for-me` contains only `allow` rules; `approve-for-me` is
/// "The `edit-workspace` and `read-only-shell` sets, plus an `evaluate` rule
/// for every capability" (permissions reference preset table).
pub const VALID_PERMISSION_POLICIES: &[&str] = &[
    "allow-all",
    "edit-workspace",
    "read-workspace",
    "read-all",
    "read-only-shell",
    "dev-shell",
    "approve-for-me",
];

/// Keep YAML values uncoerced: numeric/bool scalars are not pattern strings.
pub fn parse_kiro_permissions(content: &str) -> Result<serde_yaml::Value, serde_yaml::Error> {
    serde_yaml::from_str(content)
}
