//! Codex CLI managed-requirements validation (CDX-REQ-000, CDX-REQ-001).
//!
//! Validates Codex's admin-written `requirements.toml` (system location:
//! `/etc/codex/requirements.toml` on Unix, `%ProgramData%\OpenAI\Codex\
//! requirements.toml` on Windows). The file is deserialized upstream by
//! `ConfigRequirementsToml` (codex-rs/config/src/config_requirements.rs) and,
//! crucially, has **no** `#[serde(deny_unknown_fields)]` - so a typo in a
//! top-level key is silently ignored by Codex and the intended constraint is
//! never enforced. CDX-REQ-001 surfaces those typos; CDX-REQ-000 reports
//! invalid TOML syntax.
use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
};
use std::path::Path;

const RULE_IDS: &[&str] = &["CDX-REQ-000", "CDX-REQ-001"];

/// Known top-level keys of `requirements.toml`, sourced from
/// `ConfigRequirementsToml` @ openai/codex rust-v0.136.0. Both `features`
/// (serde rename) and `feature_requirements` (serde alias) are accepted on
/// disk; the network table is only valid as `experimental_network` (serde
/// rename - the bare `network` field name is not a valid TOML key).
const KNOWN_REQUIREMENTS_KEYS: &[&str] = &[
    "allow_appshots",
    "allow_managed_hooks_only",
    "allowed_approval_policies",
    "allowed_approvals_reviewers",
    "allowed_permissions",
    "allowed_sandbox_modes",
    "allowed_web_search_modes",
    "apps",
    "computer_use",
    "enforce_residency",
    "experimental_network",
    "feature_requirements",
    "features",
    "guardian_policy_config",
    "hooks",
    "mcp_servers",
    "permissions",
    "plugins",
    "remote_sandbox_config",
    "rules",
    "windows",
];

pub struct CodexRequirementsValidator;

impl Validator for CodexRequirementsValidator {
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
        if !config.rules().codex {
            return diagnostics;
        }

        // CDX-REQ-000: TOML syntax. Parse as a top-level table (stable across
        // toml crate versions, unlike `toml::Value` document parsing).
        let table: toml::Table = match toml::from_str::<toml::Table>(content) {
            Ok(table) => table,
            Err(error) => {
                let (line, column) = toml_error_location(content, &error);
                if config.is_rule_enabled("CDX-REQ-000") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            column,
                            "CDX-REQ-000",
                            format!("Invalid TOML in requirements.toml: {}", error.message()),
                        )
                        .with_suggestion("Fix the TOML syntax error."),
                    );
                }
                return diagnostics;
            }
        };

        // CDX-REQ-001: unknown top-level key. requirements.toml has no
        // deny_unknown_fields upstream, so a typo'd key is silently dropped.
        if config.is_rule_enabled("CDX-REQ-001") {
            for key in table.keys() {
                if !KNOWN_REQUIREMENTS_KEYS.contains(&key.as_str()) {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            find_toml_key_line(content, key).unwrap_or(1),
                            0,
                            "CDX-REQ-001",
                            format!(
                                "Unknown requirements.toml key '{key}'. Codex silently ignores unknown keys, so this constraint will not be enforced. Known keys: {}",
                                KNOWN_REQUIREMENTS_KEYS.join(", ")
                            ),
                        )
                        .with_suggestion(
                            "Remove or rename the key (Codex does not reject unknown keys, so typos go unnoticed).",
                        ),
                    );
                }
            }
        }

        // `toml::Table` is a BTreeMap, so `keys()` yields keys in lexical
        // order rather than file order. Sort so diagnostics read top-to-bottom.
        diagnostics.sort_by_key(|d| (d.line, d.column));
        diagnostics
    }
}

/// Convert a `toml` parse error's byte span into a (line, column). The line is
/// 1-indexed and the column 1-indexed when a span is available; the fallback
/// `(1, 0)` (column 0) is used only when the error carries no span.
fn toml_error_location(content: &str, error: &toml::de::Error) -> (usize, usize) {
    error
        .span()
        .map(|span| {
            let mut line = 1usize;
            let mut column = 1usize;
            for (idx, ch) in content.char_indices() {
                if idx >= span.start {
                    break;
                }
                if ch == '\n' {
                    line += 1;
                    column = 1;
                } else {
                    column += 1;
                }
            }
            (line, column)
        })
        .unwrap_or((1, 0))
}

/// Find the 1-indexed line of a top-level TOML key. Matches the table-header
/// form (`[key]`, `[key.sub]`, `[[key]]`), the plain assignment (`key = ...`),
/// and the dotted-key assignment (`key.sub = ...`). Inline comments are
/// stripped before matching so a trailing `# ...` cannot interfere.
fn find_toml_key_line(content: &str, key: &str) -> Option<usize> {
    let quoted = format!("\"{key}\"");
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.split('#').next().unwrap_or("").trim();
        // Table header: [key], [key.sub], or [[key]]
        let header = trimmed
            .strip_prefix("[[")
            .or_else(|| trimmed.strip_prefix('['));
        if let Some(rest) = header {
            let name = rest.trim_start();
            let is_header = |after: &str| {
                let a = after.trim_start();
                a.starts_with('.') || a.starts_with(']')
            };
            if name.strip_prefix(key).is_some_and(is_header)
                || name.strip_prefix(&quoted).is_some_and(is_header)
            {
                return Some(idx + 1);
            }
            continue;
        }
        // Plain or dotted-key assignment: `key = ...`, `"key" = ...`, or
        // `key.sub = ...`.
        if let Some(after) = trimmed
            .strip_prefix(key)
            .or_else(|| trimmed.strip_prefix(&quoted))
        {
            let a = after.trim_start();
            if a.starts_with('=') || a.starts_with('.') {
                return Some(idx + 1);
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
        let config = LintConfig::builder().build().expect("config");
        let per_file = config.for_path(&PathBuf::from("/etc/codex/requirements.toml"));
        CodexRequirementsValidator.validate_per_file(
            &PathBuf::from("/etc/codex/requirements.toml"),
            content,
            &per_file,
        )
    }

    fn rules(content: &str, code: &str) -> Vec<String> {
        validate(content)
            .into_iter()
            .filter(|d| d.rule == code)
            .map(|d| d.message)
            .collect()
    }

    #[test]
    fn parse_error_flagged() {
        let diags = rules("allowed_sandbox_modes = [unclosed", "CDX-REQ-000");
        assert_eq!(diags.len(), 1, "expected one CDX-REQ-000, got: {diags:?}");
    }

    #[test]
    fn known_keys_not_flagged() {
        // A realistic managed requirements.toml exercising scalar, array, and
        // table forms plus the `experimental_network` rename and a permission
        // profile sub-table.
        let content = r#"
allow_managed_hooks_only = true
allow_appshots = true
allowed_sandbox_modes = ["read-only", "workspace-write"]
allowed_permissions = ["locked"]
enforce_residency = "us"
guardian_policy_config = "deny everything"

[computer_use]
allow_locked_computer_use = false

[features]
some_feature = true

[mcp_servers.docs]
command = "docs-server"

[apps.example]
enabled = true

[rules]
prefix_rules = []

[experimental_network]
enabled = true

[permissions.locked]
description = "locked profile"

[windows]
allowed_sandbox_implementations = ["elevated", "unelevated"]
"#;
        let diags = rules(content, "CDX-REQ-001");
        assert!(
            diags.is_empty(),
            "known keys should not be flagged: {diags:?}"
        );
        assert!(rules(content, "CDX-REQ-000").is_empty());
    }

    #[test]
    fn feature_requirements_alias_accepted() {
        let diags = rules("[feature_requirements]\nx = true\n", "CDX-REQ-001");
        assert!(
            diags.is_empty(),
            "feature_requirements alias is valid: {diags:?}"
        );
    }

    #[test]
    fn unknown_scalar_key_flagged() {
        // Typo of allowed_sandbox_modes - silently ignored by Codex.
        let diags = rules("allowed_sandbox_mode = [\"read-only\"]\n", "CDX-REQ-001");
        assert_eq!(
            diags.len(),
            1,
            "typo'd scalar key should be flagged: {diags:?}"
        );
    }

    #[test]
    fn unknown_table_key_flagged_with_line() {
        // `network` is the old name; the on-disk key was renamed to
        // experimental_network, so a bare [network] table is unknown.
        let content = "allow_managed_hooks_only = true\n\n[network]\nenabled = true\n";
        let diags = validate(content);
        let req001: Vec<_> = diags.iter().filter(|d| d.rule == "CDX-REQ-001").collect();
        assert_eq!(req001.len(), 1, "renamed [network] table should be flagged");
        assert_eq!(
            req001[0].line, 3,
            "should point at the [network] header line"
        );
    }

    #[test]
    fn unknown_dotted_key_flagged_with_line() {
        // Dotted-key assignment: `unknown.enabled = true` parses to top-level
        // key `unknown`. The line-finder must locate the dotted form, not fall
        // back to line 1.
        let content = "allow_managed_hooks_only = true\nunknown.enabled = true\n";
        let diags = validate(content);
        let req001: Vec<_> = diags.iter().filter(|d| d.rule == "CDX-REQ-001").collect();
        assert_eq!(req001.len(), 1, "dotted unknown key should be flagged");
        assert_eq!(req001[0].line, 2, "should point at the dotted-key line");
    }

    #[test]
    fn diagnostics_sorted_by_line() {
        // Keys chosen so BTreeMap order (alpha) differs from file order:
        // `zzz_unknown` sorts after `aaa_unknown` but appears first in the file.
        let content = "zzz_unknown = 1\naaa_unknown = 2\n";
        let diags = validate(content);
        let lines: Vec<usize> = diags
            .iter()
            .filter(|d| d.rule == "CDX-REQ-001")
            .map(|d| d.line)
            .collect();
        assert_eq!(lines, vec![1, 2], "diagnostics should be in file order");
    }

    #[test]
    fn disabled_when_codex_rules_off() {
        let mut config = LintConfig::builder().build().expect("config");
        config.rules_mut().codex = false;
        let per_file = config.for_path(&PathBuf::from("/etc/codex/requirements.toml"));
        let diags = CodexRequirementsValidator.validate_per_file(
            &PathBuf::from("/etc/codex/requirements.toml"),
            "totally_unknown_key = true\n",
            &per_file,
        );
        assert!(diags.is_empty(), "no diagnostics when codex rules disabled");
    }
}
