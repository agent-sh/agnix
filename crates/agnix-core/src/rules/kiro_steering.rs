//! Kiro steering file validation rules (KIRO-001 to KIRO-009)
//!
//! Validates:
//! - KIRO-001: Invalid steering file inclusion mode (HIGH/ERROR)
//! - KIRO-002: Missing required fields for inclusion mode (HIGH/ERROR)
//! - KIRO-003: Invalid fileMatchPattern glob (MEDIUM/WARNING)
//! - KIRO-004: Empty Kiro steering file (MEDIUM/WARNING)
//! - KIRO-005: Empty steering body after frontmatter (MEDIUM/WARNING)
//! - KIRO-006: Secrets detected in steering content (HIGH/ERROR)
//! - KIRO-007: fileMatchPattern present without inclusion: fileMatch (MEDIUM/WARNING)
//! - KIRO-008: Unknown frontmatter field (MEDIUM/WARNING)
//! - KIRO-009: Inline file reference points to missing file (MEDIUM/WARNING)

use crate::{
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    parsers::frontmatter::split_frontmatter,
    rules::{Validator, ValidatorMetadata},
};
use regex::Regex;
use rust_i18n::t;
use std::path::{Component, Path};
use std::sync::OnceLock;

const RULE_IDS: &[&str] = &[
    "KIRO-001", "KIRO-002", "KIRO-003", "KIRO-004", "KIRO-005", "KIRO-006", "KIRO-007", "KIRO-008",
    "KIRO-009",
];
const VALID_INCLUSION_MODES: &[&str] = &["always", "fileMatch", "manual", "auto"];
const VALID_FRONTMATTER_FIELDS: &[&str] = &["inclusion", "name", "description", "fileMatchPattern"];

fn line_col_at_offset(content: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;

    for (idx, ch) in content.char_indices() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}

fn find_frontmatter_key_line(frontmatter: &str, key: &str) -> usize {
    for (idx, line) in frontmatter.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(key)
            && rest.trim_start().starts_with(':')
        {
            return idx + 2;
        }
    }
    1
}

fn secret_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?im)\b(?P<marker>api[_-]?key|token|password|[a-z0-9_-]+secret|secret[a-z0-9_-]+)\b\s*[:=]\s*(?P<value>[^\s#]+)",
        )
        .expect("secret pattern must compile")
    })
}

fn inline_file_ref_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"#\[\[file:(?P<path>[^\]\n]+)\]\]").expect("inline file pattern must compile")
    })
}

fn seems_plaintext_secret(value: &str) -> bool {
    let trimmed = value.trim_matches(|ch| ch == '"' || ch == '\'').trim();
    !trimmed.is_empty()
        && !trimmed.starts_with("${")
        && !trimmed.starts_with("$(")
        && !trimmed.starts_with("{{")
        && !trimmed.starts_with('<')
        && trimmed.len() >= 8
}

fn has_parent_dir_traversal(reference: &str) -> bool {
    Path::new(reference)
        .components()
        .any(|component| matches!(component, Component::ParentDir))
}

/// Adapter to use raw frontmatter with `find_yaml_value_range`.
struct FrontmatterAdapter<'a> {
    raw: &'a str,
}

impl crate::rules::FrontmatterRanges for FrontmatterAdapter<'_> {
    fn raw_content(&self) -> &str {
        self.raw
    }
    fn start_line(&self) -> usize {
        1 // Opening --- is file line 1; frontmatter content starts at line 2
    }
}

pub struct KiroSteeringValidator;

impl Validator for KiroSteeringValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // KIRO-004: Empty steering file (check first, return early)
        if config.is_rule_enabled("KIRO-004") && content.trim().is_empty() {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "KIRO-004",
                    t!("rules.kiro_004.message"),
                )
                .with_suggestion(t!("rules.kiro_004.suggestion")),
            );
            return diagnostics;
        }

        // KIRO-006: Secrets in steering content
        if config.is_rule_enabled("KIRO-006") {
            for captures in secret_pattern().captures_iter(content) {
                let Some(full_match) = captures.get(0) else {
                    continue;
                };
                let marker = captures
                    .name("marker")
                    .map(|m| m.as_str().to_ascii_lowercase())
                    .unwrap_or_else(|| "secret".to_string());
                let value = captures
                    .name("value")
                    .map(|m| m.as_str())
                    .unwrap_or_default();
                if seems_plaintext_secret(value) {
                    let (line, col) = line_col_at_offset(content, full_match.start());
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            col,
                            "KIRO-006",
                            t!("rules.kiro_006.message", marker = marker),
                        )
                        .with_suggestion(t!("rules.kiro_006.suggestion")),
                    );
                    break;
                }
            }
        }

        // KIRO-009: Broken inline file references
        if config.is_rule_enabled("KIRO-009") {
            let fs = config.fs();
            for captures in inline_file_ref_pattern().captures_iter(content) {
                let Some(full_match) = captures.get(0) else {
                    continue;
                };
                let Some(path_match) = captures.name("path") else {
                    continue;
                };

                let reference = path_match.as_str().trim();
                if reference.is_empty()
                    || reference.starts_with("http://")
                    || reference.starts_with("https://")
                    || Path::new(reference).is_absolute()
                    || has_parent_dir_traversal(reference)
                {
                    continue;
                }

                let resolved = path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(reference);

                if !fs.exists(&resolved) {
                    let (line, col) = line_col_at_offset(content, full_match.start());
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            line,
                            col,
                            "KIRO-009",
                            t!("rules.kiro_009.message", reference = reference),
                        )
                        .with_suggestion(t!("rules.kiro_009.suggestion")),
                    );
                }
            }
        }

        // Parse frontmatter
        let parts = split_frontmatter(content);
        if !parts.has_frontmatter || !parts.has_closing {
            return diagnostics; // No frontmatter - skip frontmatter-based rules
        }

        // KIRO-005: Empty body after valid frontmatter delimiters
        if config.is_rule_enabled("KIRO-005") && parts.body.trim().is_empty() {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "KIRO-005",
                    t!("rules.kiro_005.message"),
                )
                .with_suggestion(t!("rules.kiro_005.suggestion")),
            );
        }

        // Parse YAML
        let yaml: serde_yaml::Value = match serde_yaml::from_str(&parts.frontmatter) {
            Ok(v) => v,
            Err(_) => return diagnostics, // Malformed YAML - skip gracefully
        };

        let mapping = match yaml.as_mapping() {
            Some(m) => m,
            None => return diagnostics,
        };

        // Extract commonly accessed keys once to avoid repeated allocations
        let key_inclusion = serde_yaml::Value::String("inclusion".into());
        let key_name = serde_yaml::Value::String("name".into());
        let key_description = serde_yaml::Value::String("description".into());
        let key_file_match_pattern = serde_yaml::Value::String("fileMatchPattern".into());

        // Look up inclusion once - used by both KIRO-001 and KIRO-002
        let inclusion_val = mapping.get(&key_inclusion);
        let inclusion_str = inclusion_val.and_then(|v| v.as_str());

        // KIRO-001: Invalid inclusion mode
        if config.is_rule_enabled("KIRO-001") {
            if let Some(val) = inclusion_val {
                match val.as_str() {
                    Some(inclusion) if VALID_INCLUSION_MODES.contains(&inclusion) => {
                        // Valid mode - no diagnostic
                    }
                    Some(inclusion) => {
                        let mut diagnostic = Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "KIRO-001",
                            t!("rules.kiro_001.message", value = inclusion),
                        )
                        .with_suggestion(t!("rules.kiro_001.suggestion"));

                        if let Some(suggested) =
                            crate::rules::find_closest_value(inclusion, VALID_INCLUSION_MODES)
                        {
                            // Find byte range of the value in the frontmatter
                            let adapter = FrontmatterAdapter {
                                raw: &parts.frontmatter,
                            };
                            if let Some((start, end)) = crate::rules::find_yaml_value_range(
                                content,
                                &adapter,
                                "inclusion",
                                true,
                            ) {
                                let slice = content.get(start..end).unwrap_or("");
                                let replacement = if slice.starts_with('"') {
                                    format!("\"{}\"", suggested)
                                } else if slice.starts_with('\'') {
                                    format!("'{}'", suggested)
                                } else {
                                    suggested.to_string()
                                };
                                diagnostic = diagnostic.with_fix(Fix::replace(
                                    start,
                                    end,
                                    replacement,
                                    format!("Replace inclusion mode with '{}'", suggested),
                                    false,
                                ));
                            }
                        }

                        diagnostics.push(diagnostic);
                    }
                    None => {
                        // Non-string value (number, bool, etc.) - also invalid
                        let display = format!("{val:?}");
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "KIRO-001",
                                t!("rules.kiro_001.message", value = display),
                            )
                            .with_suggestion(t!("rules.kiro_001.suggestion")),
                        );
                    }
                }
            }
        }

        // KIRO-002: Missing required fields for inclusion mode
        if config.is_rule_enabled("KIRO-002") {
            if let Some(mode) = inclusion_str {
                match mode {
                    "auto" => {
                        let name_valid = mapping
                            .get(&key_name)
                            .and_then(|v| v.as_str())
                            .is_some_and(|s| !s.trim().is_empty());
                        let desc_valid = mapping
                            .get(&key_description)
                            .and_then(|v| v.as_str())
                            .is_some_and(|s| !s.trim().is_empty());
                        if !name_valid {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "KIRO-002",
                                    t!("rules.kiro_002_auto.message", field = "name"),
                                )
                                .with_suggestion(t!("rules.kiro_002_auto.suggestion")),
                            );
                        }
                        if !desc_valid {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "KIRO-002",
                                    t!("rules.kiro_002_auto.message", field = "description"),
                                )
                                .with_suggestion(t!("rules.kiro_002_auto.suggestion")),
                            );
                        }
                    }
                    "fileMatch" => {
                        if !mapping.contains_key(&key_file_match_pattern) {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "KIRO-002",
                                    t!("rules.kiro_002_filematch.message"),
                                )
                                .with_suggestion(t!("rules.kiro_002_filematch.suggestion")),
                            );
                        }
                    }
                    _ => {} // always and manual have no extra required fields
                }
            }
        }

        // KIRO-003: Invalid fileMatchPattern glob
        if config.is_rule_enabled("KIRO-003") {
            if let Some(pattern_val) = mapping.get(&key_file_match_pattern) {
                match pattern_val.as_str() {
                    Some(pattern) => {
                        if let Err(e) = glob::Pattern::new(pattern) {
                            diagnostics.push(
                                Diagnostic::warning(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "KIRO-003",
                                    t!("rules.kiro_003.message", error = e.to_string()),
                                )
                                .with_suggestion(t!("rules.kiro_003.suggestion")),
                            );
                        }
                    }
                    None => {
                        // Non-string value (number, bool, etc.) - not a valid glob
                        let display = format!("{pattern_val:?}");
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "KIRO-003",
                                t!(
                                    "rules.kiro_003.message",
                                    error = format!("expected string, got {display}")
                                ),
                            )
                            .with_suggestion(t!("rules.kiro_003.suggestion")),
                        );
                    }
                }
            }
        }

        // KIRO-007: fileMatchPattern without inclusion: fileMatch
        if config.is_rule_enabled("KIRO-007")
            && mapping.contains_key(&key_file_match_pattern)
            && !matches!(inclusion_str, Some("fileMatch"))
        {
            let inclusion_display = inclusion_str.unwrap_or("<missing>");
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    find_frontmatter_key_line(&parts.frontmatter, "fileMatchPattern"),
                    0,
                    "KIRO-007",
                    t!("rules.kiro_007.message", inclusion = inclusion_display),
                )
                .with_suggestion(t!("rules.kiro_007.suggestion")),
            );
        }

        // KIRO-008: Unknown frontmatter fields
        if config.is_rule_enabled("KIRO-008") {
            for key in mapping.keys() {
                let Some(field) = key.as_str() else {
                    continue;
                };
                if VALID_FRONTMATTER_FIELDS.contains(&field) {
                    continue;
                }
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        find_frontmatter_key_line(&parts.frontmatter, field),
                        0,
                        "KIRO-008",
                        t!("rules.kiro_008.message", field = field),
                    )
                    .with_suggestion(t!("rules.kiro_008.suggestion")),
                );
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate(path: &str, content: &str) -> Vec<Diagnostic> {
        let validator = KiroSteeringValidator;
        validator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn validate_steering(content: &str) -> Vec<Diagnostic> {
        validate(".kiro/steering/test.md", content)
    }

    // ===== KIRO-001: Invalid inclusion mode =====

    #[test]
    fn test_kiro_001_invalid_mode() {
        let content = "---\ninclusion: invalid_mode\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-001")
            .collect();
        assert_eq!(kiro_001.len(), 1);
        assert_eq!(kiro_001[0].level, DiagnosticLevel::Error);
        assert!(kiro_001[0].message.contains("invalid_mode"));
    }

    #[test]
    fn test_kiro_001_valid_always() {
        let content = "---\ninclusion: always\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-001")
            .collect();
        assert!(kiro_001.is_empty());
    }

    #[test]
    fn test_kiro_001_valid_auto() {
        let content = "---\ninclusion: auto\nname: test\ndescription: test desc\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-001")
            .collect();
        assert!(kiro_001.is_empty());
    }

    #[test]
    fn test_kiro_001_valid_filematch() {
        let content = "---\ninclusion: fileMatch\nfileMatchPattern: \"**/*.ts\"\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-001")
            .collect();
        assert!(kiro_001.is_empty());
    }

    #[test]
    fn test_kiro_001_valid_manual() {
        let content = "---\ninclusion: manual\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-001")
            .collect();
        assert!(kiro_001.is_empty());
    }

    #[test]
    fn test_kiro_001_has_fix() {
        // Use a case-insensitive mismatch that find_closest_value can match
        let content = "---\ninclusion: Always\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-001")
            .collect();
        assert_eq!(kiro_001.len(), 1);
        assert!(
            kiro_001[0].has_fixes(),
            "KIRO-001 should have auto-fix for case-mismatched inclusion mode"
        );
        let fix = &kiro_001[0].fixes[0];
        assert!(!fix.safe, "KIRO-001 fix should be unsafe");
        assert!(
            fix.replacement.contains("always"),
            "Fix should suggest 'always' as closest match, got: {}",
            fix.replacement
        );

        // Apply the fix and verify the resulting content is correct
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        assert!(
            fixed.contains("inclusion: always"),
            "Applied fix should produce valid content"
        );
    }

    #[test]
    fn test_kiro_001_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KIRO-001".to_string()];
        let validator = KiroSteeringValidator;
        let diagnostics = validator.validate(
            Path::new(".kiro/steering/test.md"),
            "---\ninclusion: invalid_mode\n---\n# Steering\n",
            &config,
        );
        let kiro_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-001")
            .collect();
        assert!(kiro_001.is_empty());
    }

    // ===== KIRO-002: Missing required fields =====

    #[test]
    fn test_kiro_002_auto_missing_name() {
        let content = "---\ninclusion: auto\ndescription: test desc\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-002")
            .collect();
        assert_eq!(kiro_002.len(), 1);
        assert_eq!(kiro_002[0].level, DiagnosticLevel::Error);
        assert!(kiro_002[0].message.contains("name"));
    }

    #[test]
    fn test_kiro_002_auto_missing_description() {
        let content = "---\ninclusion: auto\nname: my-steering\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-002")
            .collect();
        assert_eq!(kiro_002.len(), 1);
        assert!(kiro_002[0].message.contains("description"));
    }

    #[test]
    fn test_kiro_002_auto_missing_both() {
        let content = "---\ninclusion: auto\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-002")
            .collect();
        assert_eq!(kiro_002.len(), 2);
    }

    #[test]
    fn test_kiro_002_auto_valid() {
        let content =
            "---\ninclusion: auto\nname: my-steering\ndescription: test desc\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-002")
            .collect();
        assert!(kiro_002.is_empty());
    }

    #[test]
    fn test_kiro_002_filematch_missing_pattern() {
        let content = "---\ninclusion: fileMatch\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-002")
            .collect();
        assert_eq!(kiro_002.len(), 1);
        assert!(kiro_002[0].message.contains("fileMatchPattern"));
    }

    #[test]
    fn test_kiro_002_filematch_valid() {
        let content = "---\ninclusion: fileMatch\nfileMatchPattern: \"**/*.ts\"\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-002")
            .collect();
        assert!(kiro_002.is_empty());
    }

    #[test]
    fn test_kiro_002_always_no_extra_fields_needed() {
        let content = "---\ninclusion: always\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-002")
            .collect();
        assert!(kiro_002.is_empty());
    }

    #[test]
    fn test_kiro_002_manual_no_extra_fields_needed() {
        let content = "---\ninclusion: manual\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-002")
            .collect();
        assert!(kiro_002.is_empty());
    }

    #[test]
    fn test_kiro_002_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KIRO-002".to_string()];
        let validator = KiroSteeringValidator;
        let diagnostics = validator.validate(
            Path::new(".kiro/steering/test.md"),
            "---\ninclusion: auto\n---\n# Steering\n",
            &config,
        );
        let kiro_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-002")
            .collect();
        assert!(kiro_002.is_empty());
    }

    // ===== KIRO-003: Invalid fileMatchPattern glob =====

    #[test]
    fn test_kiro_003_bad_glob() {
        let content = "---\nfileMatchPattern: \"[unclosed\"\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-003")
            .collect();
        assert_eq!(kiro_003.len(), 1);
        assert_eq!(kiro_003[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_kiro_003_valid_glob() {
        let content = "---\nfileMatchPattern: \"**/*.ts\"\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-003")
            .collect();
        assert!(kiro_003.is_empty());
    }

    #[test]
    fn test_kiro_003_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KIRO-003".to_string()];
        let validator = KiroSteeringValidator;
        let diagnostics = validator.validate(
            Path::new(".kiro/steering/test.md"),
            "---\nfileMatchPattern: \"[unclosed\"\n---\n# Steering\n",
            &config,
        );
        let kiro_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-003")
            .collect();
        assert!(kiro_003.is_empty());
    }

    // ===== KIRO-004: Empty steering file =====

    #[test]
    fn test_kiro_004_empty_file() {
        let diagnostics = validate_steering("");
        let kiro_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-004")
            .collect();
        assert_eq!(kiro_004.len(), 1);
        assert_eq!(kiro_004[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_kiro_004_whitespace_only() {
        let diagnostics = validate_steering("   \n\n  ");
        let kiro_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-004")
            .collect();
        assert_eq!(kiro_004.len(), 1);
    }

    #[test]
    fn test_kiro_004_valid_file() {
        let diagnostics = validate_steering("---\ninclusion: always\n---\n# Guidelines\n");
        let kiro_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-004")
            .collect();
        assert!(kiro_004.is_empty());
    }

    #[test]
    fn test_kiro_004_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KIRO-004".to_string()];
        let validator = KiroSteeringValidator;
        let diagnostics = validator.validate(Path::new(".kiro/steering/test.md"), "", &config);
        let kiro_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-004")
            .collect();
        assert!(kiro_004.is_empty());
    }

    // ===== Category disable =====

    #[test]
    fn test_kiro_steering_category_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().kiro_steering = false;
        let validator = KiroSteeringValidator;

        let diagnostics = validator.validate(Path::new(".kiro/steering/test.md"), "", &config);
        let kiro_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("KIRO-"))
            .collect();
        assert!(kiro_rules.is_empty());

        let diagnostics = validator.validate(
            Path::new(".kiro/steering/test.md"),
            "---\ninclusion: invalid\n---\n# Test\n",
            &config,
        );
        let kiro_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("KIRO-"))
            .collect();
        assert!(kiro_rules.is_empty());
    }

    // ===== Edge cases =====

    #[test]
    fn test_no_frontmatter_no_diagnostics() {
        let diagnostics = validate_steering("# Just a heading\nSome content.");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_malformed_yaml_no_crash() {
        let content = "---\ninclusion: auto\n  bad: indentation\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        // Malformed YAML is handled gracefully - no panic, no diagnostics
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_non_mapping_yaml_no_crash() {
        let content = "---\n- item1\n- item2\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_kiro_001_non_string_inclusion_flagged() {
        // Non-string inclusion values (number, bool) are flagged as invalid
        let content = "---\ninclusion: 123\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        let kiro_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-001")
            .collect();
        assert_eq!(kiro_001.len(), 1);
        assert_eq!(kiro_001[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_kiro_002_auto_empty_name_flagged() {
        // Empty string name should be flagged
        let content = "---\ninclusion: auto\nname: \"\"\ndescription: test desc\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-002")
            .collect();
        assert_eq!(kiro_002.len(), 1);
        assert!(kiro_002[0].message.contains("name"));
    }

    #[test]
    fn test_kiro_002_auto_null_name_flagged() {
        // Null/non-string name should be flagged
        let content = "---\ninclusion: auto\nname: null\ndescription: test desc\n---\n# Steering\n";
        let diagnostics = validate_steering(content);
        let kiro_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-002")
            .collect();
        assert_eq!(kiro_002.len(), 1);
        assert!(kiro_002[0].message.contains("name"));
    }

    #[test]
    fn test_kiro_001_case_sensitive() {
        // Inclusion modes are case-sensitive - "ALWAYS" is not valid
        let content = "---\ninclusion: ALWAYS\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        let kiro_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-001")
            .collect();
        assert_eq!(kiro_001.len(), 1);
    }

    #[test]
    fn test_kiro_003_non_string_pattern_flagged() {
        // Non-string fileMatchPattern values are flagged as invalid
        let content = "---\nfileMatchPattern: 123\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        let kiro_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-003")
            .collect();
        assert_eq!(kiro_003.len(), 1);
        assert_eq!(kiro_003[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_line_col_at_offset_is_one_based() {
        assert_eq!(line_col_at_offset("secret", 0), (1, 1));
        assert_eq!(line_col_at_offset("x\nsecret", 2), (2, 1));
    }

    #[test]
    fn test_kiro_003_empty_string_pattern() {
        // Empty string is a valid glob pattern
        let content = "---\nfileMatchPattern: \"\"\n---\n# Content\n";
        let diagnostics = validate_steering(content);
        let kiro_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-003")
            .collect();
        assert!(kiro_003.is_empty());
    }

    #[test]
    fn test_frontmatter_only_no_body_not_empty() {
        // File with frontmatter but no body is not "empty"
        let content = "---\ninclusion: always\n---\n";
        let diagnostics = validate_steering(content);
        let kiro_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KIRO-004")
            .collect();
        assert!(kiro_004.is_empty());
    }

    #[test]
    fn test_kiro_005_frontmatter_only_body() {
        let content = "---\ninclusion: always\n---\n";
        let diagnostics = validate_steering(content);
        assert!(diagnostics.iter().any(|d| d.rule == "KIRO-005"));
    }

    #[test]
    fn test_kiro_006_secrets_detected() {
        let content = "---\ninclusion: always\n---\nAPI_KEY=hardcodedsecret123\n";
        let diagnostics = validate_steering(content);
        assert!(diagnostics.iter().any(|d| d.rule == "KIRO-006"));
    }

    #[test]
    fn test_kiro_006_scans_past_template_values() {
        let content =
            "---\ninclusion: always\n---\nTOKEN=${ENV_TOKEN}\npassword=plaintextsecret123\n";
        let diagnostics = validate_steering(content);
        assert!(diagnostics.iter().any(|d| d.rule == "KIRO-006"));
    }

    #[test]
    fn test_kiro_006_ignores_plain_prose_secret_label() {
        let content = "---\ninclusion: always\n---\nSecret: guidelines\n";
        let diagnostics = validate_steering(content);
        assert!(diagnostics.iter().all(|d| d.rule != "KIRO-006"));
    }

    #[test]
    fn test_kiro_006_detects_identifier_style_secret_keys() {
        let content = "---\ninclusion: always\n---\nclient_secret: hardcodedsecret123\n";
        let diagnostics = validate_steering(content);
        assert!(diagnostics.iter().any(|d| d.rule == "KIRO-006"));
    }

    #[test]
    fn test_kiro_007_file_match_pattern_without_file_match_mode() {
        let content = "---\ninclusion: always\nfileMatchPattern: \"**/*.md\"\n---\n# body\n";
        let diagnostics = validate_steering(content);
        assert!(diagnostics.iter().any(|d| d.rule == "KIRO-007"));
    }

    #[test]
    fn test_kiro_008_unknown_frontmatter_field() {
        let content = "---\ninclusion: always\ninclusions: true\n---\n# body\n";
        let diagnostics = validate_steering(content);
        assert!(diagnostics.iter().any(|d| d.rule == "KIRO-008"));
    }

    #[test]
    fn test_kiro_009_missing_inline_file_reference() {
        let content = "---\ninclusion: always\n---\nUse #[[file:docs/missing.md]]\n";
        let diagnostics = validate_steering(content);
        assert!(diagnostics.iter().any(|d| d.rule == "KIRO-009"));
    }

    #[test]
    fn test_kiro_009_skips_absolute_inline_file_reference() {
        let content = "---\ninclusion: always\n---\nUse #[[file:/etc/passwd]]\n";
        let diagnostics = validate_steering(content);
        assert!(diagnostics.iter().all(|d| d.rule != "KIRO-009"));
    }

    #[test]
    fn test_kiro_009_skips_parent_dir_traversal_inline_file_reference() {
        let content = "---\ninclusion: always\n---\nUse #[[file:../../secrets.txt]]\n";
        let diagnostics = validate_steering(content);
        assert!(diagnostics.iter().all(|d| d.rule != "KIRO-009"));
    }

    // ===== Metadata =====

    #[test]
    fn test_metadata() {
        let v = KiroSteeringValidator;
        let meta = v.metadata();
        assert_eq!(meta.name, "KiroSteeringValidator");
        assert_eq!(
            meta.rule_ids,
            &[
                "KIRO-001", "KIRO-002", "KIRO-003", "KIRO-004", "KIRO-005", "KIRO-006", "KIRO-007",
                "KIRO-008", "KIRO-009",
            ]
        );
    }
}
