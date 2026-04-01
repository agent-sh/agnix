//! Codex CLI plugin manifest schema (.codex-plugin/plugin.json)

use serde::{Deserialize, Serialize};

/// .codex-plugin/plugin.json top-level schema
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // schema-level API; validation uses raw serde_json::Value
pub struct CodexPluginSchema {
    /// Required: plugin name (ASCII alphanumeric + hyphens + underscores)
    #[serde(default)]
    pub name: Option<String>,
    /// Optional: description
    #[serde(default)]
    pub description: Option<String>,
    /// Optional: skills component path (must start with ./)
    #[serde(default)]
    pub skills: Option<String>,
    /// Optional: MCP servers component path (must start with ./)
    #[serde(default)]
    pub mcp_servers: Option<String>,
    /// Optional: apps component path (must start with ./)
    #[serde(default)]
    pub apps: Option<String>,
    /// Optional: interface metadata for marketplace display
    #[serde(default)]
    pub interface: Option<CodexPluginInterface>,
}

/// Interface metadata for marketplace display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // schema-level API; validation uses raw serde_json::Value
pub struct CodexPluginInterface {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub short_description: Option<String>,
    #[serde(default)]
    pub long_description: Option<String>,
    #[serde(default)]
    pub developer_name: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default, alias = "websiteURL")]
    pub website_url: Option<String>,
    #[serde(default, alias = "privacyPolicyURL")]
    pub privacy_policy_url: Option<String>,
    #[serde(default, alias = "termsOfServiceURL")]
    pub terms_of_service_url: Option<String>,
    /// String or array of strings; max 3, max 128 chars each
    #[serde(default)]
    pub default_prompt: Option<serde_json::Value>,
    #[serde(default)]
    pub brand_color: Option<String>,
    #[serde(default)]
    pub composer_icon: Option<String>,
    #[serde(default)]
    pub logo: Option<String>,
    #[serde(default)]
    pub screenshots: Vec<String>,
}
