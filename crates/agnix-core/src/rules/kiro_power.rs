//! Kiro POWER.md validation rules (KR-PW-001 to KR-PW-008).
//!
//! Validates:
//! - KR-PW-001: Missing required POWER.md frontmatter fields
//! - KR-PW-002: Empty POWER.md keywords array
//! - KR-PW-003: Empty POWER.md body
//! - KR-PW-004: Adjacent power mcp.json has invalid mcpServers structure
//! - KR-PW-005: Step missing description
//! - KR-PW-006: Duplicate keywords
//! - KR-PW-007: Name invalid characters
//! - KR-PW-008: Secrets in power body

use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata, seems_plaintext_secret},
    schemas::{kiro_mcp::parse_kiro_mcp_config, kiro_power::parse_kiro_power},
};
use regex::Regex;
use rust_i18n::t;
use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

const RULE_IDS: &[&str] = &[
    "KR-PW-001",
    "KR-PW-002",
    "KR-PW-003",
    "KR-PW-004",
    "KR-PW-005",
    "KR-PW-006",
    "KR-PW-007",
    "KR-PW-008",
];

fn power_name_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^[a-z0-9][a-z0-9_-]*$").expect("power name pattern must compile")
    })
}

fn power_secret_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?im)\b(?:api[_-]?key|token|password|[a-z0-9_-]+secret|secret[a-z0-9_-]+)\b\s*[:=]\s*(?P<value>[^\s#]+)",
        )
        .expect("power secret pattern must compile")
    })
}

pub struct KiroPowerValidator;

impl Validator for KiroPowerValidator {
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

        // KR-PW-005: Step missing description (empty heading section in body)
        // Intentionally checks only ## headings - Kiro power files use ## for steps
        // per Kiro convention. Higher/lower heading levels are not step markers.
        if config.is_rule_enabled("KR-PW-005")
            && parsed.has_frontmatter
            && parsed.parse_error.is_none()
        {
            let body = parsed.body.trim();
            // Collected into Vec intentionally for look-ahead (checking next headings).
            // Power files are small, so the allocation is negligible.
            let lines: Vec<&str> = body.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if line.starts_with("## ") {
                    // Check if the next non-empty line is another heading or end of file
                    let has_content = lines[i + 1..]
                        .iter()
                        .take_while(|l| !l.starts_with("## "))
                        .any(|l| !l.trim().is_empty());
                    if !has_content {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "KR-PW-005",
                                t!("rules.kr_pw_005.message", step = line.trim()),
                            )
                            .with_suggestion(t!("rules.kr_pw_005.suggestion")),
                        );
                    }
                }
            }
        }

        // KR-PW-006: Duplicate keywords
        if config.is_rule_enabled("KR-PW-006")
            && let Some(frontmatter) = parsed.frontmatter.as_ref()
            && let Some(keywords) = frontmatter.keywords.as_ref()
        {
            let mut seen = HashSet::new();
            for keyword in keywords {
                let normalized = keyword.trim().to_ascii_lowercase();
                if !normalized.is_empty() && !seen.insert(normalized) {
                    diagnostics.push(
                        Diagnostic::info(
                            path.to_path_buf(),
                            1,
                            0,
                            "KR-PW-006",
                            t!("rules.kr_pw_006.message", keyword = keyword.as_str()),
                        )
                        .with_suggestion(t!("rules.kr_pw_006.suggestion")),
                    );
                }
            }
        }

        // KR-PW-007: Name invalid characters
        if config.is_rule_enabled("KR-PW-007")
            && let Some(frontmatter) = parsed.frontmatter.as_ref()
            && let Some(name) = frontmatter.name.as_deref()
            && !name.trim().is_empty()
        {
            if !power_name_pattern().is_match(name.trim()) {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "KR-PW-007",
                        t!("rules.kr_pw_007.message", name = name.trim()),
                    )
                    .with_suggestion(t!("rules.kr_pw_007.suggestion")),
                );
            }
        }

        // KR-PW-008: Secrets in power body
        if config.is_rule_enabled("KR-PW-008")
            && parsed.has_frontmatter
            && parsed.parse_error.is_none()
        {
            let body = &parsed.body;
            for captures in power_secret_pattern().captures_iter(body) {
                let value = captures
                    .name("value")
                    .map(|m| m.as_str())
                    .unwrap_or_default();
                if seems_plaintext_secret(value) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "KR-PW-008",
                            t!("rules.kr_pw_008.message"),
                        )
                        .with_suggestion(t!("rules.kr_pw_008.suggestion")),
                    );
                    break;
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
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
    fn test_kr_pw_005_step_missing_description() {
        let diagnostics = validate(
            r#"---
name: empty-step
description: test
keywords:
  - one
---
## Step 1
## Step 2
Some content here.
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-PW-005"));
    }

    #[test]
    fn test_kr_pw_005_step_with_description_no_diagnostic() {
        let diagnostics = validate(
            r#"---
name: good-steps
description: test
keywords:
  - one
---
## Step 1
This step does something.
## Step 2
This step does another thing.
"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-PW-005"));
    }

    #[test]
    fn test_kr_pw_006_duplicate_keywords() {
        let diagnostics = validate(
            r#"---
name: dupes
description: test
keywords:
  - foo
  - bar
  - foo
---
# Body
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-PW-006"));
    }

    #[test]
    fn test_kr_pw_006_unique_keywords_no_diagnostic() {
        let diagnostics = validate(
            r#"---
name: unique
description: test
keywords:
  - foo
  - bar
---
# Body
"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-PW-006"));
    }

    #[test]
    fn test_kr_pw_007_invalid_name_characters() {
        let diagnostics = validate(
            r#"---
name: My Power!
description: test
keywords:
  - one
---
# Body
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-PW-007"));
    }

    #[test]
    fn test_kr_pw_007_valid_name_no_diagnostic() {
        let diagnostics = validate(
            r#"---
name: my-power-1
description: test
keywords:
  - one
---
# Body
"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-PW-007"));
    }

    #[test]
    fn test_kr_pw_008_secrets_in_body() {
        let diagnostics = validate(
            r#"---
name: secrets
description: test
keywords:
  - one
---
Configure with api_key= hardcodedsecret123value
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-PW-008"));
    }

    #[test]
    fn test_kr_pw_008_no_secrets_no_diagnostic() {
        let diagnostics = validate(
            r#"---
name: clean
description: test
keywords:
  - one
---
# Body
Normal instructions here.
"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-PW-008"));
    }

    #[test]
    fn test_kr_pw_006_case_insensitive_duplicate_keywords() {
        let diagnostics = validate(
            r#"---
name: case-dupes
description: test
keywords:
  - foo
  - Foo
---
# Body
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-PW-006"));
    }

    #[test]
    fn test_kr_pw_008_template_values_not_flagged() {
        let diagnostics = validate(
            r#"---
name: template
description: test
keywords:
  - one
---
Configure with api_key= ${API_KEY}
"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-PW-008"));
    }

    // M14: KR-PW-005 step at end of body with no content after
    #[test]
    fn test_kr_pw_005_step_at_end_of_body() {
        let diagnostics = validate(
            r#"---
name: end-step
description: test
keywords:
  - one
---
## Step 1
"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "KR-PW-005"));
    }

    // L4: KR-PW-007 valid kebab-case name should not trigger
    #[test]
    fn test_kr_pw_007_valid_kebab_case_no_diagnostic() {
        let diagnostics = validate(
            r#"---
name: my-power-test-123
description: test
keywords:
  - one
---
# Body
"#,
        );
        assert!(diagnostics.iter().all(|d| d.rule != "KR-PW-007"));
    }

    #[test]
    fn test_metadata() {
        let validator = KiroPowerValidator;
        let metadata = validator.metadata();
        assert_eq!(metadata.name, "KiroPowerValidator");
        assert_eq!(
            metadata.rule_ids,
            &[
                "KR-PW-001",
                "KR-PW-002",
                "KR-PW-003",
                "KR-PW-004",
                "KR-PW-005",
                "KR-PW-006",
                "KR-PW-007",
                "KR-PW-008",
            ]
        );
    }
}
