//! Plugin manifest schema

use serde::{Deserialize, Serialize};

/// plugin.json schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSchema {
    /// Required: plugin name
    pub name: String,

    /// Recommended: description
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Recommended: version string. Claude Code does not require semver.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Optional: author info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<AuthorInfo>,

    /// Optional: homepage URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,

    /// Optional: repository URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,

    /// Optional: license
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Optional: keywords
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorInfo {
    #[serde(default)]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl PluginSchema {
    /// Check optional semver suitability for version-constrained consumers.
    /// This helper does not determine whether Claude Code loads the manifest.
    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn validate_version(&self) -> Result<(), String> {
        if let Some(ref version) = self.version {
            let trimmed = version.trim();
            if !trimmed.is_empty() {
                semver::Version::parse(trimmed).map_err(|e| {
                    format!(
                        "Invalid semver format '{}': {}",
                        trimmed,
                        e.to_string().to_lowercase()
                    )
                })?;
            }
        }
        Ok(())
    }

    /// Run all validations
    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.name.trim().is_empty() {
            errors.push("Plugin name cannot be empty".to_string());
        }

        if let Some(ref description) = self.description {
            if description.trim().is_empty() {
                errors.push("Plugin description cannot be empty".to_string());
            }
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::PluginSchema;

    #[test]
    fn arbitrary_version_is_loadable_but_semver_check_remains_opt_in() {
        let schema: PluginSchema = serde_json::from_str(
            r#"{"name":"release-plugin","description":"Release plugin","version":"release-2026-10"}"#,
        )
        .unwrap();
        assert!(schema.validate().is_empty());
        assert!(schema.validate_version().is_err());
    }
}
