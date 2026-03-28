//! Agent Skills schema (agentskills.io spec)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// SKILL.md frontmatter schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSchema {
    /// Required: skill name (lowercase, hyphens, 1-64 chars)
    pub name: String,

    /// Required: description (1-1024 chars)
    pub description: String,

    /// Optional: license identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Optional: compatibility notes (1-500 chars)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<String>,

    /// Optional: arbitrary metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Optional: space-delimited list of allowed tools (experimental)
    #[serde(skip_serializing_if = "Option::is_none", rename = "allowed-tools")]
    pub allowed_tools: Option<String>,

    // Claude Code extensions
    /// Optional: argument hint for autocomplete
    #[serde(skip_serializing_if = "Option::is_none", rename = "argument-hint")]
    pub argument_hint: Option<String>,

    /// Optional: disable model invocation
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "disable-model-invocation"
    )]
    pub disable_model_invocation: Option<bool>,

    /// Optional: user invocable
    #[serde(skip_serializing_if = "Option::is_none", rename = "user-invocable")]
    pub user_invocable: Option<bool>,

    /// Optional: model override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Optional: context mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,

    /// Optional: agent type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    /// Optional: reasoning effort level (low, medium, high, max)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,

    /// Optional: comma-separated glob patterns or YAML list of file paths
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths: Option<serde_yaml::Value>,

    /// Optional: shell to use (bash, powershell)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
}

/// Known top-level frontmatter fields for SKILL.md
#[cfg(test)]
pub const KNOWN_SKILL_FRONTMATTER_FIELDS: &[&str] = &[
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
    "argument-hint",
    "disable-model-invocation",
    "user-invocable",
    "model",
    "context",
    "agent",
    "hooks",
    "effort",
    "paths",
    "shell",
];

/// Valid model aliases for skill frontmatter
pub const VALID_MODEL_ALIASES: &[&str] = &["sonnet", "opus", "haiku", "inherit"];

/// Check whether a model value is valid.
pub fn is_valid_skill_model(model: &str) -> bool {
    VALID_MODEL_ALIASES.contains(&model) || model.starts_with("claude-")
}

/// Valid effort levels for skill frontmatter
pub const VALID_EFFORT_LEVELS: &[&str] = &["low", "medium", "high", "max"];

/// Valid shell values for skill frontmatter
pub const VALID_SHELLS: &[&str] = &["bash", "powershell"];

impl SkillSchema {
    /// Validate skill name format
    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn validate_name(&self) -> Result<(), String> {
        let name = &self.name;

        // Length check
        if name.is_empty() || name.len() > 64 {
            return Err(format!("Name must be 1-64 characters, got {}", name.len()));
        }

        // Character check
        for ch in name.chars() {
            if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' {
                return Err(format!(
                    "Name must contain only lowercase letters, digits, and hyphens, found '{}'",
                    ch
                ));
            }
        }

        // Start/end check
        if name.starts_with('-') || name.ends_with('-') {
            return Err("Name cannot start or end with hyphen".to_string());
        }

        // Consecutive hyphens
        if name.contains("--") {
            return Err("Name cannot contain consecutive hyphens".to_string());
        }

        Ok(())
    }

    /// Validate description length
    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn validate_description(&self) -> Result<(), String> {
        let len = self.description.len();
        if len == 0 || len > 1024 {
            return Err(format!(
                "Description must be 1-1024 characters, got {}",
                len
            ));
        }
        Ok(())
    }

    /// Validate compatibility length
    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn validate_compatibility(&self) -> Result<(), String> {
        if let Some(compat) = &self.compatibility {
            let len = compat.len();
            if len == 0 || len > 500 {
                return Err(format!(
                    "Compatibility must be 1-500 characters, got {}",
                    len
                ));
            }
        }
        Ok(())
    }

    /// Validate model value.
    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn validate_model(&self) -> Result<(), String> {
        if let Some(model) = &self.model {
            if !is_valid_skill_model(model) {
                return Err(format!(
                    "Model must be one of {:?} or a full model ID matching 'claude-*', got '{}'",
                    VALID_MODEL_ALIASES, model
                ));
            }
        }
        Ok(())
    }

    /// Validate effort value
    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn validate_effort(&self) -> Result<(), String> {
        if let Some(effort) = &self.effort {
            if !VALID_EFFORT_LEVELS.contains(&effort.as_str()) {
                return Err(format!(
                    "Effort must be one of: {:?}, got '{}'",
                    VALID_EFFORT_LEVELS, effort
                ));
            }
        }
        Ok(())
    }

    /// Validate shell value
    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn validate_shell(&self) -> Result<(), String> {
        if let Some(shell) = &self.shell {
            if !VALID_SHELLS.contains(&shell.as_str()) {
                return Err(format!(
                    "Shell must be one of: {:?}, got '{}'",
                    VALID_SHELLS, shell
                ));
            }
        }
        Ok(())
    }

    /// Validate context value
    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn validate_context(&self) -> Result<(), String> {
        if let Some(context) = &self.context {
            if context != "fork" {
                return Err(format!("Context must be 'fork', got '{}'", context));
            }
        }
        Ok(())
    }

    /// Run all validations
    #[allow(dead_code)] // schema-level API; validation uses Validator trait
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if let Err(e) = self.validate_name() {
            errors.push(e);
        }
        if let Err(e) = self.validate_description() {
            errors.push(e);
        }
        if let Err(e) = self.validate_compatibility() {
            errors.push(e);
        }
        if let Err(e) = self.validate_model() {
            errors.push(e);
        }
        if let Err(e) = self.validate_context() {
            errors.push(e);
        }
        if let Err(e) = self.validate_effort() {
            errors.push(e);
        }
        if let Err(e) = self.validate_shell() {
            errors.push(e);
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_skill_name() {
        let skill = SkillSchema {
            name: "code-review".to_string(),
            description: "Reviews code".to_string(),
            license: None,
            compatibility: None,
            metadata: None,
            allowed_tools: None,
            argument_hint: None,
            disable_model_invocation: None,
            user_invocable: None,
            model: None,
            context: None,
            agent: None,
            effort: None,
            paths: None,
            shell: None,
        };
        assert!(skill.validate_name().is_ok());
    }

    #[test]
    fn test_invalid_skill_name_uppercase() {
        let skill = SkillSchema {
            name: "Code-Review".to_string(),
            description: "Reviews code".to_string(),
            license: None,
            compatibility: None,
            metadata: None,
            allowed_tools: None,
            argument_hint: None,
            disable_model_invocation: None,
            user_invocable: None,
            model: None,
            context: None,
            agent: None,
            effort: None,
            paths: None,
            shell: None,
        };
        assert!(skill.validate_name().is_err());
    }

    #[test]
    fn test_invalid_model() {
        let skill = SkillSchema {
            name: "test".to_string(),
            description: "Test".to_string(),
            license: None,
            compatibility: None,
            metadata: None,
            allowed_tools: None,
            argument_hint: None,
            disable_model_invocation: None,
            user_invocable: None,
            model: Some("gpt-4".to_string()),
            context: None,
            agent: None,
            effort: None,
            paths: None,
            shell: None,
        };
        assert!(skill.validate_model().is_err());
    }

    fn make_skill(name: &str, description: &str) -> SkillSchema {
        SkillSchema {
            name: name.to_string(),
            description: description.to_string(),
            license: None,
            compatibility: None,
            metadata: None,
            allowed_tools: None,
            argument_hint: None,
            disable_model_invocation: None,
            user_invocable: None,
            model: None,
            context: None,
            agent: None,
            effort: None,
            paths: None,
            shell: None,
        }
    }

    #[test]
    fn test_empty_name_rejected() {
        let skill = make_skill("", "Valid description");
        assert!(skill.validate_name().is_err());
    }

    #[test]
    fn test_max_length_name_accepted() {
        // Exactly 64 chars - should be accepted
        let name = "a".repeat(64);
        let skill = make_skill(&name, "Valid description");
        assert!(skill.validate_name().is_ok());
    }

    #[test]
    fn test_over_max_length_name_rejected() {
        // 65 chars - should be rejected
        let name = "a".repeat(65);
        let skill = make_skill(&name, "Valid description");
        assert!(skill.validate_name().is_err());
    }

    #[test]
    fn test_empty_description_rejected() {
        let skill = make_skill("valid-name", "");
        assert!(skill.validate_description().is_err());
    }

    #[test]
    fn test_max_length_description_accepted() {
        let desc = "x".repeat(1024);
        let skill = make_skill("valid-name", &desc);
        assert!(skill.validate_description().is_ok());
    }

    #[test]
    fn test_over_max_length_description_rejected() {
        let desc = "x".repeat(1025);
        let skill = make_skill("valid-name", &desc);
        assert!(skill.validate_description().is_err());
    }

    #[test]
    fn test_empty_compatibility_rejected() {
        let mut skill = make_skill("valid-name", "Valid description");
        skill.compatibility = Some(String::new());
        assert!(skill.validate_compatibility().is_err());
    }

    #[test]
    fn test_over_max_compatibility_rejected() {
        let mut skill = make_skill("valid-name", "Valid description");
        skill.compatibility = Some("x".repeat(501));
        assert!(skill.validate_compatibility().is_err());
    }

    #[test]
    fn test_validate_collects_all_errors() {
        // Multiple invalid fields should all be reported
        let skill = make_skill("", "");
        let errors = skill.validate();
        assert!(
            errors.len() >= 2,
            "Should report errors for both name and description, got: {:?}",
            errors
        );
    }

    // ===== Model Validation =====

    #[test]
    fn test_valid_model_aliases() {
        for model in &["sonnet", "opus", "haiku", "inherit"] {
            let mut skill = make_skill("test", "Test skill");
            skill.model = Some(model.to_string());
            assert!(
                skill.validate_model().is_ok(),
                "Model alias '{}' should be valid",
                model
            );
        }
    }

    #[test]
    fn test_valid_model_full_ids() {
        for model in &[
            "claude-opus-4-6",
            "claude-sonnet-4-6",
            "claude-haiku-4-5-20251001",
            "claude-sonnet-4-5-20250929",
        ] {
            let mut skill = make_skill("test", "Test skill");
            skill.model = Some(model.to_string());
            assert!(
                skill.validate_model().is_ok(),
                "Full model ID '{}' should be valid",
                model
            );
        }
    }

    #[test]
    fn test_invalid_model_not_claude_prefix() {
        let mut skill = make_skill("test", "Test skill");
        skill.model = Some("gemini-pro".to_string());
        assert!(skill.validate_model().is_err());
    }

    #[test]
    fn test_is_valid_skill_model() {
        assert!(is_valid_skill_model("sonnet"));
        assert!(is_valid_skill_model("opus"));
        assert!(is_valid_skill_model("haiku"));
        assert!(is_valid_skill_model("inherit"));
        assert!(is_valid_skill_model("claude-opus-4-6"));
        assert!(is_valid_skill_model("claude-sonnet-4-5-20250929"));
        assert!(!is_valid_skill_model("gpt-4"));
        assert!(!is_valid_skill_model("gemini-pro"));
    }

    // ===== Effort Validation =====

    #[test]
    fn test_valid_effort_values() {
        for effort in &["low", "medium", "high", "max"] {
            let mut skill = make_skill("test", "Test skill");
            skill.effort = Some(effort.to_string());
            assert!(
                skill.validate_effort().is_ok(),
                "Effort '{}' should be valid",
                effort
            );
        }
    }

    #[test]
    fn test_invalid_effort_value() {
        let mut skill = make_skill("test", "Test skill");
        skill.effort = Some("extreme".to_string());
        assert!(skill.validate_effort().is_err());
    }

    #[test]
    fn test_effort_none_ok() {
        let skill = make_skill("test", "Test skill");
        assert!(skill.validate_effort().is_ok());
    }

    // ===== Shell Validation =====

    #[test]
    fn test_valid_shell_values() {
        for shell_val in &["bash", "powershell"] {
            let mut skill = make_skill("test", "Test skill");
            skill.shell = Some(shell_val.to_string());
            assert!(
                skill.validate_shell().is_ok(),
                "Shell '{}' should be valid",
                shell_val
            );
        }
    }

    #[test]
    fn test_invalid_shell_value() {
        let mut skill = make_skill("test", "Test skill");
        skill.shell = Some("zsh".to_string());
        assert!(skill.validate_shell().is_err());
    }

    #[test]
    fn test_shell_none_ok() {
        let skill = make_skill("test", "Test skill");
        assert!(skill.validate_shell().is_ok());
    }

    // ===== Paths Field =====

    #[test]
    fn test_paths_field_stores_string_value() {
        let mut skill = make_skill("test", "Test skill");
        skill.paths = Some(serde_yaml::Value::String(
            "src/**/*.rs, tests/**/*.rs".to_string(),
        ));
        assert!(skill.paths.is_some());
        match &skill.paths {
            Some(serde_yaml::Value::String(s)) => {
                assert_eq!(s, "src/**/*.rs, tests/**/*.rs");
            }
            _ => panic!("Expected String value"),
        }
    }

    #[test]
    fn test_paths_field_stores_sequence_value() {
        let mut skill = make_skill("test", "Test skill");
        skill.paths = Some(serde_yaml::Value::Sequence(vec![
            serde_yaml::Value::String("src/**/*.rs".to_string()),
            serde_yaml::Value::String("tests/**/*.rs".to_string()),
        ]));
        match &skill.paths {
            Some(serde_yaml::Value::Sequence(seq)) => {
                assert_eq!(seq.len(), 2);
            }
            _ => panic!("Expected Sequence value"),
        }
    }

    // ===== Known Frontmatter Fields =====

    #[test]
    fn test_known_fields_include_new_fields() {
        assert!(KNOWN_SKILL_FRONTMATTER_FIELDS.contains(&"effort"));
        assert!(KNOWN_SKILL_FRONTMATTER_FIELDS.contains(&"paths"));
        assert!(KNOWN_SKILL_FRONTMATTER_FIELDS.contains(&"shell"));
    }

    #[test]
    fn test_known_fields_include_existing_fields() {
        for field in &[
            "name",
            "description",
            "model",
            "context",
            "agent",
            "hooks",
            "allowed-tools",
        ] {
            assert!(
                KNOWN_SKILL_FRONTMATTER_FIELDS.contains(field),
                "Known fields should include '{}'",
                field
            );
        }
    }
}
