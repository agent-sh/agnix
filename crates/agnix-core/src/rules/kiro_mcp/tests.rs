use super::*;
use crate::config::{LintConfig, ToolVersions};
use crate::diagnostics::DiagnosticLevel;

const SETTINGS_PATH: &str = ".kiro/settings/mcp.json";

fn validate(content: &str) -> Vec<Diagnostic> {
    KiroMcpValidator.validate(Path::new(SETTINGS_PATH), content, &LintConfig::default())
}

fn pinned(version: &str) -> LintConfig {
    LintConfig::builder()
        .tool_versions(ToolVersions {
            kiro: Some(version.to_string()),
            ..ToolVersions::default()
        })
        .build()
        .unwrap()
}

fn validate_pinned(content: &str, version: &str) -> Vec<Diagnostic> {
    KiroMcpValidator.validate(Path::new(SETTINGS_PATH), content, &pinned(version))
}

fn rules(diagnostics: &[Diagnostic], rule: &str) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|d| d.rule == rule)
        .cloned()
        .collect()
}

fn only(diagnostics: &[Diagnostic], rule: &str) -> Diagnostic {
    let hits = rules(diagnostics, rule);
    assert_eq!(hits.len(), 1, "expected one {rule}: {diagnostics:#?}");
    hits.into_iter().next().unwrap()
}

fn server(fields: &str) -> String {
    format!(r#"{{"mcpServers": {{"s": {{{fields}}}}}}}"#)
}

// ===== Fixtures and existing behavior =====

#[test]
fn test_kr_mcp_001_missing_command_and_url() {
    let diagnostics = validate(include_str!(
        "../../../../../tests/fixtures/kiro-mcp/.kiro/settings/missing-command-url.json"
    ));
    let d = only(&diagnostics, "KR-MCP-001");
    // Points at the server name, not line 1.
    assert_eq!((d.line, d.column), (3, 5));
}

#[test]
fn test_kr_mcp_002_hardcoded_secret() {
    let diagnostics = validate(include_str!(
        "../../../../../tests/fixtures/kiro-mcp/.kiro/settings/hardcoded-secrets.json"
    ));
    let d = only(&diagnostics, "KR-MCP-002");
    assert_eq!(d.line, 7);
}

#[test]
fn test_valid_kiro_mcp_configs_have_no_kr_mcp_diagnostics() {
    for fixture in [
        include_str!("../../../../../tests/fixtures/kiro-mcp/.kiro/settings/valid-local-mcp.json"),
        include_str!("../../../../../tests/fixtures/kiro-mcp/.kiro/settings/valid-remote-mcp.json"),
        include_str!("../../../../../tests/fixtures/kiro-mcp/.kiro/settings/mcp.json"),
    ] {
        let diagnostics = validate(fixture);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }
}

#[test]
fn test_kr_mcp_001_malformed_json() {
    let d = only(&validate(r#"{"mcpServers": {not valid json"#), "KR-MCP-001");
    assert_eq!(d.level, DiagnosticLevel::Error);
    assert_eq!(d.line, 1);
    assert!(d.column > 1, "{d:?}");
}

#[test]
fn test_kr_mcp_001_missing_or_invalid_root() {
    for content in ["{}", "null", "[]", "\"x\""] {
        let d = only(&validate(content), "KR-MCP-001");
        assert_eq!(d.level, DiagnosticLevel::Error, "{content}");
    }
    for content in [r#"{"mcpServers": []}"#, r#"{"mcpServers": null}"#] {
        let d = only(&validate(content), "KR-MCP-001");
        assert!(d.message.contains("must be an object"), "{content}: {d:?}");
    }
    assert!(validate(r#"{"mcpServers": {}}"#).is_empty());
}

#[test]
fn test_user_mcp_json_powers_section_is_not_flagged() {
    let diagnostics = validate(r#"{"mcpServers": {}, "powers": {"mcpServers": {}}}"#);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

// ===== JSONC: V3 accepts, V2 rejects the whole file =====

#[test]
fn test_jsonc_is_v2_only_failure() {
    let content = "{\n  // local servers\n  \"mcpServers\": {\"s\": {\"command\": \"node\",},},\n}";
    let d = only(&validate(content), "KR-MCP-001");
    assert_eq!(d.level, DiagnosticLevel::Warning);
    assert!(d.message.contains("V2"), "{d:?}");
    assert_eq!((d.line, d.column), (2, 3));
    assert_eq!(
        only(&validate_pinned(content, "2.32.1"), "KR-MCP-001").level,
        DiagnosticLevel::Error
    );
    assert!(validate_pinned(content, "3.0.0").is_empty());
}

#[test]
fn test_jsonc_document_is_still_validated() {
    let content = "{\"mcpServers\": {\"s\": {\"command\": 5}}} // trailing comment";
    let diagnostics = validate_pinned(content, "3.0.0");
    assert_eq!(
        only(&diagnostics, "KR-MCP-007").level,
        DiagnosticLevel::Error
    );
}

#[test]
fn test_unterminated_comment_is_a_parse_error() {
    let d = only(&validate("{\"mcpServers\": {}} /* open"), "KR-MCP-001");
    assert_eq!(d.level, DiagnosticLevel::Error);
}

// ===== KR-MCP-002 =====

#[test]
fn test_kr_mcp_002_ignores_corpus_false_positives() {
    let content = r#"{"mcpServers": {"s": {"command": "node", "env": {
        "TOKEN_INDEX_DIR": "/Users/3fn/Documents/Work Projects/Kiro/DesignerPunk/tokens",
        "CONFLUENCE_API_TOKEN": "$CONFLUENCE_API_TOKEN",
        "ANTHROPIC_API_KEY": "YOUR_ANTHROPIC_API_KEY",
        "GITHUB_PERSONAL_ACCESS_TOKEN": "${GITHUB_TOKEN}",
        "OPENAI_API_KEY": "OPENAI_API_KEY_HERE",
        "DATADOG_API_KEY": "your-datadog-api-key",
        "SLACK_BOT_TOKEN": "xoxb-your-token-here",
        "MAX_TOKENS": "4096000000",
        "API_KEY": "<YOUR_KEY>"
    }}}}"#;
    let diagnostics = validate(content);
    assert!(
        rules(&diagnostics, "KR-MCP-002").is_empty(),
        "{diagnostics:#?}"
    );
}

#[test]
fn test_kr_mcp_002_flags_real_looking_values_in_env_and_headers() {
    let content = r#"{"mcpServers": {"s": {"url": "https://example.com/mcp",
        "env": {"GITHUB_PAT": "q8Zt3vWx9LmN2pRs7KdYb4Hc"},
        "headers": {"Authorization": "Bearer abcdef0123456789abcdef", "X-Request-Id": "abcdef0123456789"}
    }}}"#;
    let hits = rules(&validate(content), "KR-MCP-002");
    assert_eq!(hits.len(), 2, "{hits:#?}");
    assert!(
        hits.iter()
            .any(|d| d.message.contains("headers 'Authorization'"))
    );
}

#[test]
fn test_kr_mcp_002_env_var_not_flagged() {
    let content = server(r#""command": "node", "env": {"API_KEY": "${MY_SECRET}"}"#);
    assert!(rules(&validate(&content), "KR-MCP-002").is_empty());
}

#[test]
fn test_sensitive_name_words() {
    for name in [
        "API_KEY",
        "OPENAI_API_KEY",
        "AWS_SECRET_ACCESS_KEY",
        "clientSecret",
        "x-api-key",
        "Authorization",
        "GITHUB_PAT",
        "DB_PASSWORD",
    ] {
        assert!(is_sensitive_name(name), "{name}");
    }
    for name in [
        "TOKEN_INDEX_DIR",
        "MAX_TOKENS",
        "GITHUB_TOKEN_FILE",
        "SECRET_NAME",
        "PUBLIC_KEY",
        "LOG_LEVEL",
        "keyboard",
    ] {
        assert!(!is_sensitive_name(name), "{name}");
    }
}

#[test]
fn test_kr_mcp_002_command_line_credentials() {
    let content = server(
        r#""command": "npx", "args": ["-y", "mcp-remote", "https://x.dev/mcp", "--header", "Authorization: Bearer q8Zt3vWx9LmN2pRs7KdYb4Hc", "--api-key=x7Rf2kLp9QwZ3mTnB5vD", "--token", "Zk3mQ8vT2pLw9xRb"]"#,
    );
    let hits = rules(&validate(&content), "KR-MCP-002");
    assert_eq!(hits.len(), 3, "{hits:#?}");
    assert!(hits.iter().all(|d| d.message.contains("in args")));
    let safe = server(
        r#""command": "npx", "args": ["--header", "Authorization: Bearer ${TOKEN}", "--api-key=$KEY", "--token", "--verbose", "--log-level=debug", "AWS_REGION=us-west-2", "https://user:pw@x"]"#,
    );
    assert!(rules(&validate(&safe), "KR-MCP-002").is_empty());
}

// ===== KR-MCP-015 =====

#[test]
fn test_kr_mcp_015_auto_approve_wildcard() {
    let d = only(
        &validate(&server(
            r#""command": "uvx", "args": ["aws-docs"], "autoApprove": ["*"]"#,
        )),
        "KR-MCP-015",
    );
    assert_eq!(d.level, DiagnosticLevel::Warning);
    for list in ["[]", r#"["search_docs", "read_doc"]"#] {
        let content = server(&format!(
            r#""command": "uvx", "args": [], "autoApprove": {list}"#
        ));
        assert!(
            rules(&validate(&content), "KR-MCP-015").is_empty(),
            "{list}"
        );
    }
}

// ===== KR-MCP-003 =====

#[test]
fn test_kr_mcp_003_args_are_optional() {
    // Previously flagged as "missing required args": args is optional (spec 3.2)
    // and kiro.dev's own example uses "args": [].
    for fields in [
        r#""command": "node""#,
        r#""command": "cockpit-mcp", "args": []"#,
    ] {
        assert!(
            rules(&validate(&server(fields)), "KR-MCP-003").is_empty(),
            "{fields}"
        );
    }
}

#[test]
fn test_kr_mcp_003_command_with_embedded_arguments() {
    let content = server(r#""command": "npx -y @modelcontextprotocol/server-github""#);
    let d = only(&validate(&content), "KR-MCP-003");
    assert_eq!(d.level, DiagnosticLevel::Warning);
    for command in [
        "/Applications/My App/bin/server",
        "C:\\\\Program Files\\\\nodejs\\\\node.exe",
        "uvx",
        "${HOME}/bin/my server",
    ] {
        let content = server(&format!(r#""command": "{command}""#));
        assert!(
            rules(&validate(&content), "KR-MCP-003").is_empty(),
            "{command}"
        );
    }
}

// ===== KR-MCP-004 / KR-MCP-008 =====

#[test]
fn test_kr_mcp_004_invalid_url() {
    for url in [
        "ftp://example.com/mcp",
        "not a url",
        "wss://example.com/mcp",
        "sse://example.com/mcp",
    ] {
        let content = server(&format!(r#""url": "{url}""#));
        let d = only(&validate(&content), "KR-MCP-004");
        assert_eq!(d.level, DiagnosticLevel::Error, "{url}");
    }
}

#[test]
fn test_kr_mcp_004_loopback_non_http_scheme_is_a_warning() {
    // Previously accepted: Kiro documents only https and http on localhost; the
    // V3 engine accepts any scheme for a loopback host at load time but connects
    // over HTTP.
    let d = only(
        &validate(&server(r#""url": "ws://localhost:8080/mcp""#)),
        "KR-MCP-004",
    );
    assert_eq!(d.level, DiagnosticLevel::Warning);
}

#[test]
fn test_kr_mcp_004_valid_urls() {
    for url in [
        "https://example.com/mcp",
        "http://localhost:9200/mcp",
        "http://127.0.0.1:3845/mcp",
        "http://[::1]:8080/mcp",
        "${MCP_URL}/mcp",
    ] {
        let diagnostics = validate(&server(&format!(r#""url": "{url}""#)));
        assert!(diagnostics.is_empty(), "{url}: {diagnostics:#?}");
    }
}

#[test]
fn test_kr_mcp_008_insecure_remote_url_is_v3_only() {
    let content = server(r#""url": "http://example.com/mcp""#);
    let d = only(&validate(&content), "KR-MCP-008");
    assert_eq!(d.level, DiagnosticLevel::Warning);
    assert!(d.message.contains("V3"));
    assert_eq!(
        only(&validate_pinned(&content, "3.1.0"), "KR-MCP-008").level,
        DiagnosticLevel::Error
    );
    assert!(validate_pinned(&content, "2.32.1").is_empty());
}

#[test]
fn test_url_ignored_when_command_present() {
    let content = server(r#""command": "node", "url": "http://example.com/mcp""#);
    let diagnostics = validate(&content);
    assert!(rules(&diagnostics, "KR-MCP-008").is_empty());
    assert_eq!(
        only(&diagnostics, "KR-MCP-011").level,
        DiagnosticLevel::Info
    );
}

// ===== KR-MCP-005 =====

#[test]
fn test_kr_mcp_005_duplicate_server_name() {
    let content =
        "{\"mcpServers\": {\n  \"a\": {\"command\": 5},\n  \"a\": {\"command\": \"node\"}\n}}";
    let diagnostics = validate(content);
    let d = only(&diagnostics, "KR-MCP-005");
    assert_eq!(d.line, 2);
    assert!(d.message.contains("line 3"), "{d:?}");
    // Only the effective (last) definition is validated.
    assert!(rules(&diagnostics, "KR-MCP-007").is_empty());
}

// ===== KR-MCP-006 =====

#[test]
fn test_kr_mcp_006_documented_fields_allowed() {
    let content = server(
        r#""url": "https://mcp.figma.com/mcp", "oauth": {"clientId": "registered-client",
        "clientSecret": "registered-secret", "redirectUri": "http://localhost:7778/oauth/callback",
        "oauthScopes": ["files:read"]}"#,
    );
    let diagnostics = validate(&content);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn test_kr_mcp_006_client_id_is_optional_for_dcr() {
    let content = server(
        r#""url": "https://example.com/mcp", "oauth": {"redirectUri": ":7778", "oauthScopes": ["read"]}"#,
    );
    assert!(validate(&content).is_empty());
}

#[test]
fn test_kr_mcp_006_semantic_problems() {
    for oauth in [
        r#"{"clientId": ""}"#,
        r#"{"clientSecret": "secret"}"#,
        r#"{"clientId": "id", "clientSecret": ""}"#,
    ] {
        let content = server(&format!(
            r#""url": "https://example.com/mcp", "oauth": {oauth}"#
        ));
        let d = only(&validate(&content), "KR-MCP-006");
        assert_eq!(d.level, DiagnosticLevel::Warning, "{oauth}");
    }
    let d = only(
        &validate(&server(
            r#""url": "https://example.com/mcp", "oauth": {"clientSecret": "secret"}"#,
        )),
        "KR-MCP-006",
    );
    assert!(d.message.contains("clientId"));
}

#[test]
fn test_kr_mcp_006_redirect_uri_requires_http_loopback() {
    for redirect_uri in [
        "https://localhost:7778/oauth/callback",
        "http://example.com:7778/oauth/callback",
        "localhost:not-a-port",
        "127.0.0.1:7778/oauth/callback",
        ":7778/oauth/callback",
        ":0",
        "http://localhost:7778/cb?x=1",
        "http://localhost/oauth/callback",
    ] {
        let content = server(&format!(
            r#""url": "https://example.com/mcp", "oauth": {{"redirectUri": "{redirect_uri}"}}"#
        ));
        let d = only(&validate(&content), "KR-MCP-006");
        // The V3 engine skips the server; Kiro CLI 2.x does not check it at load.
        assert_eq!(d.level, DiagnosticLevel::Warning, "{redirect_uri}");
        assert_eq!(
            only(&validate_pinned(&content, "3.0.0"), "KR-MCP-006").level,
            DiagnosticLevel::Error,
            "{redirect_uri}"
        );
    }
}

#[test]
fn test_kr_mcp_006_documented_redirect_uri_formats_allowed() {
    for redirect_uri in [
        "http://localhost:7778/oauth/callback",
        "127.0.0.1:7778",
        ":7778",
        "localhost:65535",
    ] {
        let content = server(&format!(
            r#""url": "https://example.com/mcp", "oauth": {{"redirectUri": "{redirect_uri}"}}"#
        ));
        assert!(validate(&content).is_empty(), "{redirect_uri}");
    }
}

#[test]
fn test_kr_mcp_006_client_metadata_url() {
    let bad = server(
        r#""url": "https://example.com/mcp", "oauth": {"clientMetadataUrl": "http://example.com/meta.json", "redirectUri": ":8080"}"#,
    );
    only(&validate(&bad), "KR-MCP-006");
    // Ignored when clientId is set ("Do not set both"); this used to pass.
    let ignored = server(
        r#""url": "https://example.com/mcp", "oauth": {"clientId": "id", "clientMetadataUrl": "https://example.com/meta.json"}"#,
    );
    let d = only(&validate(&ignored), "KR-MCP-006");
    assert!(
        d.message
            .contains("ignored because 'oauth.clientId' is set"),
        "{d:?}"
    );
    // Without a pinned redirectUri the callback is rejected on every connect.
    let unpinned = server(
        r#""url": "https://example.com/mcp", "oauth": {"clientMetadataUrl": "https://example.com/meta.json"}"#,
    );
    let d = only(&validate(&unpinned), "KR-MCP-006");
    assert!(d.message.contains("pinned 'oauth.redirectUri'"), "{d:?}");
    let good = server(
        r#""url": "https://example.com/mcp", "oauth": {"clientMetadataUrl": "https://apps.example.com/kiro-client.json", "redirectUri": "http://127.0.0.1:8080/oauth/callback"}"#,
    );
    assert!(validate(&good).is_empty());
}

#[test]
fn test_kr_mcp_006_oauth_on_command_or_non_http_server() {
    for fields in [
        r#""command": "node", "args": ["server.js"], "oauth": {"clientId": "c"}"#,
        r#""url": "sse://example.com/mcp", "oauth": {"clientId": "c"}"#,
    ] {
        only(&validate(&server(fields)), "KR-MCP-006");
    }
}

// ===== KR-MCP-007 =====

#[test]
fn test_kr_mcp_007_type_errors_no_longer_hide_other_checks() {
    // Previously any typed-schema error became KR-MCP-001 and stopped all
    // other checks for the file.
    let content = r#"{"mcpServers": {
        "a": {"command": "node", "args": "server.js"},
        "b": {"url": "ftp://example.com"}
    }}"#;
    let diagnostics = validate(content);
    let d = only(&diagnostics, "KR-MCP-007");
    assert_eq!(d.level, DiagnosticLevel::Error);
    assert!(d.message.contains("rejects the whole mcp.json"), "{d:?}");
    assert_eq!(d.line, 2);
    only(&diagnostics, "KR-MCP-004");
    assert!(rules(&diagnostics, "KR-MCP-001").is_empty());
}

#[test]
fn test_kr_mcp_007_engine_specific_types() {
    // (fields, fatal in V2, fatal in V3), observed with kiro-cli mcp list 2.32.1
    // and read from the V3 engine schema.
    let cases = [
        (r#""command": "n", "timeout": -1"#, true, true),
        (r#""command": "n", "timeout": 1.5"#, true, false),
        (r#""command": "n", "timeout": 1e3"#, true, false),
        (r#""command": "n", "timeout": 0"#, false, true),
        (r#""command": "n", "timeout": "100""#, true, true),
        (r#""command": "n", "env": null"#, false, true),
        (r#""command": "n", "env": {"A": 1}"#, true, true),
        (r#""command": "n", "args": null"#, true, true),
        (r#""command": "n", "args": [1]"#, true, true),
        (r#""command": "n", "disabled": "true""#, true, true),
        (r#""command": "n", "type": null"#, false, true),
        (r#""command": "n", "type": 5"#, true, true),
        (r#""command": "n", "cwd": 5"#, false, true),
        (r#""command": "n", "waitForReady": "x""#, false, true),
        (r#""command": "n", "autoApprove": "x""#, false, true),
        (
            r#""command": "n", "versionNegotiation": "weird""#,
            false,
            true,
        ),
        (r#""command": "n", "disabledTools": "x""#, true, true),
        (r#""url": "https://x.dev", "headers": {"A": 1}"#, true, true),
        (r#""url": "https://x.dev", "oauth": "x""#, true, true),
        (r#""url": "https://x.dev", "oauth": null"#, false, true),
        (
            r#""url": "https://x.dev", "oauth": {"clientId": 5}"#,
            true,
            true,
        ),
        (
            r#""url": "https://x.dev", "oauth": {"clientId": null}"#,
            false,
            true,
        ),
        (
            r#""url": "https://x.dev", "oauth": {"clientSecret": 5}"#,
            true,
            false,
        ),
        (
            r#""url": "https://x.dev", "oauth": {"oauthScopes": "x"}"#,
            true,
            false,
        ),
        (r#""url": "https://x.dev", "oauthScopes": [1]"#, true, true),
        (r#""command": 5"#, true, true),
        (r#""url": 5"#, true, true),
    ];
    for (fields, v2, v3) in cases {
        let content = server(fields);
        for (version, expect) in [("2.32.1", v2), ("3.0.0", v3)] {
            let hits = rules(&validate_pinned(&content, version), "KR-MCP-007");
            assert_eq!(
                !hits.is_empty(),
                expect,
                "{fields} pinned {version}: {hits:#?}"
            );
            if expect {
                assert_eq!(hits[0].level, DiagnosticLevel::Error, "{fields} {version}");
            }
        }
        let hits = rules(&validate(&content), "KR-MCP-007");
        let expected = match (v2, v3) {
            (true, true) => Some(DiagnosticLevel::Error),
            (false, false) => None,
            _ => Some(DiagnosticLevel::Warning),
        };
        assert_eq!(hits.first().map(|d| d.level), expected, "{fields} unpinned");
    }
}

#[test]
fn test_kr_mcp_007_valid_values_accepted() {
    let content = server(
        r#""command": "n", "args": [], "env": {"A": "b"}, "timeout": 100000, "disabled": false,
        "disabledTools": ["t"], "autoApprove": [], "cwd": "/tmp", "type": "stdio",
        "waitForReady": true, "versionNegotiation": "legacy""#,
    );
    let diagnostics = validate(&content);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn test_kr_mcp_007_server_not_an_object() {
    let d = only(&validate(r#"{"mcpServers": {"s": "node"}}"#), "KR-MCP-007");
    assert_eq!(d.level, DiagnosticLevel::Error);
}

#[test]
fn test_kr_mcp_007_location_points_at_value() {
    let content = "{\n  \"mcpServers\": {\n    \"s\": {\n      \"command\": \"node\",\n      \"args\": \"x\"\n    }\n  }\n}";
    let d = only(&validate(content), "KR-MCP-007");
    assert_eq!((d.line, d.column), (5, 15));
}

#[test]
fn test_disabled_and_registry_servers_need_no_transport() {
    for fields in [r#""disabled": true"#, r#""type": "registry""#] {
        let diagnostics = validate(&server(fields));
        assert!(diagnostics.is_empty(), "{fields}: {diagnostics:#?}");
    }
    let content = server(r#""disabled": false"#);
    only(&validate(&content), "KR-MCP-001");
}

// ===== KR-MCP-009 =====

#[test]
fn test_kr_mcp_009_unexpanded_references() {
    let content = r#"{"mcpServers": {"s": {"command": "uvx", "args": ["mcp-atlassian"],
        "env": {"JIRA_API_TOKEN": "$JIRA_API_TOKEN", "B": "${env:B}", "C": "${C}", "D": "price $5"},
        "cwd": "${workspaceFolder}"}}}"#;
    let hits = rules(&validate(content), "KR-MCP-009");
    assert_eq!(hits.len(), 3, "{hits:#?}");
    assert!(hits.iter().all(|d| d.level == DiagnosticLevel::Warning));
    assert!(hits.iter().any(|d| d.message.contains("'$JIRA_API_TOKEN'")));
    assert!(
        hits.iter()
            .any(|d| d.message.contains("editor variable '${workspaceFolder}'"))
    );
    // Kiro CLI 2.x expansion beyond ${NAME} is undocumented.
    assert!(rules(&validate_pinned(content, "2.32.1"), "KR-MCP-009").is_empty());
}

#[test]
fn test_kr_mcp_009_shell_arguments_are_expanded_by_the_shell() {
    let content = server(r#""command": "bash", "args": ["-c", "exec server --token $TOKEN"]"#);
    assert!(rules(&validate(&content), "KR-MCP-009").is_empty());
}

#[test]
fn test_unsupported_reference_scanner() {
    assert_eq!(unsupported_reference("$A_B c"), Some("$A_B".into()));
    assert_eq!(
        unsupported_reference("x ${env:TOKEN}"),
        Some("${env:TOKEN}".into())
    );
    assert_eq!(unsupported_reference("${HOME}/x"), None);
    assert_eq!(unsupported_reference("pa$$word"), None);
    assert_eq!(unsupported_reference("\\$HOME"), None);
    assert_eq!(unsupported_reference("cost: $5"), None);
    assert_eq!(unsupported_reference("${unterminated"), None);
}

// ===== KR-MCP-010 =====

#[test]
fn test_kr_mcp_010_unknown_field_with_suggestion() {
    let content = server(r#""command": "uvx", "args": [], "transport": "stdio", "evn": {}"#);
    let hits = rules(&validate(&content), "KR-MCP-010");
    assert_eq!(hits.len(), 2);
    assert!(hits.iter().all(|d| d.level == DiagnosticLevel::Info));
    let typo = hits.iter().find(|d| d.message.contains("'evn'")).unwrap();
    assert!(typo.suggestion.as_deref().unwrap().contains("'env'"));
}

// ===== Power variants =====

fn power_dir(manifest: Option<&str>) -> tempfile::TempDir {
    let temp = tempfile::TempDir::new().unwrap();
    match manifest {
        Some(manifest) => std::fs::write(temp.path().join("plugin.json"), manifest).unwrap(),
        None => std::fs::write(temp.path().join("POWER.md"), "---\nname: p\n---\nBody\n").unwrap(),
    }
    temp
}

fn validate_in(dir: &tempfile::TempDir, content: &str) -> Vec<Diagnostic> {
    let path = dir.path().join("mcp.json");
    std::fs::write(&path, content).unwrap();
    KiroMcpValidator.validate(&path, content, &LintConfig::default())
}

const MANIFEST: &str =
    r#"{"$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json", "name": "p"}"#;
const PLUGIN_SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json";

#[test]
fn test_legacy_power_mcp_is_v3_only() {
    let dir = power_dir(None);
    let content = r#"{"mcpServers": {"s": {"url": "http://example.com/mcp"}}}"#;
    let diagnostics = validate_in(&dir, content);
    // Insecure http is fatal in the only engine with powers.
    assert_eq!(
        only(&diagnostics, "KR-MCP-008").level,
        DiagnosticLevel::Error
    );
}

#[test]
fn test_legacy_power_file_problems_are_left_to_kr_pw_004() {
    // The V3 engine reads a legacy power's mcp.json with JSON.parse; with a
    // POWER.md next to it, KR-PW-004 reports parse and mcpServers problems.
    let dir = power_dir(None);
    for content in [
        "// docs\n{\"mcpServers\": {}}",
        "{\"mcpServers\": []}",
        "{}",
    ] {
        assert!(validate_in(&dir, content).is_empty(), "{content}");
    }
    // Without a POWER.md (only the .kiro/powers path) this validator reports them.
    let content = "// docs\n{\"mcpServers\": {}}";
    let diagnostics = KiroMcpValidator.validate(
        Path::new(".kiro/powers/x/mcp.json"),
        content,
        &LintConfig::default(),
    );
    assert_eq!(
        only(&diagnostics, "KR-MCP-001").level,
        DiagnosticLevel::Error
    );
}

#[test]
fn test_legacy_power_mcp_under_kiro_powers_path() {
    let content = r#"{"mcpServers": {"s": {"url": "https://example.com/mcp", "autoApprove": []}}}"#;
    let diagnostics = KiroMcpValidator.validate(
        Path::new(".kiro/powers/design/mcp.json"),
        content,
        &LintConfig::default(),
    );
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn test_plugin_mcp_valid() {
    let dir = power_dir(Some(MANIFEST));
    let content = format!(
        r#"{{"$schema": "{PLUGIN_SCHEMA}", "mcpServers": {{
        "local": {{"type": "stdio", "command": "./bin/server", "args": ["--x"], "env": {{"A": "b"}}, "cwd": "${{PLUGIN_ROOT}}/data"}},
        "remote": {{"type": "streamable-http", "url": "https://example.com/mcp", "headers": {{"X-A": "b"}}}},
        "events": {{"type": "sse", "url": "http://localhost:3000/sse"}}
    }}}}"#
    );
    let diagnostics = validate_in(&dir, &content);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn test_plugin_mcp_schema_and_top_level_keys() {
    let dir = power_dir(Some(MANIFEST));
    let missing = r#"{"mcpServers": {}}"#;
    only(&validate_in(&dir, missing), "KR-MCP-012");
    let other_version = r#"{"$schema": "https://agent-plugins.org/schemas/1.1.0/mcp.schema.json", "mcpServers": {}}"#;
    let d = only(&validate_in(&dir, other_version), "KR-MCP-012");
    assert!(
        d.message.contains("1.1.0") && d.message.contains("1.0.0"),
        "{d:?}"
    );
    let extra = format!(r#"{{"$schema": "{PLUGIN_SCHEMA}", "mcpServers": {{}}, "inputs": []}}"#);
    let d = only(&validate_in(&dir, &extra), "KR-MCP-013");
    assert_eq!(d.level, DiagnosticLevel::Error);
    let comments = format!("// x\n{{\"$schema\": \"{PLUGIN_SCHEMA}\", \"mcpServers\": {{}}}}");
    only(&validate_in(&dir, &comments), "KR-MCP-001");
}

#[test]
fn test_plugin_mcp_server_entries() {
    let dir = power_dir(Some(MANIFEST));
    for (server_json, needle) in [
        (r#""s": {"command": "node"}"#, "\"type\" must be"),
        (
            r#""s": {"type": "http", "url": "https://x.dev"}"#,
            "type \"http\"",
        ),
        (
            r#""s": {"type": "stdio", "command": "node", "disabled": false}"#,
            "field 'disabled'",
        ),
        (
            r#""s": {"type": "stdio", "command": "/usr/bin/node"}"#,
            "bare executable",
        ),
        (
            r#""s": {"type": "stdio", "command": "./../x"}"#,
            "inside the power",
        ),
        (
            r#""s": {"type": "stdio", "command": "node", "env": {"PLUGIN_ROOT": "x"}}"#,
            "PLUGIN_ROOT",
        ),
        (
            r#""s": {"type": "stdio", "command": "node", "cwd": "/tmp"}"#,
            "cwd must",
        ),
        (
            r#""s": {"type": "streamable-http", "url": "http://example.com"}"#,
            "must use https",
        ),
        (
            r#""s": {"type": "sse", "url": "https://u:p@example.com"}"#,
            "user name",
        ),
        (
            r#""s": {"type": "sse", "url": "https://example.com", "headers": {"A": "1", "a": "2"}}"#,
            "differ only in case",
        ),
    ] {
        let content =
            format!(r#"{{"$schema": "{PLUGIN_SCHEMA}", "mcpServers": {{{server_json}}}}}"#);
        let diagnostics = validate_in(&dir, &content);
        let d = only(&diagnostics, "KR-MCP-014");
        assert!(d.message.contains(needle), "{server_json}: {d:?}");
        // Kiro mcp.json rules do not apply to the plugin format.
        assert_eq!(diagnostics.len(), 1, "{server_json}: {diagnostics:#?}");
    }
}

#[test]
fn test_plugin_mcp_secrets_are_checked() {
    let dir = power_dir(Some(MANIFEST));
    let content = format!(
        r#"{{"$schema": "{PLUGIN_SCHEMA}", "mcpServers": {{"s": {{"type": "stdio", "command": "node", "env": {{"API_KEY": "x7Rf2kLp9QwZ3mTnB5vD"}}}}}}}}"#
    );
    only(&validate_in(&dir, &content), "KR-MCP-002");
}

#[test]
fn test_plugin_schema_identifies_variant_without_manifest() {
    let content = format!(
        r#"{{"$schema": "{PLUGIN_SCHEMA}", "mcpServers": {{"s": {{"command": "node"}}}}}}"#
    );
    let diagnostics = KiroMcpValidator.validate(
        Path::new("powers/x/mcp.json"),
        &content,
        &LintConfig::default(),
    );
    only(&diagnostics, "KR-MCP-014");
}

#[test]
fn test_plugin_schema_version_parsing() {
    assert_eq!(
        plugin_schema_version(PLUGIN_SCHEMA, "mcp.schema.json"),
        Some("1.0.0")
    );
    for bad in [
        "https://agent-plugins.org/schemas/2.0.0/mcp.schema.json",
        "https://agent-plugins.org/schemas/1.0/mcp.schema.json",
        "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
        "http://agent-plugins.org/schemas/1.0.0/mcp.schema.json",
    ] {
        assert_eq!(plugin_schema_version(bad, "mcp.schema.json"), None, "{bad}");
    }
}

#[test]
fn test_rules_can_be_disabled_individually() {
    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = RULE_IDS.iter().map(|id| id.to_string()).collect();
    let content = r#"{"mcpServers": {"a": {"command": "npx -y x", "args": "y", "env": {"API_KEY": "abcdef123456789"}},
        "b": {"url": "http://example.com", "x": 1}, "b": {}}}"#;
    let diagnostics = KiroMcpValidator.validate(Path::new(SETTINGS_PATH), content, &config);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn test_metadata() {
    let metadata = KiroMcpValidator.metadata();
    assert_eq!(metadata.name, "KiroMcpValidator");
    assert_eq!(metadata.rule_ids, RULE_IDS);
    assert_eq!(metadata.rule_ids.len(), 15);
}
