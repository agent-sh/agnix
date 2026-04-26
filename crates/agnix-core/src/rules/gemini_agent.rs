//! Gemini CLI agent definition validation (GM-AG-*).
//!
//! Validates the YAML frontmatter of `.gemini/agents/*.md` files against the
//! schema documented in `packages/core/src/agents/agentLoader.ts` in the
//! gemini-cli repo. Today's scope is narrow: the `mcp_servers.*.auth` block
//! added in gemini-cli v0.39.0 (google-gemini/gemini-cli#24770).
//!
//! ## Auth block schema
//!
//! Two variants, distinguished by `type`:
//!
//! 1. `type: "google-credentials"` - Application Default Credentials
//!    - `scopes: [string]` (optional)
//!
//! 2. `type: "oauth"` - Standard OAuth 2.0 flow
//!    - `client_id: string` (optional)
//!    - `client_secret: string` (optional)
//!    - `scopes: [string]` (optional)
//!    - `authorization_url: string (URL)` (optional)
//!    - `token_url: string (URL)` (optional)
//!
//! This validator deliberately does NOT enforce frontmatter structure
//! beyond the auth block - the larger Gemini agent schema (kind, name,
//! description, tools, system_prompt, mcp_servers shape) is deferred to a
//! future expansion so #809 can ship as a focused rule.

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["GM-AG-001"];

/// Allowed auth `type` discriminator values.
const VALID_AUTH_TYPES: &[&str] = &["google-credentials", "oauth"];

pub struct GeminiAgentValidator;

impl Validator for GeminiAgentValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if !config.is_rule_enabled("GM-AG-001") {
            return diagnostics;
        }

        // Gemini agent files have YAML frontmatter between `---` markers.
        // Bail silently if no frontmatter - this isn't a frontmatter-shape
        // rule, only an auth-block-contents rule.
        let Some(frontmatter_raw) = extract_frontmatter(content) else {
            return diagnostics;
        };

        let parsed: serde_yaml::Value = match serde_yaml::from_str(frontmatter_raw) {
            Ok(v) => v,
            Err(_) => return diagnostics, // Malformed YAML surfaced by other validators
        };

        // Walk mcp_servers.*.auth and validate each auth block.
        let Some(mcp_servers) = parsed
            .get("mcp_servers")
            .and_then(serde_yaml::Value::as_mapping)
        else {
            return diagnostics;
        };

        for (server_key, server_value) in mcp_servers {
            let server_name = server_key.as_str().unwrap_or("<unnamed>").to_string();
            let Some(server_obj) = server_value.as_mapping() else {
                continue;
            };
            let Some(auth) = server_obj.get("auth") else {
                continue;
            };

            validate_auth_block(path, content, &server_name, auth, &mut diagnostics);
        }

        diagnostics
    }
}

fn validate_auth_block(
    path: &Path,
    content: &str,
    server: &str,
    auth: &serde_yaml::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let line = find_auth_line(content, server).unwrap_or(1);

    // Must be a mapping.
    let Some(auth_obj) = auth.as_mapping() else {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                0,
                "GM-AG-001",
                t!("rules.gm_ag_001.not_object", server = server),
            )
            .with_suggestion(t!("rules.gm_ag_001.suggestion")),
        );
        return;
    };

    // Discriminator check: `type` must be present, a string, and one of
    // the valid literals. Non-string and missing get different diagnostics
    // because "type is a number" is a different mistake from "type is
    // absent entirely".
    let Some(type_entry) = auth_obj.get("type") else {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                0,
                "GM-AG-001",
                t!("rules.gm_ag_001.missing_type", server = server),
            )
            .with_suggestion(t!("rules.gm_ag_001.suggestion")),
        );
        return;
    };
    let Some(type_str) = type_entry.as_str().map(str::to_string) else {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                0,
                "GM-AG-001",
                t!(
                    "rules.gm_ag_001.not_string",
                    server = server,
                    field = "type"
                ),
            )
            .with_suggestion(t!("rules.gm_ag_001.suggestion")),
        );
        return;
    };

    if !VALID_AUTH_TYPES.contains(&type_str.as_str()) {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                0,
                "GM-AG-001",
                t!(
                    "rules.gm_ag_001.invalid_type",
                    server = server,
                    value = type_str.as_str(),
                    valid = VALID_AUTH_TYPES.join(", ")
                ),
            )
            .with_suggestion(t!("rules.gm_ag_001.suggestion")),
        );
        return;
    }

    // Per-variant schema checks.
    match type_str.as_str() {
        "google-credentials" => {
            // Only `scopes` is allowed beyond `type`. Reject unknown fields
            // to catch mistakes like pasting OAuth fields under
            // google-credentials.
            for (k, _) in auth_obj {
                let Some(key_str) = k.as_str() else { continue };
                if !matches!(key_str, "type" | "scopes") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            0,
                            "GM-AG-001",
                            t!(
                                "rules.gm_ag_001.unknown_field_google",
                                server = server,
                                field = key_str
                            ),
                        )
                        .with_suggestion(t!("rules.gm_ag_001.suggestion")),
                    );
                }
            }
            // `scopes`, if present, must be a list of strings.
            if let Some(scopes) = auth_obj.get("scopes") {
                validate_scopes(path, line, server, scopes, diagnostics);
            }
        }
        "oauth" => {
            let oauth_keys = [
                "type",
                "client_id",
                "client_secret",
                "scopes",
                "authorization_url",
                "token_url",
            ];
            for (k, _) in auth_obj {
                let Some(key_str) = k.as_str() else { continue };
                if !oauth_keys.contains(&key_str) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            0,
                            "GM-AG-001",
                            t!(
                                "rules.gm_ag_001.unknown_field_oauth",
                                server = server,
                                field = key_str
                            ),
                        )
                        .with_suggestion(t!("rules.gm_ag_001.suggestion")),
                    );
                }
            }
            // String fields, when present, must be strings.
            for string_key in ["client_id", "client_secret"] {
                if let Some(v) = auth_obj.get(string_key)
                    && v.as_str().is_none()
                {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            0,
                            "GM-AG-001",
                            t!(
                                "rules.gm_ag_001.not_string",
                                server = server,
                                field = string_key
                            ),
                        )
                        .with_suggestion(t!("rules.gm_ag_001.suggestion")),
                    );
                }
            }
            // URL fields, when present, must look like URLs (basic sanity -
            // upstream uses Zod's .url() check which rejects non-HTTPS/non-
            // HTTP and malformed strings).
            for url_key in ["authorization_url", "token_url"] {
                if let Some(v) = auth_obj.get(url_key) {
                    match v.as_str() {
                        None => diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                line,
                                0,
                                "GM-AG-001",
                                t!(
                                    "rules.gm_ag_001.not_string",
                                    server = server,
                                    field = url_key
                                ),
                            )
                            .with_suggestion(t!("rules.gm_ag_001.suggestion")),
                        ),
                        Some(s) if !looks_like_url(s) => diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                line,
                                0,
                                "GM-AG-001",
                                t!(
                                    "rules.gm_ag_001.invalid_url",
                                    server = server,
                                    field = url_key,
                                    value = s
                                ),
                            )
                            .with_suggestion(t!("rules.gm_ag_001.suggestion")),
                        ),
                        _ => {}
                    }
                }
            }
            if let Some(scopes) = auth_obj.get("scopes") {
                validate_scopes(path, line, server, scopes, diagnostics);
            }
        }
        _ => unreachable!("type_str already validated against VALID_AUTH_TYPES"),
    }
}

fn validate_scopes(
    path: &Path,
    line: usize,
    server: &str,
    scopes: &serde_yaml::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(seq) = scopes.as_sequence() else {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                0,
                "GM-AG-001",
                t!("rules.gm_ag_001.scopes_not_array", server = server),
            )
            .with_suggestion(t!("rules.gm_ag_001.suggestion")),
        );
        return;
    };
    for (idx, scope) in seq.iter().enumerate() {
        if scope.as_str().is_none() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    0,
                    "GM-AG-001",
                    t!(
                        "rules.gm_ag_001.scope_not_string",
                        server = server,
                        index = idx.to_string().as_str()
                    ),
                )
                .with_suggestion(t!("rules.gm_ag_001.suggestion")),
            );
        }
    }
}

/// Basic URL sanity: must start with `http://` or `https://` and have a
/// host segment after the scheme. Deliberately lenient - upstream uses
/// Zod's URL validator which is also lenient about paths and query strings.
fn looks_like_url(s: &str) -> bool {
    if let Some(rest) = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
    {
        // There must be at least one character for the host.
        !rest.is_empty()
            && !rest.starts_with('/')
            && !rest.starts_with('?')
            && !rest.starts_with('#')
    } else {
        false
    }
}

/// Extract the YAML frontmatter block (between the first two `---` lines).
/// Returns the YAML body only; returns None if no frontmatter is present.
///
/// Walks lines once tracking a running byte offset, so locating the closing
/// marker is O(N) rather than O(N²) from re-splitting + summing on each hit.
fn extract_frontmatter(content: &str) -> Option<&str> {
    let stripped = content.strip_prefix('\u{FEFF}').unwrap_or(content);
    let rest = stripped
        .strip_prefix("---\n")
        .or_else(|| stripped.strip_prefix("---\r\n"))?;
    let mut offset = 0usize;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            return Some(&rest[..offset]);
        }
        offset += line.len();
    }
    None
}

/// 1-indexed line of the `auth:` key under `mcp_servers.<server>:` in a
/// YAML document. Falls back to 1 if the line cannot be located. This is a
/// best-effort scanner: when the server name has special YAML characters or
/// the auth block is on a single line (flow style), we accept a less-
/// precise line number.
fn find_auth_line(content: &str, server: &str) -> Option<usize> {
    let mut in_mcp_servers = false;
    let mut in_server = false;
    let server_prefix = format!("{server}:");
    let server_quoted_prefix = format!("\"{server}\":");

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        let lead_ws_len = line.len() - trimmed.len();

        if trimmed == "mcp_servers:" {
            in_mcp_servers = true;
            in_server = false;
            continue;
        }
        if in_mcp_servers {
            // Leave mcp_servers section when we hit a top-level key (zero indent).
            if lead_ws_len == 0 && !trimmed.starts_with('#') && !trimmed.is_empty() {
                in_mcp_servers = false;
                continue;
            }
            if trimmed.starts_with(&server_prefix) || trimmed.starts_with(&server_quoted_prefix) {
                in_server = true;
                continue;
            }
            if in_server {
                // Sibling server starts - stop looking.
                if lead_ws_len <= 2 && trimmed.ends_with(':') && !trimmed.starts_with("auth:") {
                    in_server = false;
                    continue;
                }
                if trimmed.starts_with("auth:") {
                    return Some(idx + 1);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use std::path::PathBuf;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = GeminiAgentValidator;
        validator.validate(
            &PathBuf::from(".gemini/agents/test.md"),
            content,
            &LintConfig::default(),
        )
    }

    const AGENT_HEADER: &str = "---\nkind: local\nname: test-agent\ndescription: Test\n";
    const AGENT_FOOTER: &str = "system_prompt: You are a test.\n---\n\nBody.\n";

    fn wrap_mcp(mcp: &str) -> String {
        format!("{AGENT_HEADER}mcp_servers:\n{mcp}{AGENT_FOOTER}")
    }

    // ===== Positive cases =====

    #[test]
    fn test_google_credentials_with_scopes_is_valid() {
        let content = wrap_mcp(
            "  spanner:\n    url: https://spanner.googleapis.com/mcp\n    type: http\n    auth:\n      type: google-credentials\n      scopes:\n        - https://www.googleapis.com/auth/cloud-platform\n",
        );
        let diagnostics = validate(&content);
        assert!(
            diagnostics.is_empty(),
            "Valid google-credentials auth must not flag, got {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_oauth_full_shape_is_valid() {
        let content = wrap_mcp(
            "  myserver:\n    url: https://example.com/mcp\n    type: http\n    auth:\n      type: oauth\n      client_id: abc\n      client_secret: secret\n      scopes: [read, write]\n      authorization_url: https://accounts.example.com/authorize\n      token_url: https://accounts.example.com/token\n",
        );
        let diagnostics = validate(&content);
        assert!(
            diagnostics.is_empty(),
            "Valid oauth auth must not flag, got {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_google_credentials_no_scopes_is_valid() {
        // scopes is optional per the Zod schema.
        let content = wrap_mcp(
            "  myserver:\n    url: https://example.com/mcp\n    auth:\n      type: google-credentials\n",
        );
        let diagnostics = validate(&content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_oauth_minimal_shape_is_valid() {
        // All oauth fields except `type` are optional.
        let content = wrap_mcp(
            "  myserver:\n    url: https://example.com/mcp\n    auth:\n      type: oauth\n",
        );
        let diagnostics = validate(&content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_no_auth_block_is_fine() {
        let content = wrap_mcp("  myserver:\n    url: https://example.com/mcp\n");
        let diagnostics = validate(&content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_no_mcp_servers_is_fine() {
        let content = format!("{AGENT_HEADER}{AGENT_FOOTER}");
        let diagnostics = validate(&content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_missing_frontmatter_is_silent() {
        let diagnostics = validate("# Just markdown, no frontmatter\n");
        assert!(diagnostics.is_empty());
    }

    // ===== Type discriminator =====

    #[test]
    fn test_missing_type_flags() {
        let content = wrap_mcp("  myserver:\n    auth:\n      scopes:\n        - foo\n");
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.to_lowercase().contains("missing"));
    }

    #[test]
    fn test_non_string_type_flags_as_type_not_string() {
        // Regression: cursor caught that non-string type was being reported
        // as "missing" because and_then(as_str) collapses both cases. The
        // validator now emits a distinct "type must be a string" message.
        let content = wrap_mcp("  myserver:\n    auth:\n      type: 12345\n");
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
        let msg = hits[0].message.to_lowercase();
        assert!(
            msg.contains("auth.type") && msg.contains("string"),
            "expected 'auth.type must be a string' style message, got: {}",
            hits[0].message
        );
        assert!(!msg.contains("missing"));
    }

    #[test]
    fn test_invalid_type_flags() {
        let content = wrap_mcp("  myserver:\n    auth:\n      type: basic-auth\n");
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("basic-auth"));
        assert!(hits[0].message.contains("google-credentials"));
    }

    #[test]
    fn test_non_object_auth_flags() {
        let content = wrap_mcp("  myserver:\n    auth: \"google-credentials\"\n");
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    // ===== Unknown fields =====

    #[test]
    fn test_oauth_field_on_google_credentials_flags() {
        // client_id doesn't belong on google-credentials.
        let content = wrap_mcp(
            "  myserver:\n    auth:\n      type: google-credentials\n      client_id: abc\n",
        );
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("client_id"));
    }

    #[test]
    fn test_unknown_field_on_oauth_flags() {
        let content =
            wrap_mcp("  myserver:\n    auth:\n      type: oauth\n      random_extra: yes\n");
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("random_extra"));
    }

    // ===== Value-type checks =====

    #[test]
    fn test_client_id_non_string_flags() {
        let content =
            wrap_mcp("  myserver:\n    auth:\n      type: oauth\n      client_id: 12345\n");
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("client_id"));
    }

    #[test]
    fn test_authorization_url_malformed_flags() {
        let content = wrap_mcp(
            "  myserver:\n    auth:\n      type: oauth\n      authorization_url: \"not a url\"\n",
        );
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].message.contains("authorization_url"));
    }

    #[test]
    fn test_authorization_url_valid_https_accepted() {
        let content = wrap_mcp(
            "  myserver:\n    auth:\n      type: oauth\n      authorization_url: https://example.com/authorize\n",
        );
        let diagnostics = validate(&content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_scopes_not_array_flags() {
        let content = wrap_mcp(
            "  myserver:\n    auth:\n      type: google-credentials\n      scopes: one-scope\n",
        );
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn test_scope_non_string_flags() {
        let content = wrap_mcp(
            "  myserver:\n    auth:\n      type: google-credentials\n      scopes:\n        - 12345\n",
        );
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1);
    }

    // ===== Multiple servers =====

    #[test]
    fn test_per_server_independent_validation() {
        let content = wrap_mcp(
            "  s1:\n    auth:\n      type: google-credentials\n  s2:\n    auth:\n      type: basic-auth\n",
        );
        let diagnostics = validate(&content);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GM-AG-001")
            .collect();
        assert_eq!(hits.len(), 1, "only s2 should flag, got {:?}", hits);
        assert!(hits[0].message.contains("s2"));
    }

    // ===== Disable path =====

    #[test]
    fn test_can_be_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["GM-AG-001".to_string()];
        let content = wrap_mcp("  myserver:\n    auth:\n      type: basic-auth\n");
        let validator = GeminiAgentValidator;
        let diagnostics =
            validator.validate(&PathBuf::from(".gemini/agents/test.md"), &content, &config);
        assert!(diagnostics.is_empty());
    }

    // ===== Helper unit tests =====

    #[test]
    fn test_looks_like_url() {
        assert!(looks_like_url("https://example.com"));
        assert!(looks_like_url("http://localhost:8080/path"));
        assert!(!looks_like_url("not-a-url"));
        assert!(!looks_like_url("ftp://example.com"));
        assert!(!looks_like_url("https://"));
        assert!(!looks_like_url("https:///empty-host"));
    }

    #[test]
    fn test_extract_frontmatter_basic() {
        let content = "---\nkey: value\n---\nBody\n";
        assert_eq!(extract_frontmatter(content), Some("key: value\n"));
    }

    #[test]
    fn test_extract_frontmatter_bom() {
        let content = "\u{FEFF}---\nkey: value\n---\nBody\n";
        assert_eq!(extract_frontmatter(content), Some("key: value\n"));
    }

    #[test]
    fn test_extract_frontmatter_absent() {
        assert_eq!(extract_frontmatter("# No frontmatter\n"), None);
    }

    #[test]
    fn test_extract_frontmatter_unterminated() {
        assert_eq!(extract_frontmatter("---\nkey: value\nno closing\n"), None);
    }
}
