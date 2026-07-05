use super::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Known rule-ID prefixes used to validate `disabled_rules` entries.
///
/// A rule ID is considered known if it starts with any of these prefixes.
/// Note: `imports::` is a legacy prefix used in some internal diagnostics.
pub(in crate::config) const KNOWN_RULE_PREFIXES: &[&str] = &[
    "AS-",
    "CC-SK-",
    "CC-HK-",
    "CC-AG-",
    "CC-MEM-",
    "CC-OS-",
    "CC-PL-",
    "CC-SET-",
    "CDX-",
    "XML-",
    "MCP-",
    "REF-",
    "XP-",
    "AGM-",
    "COP-",
    "CUR-",
    "CLN-",
    "OC-",
    "GM-",
    "PE-",
    "VER-",
    "ROO-",
    "AMP-",
    "WS-",
    "WS-SK-",
    "KIRO-",
    "KR-AG-",
    "KR-HK-",
    "KR-MCP-",
    "KR-PW-",
    "KR-SET-",
    "KR-SK-",
    "CL-SK-",
    "CP-SK-",
    "CR-SK-",
    "CX-SK-",
    "RC-SK-",
    "imports::",
];

const REMOVED_RULES_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/removed-rules.json"));

#[derive(Debug, Clone, Deserialize)]
struct RemovedRule {
    id: String,
    removed_in: String,
    reason: String,
    #[serde(default)]
    replaced_by: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RemovedRulesIndex {
    removed_rules: Vec<RemovedRule>,
}

static REMOVED_RULES: LazyLock<HashMap<String, RemovedRule>> = LazyLock::new(|| {
    let index: RemovedRulesIndex = serde_json::from_str(REMOVED_RULES_JSON)
        .expect("crates/agnix-core/removed-rules.json must be valid JSON");
    index
        .removed_rules
        .into_iter()
        .map(|rule| (rule.id.clone(), rule))
        .collect()
});

impl LintConfig {
    /// Validate the configuration and return any warnings.
    ///
    /// This performs semantic validation beyond what TOML parsing can check:
    /// - Validates that disabled_rules match known rule ID patterns
    /// - Validates that tools array contains known tool names
    /// - Warns on deprecated fields
    pub fn validate(&self) -> Vec<ConfigWarning> {
        let mut warnings = Vec::new();

        // Validate disabled_rules match known patterns
        validate_rule_ids(
            "rules.disabled_rules",
            &self.data.rules.disabled_rules,
            &mut warnings,
        );
        let severity_rule_ids: Vec<String> = self.data.rules.severity.keys().cloned().collect();
        validate_rule_ids("rules.severity", &severity_rule_ids, &mut warnings);

        // Validate tools array contains known tools
        let known_tools = [
            "claude-code",
            "cursor",
            "codex",
            "kiro",
            "copilot",
            "github-copilot",
            "cline",
            "opencode",
            "gemini-cli",
            "amp",
            "roo-code",
            "windsurf",
            "generic",
        ];
        for tool in &self.data.tools {
            let tool_lower = tool.to_lowercase();
            if !known_tools
                .iter()
                .any(|k| k.eq_ignore_ascii_case(&tool_lower))
            {
                warnings.push(ConfigWarning {
                    field: "tools".to_string(),
                    message: t!(
                        "core.config.unknown_tool",
                        tool = tool.as_str(),
                        valid = known_tools.join(", ")
                    )
                    .to_string(),
                    suggestion: Some(t!("core.config.unknown_tool_suggestion").to_string()),
                });
            }
        }

        // Warn on deprecated fields
        if self.data.target != TargetTool::Generic && self.data.tools.is_empty() {
            // Only warn if target is non-default and tools is empty
            // (if both are set, tools takes precedence silently)
            warnings.push(ConfigWarning {
                field: "target".to_string(),
                message: t!("core.config.deprecated_target").to_string(),
                suggestion: Some(t!("core.config.deprecated_target_suggestion").to_string()),
            });
        }
        if self.data.mcp_protocol_version.is_some() {
            warnings.push(ConfigWarning {
                field: "mcp_protocol_version".to_string(),
                message: t!("core.config.deprecated_mcp_version").to_string(),
                suggestion: Some(t!("core.config.deprecated_mcp_version_suggestion").to_string()),
            });
        }

        // Validate files config glob patterns
        validate_pattern_list(
            "files.include_as_memory",
            &self.data.files.include_as_memory,
            &mut warnings,
        );
        validate_pattern_list(
            "files.include_as_generic",
            &self.data.files.include_as_generic,
            &mut warnings,
        );
        validate_pattern_list("files.exclude", &self.data.files.exclude, &mut warnings);

        // Validate per-file overrides: glob patterns and disabled-rule prefixes.
        for (idx, ov) in self.data.overrides.iter().enumerate() {
            let paths_field = format!("overrides[{}].paths", idx);
            validate_pattern_list(&paths_field, &ov.paths, &mut warnings);

            let rules_field = format!("overrides[{}].disabled_rules", idx);
            validate_rule_ids(&rules_field, &ov.disabled_rules, &mut warnings);
        }

        warnings
    }
}

/// Validate rule IDs against known prefixes, emitting a `ConfigWarning`
/// per unknown ID.
///
/// Used by [`LintConfig::validate`] to check both `[rules].disabled_rules`
/// and `[[overrides]].disabled_rules`. The `field` argument is propagated
/// into each emitted [`ConfigWarning`] so callers can identify the source
/// location (`"rules.disabled_rules"` vs `"overrides[N].disabled_rules"`).
fn validate_rule_ids(field: &str, rule_ids: &[String], warnings: &mut Vec<ConfigWarning>) {
    for rule_id in rule_ids {
        if let Some(removed) = REMOVED_RULES.get(rule_id) {
            let replacement = if removed.replaced_by.is_empty() {
                "no direct replacement".to_string()
            } else {
                format!("use {}", removed.replaced_by.join(", "))
            };
            warnings.push(ConfigWarning {
                field: field.to_string(),
                message: format!(
                    "Rule {} was removed in {}: {}",
                    removed.id, removed.removed_in, removed.reason
                ),
                suggestion: Some(format!(
                    "Remove {} from {}; {}.",
                    removed.id, field, replacement
                )),
            });
            continue;
        }
        let matches_known = KNOWN_RULE_PREFIXES
            .iter()
            .any(|prefix| rule_id.starts_with(prefix));
        if !matches_known {
            warnings.push(ConfigWarning {
                field: field.to_string(),
                message: t!(
                    "core.config.unknown_rule",
                    rule = rule_id.as_str(),
                    prefixes = KNOWN_RULE_PREFIXES.join(", ")
                )
                .to_string(),
                suggestion: Some(t!("core.config.unknown_rule_suggestion").to_string()),
            });
        }
    }
}

/// Validate a list of glob patterns (count limit, syntax, traversal, absolute paths).
///
/// Used by [`LintConfig::validate`] to check `files.*` lists and
/// `overrides[*].paths`. The `field` argument is propagated into each
/// emitted [`ConfigWarning`] so callers can identify the source location.
pub(in crate::config) fn validate_pattern_list(
    field: &str,
    patterns: &[String],
    warnings: &mut Vec<ConfigWarning>,
) {
    // Warn if pattern count exceeds recommended limit
    if patterns.len() > MAX_FILE_PATTERNS {
        warnings.push(ConfigWarning {
            field: field.to_string(),
            message: t!(
                "core.config.files_pattern_count_limit",
                field = field,
                count = patterns.len(),
                limit = MAX_FILE_PATTERNS
            )
            .to_string(),
            suggestion: Some(t!("core.config.files_pattern_count_limit_suggestion").to_string()),
        });
    }
    for pattern in patterns {
        let normalized = pattern.replace('\\', "/");
        if let Err(e) = glob::Pattern::new(&normalized) {
            warnings.push(ConfigWarning {
                field: field.to_string(),
                message: t!(
                    "core.config.invalid_files_pattern",
                    pattern = pattern.as_str(),
                    message = e.to_string()
                )
                .to_string(),
                suggestion: Some(t!("core.config.invalid_files_pattern_suggestion").to_string()),
            });
        }
        // Reject path traversal patterns
        if has_path_traversal(&normalized) {
            warnings.push(ConfigWarning {
                field: field.to_string(),
                message: t!(
                    "core.config.files_path_traversal",
                    pattern = pattern.as_str()
                )
                .to_string(),
                suggestion: Some(t!("core.config.files_path_traversal_suggestion").to_string()),
            });
        }
        // Reject absolute paths (Unix-style leading slash or Windows drive letter)
        if normalized.starts_with('/')
            || (normalized.len() >= 3
                && normalized.as_bytes()[0].is_ascii_alphabetic()
                && normalized.as_bytes().get(1..3) == Some(b":/"))
        {
            warnings.push(ConfigWarning {
                field: field.to_string(),
                message: t!(
                    "core.config.files_absolute_path",
                    pattern = pattern.as_str()
                )
                .to_string(),
                suggestion: Some(t!("core.config.files_absolute_path_suggestion").to_string()),
            });
        }
    }
}

/// Warning from configuration validation.
///
/// These warnings indicate potential issues with the configuration that
/// don't prevent validation from running but may indicate user mistakes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigWarning {
    /// The field path that has the issue (e.g., "rules.disabled_rules")
    pub field: String,
    /// Description of the issue
    pub message: String,
    /// Optional suggestion for how to fix the issue
    pub suggestion: Option<String>,
}

/// Generate a JSON Schema for the LintConfig type.
///
/// This can be used to provide editor autocompletion and validation
/// for `.agnix.toml` configuration files.
///
/// # Example
///
/// ```rust
/// use agnix_core::config::generate_schema;
///
/// let schema = generate_schema();
/// let json = serde_json::to_string_pretty(&schema).unwrap();
/// println!("{}", json);
/// ```
pub fn generate_schema() -> schemars::Schema {
    schemars::schema_for!(LintConfig)
}

#[cfg(test)]
mod tests {
    use super::KNOWN_RULE_PREFIXES;

    /// Every shipped rule ID must be covered by a `KNOWN_RULE_PREFIXES` entry.
    /// Otherwise disabling that rule in `.agnix.toml` emits a spurious
    /// `core.config.unknown_rule` warning even though the suppression works.
    /// This guards the prefix list against drifting behind
    /// `knowledge-base/rules.json`.
    #[test]
    fn known_rule_prefixes_cover_all_shipped_rules() {
        for &(id, _name) in agnix_rules::RULES_DATA {
            assert!(
                KNOWN_RULE_PREFIXES
                    .iter()
                    .any(|prefix| id.starts_with(prefix)),
                "rule ID `{id}` matches no KNOWN_RULE_PREFIXES entry - \
                 add its prefix in config/schema.rs"
            );
        }
    }
}
