//! Kiro MCP validation rules (KR-MCP-001 to KR-MCP-006).
//!
//! Validates `.kiro/settings/mcp.json`:
//! - KR-MCP-001: Server missing both command and url
//! - KR-MCP-002: Hardcoded secrets in env values
//! - KR-MCP-003: Missing required args
//! - KR-MCP-004: Invalid MCP URL
//! - KR-MCP-005: Duplicate MCP server names
//! - KR-MCP-006: Invalid OAuth client ID configuration

use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata, seems_plaintext_secret},
    schemas::kiro_mcp::parse_kiro_mcp_config,
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &[
    "KR-MCP-001",
    "KR-MCP-002",
    "KR-MCP-003",
    "KR-MCP-004",
    "KR-MCP-005",
    "KR-MCP-006",
];

pub struct KiroMcpValidator;

impl Validator for KiroMcpValidator {
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

            // KR-MCP-003: Missing required args for command-based servers
            if config.is_rule_enabled("KR-MCP-003") && has_command {
                let has_args = server.args.as_ref().is_some_and(|args| !args.is_empty());
                if !has_args {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            1,
                            0,
                            "KR-MCP-003",
                            t!("rules.kr_mcp_003.message", server = server_name.as_str()),
                        )
                        .with_suggestion(t!("rules.kr_mcp_003.suggestion")),
                    );
                }
            }

            // KR-MCP-004: Invalid MCP URL
            if config.is_rule_enabled("KR-MCP-004") && has_url {
                let url_str = server.url.as_deref().unwrap_or_default();
                let is_valid_url = url_str.starts_with("http://")
                    || url_str.starts_with("https://")
                    || url_str.starts_with("ws://")
                    || url_str.starts_with("wss://")
                    // sse:// is an MCP transport convention for Server-Sent Events endpoints
                    || url_str.starts_with("sse://");
                if !is_valid_url {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "KR-MCP-004",
                            t!(
                                "rules.kr_mcp_004.message",
                                server = server_name.as_str(),
                                url = url_str
                            ),
                        )
                        .with_suggestion(t!("rules.kr_mcp_004.suggestion")),
                    );
                }
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

            // KR-MCP-006: oauth.clientId is only meaningful on HTTP(S) remote servers
            if config.is_rule_enabled("KR-MCP-006")
                && let Some(issue) = invalid_oauth_client_id_issue(&server)
            {
                let reason = issue.localized_reason();
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "KR-MCP-006",
                        t!(
                            "rules.kr_mcp_006.message",
                            server = server_name.as_str(),
                            reason = reason.as_str()
                        ),
                    )
                    .with_suggestion(t!("rules.kr_mcp_006.suggestion")),
                );
            }
        }

        // Note: KR-MCP-005 (duplicate MCP server names) is a project-level check
        // requiring cross-file analysis; registered in RULE_IDS but checked
        // at the project validator layer.

        diagnostics
    }
}

fn invalid_oauth_client_id_issue(
    server: &crate::schemas::kiro_mcp::KiroMcpServerConfig,
) -> Option<OauthClientIdIssue> {
    let oauth = server.extra.get("oauth")?;
    let Some(oauth) = oauth.as_object() else {
        return Some(OauthClientIdIssue::OauthMustBeObject);
    };

    let Some(client_id) = oauth.get("clientId") else {
        return Some(OauthClientIdIssue::MissingClientId);
    };
    let Some(client_id) = client_id.as_str() else {
        return Some(OauthClientIdIssue::ClientIdMustBeString);
    };
    if client_id.trim().is_empty() {
        return Some(OauthClientIdIssue::EmptyClientId);
    }

    let is_http_url = server
        .url
        .as_deref()
        .map(str::trim)
        .is_some_and(|url| url.starts_with("http://") || url.starts_with("https://"));
    if !is_http_url {
        return Some(OauthClientIdIssue::RequiresHttpUrl);
    }

    None
}

#[derive(Clone, Copy)]
enum OauthClientIdIssue {
    OauthMustBeObject,
    MissingClientId,
    ClientIdMustBeString,
    EmptyClientId,
    RequiresHttpUrl,
}

impl OauthClientIdIssue {
    fn localized_reason(self) -> String {
        match self {
            Self::OauthMustBeObject => t!("rules.kr_mcp_006.reasons.oauth_object").to_string(),
            Self::MissingClientId => t!("rules.kr_mcp_006.reasons.missing_client_id").to_string(),
            Self::ClientIdMustBeString => {
                t!("rules.kr_mcp_006.reasons.client_id_string").to_string()
            }
            Self::EmptyClientId => t!("rules.kr_mcp_006.reasons.client_id_non_empty").to_string(),
            Self::RequiresHttpUrl => t!("rules.kr_mcp_006.reasons.http_url").to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;

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
    fn test_kr_mcp_003_missing_args_for_command_server() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "local": {
      "command": "node"
    }
  }
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-MCP-003"));
    }

    #[test]
    fn test_kr_mcp_003_has_args_no_diagnostic() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "local": {
      "command": "node",
      "args": ["server.js"]
    }
  }
}"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-MCP-003"));
    }

    #[test]
    fn test_kr_mcp_004_invalid_url() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "remote": {
      "url": "ftp://example.com/mcp"
    }
  }
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-MCP-004"));
    }

    #[test]
    fn test_kr_mcp_004_valid_url_no_diagnostic() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "remote": {
      "url": "https://example.com/mcp"
    }
  }
}"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-MCP-004"));
    }

    #[test]
    fn test_kr_mcp_004_sse_url_accepted() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "remote": {
      "url": "sse://example.com/mcp"
    }
  }
}"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-MCP-004"));
    }

    #[test]
    fn test_kr_mcp_005_is_registered_in_metadata() {
        // KR-MCP-005 (duplicate server names) requires project-level context;
        // verify it is registered in metadata.
        let validator = KiroMcpValidator;
        let metadata = validator.metadata();
        assert!(metadata.rule_ids.contains(&"KR-MCP-005"));
    }

    #[test]
    fn test_kr_mcp_006_http_oauth_client_id_allowed() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "remote": {
      "url": "https://example.com/mcp",
      "oauth": {
        "clientId": "registered-client"
      }
    }
  }
}"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-MCP-006"));
    }

    #[test]
    fn test_kr_mcp_006_oauth_must_be_object() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "remote": {
      "url": "https://example.com/mcp",
      "oauth": "registered-client"
    }
  }
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-MCP-006"));
    }

    #[test]
    fn test_kr_mcp_006_requires_non_empty_client_id() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "remote": {
      "url": "https://example.com/mcp",
      "oauth": {
        "clientId": ""
      }
    }
  }
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-MCP-006"));
    }

    #[test]
    fn test_kr_mcp_006_missing_client_id() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "remote": {
      "url": "https://example.com/mcp",
      "oauth": {}
    }
  }
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-MCP-006"));
    }

    #[test]
    fn test_kr_mcp_006_oauth_on_command_server() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "local": {
      "command": "node",
      "args": ["server.js"],
      "oauth": {
        "clientId": "registered-client"
      }
    }
  }
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-MCP-006"));
    }

    #[test]
    fn test_kr_mcp_006_oauth_on_non_http_url() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "remote": {
      "url": "sse://example.com/mcp",
      "oauth": {
        "clientId": "registered-client"
      }
    }
  }
}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-MCP-006"));
    }

    // M16: KR-MCP-001 fires on malformed JSON
    #[test]
    fn test_kr_mcp_001_malformed_json() {
        let diagnostics = validate(r#"{"mcpServers": {not valid json"#);
        assert!(diagnostics.iter().any(|d| d.rule == "KR-MCP-001"));
    }

    // M17: KR-MCP-002 env var exclusion (negative test)
    #[test]
    fn test_kr_mcp_002_env_var_not_flagged() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "svc": {
      "command": "node",
      "args": ["server.js"],
      "env": {
        "API_KEY": "${MY_SECRET}"
      }
    }
  }
}"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-MCP-002"));
    }

    // L7: KR-MCP-004 ws:// and wss:// accepted
    #[test]
    fn test_kr_mcp_004_ws_url_accepted() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "ws-server": {
      "url": "ws://localhost:8080/mcp"
    }
  }
}"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-MCP-004"));
    }

    #[test]
    fn test_kr_mcp_004_wss_url_accepted() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "wss-server": {
      "url": "wss://example.com/mcp"
    }
  }
}"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-MCP-004"));
    }

    #[test]
    fn test_metadata() {
        let validator = KiroMcpValidator;
        let metadata = validator.metadata();
        assert_eq!(metadata.name, "KiroMcpValidator");
        assert_eq!(
            metadata.rule_ids,
            &[
                "KR-MCP-001",
                "KR-MCP-002",
                "KR-MCP-003",
                "KR-MCP-004",
                "KR-MCP-005",
                "KR-MCP-006"
            ]
        );
    }
}
