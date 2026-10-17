//! Codex CLI managed-requirements validation (CDX-REQ-000 to CDX-REQ-004).
//!
//! Validates Codex's admin-written `requirements.toml` (system location:
//! `/etc/codex/requirements.toml` on Unix, `%ProgramData%\OpenAI\Codex\
//! requirements.toml` on Windows). The file is deserialized upstream by
//! `ConfigRequirementsToml` (codex-rs/config/src/config_requirements.rs) and,
//! crucially, has **no** `#[serde(deny_unknown_fields)]` - so a typo in a
//! top-level key is silently ignored by Codex and the intended constraint is
//! never enforced. CDX-REQ-001 surfaces those typos; CDX-REQ-000 reports
//! invalid TOML syntax. CDX-REQ-002 checks `[rules].prefix_rules` decisions,
//! CDX-REQ-003 managed hook script paths and CDX-REQ-004 the
//! `[experimental_network]` domain settings. Inline `[hooks]` also get the
//! CDX-HK-* hook checks.
use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    rules::codex_hooks::{HookCheck, HookLayer, HookSpans, check_hook_events},
    rules::config_span::{Seg, TomlSpans, line_col},
    rules::{Validator, ValidatorMetadata},
};
use rust_i18n::t;
use serde_json::Value;
use std::path::Path;

const RULE_IDS: &[&str] = &[
    "CDX-REQ-000",
    "CDX-REQ-001",
    "CDX-REQ-002",
    "CDX-REQ-003",
    "CDX-REQ-004",
];

/// Known top-level keys of `requirements.toml`, sourced from
/// `ConfigRequirementsToml` @ openai/codex rust-v0.146.0. Both `features`
/// (serde rename) and `feature_requirements` (serde alias) are accepted on
/// disk; the network table is only valid as `experimental_network` (serde
/// rename - the bare `network` field name is not a valid TOML key).
const KNOWN_REQUIREMENTS_KEYS: &[&str] = &[
    "allow_appshots",
    "allow_login_shell",
    "allow_managed_hooks_only",
    "allow_remote_control",
    "allowed_approval_policies",
    "allowed_approvals_reviewers",
    "allowed_permission_profiles",
    "allowed_sandbox_modes",
    "allowed_web_search_modes",
    "apps",
    "browser_use",
    "check_for_update_on_startup",
    "computer_use",
    "default_permissions",
    "enforce_residency",
    "experimental_network",
    "feature_requirements",
    "features",
    "feedback",
    "guardian_policy_config",
    "hooks",
    "log_dir",
    "marketplaces",
    "mcp_servers",
    "model_catalog_json",
    "models",
    "permissions",
    "plugins",
    "remote_sandbox_config",
    "rules",
    "sqlite_home",
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

        let spans = TomlSpans::parse(content);
        let pos = |segs: &[Seg<'_>]| {
            spans
                .as_ref()
                .and_then(|spans| spans.offset(segs))
                .map_or((1, 1), |offset| line_col(content, offset))
        };
        let root = serde_json::to_value(&table).unwrap_or(Value::Null);

        // CDX-REQ-002: "requirements rules must specify `decision`, and that
        // decision must be `"prompt"` or `"forbidden"` (not `"allow"`)."
        if config.is_rule_enabled("CDX-REQ-002")
            && let Some(rules) = root
                .get("rules")
                .and_then(|rules| rules.get("prefix_rules"))
                .and_then(Value::as_array)
        {
            for (index, rule) in rules.iter().enumerate() {
                let decision = rule.get("decision");
                let problem = match decision.and_then(Value::as_str) {
                    Some("prompt" | "forbidden") => None,
                    Some(other) => Some(t!("rules.cdx_req_002.invalid", value = other)),
                    None if decision.is_some() => Some(t!("rules.cdx_req_002.not_string")),
                    None => Some(t!("rules.cdx_req_002.missing")),
                };
                if let Some(problem) = problem {
                    let mut segs = vec![
                        Seg::Key("rules"),
                        Seg::Key("prefix_rules"),
                        Seg::Index(index),
                    ];
                    if decision.is_some() {
                        segs.push(Seg::Key("decision"));
                    }
                    let (line, column) = pos(&segs);
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            column,
                            "CDX-REQ-002",
                            t!(
                                "rules.cdx_req_002.message",
                                index = index + 1,
                                problem = problem.to_string()
                            ),
                        )
                        .with_suggestion(t!("rules.cdx_req_002.suggestion")),
                    );
                }
            }
        }

        if let Some(hooks) = root.get("hooks") {
            // CDX-HK-*: managed hooks use the same event structure.
            if let Some(spans) = spans.as_ref() {
                let check = HookCheck {
                    path,
                    content,
                    spans: HookSpans::Toml(spans),
                    layer: HookLayer::RequirementsToml,
                };
                check_hook_events(
                    hooks,
                    &[Seg::Key("hooks")],
                    &check,
                    config,
                    &mut diagnostics,
                );
            }
            // CDX-REQ-003: "Managed hook commands should use absolute script
            // paths under the configured managed directory."
            if config.is_rule_enabled("CDX-REQ-003") {
                managed_hook_paths(hooks, &pos, path, &mut diagnostics);
            }
        }

        // CDX-REQ-004: "Do not combine the canonical `domains` map with the
        // legacy `allowed_domains` or `denied_domains` lists", and use
        // `managed_allowed_domains_only = true` "only when you also define
        // administrator-owned "allow" entries".
        if config.is_rule_enabled("CDX-REQ-004")
            && let Some(network) = root.get("experimental_network").and_then(Value::as_object)
        {
            let domains = network.get("domains").and_then(Value::as_object);
            for legacy in ["allowed_domains", "denied_domains"] {
                if domains.is_some() && network.contains_key(legacy) {
                    let (line, column) = pos(&[Seg::Key("experimental_network"), Seg::Key(legacy)]);
                    diagnostics.push(
                        Diagnostic::info(
                            path.to_path_buf(),
                            line,
                            column,
                            "CDX-REQ-004",
                            t!("rules.cdx_req_004.mixed", key = legacy),
                        )
                        .with_suggestion(t!("rules.cdx_req_004.suggestion")),
                    );
                }
            }
            let has_allow = domains.is_some_and(|domains| {
                domains
                    .values()
                    .any(|effect| effect.as_str() == Some("allow"))
            }) || network
                .get("allowed_domains")
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty());
            if network
                .get("managed_allowed_domains_only")
                .and_then(Value::as_bool)
                == Some(true)
                && !has_allow
            {
                let (line, column) = pos(&[
                    Seg::Key("experimental_network"),
                    Seg::Key("managed_allowed_domains_only"),
                ]);
                diagnostics.push(
                    Diagnostic::info(
                        path.to_path_buf(),
                        line,
                        column,
                        "CDX-REQ-004",
                        t!("rules.cdx_req_004.no_allow"),
                    )
                    .with_suggestion(t!("rules.cdx_req_004.suggestion")),
                );
            }
        }

        // `toml::Table` is a BTreeMap, so `keys()` yields keys in lexical
        // order rather than file order. Sort so diagnostics read top-to-bottom.
        diagnostics.sort_by_key(|d| (d.line, d.column));
        diagnostics
    }
}

/// CDX-REQ-003: command hooks whose script is not an absolute path under
/// `managed_dir` (or `windows_managed_dir` for `command_windows`).
fn managed_hook_paths(
    hooks: &Value,
    pos: &dyn Fn(&[Seg<'_>]) -> (usize, usize),
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(hooks) = hooks.as_object() else {
        return;
    };
    let managed_dir = hooks.get("managed_dir").and_then(Value::as_str);
    let windows_dir = hooks.get("windows_managed_dir").and_then(Value::as_str);
    let under = |command: &str, dir: &str| {
        let dir = dir.trim_end_matches(['/', '\\']);
        command
            .split_whitespace()
            .map(|token| token.trim_matches(|c| c == '"' || c == '\''))
            .any(|token| {
                token.len() > dir.len()
                    && token.starts_with(dir)
                    && token[dir.len()..].starts_with(['/', '\\'])
            })
    };
    for (event, groups) in hooks {
        let Some(groups) = groups.as_array() else {
            continue;
        };
        for (group_index, group) in groups.iter().enumerate() {
            let Some(handlers) = group.get("hooks").and_then(Value::as_array) else {
                continue;
            };
            for (index, handler) in handlers.iter().enumerate() {
                if handler.get("type").and_then(Value::as_str) != Some("command") {
                    continue;
                }
                for (field, dir_key, dir) in [
                    ("command", "managed_dir", managed_dir),
                    ("command_windows", "windows_managed_dir", windows_dir),
                ] {
                    let Some(command) = handler
                        .get(field)
                        .or_else(|| {
                            (field == "command_windows")
                                .then(|| handler.get("commandWindows"))
                                .flatten()
                        })
                        .and_then(Value::as_str)
                    else {
                        continue;
                    };
                    let message = match dir {
                        Some(dir) if under(command, dir) => continue,
                        Some(dir) => t!(
                            "rules.cdx_req_003.outside",
                            event = event.as_str(),
                            dir = dir
                        ),
                        None if field == "command" => {
                            t!(
                                "rules.cdx_req_003.no_dir",
                                event = event.as_str(),
                                key = dir_key
                            )
                        }
                        None => continue,
                    };
                    let (line, column) = pos(&[
                        Seg::Key("hooks"),
                        Seg::Key(event.as_str()),
                        Seg::Index(group_index),
                        Seg::Key("hooks"),
                        Seg::Index(index),
                        Seg::Key(field),
                    ]);
                    diagnostics.push(
                        Diagnostic::info(path.to_path_buf(), line, column, "CDX-REQ-003", message)
                            .with_suggestion(t!("rules.cdx_req_003.suggestion")),
                    );
                }
            }
        }
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
    fn documented_requirements_examples_are_clean() {
        let content = r#"allow_managed_hooks_only = true

[features]
hooks = true

[hooks]
managed_dir = "/enterprise/hooks"
windows_managed_dir = 'C:\enterprise\hooks'

[[hooks.PreToolUse]]
matcher = "^Bash$"

[[hooks.PreToolUse.hooks]]
type = "command"
command = "python3 /enterprise/hooks/pre_tool_use_policy.py"
command_windows = 'py -3 C:\enterprise\hooks\pre_tool_use_policy.py'
timeout = 30
statusMessage = "Checking managed Bash command"

[rules]
prefix_rules = [
  { pattern = [{ token = "rm" }], decision = "forbidden", justification = "Use git clean -fd instead." },
  { pattern = [{ token = "git" }, { any_of = ["push", "commit"] }], decision = "prompt", justification = "Require review before mutating history." },
]

[experimental_network]
enabled = true
managed_allowed_domains_only = true

[experimental_network.domains]
"api.openai.com" = "allow"
"blocked.example.com" = "deny"
"#;
        let diagnostics = validate(content);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn cdx_req_002_requires_prompt_or_forbidden() {
        let content = "[rules]\nprefix_rules = [\n  { pattern = [{ token = \"rm\" }] },\n  { pattern = [{ token = \"ls\" }], decision = \"allow\" },\n]\n";
        let diagnostics: Vec<_> = validate(content)
            .into_iter()
            .filter(|d| d.rule == "CDX-REQ-002")
            .collect();
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].line, 3);
        assert_eq!(diagnostics[1].line, 4);
        assert!(diagnostics[1].message.contains("allow"));
    }

    #[test]
    fn cdx_req_003_and_004_and_managed_hook_checks() {
        let content = r#"[hooks]
managed_dir = "/enterprise/hooks"

[[hooks.SessionEnd]]
[[hooks.SessionEnd.hooks]]
type = "command"
command = "python3 ./scripts/end.py"
timeout = 10

[experimental_network]
managed_allowed_domains_only = true
allowed_domains = []

[experimental_network.domains]
"x.example.com" = "deny"
"#;
        let diagnostics = validate(content);
        let summary: Vec<_> = diagnostics
            .iter()
            .map(|d| (d.rule.as_str(), d.line))
            .collect();
        assert!(summary.contains(&("CDX-REQ-003", 7)), "{summary:?}");
        assert!(summary.contains(&("CDX-HK-001", 8)), "{summary:?}");
        assert!(summary.contains(&("CDX-REQ-004", 11)), "{summary:?}");
        assert!(summary.contains(&("CDX-REQ-004", 12)), "{summary:?}");
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
        // profile sub-table. `allowed_permissions` was removed upstream and is
        // intentionally not accepted.
        let content = r#"
allow_managed_hooks_only = true
allow_appshots = true
allowed_sandbox_modes = ["read-only", "workspace-write"]
allowed_permission_profiles = ["locked"]
allow_remote_control = false
browser_use = false
default_permissions = "locked"
check_for_update_on_startup = false
allow_login_shell = false
sqlite_home = "/var/lib/codex"
log_dir = "/var/log/codex"
model_catalog_json = "/etc/codex/models.json"
feedback = false
enforce_residency = "us"
guardian_policy_config = "deny everything"

[computer_use]
allow_locked_computer_use = false

[features]
some_feature = true

[mcp_servers.docs]
command = "docs-server"

[marketplaces]
restrict_to_allowed_sources = true

[marketplaces.allowed_sources.company]
source = "git"
url = "https://github.com/example/plugins.git"
ref = "main"

[apps.example]
enabled = true

[rules]
prefix_rules = []

[experimental_network]
enabled = true

[permissions.locked]
description = "locked profile"

[models]
gpt-5 = "allowed"

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
    fn obsolete_allowed_permissions_key_flagged() {
        let diags = rules("allowed_permissions = [\"locked\"]\n", "CDX-REQ-001");
        assert_eq!(
            diags.len(),
            1,
            "removed allowed_permissions key should be flagged: {diags:?}"
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
