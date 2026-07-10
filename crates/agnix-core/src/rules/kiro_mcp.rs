//! Kiro MCP validation rules (KR-MCP-001 to KR-MCP-006).
//!
//! Validates `.kiro/settings/mcp.json`:
//! - KR-MCP-001: Server missing both command and url
//! - KR-MCP-002: Hardcoded secrets in env values
//! - KR-MCP-003: Missing required args
//! - KR-MCP-004: Invalid MCP URL
//! - KR-MCP-005: Duplicate MCP server names
//! - KR-MCP-006: Invalid OAuth configuration

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

            // KR-MCP-006: validate the optional OAuth fields documented for
            // Kiro CLI 2.12 and require OAuth only on HTTP(S) remote servers.
            if config.is_rule_enabled("KR-MCP-006")
                && let Some(issue) = invalid_oauth_configuration_issue(&server)
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

fn invalid_oauth_configuration_issue(
    server: &crate::schemas::kiro_mcp::KiroMcpServerConfig,
) -> Option<OauthConfigurationIssue> {
    let oauth = server.extra.get("oauth")?;
    let Some(oauth) = oauth.as_object() else {
        return Some(OauthConfigurationIssue::OauthMustBeObject);
    };

    let has_client_id = if let Some(client_id) = oauth.get("clientId") {
        let Some(client_id) = client_id.as_str() else {
            return Some(OauthConfigurationIssue::ClientIdMustBeString);
        };
        if client_id.trim().is_empty() {
            return Some(OauthConfigurationIssue::EmptyClientId);
        }
        true
    } else {
        false
    };

    if let Some(client_secret) = oauth.get("clientSecret") {
        let Some(client_secret) = client_secret.as_str() else {
            return Some(OauthConfigurationIssue::ClientSecretMustBeString);
        };
        if client_secret.trim().is_empty() {
            return Some(OauthConfigurationIssue::EmptyClientSecret);
        }
        if !has_client_id {
            return Some(OauthConfigurationIssue::ClientSecretRequiresClientId);
        }
    }

    if let Some(redirect_uri) = oauth.get("redirectUri") {
        let Some(redirect_uri) = redirect_uri.as_str() else {
            return Some(OauthConfigurationIssue::RedirectUriMustBeString);
        };
        if !is_valid_oauth_redirect_uri(redirect_uri) {
            return Some(OauthConfigurationIssue::InvalidRedirectUri);
        }
    }

    if let Some(oauth_scopes) = oauth.get("oauthScopes") {
        let Some(oauth_scopes) = oauth_scopes.as_array() else {
            return Some(OauthConfigurationIssue::OauthScopesMustBeArray);
        };
        if oauth_scopes.iter().any(|scope| !scope.is_string()) {
            return Some(OauthConfigurationIssue::OauthScopesMustContainStrings);
        }
    }

    let is_http_url = server
        .url
        .as_deref()
        .map(str::trim)
        .is_some_and(|url| url.starts_with("http://") || url.starts_with("https://"));
    if !is_http_url {
        return Some(OauthConfigurationIssue::RequiresHttpUrl);
    }

    None
}

fn is_valid_oauth_redirect_uri(value: &str) -> bool {
    let value = value.trim();
    if let Some(port) = value.strip_prefix(':') {
        return is_valid_port(port);
    }

    let authority = if let Some(rest) = value.strip_prefix("http://") {
        rest.split(['/', '?', '#']).next().unwrap_or_default()
    } else {
        if value.contains("://") {
            return false;
        }
        value
    };

    let Some((host, port)) = authority.rsplit_once(':') else {
        return false;
    };
    matches!(host, "localhost" | "127.0.0.1") && is_valid_port(port)
}

fn is_valid_port(value: &str) -> bool {
    value.parse::<u16>().is_ok_and(|port| port > 0)
}

#[derive(Clone, Copy)]
enum OauthConfigurationIssue {
    OauthMustBeObject,
    ClientIdMustBeString,
    EmptyClientId,
    ClientSecretMustBeString,
    EmptyClientSecret,
    ClientSecretRequiresClientId,
    RedirectUriMustBeString,
    InvalidRedirectUri,
    OauthScopesMustBeArray,
    OauthScopesMustContainStrings,
    RequiresHttpUrl,
}

impl OauthConfigurationIssue {
    fn localized_reason(self) -> String {
        match self {
            Self::OauthMustBeObject => t!("rules.kr_mcp_006.reasons.oauth_object").to_string(),
            Self::ClientIdMustBeString => {
                t!("rules.kr_mcp_006.reasons.client_id_string").to_string()
            }
            Self::EmptyClientId => t!("rules.kr_mcp_006.reasons.client_id_non_empty").to_string(),
            Self::ClientSecretMustBeString => {
                t!("rules.kr_mcp_006.reasons.client_secret_string").to_string()
            }
            Self::EmptyClientSecret => {
                t!("rules.kr_mcp_006.reasons.client_secret_non_empty").to_string()
            }
            Self::ClientSecretRequiresClientId => {
                t!("rules.kr_mcp_006.reasons.client_secret_requires_client_id").to_string()
            }
            Self::RedirectUriMustBeString => {
                t!("rules.kr_mcp_006.reasons.redirect_uri_string").to_string()
            }
            Self::InvalidRedirectUri => {
                t!("rules.kr_mcp_006.reasons.redirect_uri_format").to_string()
            }
            Self::OauthScopesMustBeArray => {
                t!("rules.kr_mcp_006.reasons.oauth_scopes_array").to_string()
            }
            Self::OauthScopesMustContainStrings => {
                t!("rules.kr_mcp_006.reasons.oauth_scopes_strings").to_string()
            }
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
    fn test_kr_mcp_006_kiro_2_12_oauth_fields_allowed() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "figma": {
      "url": "https://mcp.figma.com/mcp",
      "oauth": {
        "clientId": "registered-client",
        "clientSecret": "registered-secret",
        "redirectUri": "http://localhost:7778/oauth/callback",
        "oauthScopes": ["files:read"]
      }
    }
  }
}"#,
        );
        assert!(
            diagnostics.iter().all(|d| d.rule != "KR-MCP-006"),
            "Kiro 2.12 OAuth fields should be accepted: {diagnostics:?}"
        );
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
    fn test_kr_mcp_006_client_id_is_optional_for_dcr() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "remote": {
      "url": "https://example.com/mcp",
      "oauth": {
        "redirectUri": ":7778",
        "oauthScopes": ["read"]
      }
    }
  }
}"#,
        );
        assert!(
            diagnostics.iter().all(|d| d.rule != "KR-MCP-006"),
            "clientId is optional when Kiro uses dynamic client registration"
        );
    }

    #[test]
    fn test_kr_mcp_006_client_secret_requires_client_id() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "remote": {
      "url": "https://example.com/mcp",
      "oauth": {
        "clientSecret": "secret"
      }
    }
  }
}"#,
        );
        let diagnostic = diagnostics
            .iter()
            .find(|d| d.rule == "KR-MCP-006")
            .expect("clientSecret without clientId should be flagged");
        assert!(diagnostic.message.contains("clientId"));
    }

    #[test]
    fn test_kr_mcp_006_redirect_uri_requires_http_loopback() {
        for redirect_uri in [
            "https://localhost:7778/oauth/callback",
            "http://example.com:7778/oauth/callback",
            "localhost:not-a-port",
            ":0",
        ] {
            let content = format!(
                r#"{{
  "mcpServers": {{
    "remote": {{
      "url": "https://example.com/mcp",
      "oauth": {{
        "redirectUri": "{redirect_uri}"
      }}
    }}
  }}
}}"#
            );
            let diagnostics = validate(&content);
            assert!(
                diagnostics.iter().any(|d| d.rule == "KR-MCP-006"),
                "invalid redirectUri should be flagged: {redirect_uri}"
            );
        }
    }

    #[test]
    fn test_kr_mcp_006_documented_redirect_uri_formats_allowed() {
        for redirect_uri in [
            "http://localhost:7778/oauth/callback",
            "127.0.0.1:7778",
            ":7778",
        ] {
            let content = format!(
                r#"{{
  "mcpServers": {{
    "remote": {{
      "url": "https://example.com/mcp",
      "oauth": {{
        "redirectUri": "{redirect_uri}"
      }}
    }}
  }}
}}"#
            );
            let diagnostics = validate(&content);
            assert!(
                diagnostics.iter().all(|d| d.rule != "KR-MCP-006"),
                "documented redirectUri should be accepted: {redirect_uri}"
            );
        }
    }

    #[test]
    fn test_kr_mcp_006_optional_field_types_are_checked() {
        let invalid_fragments = [
            r#""clientId": 42"#,
            r#""clientId": "id", "clientSecret": []"#,
            r#""clientId": "id", "clientSecret": """#,
            r#""redirectUri": 7778"#,
            r#""oauthScopes": "read""#,
        ];

        for fields in invalid_fragments {
            let content = format!(
                r#"{{
  "mcpServers": {{
    "remote": {{
      "url": "https://example.com/mcp",
      "oauth": {{ {fields} }}
    }}
  }}
}}"#
            );
            let diagnostics = validate(&content);
            assert!(
                diagnostics.iter().any(|d| d.rule == "KR-MCP-006"),
                "invalid OAuth field should be flagged: {fields}"
            );
        }
    }

    #[test]
    fn test_kr_mcp_006_oauth_scopes_must_be_string_array() {
        let diagnostics = validate(
            r#"{
  "mcpServers": {
    "remote": {
      "url": "https://example.com/mcp",
      "oauth": {
        "oauthScopes": ["read", 42]
      }
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
