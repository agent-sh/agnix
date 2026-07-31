//! Agent definition schema (Claude Code subagents)

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Deserialize a tool list from either a YAML list or a comma/space-separated
/// string (`tools: Read, Glob, Grep`). Claude Code's sub-agent docs show the
/// string form as canonical and also accept a list, so both must parse without
/// tripping CC-AG-007 (agent parse error).
fn de_seq_or_delimited_string<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum SeqOrString {
        Seq(Vec<String>),
        Str(String),
    }
    let value = Option::<SeqOrString>::deserialize(deserializer)?;
    Ok(value.map(|v| match v {
        SeqOrString::Seq(items) => items
            .into_iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect(),
        // Split with `split_tool_list` rather than a plain `split`: entries
        // carry parenthesized arguments containing commas and spaces, e.g. the
        // sub-agents doc's own `tools: Agent(worker, researcher), Read, Bash`
        // and `Bash(npm run test:*)`. Splitting naively shattered those into
        // fragments like `researcher)` and `run`, which then tripped
        // CC-AG-009/010 as unknown tools.
        SeqOrString::Str(s) => crate::validation::split_tool_list(&s)
            .into_iter()
            .map(String::from)
            .collect(),
    }))
}

/// Agent .md file frontmatter schema
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentSchema {
    /// Required: agent name (CC-AG-001)
    #[serde(default)]
    pub name: Option<String>,

    /// Required: description (CC-AG-002)
    #[serde(default)]
    pub description: Option<String>,

    /// Optional: tools list. Claude Code sub-agent docs accept either a
    /// comma/space-separated string (`tools: Read, Glob, Grep`, the canonical
    /// form shown in the docs) or a YAML list, so deserialize both.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_seq_or_delimited_string"
    )]
    pub tools: Option<Vec<String>>,

    /// Optional: disallowed tools (string or YAML list, like `tools`)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "disallowedTools",
        deserialize_with = "de_seq_or_delimited_string"
    )]
    pub disallowed_tools: Option<Vec<String>>,

    /// Optional: model (CC-AG-003)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Optional: permission mode (CC-AG-004)
    #[serde(skip_serializing_if = "Option::is_none", rename = "permissionMode")]
    pub permission_mode: Option<String>,

    /// Optional: skills to preload (CC-AG-005)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<String>>,

    /// Optional: memory scope (CC-AG-008)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,

    /// Optional: hooks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<Value>,

    /// Optional: max turns (positive integer)
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxTurns")]
    pub max_turns: Option<u32>,

    /// Optional: reasoning effort level (low, medium, high, max)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,

    /// Optional: run agent in background
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,

    /// Optional: isolation mode (worktree)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isolation: Option<String>,

    /// Optional: initial prompt for the agent
    #[serde(skip_serializing_if = "Option::is_none", rename = "initialPrompt")]
    pub initial_prompt: Option<String>,

    /// Optional: MCP server configurations
    #[serde(skip_serializing_if = "Option::is_none", rename = "mcpServers")]
    pub mcp_servers: Option<Value>,

    /// Optional: agent mode (e.g. "plan")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,

    /// Optional: display color in task list and transcript
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,

    /// Catch-all for unknown frontmatter fields (used by CC-AG-019)
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

// Validation is performed in rules/agent.rs (AgentValidator)
