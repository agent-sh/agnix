//! Codex CLI custom agent files (CDX-SUB-001 to CDX-SUB-003).
//!
//! Custom agents are standalone TOML files in `.codex/agents/` (project) or
//! `~/.codex/agents/` (personal). "Every standalone custom agent file must
//! define: `name`, `description`, `developer_instructions`"
//! (learn.chatgpt.com/docs/agent-configuration/subagents). The Codex loader
//! (`codex-rs/agent-roles/src/agent_role_config.rs`) rejects a file without a
//! non-empty `name` or `developer_instructions`, a blank `description`, and
//! an empty, blank, duplicate or non-ASCII `nickname_candidates` list.

use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    rules::config_span::{Seg, TomlSpans, line_col},
    rules::{Validator, ValidatorMetadata},
};
use rust_i18n::t;
use std::collections::BTreeSet;
use std::path::Path;

const RULE_IDS: &[&str] = &["CDX-SUB-001", "CDX-SUB-002", "CDX-SUB-003"];

/// Session settings a custom agent file cannot change: "Other session
/// settings, including `sandbox_mode`, `approval_policy`, `model_provider`,
/// and `mcp_servers`, don't override the parent configuration", "They
/// inherit the parent's permissions, provider, and MCP servers", and "The
/// service tier also follows the root session's preference".
const PARENT_ONLY_KEYS: &[&str] = &[
    "sandbox_mode",
    "sandbox_workspace_write",
    "approval_policy",
    "default_permissions",
    "permissions",
    "model_provider",
    "model_providers",
    "mcp_servers",
    "service_tier",
];

/// Built-in agents a custom agent with the same name replaces.
const BUILTIN_AGENTS: &[&str] = &["default", "worker", "explorer"];

pub struct CodexAgentValidator;

impl Validator for CodexAgentValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let sub_001 = config.is_rule_enabled("CDX-SUB-001");
        let table = match toml::from_str::<toml::Table>(content) {
            Ok(table) => table,
            Err(error) => {
                if sub_001 {
                    let (line, column) = error
                        .span()
                        .map_or((1, 1), |span| line_col(content, span.start));
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            column,
                            "CDX-SUB-001",
                            t!("rules.cdx_sub_001.parse_error", error = error.message()),
                        )
                        .with_suggestion(t!("rules.cdx_sub_001.suggestion")),
                    );
                }
                return diagnostics;
            }
        };
        let spans = TomlSpans::parse(content);
        let pos = |key: &str| {
            spans
                .as_ref()
                .and_then(|spans| spans.offset(&[Seg::Key(key)]))
                .map_or((1, 1), |offset| line_col(content, offset))
        };

        if sub_001 {
            for field in ["name", "description", "developer_instructions"] {
                let problem = match table.get(field) {
                    None => Some(t!("rules.cdx_sub_001.missing", field = field)),
                    Some(toml::Value::String(value)) if value.trim().is_empty() => {
                        Some(t!("rules.cdx_sub_001.blank", field = field))
                    }
                    Some(toml::Value::String(_)) => None,
                    Some(_) => Some(t!("rules.cdx_sub_001.not_string", field = field)),
                };
                if let Some(message) = problem {
                    let (line, column) = if table.contains_key(field) {
                        pos(field)
                    } else {
                        (1, 1)
                    };
                    diagnostics.push(
                        Diagnostic::error(path.to_path_buf(), line, column, "CDX-SUB-001", message)
                            .with_suggestion(t!("rules.cdx_sub_001.suggestion")),
                    );
                }
            }
            if let Some(candidates) = table.get("nickname_candidates")
                && let Some(reason) = nickname_problem(candidates)
            {
                let (line, column) = pos("nickname_candidates");
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        column,
                        "CDX-SUB-001",
                        t!("rules.cdx_sub_001.nicknames", reason = reason),
                    )
                    .with_suggestion(t!("rules.cdx_sub_001.nicknames_suggestion")),
                );
            }
        }

        if config.is_rule_enabled("CDX-SUB-002") {
            for key in PARENT_ONLY_KEYS {
                if table.contains_key(*key) {
                    let (line, column) = pos(key);
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            line,
                            column,
                            "CDX-SUB-002",
                            t!("rules.cdx_sub_002.message", key = *key),
                        )
                        .with_suggestion(t!("rules.cdx_sub_002.suggestion", key = *key)),
                    );
                }
            }
        }

        if config.is_rule_enabled("CDX-SUB-003")
            && let Some(name) = table.get("name").and_then(toml::Value::as_str)
            && BUILTIN_AGENTS.contains(&name.trim())
        {
            let (line, column) = pos("name");
            diagnostics.push(
                Diagnostic::info(
                    path.to_path_buf(),
                    line,
                    column,
                    "CDX-SUB-003",
                    t!("rules.cdx_sub_003.message", name = name.trim()),
                )
                .with_suggestion(t!("rules.cdx_sub_003.suggestion")),
            );
        }

        diagnostics
    }
}

/// Why Codex rejects a `nickname_candidates` value, mirroring
/// `normalize_agent_role_nickname_candidates`.
fn nickname_problem(value: &toml::Value) -> Option<&'static str> {
    let Some(items) = value.as_array() else {
        return Some("it must be an array of strings");
    };
    if items.is_empty() {
        return Some("it must contain at least one name");
    }
    let mut seen = BTreeSet::new();
    for item in items {
        let Some(name) = item.as_str() else {
            return Some("it must be an array of strings");
        };
        let name = name.trim();
        if name.is_empty() {
            return Some("it cannot contain blank names");
        }
        if !seen.insert(name.to_string()) {
            return Some("it cannot contain duplicates");
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        {
            return Some(
                "names may only contain ASCII letters, digits, spaces, hyphens, and underscores",
            );
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;

    fn run(content: &str) -> Vec<Diagnostic> {
        CodexAgentValidator.validate(
            Path::new(".codex/agents/reviewer.toml"),
            content,
            &LintConfig::default(),
        )
    }

    #[test]
    fn documented_examples_are_clean() {
        let content = r#"name = "reviewer"
description = "PR reviewer focused on correctness, security, and missing tests."
model = "gpt-6.1-sol"
model_reasoning_effort = "medium"
developer_instructions = """
Review code like an owner.
"""

[[skills.config]]
path = "/Users/me/.agents/skills/docs-editor/SKILL.md"
enabled = false
"#;
        assert!(run(content).is_empty(), "{:?}", run(content));
    }

    #[test]
    fn missing_or_blank_required_fields_are_errors() {
        let diagnostics = run("name = \"  \"\nmodel = \"x\"\n");
        let messages: Vec<_> = diagnostics.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(diagnostics.len(), 3, "{messages:?}");
        assert!(diagnostics.iter().all(|d| d.rule == "CDX-SUB-001"));
        assert_eq!(diagnostics[0].line, 1);
        assert!(
            messages
                .iter()
                .any(|m| m.contains("developer_instructions"))
        );
        let bad_toml = run("name = ");
        assert_eq!(bad_toml.len(), 1);
        assert_eq!(bad_toml[0].rule, "CDX-SUB-001");
    }

    #[test]
    fn nickname_candidates_follow_the_loader() {
        let base = "name = \"a\"\ndescription = \"d\"\ndeveloper_instructions = \"i\"\n";
        for (list, bad) in [
            ("[]", true),
            ("[\"ok\", \"ok\"]", true),
            ("[\"bad!\"]", true),
            ("[\"Ann\", \"bob-2\"]", false),
        ] {
            let diagnostics = run(&format!("{base}nickname_candidates = {list}\n"));
            assert_eq!(!diagnostics.is_empty(), bad, "{list}: {diagnostics:?}");
        }
    }

    #[test]
    fn parent_only_settings_and_builtin_names() {
        let content = "name = \"explorer\"\ndescription = \"d\"\ndeveloper_instructions = \"i\"\nsandbox_mode = \"read-only\"\n\n[mcp_servers.docs]\nurl = \"https://x\"\n";
        let diagnostics = run(content);
        let rules: Vec<_> = diagnostics.iter().map(|d| d.rule.as_str()).collect();
        assert_eq!(rules, ["CDX-SUB-002", "CDX-SUB-002", "CDX-SUB-003"]);
        assert_eq!(diagnostics[0].line, 4);
        assert_eq!(diagnostics[1].line, 6);
    }
}
