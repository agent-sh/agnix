//! Agent definition schema (Claude Code subagents)

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Agent .md file frontmatter schema
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentSchema {
    /// Required: agent name (CC-AG-001)
    #[serde(default)]
    pub name: Option<String>,

    /// Required: description (CC-AG-002)
    #[serde(default)]
    pub description: Option<String>,

    /// Optional: tools list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,

    /// Optional: disallowed tools
    #[serde(skip_serializing_if = "Option::is_none", rename = "disallowedTools")]
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
}

// Validation is performed in rules/agent.rs (AgentValidator)
