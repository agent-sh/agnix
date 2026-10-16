//! Kiro capability permission policies (KR-PM-001..004).
//!
//! Validates the documented user settings and per-workspace store. It does not
//! claim that repository-local settings are loaded as workspace permissions.
use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
    schemas::kiro_permissions::{
        VALID_PERMISSION_CAPABILITIES, VALID_PERMISSION_EFFECTS, parse_kiro_permissions,
    },
};
use rust_i18n::t;
use serde_yaml::Value;
use std::path::Path;

const RULE_IDS: &[&str] = &["KR-PM-001", "KR-PM-002", "KR-PM-003", "KR-PM-004"];

pub struct KiroPermissionsValidator;

impl Validator for KiroPermissionsValidator {
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
        let policy = match parse_kiro_permissions(content) {
            Ok(policy) => policy,
            Err(error) => {
                if config.is_rule_enabled("KR-PM-001") {
                    let (line, column) =
                        error.location().map_or((1, 0), |l| (l.line(), l.column()));
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            column,
                            "KR-PM-001",
                            t!("rules.kr_pm_001.parse_error", error = error.to_string()),
                        )
                        .with_suggestion(t!("rules.kr_pm_001.suggestion")),
                    );
                }
                return diagnostics;
            }
        };
        let Some(rules) = policy
            .as_mapping()
            .and_then(|m| m.get("rules"))
            .and_then(Value::as_sequence)
        else {
            if config.is_rule_enabled("KR-PM-001") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "KR-PM-001",
                        t!("rules.kr_pm_001.shape"),
                    )
                    .with_suggestion(t!("rules.kr_pm_001.suggestion")),
                );
            }
            return diagnostics;
        };
        for (index, rule) in rules.iter().enumerate() {
            let Some(rule) = rule.as_mapping() else {
                if config.is_rule_enabled("KR-PM-001") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "KR-PM-001",
                            t!("rules.kr_pm_001.entry", index = index),
                        )
                        .with_suggestion(t!("rules.kr_pm_001.suggestion")),
                    );
                }
                continue;
            };
            if config.is_rule_enabled("KR-PM-002")
                && !rule
                    .get("capability")
                    .and_then(Value::as_str)
                    .is_some_and(|v| VALID_PERMISSION_CAPABILITIES.contains(&v))
            {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "KR-PM-002",
                        t!("rules.kr_pm_002.message", index = index),
                    )
                    .with_suggestion(t!("rules.kr_pm_002.suggestion")),
                );
            }
            if config.is_rule_enabled("KR-PM-003")
                && !rule
                    .get("effect")
                    .and_then(Value::as_str)
                    .is_some_and(|v| VALID_PERMISSION_EFFECTS.contains(&v))
            {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "KR-PM-003",
                        t!("rules.kr_pm_003.message", index = index),
                    )
                    .with_suggestion(t!("rules.kr_pm_003.suggestion")),
                );
            }
            if config.is_rule_enabled("KR-PM-004") {
                for field in ["match", "exclude"] {
                    if let Some(value) = rule.get(field)
                        && !value.as_sequence().is_some_and(|patterns| {
                            patterns.iter().all(|pattern| pattern.as_str().is_some())
                        })
                    {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "KR-PM-004",
                                t!("rules.kr_pm_004.message", index = index, field = field),
                            )
                            .with_suggestion(t!("rules.kr_pm_004.suggestion")),
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
    use crate::config::LintConfig;

    fn validate(content: &str) -> Vec<Diagnostic> {
        KiroPermissionsValidator.validate(
            Path::new(".kiro/settings/permissions.yaml"),
            content,
            &LintConfig::default(),
        )
    }

    #[test]
    fn kiro_permissions_valid_vendor_examples_and_all_capabilities() {
        for capability in VALID_PERMISSION_CAPABILITIES {
            for effect in VALID_PERMISSION_EFFECTS {
                let content =
                    format!("rules:\n  - capability: {capability}\n    effect: {effect}\n");
                assert!(validate(&content).is_empty(), "{content}");
            }
        }
        assert!(validate("rules: []").is_empty());
        assert!(validate("rules:\n  - capability: mcp\n    match: [\"my-server/*\"]\n    exclude: [\"my-server/dangerous-tool\"]\n    effect: allow\n").is_empty());
        // Do not invent undocumented non-empty or unknown-field restrictions.
        assert!(
            validate(
                "rules: [{capability: shell, effect: ask, match: [], exclude: [], future: true}]"
            )
            .is_empty()
        );
    }

    #[test]
    fn kiro_permissions_invalid_document_shapes() {
        for content in [
            "rules: [",
            "",
            "[]",
            "rules: null",
            "rules: {}",
            "rules: [false]",
            "rules: [{capability: shell, capability: mcp}]",
        ] {
            assert!(
                validate(content).iter().any(|d| d.rule == "KR-PM-001"),
                "{content}"
            );
        }
    }

    #[test]
    fn kiro_permissions_invalid_capability_and_effect() {
        for content in [
            "rules: [{}]",
            "rules: [{capability: SHELL, effect: Allow}]",
            "rules: [{capability: 1, effect: true}]",
            "rules: [{capability: null, effect: null}]",
        ] {
            let diagnostics = validate(content);
            assert!(
                diagnostics.iter().any(|d| d.rule == "KR-PM-002"),
                "{content}"
            );
            assert!(
                diagnostics.iter().any(|d| d.rule == "KR-PM-003"),
                "{content}"
            );
            assert!(diagnostics.iter().all(|d| d.fixes.is_empty()));
        }
    }

    #[test]
    fn kiro_permissions_patterns_require_string_arrays() {
        for field in ["match", "exclude"] {
            for value in [
                "null",
                "true",
                "'git *'",
                "{}",
                "[12]",
                "[false]",
                "['git *', null]",
            ] {
                let content =
                    format!("rules: [{{capability: shell, effect: allow, {field}: {value}}}]");
                assert!(
                    validate(&content).iter().any(|d| d.rule == "KR-PM-004"),
                    "{content}"
                );
            }
        }
    }

    #[test]
    fn kiro_permissions_disabled_rule_is_respected() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = RULE_IDS.iter().map(|id| id.to_string()).collect();
        assert!(
            KiroPermissionsValidator
                .validate(
                    Path::new(".kiro/settings/permissions.yaml"),
                    "rules: [{capability: nope, effect: nope, match: true}]",
                    &config
                )
                .is_empty()
        );
        assert!(
            KiroPermissionsValidator
                .validate(
                    Path::new(".kiro/settings/permissions.yaml"),
                    "rules: [",
                    &config
                )
                .is_empty()
        );
    }
}
