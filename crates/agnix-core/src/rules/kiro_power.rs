//! Kiro POWER.md validation rules (KR-PW-001 to KR-PW-004).
//!
//! Validates:
//! - KR-PW-001: Missing required POWER.md frontmatter fields
//! - KR-PW-002: Empty POWER.md keywords array
//! - KR-PW-003: Empty POWER.md body
//! - KR-PW-004: Adjacent power mcp.json has invalid mcpServers structure

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
    schemas::{kiro_mcp::parse_kiro_mcp_config, kiro_power::parse_kiro_power},
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["KR-PW-001", "KR-PW-002", "KR-PW-003", "KR-PW-004"];

pub struct KiroPowerValidator;

impl Validator for KiroPowerValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let parsed = parse_kiro_power(content);

        if config.is_rule_enabled("KR-PW-001") {
            if !parsed.has_frontmatter {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "KR-PW-001",
                        t!(
                            "rules.kr_pw_001.message",
                            fields = "name, description, keywords"
                        ),
                    )
                    .with_suggestion(t!("rules.kr_pw_001.suggestion")),
                );
            } else if let Some(parse_error) = parsed.parse_error.as_ref() {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        parse_error.line,
                        parse_error.column,
                        "KR-PW-001",
                        t!(
                            "rules.kr_pw_001_parse.message",
                            error = parse_error.message.as_str()
                        ),
                    )
                    .with_suggestion(t!("rules.kr_pw_001_parse.suggestion")),
                );
            } else if let Some(frontmatter) = parsed.frontmatter.as_ref() {
                let mut missing = Vec::new();
                if frontmatter.name.as_deref().is_none()
                    || frontmatter
                        .name
                        .as_deref()
                        .is_some_and(|value| value.trim().is_empty())
                {
                    missing.push("name");
                }
                if frontmatter.description.as_deref().is_none()
                    || frontmatter
                        .description
                        .as_deref()
                        .is_some_and(|value| value.trim().is_empty())
                {
                    missing.push("description");
                }
                if frontmatter.keywords.is_none() {
                    missing.push("keywords");
                }

                if !missing.is_empty() {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "KR-PW-001",
                            t!("rules.kr_pw_001.message", fields = missing.join(", ")),
                        )
                        .with_suggestion(t!("rules.kr_pw_001.suggestion")),
                    );
                }
            }
        }

        if config.is_rule_enabled("KR-PW-002")
            && let Some(frontmatter) = parsed.frontmatter.as_ref()
            && let Some(keywords) = frontmatter.keywords.as_ref()
            && keywords.is_empty()
        {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "KR-PW-002",
                    t!("rules.kr_pw_002.message"),
                )
                .with_suggestion(t!("rules.kr_pw_002.suggestion")),
            );
        }

        if config.is_rule_enabled("KR-PW-003")
            && parsed.has_frontmatter
            && parsed.has_closing_frontmatter
            && parsed.parse_error.is_none()
            && parsed.body.trim().is_empty()
        {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "KR-PW-003",
                    t!("rules.kr_pw_003.message"),
                )
                .with_suggestion(t!("rules.kr_pw_003.suggestion")),
            );
        }

        if config.is_rule_enabled("KR-PW-004")
            && let Some(power_dir) = path.parent()
        {
            let mcp_path = power_dir.join("mcp.json");
            let fs = config.fs();
            if fs.exists(&mcp_path) {
                match fs.read_to_string(&mcp_path) {
                    Ok(mcp_content) => {
                        let parsed_mcp = parse_kiro_mcp_config(&mcp_content);
                        let invalid_structure = parsed_mcp.parse_error.is_some()
                            || parsed_mcp
                                .config
                                .as_ref()
                                .and_then(|cfg| cfg.mcp_servers.as_ref())
                                .is_none();

                        if invalid_structure {
                            diagnostics.push(
                                Diagnostic::warning(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "KR-PW-004",
                                    t!("rules.kr_pw_004.message"),
                                )
                                .with_suggestion(t!("rules.kr_pw_004.suggestion")),
                            );
                        }
                    }
                    Err(_) => {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "KR-PW-004",
                                t!("rules.kr_pw_004.message"),
                            )
                            .with_suggestion(t!("rules.kr_pw_004.suggestion")),
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
    use std::fs;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = KiroPowerValidator;
        validator.validate(
            Path::new(".kiro/powers/test-power/POWER.md"),
            content,
            &LintConfig::default(),
        )
    }

    #[test]
    fn test_kr_pw_001_missing_frontmatter() {
        let diagnostics = validate("# Missing frontmatter");
        assert!(diagnostics.iter().any(|d| d.rule == "KR-PW-001"));
    }

    #[test]
    fn test_kr_pw_001_missing_required_fields() {
        let diagnostics = validate(
            r#"---
name: sample
---
# Sample
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-PW-001"));
    }

    #[test]
    fn test_kr_pw_002_empty_keywords() {
        let diagnostics = validate(
            r#"---
name: empty-keywords
description: test
keywords: []
---
# Body
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-PW-002"));
    }

    #[test]
    fn test_kr_pw_003_empty_body() {
        let diagnostics = validate(
            r#"---
name: empty-body
description: test
keywords:
  - one
---
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-PW-003"));
    }

    #[test]
    fn test_kr_pw_004_invalid_adjacent_mcp() {
        let temp = tempfile::TempDir::new().unwrap();
        let power_dir = temp.path().join(".kiro").join("powers").join("bad");
        fs::create_dir_all(&power_dir).unwrap();

        let power_path = power_dir.join("POWER.md");
        fs::write(
            &power_path,
            r#"---
name: bad
description: test
keywords:
  - one
---
# Body
"#,
        )
        .unwrap();
        fs::write(power_dir.join("mcp.json"), r#"{"mcpServers":[]}"#).unwrap();

        let validator = KiroPowerValidator;
        let content = fs::read_to_string(&power_path).unwrap();
        let diagnostics = validator.validate(&power_path, &content, &LintConfig::default());
        assert!(diagnostics.iter().any(|d| d.rule == "KR-PW-004"));
    }

    #[test]
    fn test_kr_pw_004_allows_empty_mcp_servers_object() {
        let temp = tempfile::TempDir::new().unwrap();
        let power_dir = temp.path().join(".kiro").join("powers").join("ok");
        fs::create_dir_all(&power_dir).unwrap();

        let power_path = power_dir.join("POWER.md");
        fs::write(
            &power_path,
            r#"---
name: ok
description: test
keywords:
  - one
---
# Body
"#,
        )
        .unwrap();
        fs::write(power_dir.join("mcp.json"), r#"{"mcpServers":{}}"#).unwrap();

        let validator = KiroPowerValidator;
        let content = fs::read_to_string(&power_path).unwrap();
        let diagnostics = validator.validate(&power_path, &content, &LintConfig::default());
        assert!(!diagnostics.iter().any(|d| d.rule == "KR-PW-004"));
    }

    #[test]
    fn test_valid_power_has_no_pw_diagnostics() {
        let diagnostics = validate(
            r#"---
name: valid
description: test
keywords:
  - kiro
---
# Body
Valid content.
"#,
        );
        assert!(diagnostics.iter().all(|d| !d.rule.starts_with("KR-PW-")));
    }

    #[test]
    fn test_metadata() {
        let validator = KiroPowerValidator;
        let metadata = validator.metadata();
        assert_eq!(metadata.name, "KiroPowerValidator");
        assert_eq!(
            metadata.rule_ids,
            &["KR-PW-001", "KR-PW-002", "KR-PW-003", "KR-PW-004"]
        );
    }
}
