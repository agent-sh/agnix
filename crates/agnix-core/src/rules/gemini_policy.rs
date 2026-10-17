//! Gemini CLI policy file validation (GM-027, GM-020, GM-021, and GM-011 for
//! `mcpName`).
//!
//! Policy files are TOML files of `[[rule]]` entries
//! (geminicli.com/docs/reference/policy-engine, "TOML rule schema"), loaded
//! from the user tier (`~/.gemini/policies/*.toml`), the admin directories,
//! extensions (`<extension>/policies/*.toml`) and, once issue #18186 is
//! fixed, the workspace tier (`.gemini/policies/*.toml`).
//!
//! The policy reference writes `[[rule]]`; the subagents page shows
//! `[[rules]]`. Which spelling the loader reads was not verified, so
//! neither is reported and the entries of both are checked.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    file_types::gemini_policy_extension_root,
    rules::{
        Validator, ValidatorMetadata,
        gemini_hooks::is_home_dir,
        gemini_mcp::{gm_011_texts, underscore_free_name},
    },
    schemas::{
        gemini_toml::{parse_toml_document, toml_entry, toml_integer},
        kiro_json::{LineIndex, closest_name},
    },
};
use rust_i18n::t;
use std::path::Path;
use toml::Spanned;
use toml::de::{DeTable, DeValue};

/// Rules this validator owns. It also emits GM-011 for `mcpName` values;
/// GeminiSettingsValidator lists that rule.
const RULE_IDS: &[&str] = &["GM-027", "GM-020", "GM-021"];

/// Every rule this validator can emit.
const EMITTED_RULE_IDS: &[&str] = &["GM-027", "GM-020", "GM-021", "GM-011"];

/// "The decision to take. Must be "allow", "deny", or "ask_user"."
const DECISIONS: &[&str] = &["allow", "deny", "ask_user"];

/// "The priority of the rule, from 0 to 999."
const MAX_PRIORITY: i128 = 999;

/// The tier a policy file belongs to, from its location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tier {
    /// `<extension>/policies/*.toml`.
    Extension,
    /// `~/.gemini/policies/*.toml`.
    User,
    /// `$WORKSPACE_ROOT/.gemini/policies/*.toml`.
    Workspace,
    /// Admin directories, or a file typed as a policy by configuration.
    Other,
}

fn tier_of(path: &Path, fs: &dyn crate::fs::FileSystem) -> Tier {
    if gemini_policy_extension_root(path, fs).is_some() {
        return Tier::Extension;
    }
    let gemini_dir = path
        .parent()
        .filter(|dir| {
            dir.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case("policies"))
        })
        .and_then(Path::parent)
        .filter(|dir| {
            dir.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case(".gemini"))
        });
    match gemini_dir.and_then(Path::parent) {
        Some(owner) if is_home_dir(owner) => Tier::User,
        Some(_) => Tier::Workspace,
        None if gemini_dir.is_some() => Tier::Workspace,
        None => Tier::Other,
    }
}

pub struct GeminiPolicyValidator;

impl Validator for GeminiPolicyValidator {
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
        if !EMITTED_RULE_IDS.iter().any(|id| config.is_rule_enabled(id)) {
            return Vec::new();
        }
        let tier = tier_of(path, config.fs().as_ref());
        let mut cx = Cx {
            path,
            lines: LineIndex::new(content),
            config,
            tier,
            diagnostics: Vec::new(),
        };

        // GM-020: "The **Workspace** tier (project-level policies) is
        // currently non-functional." (issue #18186)
        if tier == Tier::Workspace {
            cx.push(
                DiagnosticLevel::Info,
                "GM-020",
                (0, 0),
                t!("rules.gm_020.message"),
                t!("rules.gm_020.suggestion"),
            );
        }

        let doc = match parse_toml_document(content) {
            Ok(doc) => doc,
            Err(error) => {
                cx.push(
                    DiagnosticLevel::Error,
                    "GM-027",
                    (error.offset, error.offset),
                    t!("rules.gm_027.parse_error", error = error.message.as_str()),
                    t!("rules.gm_027.suggestion"),
                );
                return cx.finish();
            }
        };
        let table = doc.get_ref();
        for key in ["rule", "rules"] {
            if let Some((name, value)) = toml_entry(table, key) {
                cx.check_rules(name, value);
            }
        }
        if let Some((_, value)) = toml_entry(table, "safety_checker")
            && let DeValue::Array(entries) = value.get_ref()
        {
            for entry in entries.iter() {
                if let DeValue::Table(entry) = entry.get_ref() {
                    cx.check_mcp_name(entry);
                }
            }
        }
        cx.finish()
    }
}

struct Cx<'a, 'c> {
    path: &'a Path,
    lines: LineIndex<'a>,
    config: &'a PerFileLintConfig<'c>,
    tier: Tier,
    diagnostics: Vec<Diagnostic>,
}

impl Cx<'_, '_> {
    fn push(
        &mut self,
        level: DiagnosticLevel,
        rule: &str,
        span: (usize, usize),
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) {
        if !self.config.is_rule_enabled(rule) {
            return;
        }
        let (line, column) = self.lines.line_col(span.0);
        let (end_line, end_column) = self.lines.line_col(span.1.max(span.0));
        let path = self.path.to_path_buf();
        let diagnostic = match level {
            DiagnosticLevel::Error => Diagnostic::error(path, line, column, rule, message),
            DiagnosticLevel::Warning => Diagnostic::warning(path, line, column, rule, message),
            DiagnosticLevel::Info => Diagnostic::info(path, line, column, rule, message),
        };
        self.diagnostics.push(
            diagnostic
                .with_span(end_line, end_column)
                .with_suggestion(suggestion),
        );
    }

    fn finish(mut self) -> Vec<Diagnostic> {
        // Tables keep keys in lexical order; report top to bottom.
        self.diagnostics.sort_by_key(|d| (d.line, d.column));
        self.diagnostics
    }

    fn schema(&mut self, span: std::ops::Range<usize>, message: impl Into<String>) {
        self.push(
            DiagnosticLevel::Warning,
            "GM-027",
            (span.start, span.end),
            message,
            t!("rules.gm_027.suggestion"),
        );
    }

    fn check_rules(
        &mut self,
        name: &Spanned<std::borrow::Cow<'_, str>>,
        value: &Spanned<DeValue<'_>>,
    ) {
        let DeValue::Array(entries) = value.get_ref() else {
            self.schema(
                name.span(),
                t!("rules.gm_027.not_array", key = name.get_ref().as_ref()),
            );
            return;
        };
        for entry in entries.iter() {
            match entry.get_ref() {
                DeValue::Table(rule) => self.check_rule(entry.span(), rule),
                other => self.schema(
                    entry.span(),
                    t!("rules.gm_027.entry_not_table", found = other.type_str()),
                ),
            }
        }
    }

    fn check_rule(&mut self, header: std::ops::Range<usize>, rule: &DeTable<'_>) {
        // decision: required, one of allow / deny / ask_user.
        match toml_entry(rule, "decision") {
            None => {
                let message = if toml_entry(rule, "action").is_some() {
                    t!("rules.gm_027.action_not_decision")
                } else {
                    t!("rules.gm_027.missing_decision")
                };
                self.schema(header.clone(), message);
            }
            Some((_, value)) => match value.get_ref() {
                DeValue::String(decision) if DECISIONS.contains(&decision.as_ref()) => {
                    // GM-021: "Gemini CLI ignores any `allow` decisions ...
                    // in extension policies."
                    if self.tier == Tier::Extension && decision == "allow" {
                        self.push(
                            DiagnosticLevel::Warning,
                            "GM-021",
                            (value.span().start, value.span().end),
                            t!("rules.gm_021.allow"),
                            t!("rules.gm_021.suggestion"),
                        );
                    }
                }
                DeValue::String(decision) => {
                    let message = match closest_name(decision, DECISIONS) {
                        Some(closest) => t!(
                            "rules.gm_027.invalid_decision_closest",
                            value = decision.as_ref(),
                            closest = closest
                        ),
                        None => t!("rules.gm_027.invalid_decision", value = decision.as_ref()),
                    };
                    self.schema(value.span(), message);
                }
                other => self.schema(
                    value.span(),
                    t!("rules.gm_027.decision_not_string", found = other.type_str()),
                ),
            },
        }

        // priority: an integer from 0 to 999.
        if let Some((_, value)) = toml_entry(rule, "priority") {
            match (value.get_ref(), toml_integer(value.get_ref())) {
                (_, Some(priority)) if (0..=MAX_PRIORITY).contains(&priority) => {}
                (_, Some(priority)) => self.schema(
                    value.span(),
                    t!(
                        "rules.gm_027.priority_range",
                        value = priority.to_string().as_str()
                    ),
                ),
                (other, None) => self.schema(
                    value.span(),
                    t!(
                        "rules.gm_027.priority_not_integer",
                        found = other.type_str()
                    ),
                ),
            }
        }

        // "You cannot use commandPrefix and commandRegex in the same rule."
        if toml_entry(rule, "commandPrefix").is_some()
            && let Some((key, _)) = toml_entry(rule, "commandRegex")
        {
            self.schema(key.span(), t!("rules.gm_027.prefix_and_regex"));
        }

        // "When using `mcpName`, the `toolName` field should strictly be the
        // simple name of the tool (for example, `search`), **not** the Fully
        // Qualified Name".
        if let Some((_, mcp_name)) = toml_entry(rule, "mcpName")
            && matches!(mcp_name.get_ref(), DeValue::String(_))
            && let Some((_, tool_name)) = toml_entry(rule, "toolName")
        {
            let names: Vec<&Spanned<DeValue<'_>>> = match tool_name.get_ref() {
                DeValue::Array(items) => items.iter().collect(),
                _ => vec![tool_name],
            };
            for name in names {
                if let DeValue::String(text) = name.get_ref()
                    && text.starts_with("mcp_")
                {
                    self.schema(
                        name.span(),
                        t!("rules.gm_027.fqn_tool_name", value = text.as_ref()),
                    );
                }
            }
        }

        // GM-021: "... or `yolo` mode configurations in extension policies."
        if self.tier == Tier::Extension
            && let Some((_, modes)) = toml_entry(rule, "modes")
            && let DeValue::Array(items) = modes.get_ref()
        {
            for item in items.iter() {
                if let DeValue::String(mode) = item.get_ref()
                    && mode == "yolo"
                {
                    self.push(
                        DiagnosticLevel::Warning,
                        "GM-021",
                        (item.span().start, item.span().end),
                        t!("rules.gm_021.yolo"),
                        t!("rules.gm_021.suggestion"),
                    );
                }
            }
        }

        self.check_mcp_name(rule);
    }

    /// GM-011 for an `mcpName` value: "Do not use underscores (`_`) in your
    /// MCP server names".
    fn check_mcp_name(&mut self, entry: &DeTable<'_>) {
        if let Some((_, value)) = toml_entry(entry, "mcpName")
            && let DeValue::String(name) = value.get_ref()
            && let Some(suggested) = underscore_free_name(name)
        {
            let (message, suggestion) = gm_011_texts(name, &suggested);
            self.push(
                DiagnosticLevel::Warning,
                "GM-011",
                (value.span().start, value.span().end),
                message,
                suggestion,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::fs::MockFileSystem;
    use std::sync::Arc;

    fn validate_at(path: &str, content: &str) -> Vec<Diagnostic> {
        let fs = MockFileSystem::new();
        fs.add_file("ext/gemini-extension.json", "{}");
        let mut config = LintConfig::default();
        config.set_fs(Arc::new(fs));
        GeminiPolicyValidator.validate(Path::new(path), content, &config)
    }

    /// An admin policy: no tier-specific rule applies.
    fn validate(content: &str) -> Vec<Diagnostic> {
        validate_at("/etc/gemini-cli/policies/org.toml", content)
    }

    fn rules(diagnostics: &[Diagnostic]) -> Vec<(&str, DiagnosticLevel)> {
        diagnostics
            .iter()
            .map(|d| (d.rule.as_str(), d.level))
            .collect()
    }

    #[test]
    fn documented_examples_are_clean() {
        // The policy engine reference examples.
        let content = r#"[[rule]]
toolName = "run_shell_command"
commandPrefix = "rm -rf"
decision = "deny"
priority = 100

[[rule]]
toolName = ["write_file", "replace"]
decision = "ask_user"
priority = 10

[[rule]]
mcpName = "my-jira-server"
toolName = "search"
decision = "allow"
priority = 200

[[rule]]
mcpName = "untrusted-server"
decision = "deny"
priority = 500
denyMessage = "This server is not trusted by the admin."

[[rule]]
toolName = "*"
mcpName = "*"
decision = "ask_user"
priority = 10

[[rule]]
toolName = "run_shell_command"
subagent = "codebase_investigator"
mcpName = "my-custom-server"
toolAnnotations = { readOnlyHint = true }
argsPattern = '"command":"(git|npm)'
commandRegex = "git (commit|push)"
decision = "ask_user"
priority = 10
modes = ["default", "autoEdit", "yolo"]
interactive = true
allowRedirection = true

[[rule]]
toolName = "codebase_investigator"
decision = "deny"
priority = 500
deny_message = "Deep codebase analysis is restricted for this session."
"#;
        assert!(validate(content).is_empty(), "{:?}", validate(content));
    }

    #[test]
    fn gm_027_parse_error_is_an_error() {
        let diagnostics = validate("[[rule]]\ndecision = \n");
        assert_eq!(rules(&diagnostics), [("GM-027", DiagnosticLevel::Error)]);
        assert_eq!(diagnostics[0].line, 2);
    }

    #[test]
    fn gm_027_schema_problems_point_at_the_value() {
        let content = r#"[[rule]]
toolName = "run_shell_command"
commandPrefix = "git"
commandRegex = "git (commit|push)"
decision = "ask"
priority = 1000

[[rule]]
toolName = "x"
priority = 10.5

[[rule]]
mcpName = "jira"
toolName = ["search", "mcp_jira_search"]
decision = 1
priority = -1
"#;
        let diagnostics = validate(content);
        let found: Vec<(usize, usize)> = diagnostics.iter().map(|d| (d.line, d.column)).collect();
        assert!(
            diagnostics
                .iter()
                .all(|d| d.rule == "GM-027" && d.level == DiagnosticLevel::Warning),
            "{diagnostics:?}"
        );
        assert_eq!(
            found,
            [
                (4, 1),
                (5, 12),
                (6, 12),
                (8, 1),
                (10, 12),
                (14, 23),
                (15, 12),
                (16, 12)
            ]
        );
        assert!(diagnostics[1].message.contains("ask_user"));
        assert!(diagnostics[3].message.contains("decision"));
    }

    #[test]
    fn gm_027_rule_shapes_and_action_hint() {
        let diagnostics = validate("[rule]\ndecision = \"deny\"\n");
        assert_eq!(rules(&diagnostics), [("GM-027", DiagnosticLevel::Warning)]);

        // The subagents page writes `[[rules]]` with `action`; the entries
        // are checked, and the hint names the reference's `decision`.
        let diagnostics = validate(
            "[[rules]]\nname = \"Allow pr-creator to push code\"\nsubagent = \"pr-creator\"\naction = \"allow\"\ntoolName = \"run_shell_command\"\ncommandPrefix = \"git push\"\n",
        );
        assert_eq!(rules(&diagnostics), [("GM-027", DiagnosticLevel::Warning)]);
        assert!(diagnostics[0].message.contains("`action`"));
    }

    #[test]
    fn gm_011_mcp_name_with_underscore() {
        // The extension reference's own example uses `my_server`.
        let content = "[[rule]]\nmcpName = \"my_server\"\ntoolName = \"dangerous_tool\"\ndecision = \"ask_user\"\npriority = 100\n\n[[safety_checker]]\nmcpName = \"my_server\"\ntoolName = \"write_data\"\npriority = 200\n";
        let diagnostics = validate(content);
        assert_eq!(
            rules(&diagnostics),
            [
                ("GM-011", DiagnosticLevel::Warning),
                ("GM-011", DiagnosticLevel::Warning)
            ]
        );
        assert!(diagnostics[0].message.contains("my-server"));
        assert_eq!((diagnostics[0].line, diagnostics[0].column), (2, 11));
    }

    #[test]
    fn gm_020_workspace_tier_only() {
        let content = "[[rule]]\ntoolName = \"x\"\ndecision = \"deny\"\npriority = 1\n";
        assert_eq!(
            rules(&validate_at("proj/.gemini/policies/p.toml", content)),
            [("GM-020", DiagnosticLevel::Info)]
        );
        assert_eq!(
            rules(&validate_at(".gemini/policies/p.toml", content)),
            [("GM-020", DiagnosticLevel::Info)]
        );
        assert!(validate_at("ext/policies/p.toml", content).is_empty());
        assert!(validate_at("/etc/gemini-cli/policies/p.toml", content).is_empty());
        assert!(validate_at("/home/u/.gemini/extensions/gcp/policies/p.toml", content).is_empty());
        #[cfg(feature = "filesystem")]
        if let Some(home) = dirs::home_dir() {
            let user = home.join(".gemini/policies/p.toml");
            assert!(validate_at(user.to_str().unwrap(), content).is_empty());
        }
    }

    #[test]
    fn gm_021_extension_allow_and_yolo() {
        let content = "[[rule]]\ntoolName = \"x\"\ndecision = \"allow\"\npriority = 1\nmodes = [\"default\", \"yolo\"]\n\n[[rule]]\ntoolName = \"y\"\ndecision = \"ask_user\"\npriority = 1\n";
        let diagnostics = validate_at("ext/policies/p.toml", content);
        assert_eq!(
            rules(&diagnostics),
            [
                ("GM-021", DiagnosticLevel::Warning),
                ("GM-021", DiagnosticLevel::Warning)
            ]
        );
        assert_eq!((diagnostics[0].line, diagnostics[0].column), (3, 12));
        assert_eq!((diagnostics[1].line, diagnostics[1].column), (5, 21));
        // The same rules in a user or admin policy are fine.
        assert!(validate(content).is_empty());
    }
}
