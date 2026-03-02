//! Kiro MCP validation rules (KR-MCP-001 to KR-MCP-002).
//!
//! Validates `.kiro/settings/mcp.json`:
//! - KR-MCP-001: Server missing both command and url
//! - KR-MCP-002: Hardcoded secrets in env values

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
    schemas::kiro_mcp::parse_kiro_mcp_config,
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["KR-MCP-001", "KR-MCP-002"];

fn seems_plaintext_secret(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with("${")
        && !trimmed.starts_with("$(")
        && !trimmed.starts_with("{{")
}

pub struct KiroMcpValidator;

impl Validator for KiroMcpValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let parsed = parse_kiro_mcp_config(content);

        if config.is_rule_enabled("KR-MCP-001")
            && let Some(parse_error) = parsed.parse_error.as_ref()
        {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    parse_error.line,
                    parse_error.column,
                    "KR-MCP-001",
                    t!(
                        "rules.kr_mcp_001_parse.message",
                        error = parse_error.message.as_str()
                    ),
                )
                .with_suggestion(t!("rules.kr_mcp_001_parse.suggestion")),
            );
            return diagnostics;
        }

        let Some(config_doc) = parsed.config else {
            return diagnostics;
        };

        let Some(servers) = config_doc.mcp_servers else {
            if config.is_rule_enabled("KR-MCP-001") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "KR-MCP-001",
                        t!("rules.kr_mcp_001_root.message"),
                    )
                    .with_suggestion(t!("rules.kr_mcp_001_root.suggestion")),
                );
            }
            return diagnostics;
        };

        for (server_name, server) in servers {
            let has_command = server
                .command
                .as_deref()
                .is_some_and(|command| !command.trim().is_empty());
            let has_url = server
                .url
                .as_deref()
                .is_some_and(|url| !url.trim().is_empty());

            if config.is_rule_enabled("KR-MCP-001") && !has_command && !has_url {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "KR-MCP-001",
                        t!("rules.kr_mcp_001.message", server = server_name.as_str()),
                    )
                    .with_suggestion(t!("rules.kr_mcp_001.suggestion")),
                );
            }

            if config.is_rule_enabled("KR-MCP-002")
                && let Some(env) = server.env.as_ref()
            {
                for (env_key, env_value) in env {
                    let key_upper = env_key.to_ascii_uppercase();
                    let looks_sensitive = ["API_KEY", "SECRET", "TOKEN", "PASSWORD"]
                        .iter()
                        .any(|needle| key_upper.contains(needle));

                    if looks_sensitive && seems_plaintext_secret(env_value) {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "KR-MCP-002",
                                t!(
                                    "rules.kr_mcp_002.message",
                                    server = server_name.as_str(),
                                    env_key = env_key.as_str()
                                ),
                            )
                            .with_suggestion(t!("rules.kr_mcp_002.suggestion")),
                        );
                    }
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = KiroMcpValidator;
        validator.validate(
            Path::new(".kiro/settings/mcp.json"),
            content,
            &LintConfig::default(),
        )
    }

    #[test]
    fn test_kr_mcp_001_missing_command_and_url() {
        let diagnostics = validate(include_str!(
            "../../../../tests/fixtures/kiro-mcp/.kiro/settings/missing-command-url.json"
        ));
        assert!(diagnostics.iter().any(|d| d.rule == "KR-MCP-001"));
    }

    #[test]
    fn test_kr_mcp_002_hardcoded_secret() {
        let diagnostics = validate(include_str!(
            "../../../../tests/fixtures/kiro-mcp/.kiro/settings/hardcoded-secrets.json"
        ));
        assert!(diagnostics.iter().any(|d| d.rule == "KR-MCP-002"));
    }

    #[test]
    fn test_valid_kiro_mcp_configs_have_no_kr_mcp_diagnostics() {
        let fixtures = [
            include_str!("../../../../tests/fixtures/kiro-mcp/.kiro/settings/valid-local-mcp.json"),
            include_str!(
                "../../../../tests/fixtures/kiro-mcp/.kiro/settings/valid-remote-mcp.json"
            ),
        ];

        for fixture in fixtures {
            let diagnostics = validate(fixture);
            assert!(diagnostics.iter().all(|d| !d.rule.starts_with("KR-MCP-")));
        }
    }

    #[test]
    fn test_metadata() {
        let validator = KiroMcpValidator;
        let metadata = validator.metadata();
        assert_eq!(metadata.name, "KiroMcpValidator");
        assert_eq!(metadata.rule_ids, &["KR-MCP-001", "KR-MCP-002"]);
    }
}
