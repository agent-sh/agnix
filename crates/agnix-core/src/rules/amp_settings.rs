//! Amp settings files (AMP-004, AMP-007; AMP-006 and AMP-010 on
//! `amp.mcpServers`).
//!
//! Amp reads user settings from `~/.config/amp/settings.json` or
//! `settings.jsonc`, workspace settings from "the nearest
//! `.amp/settings.json` or `.amp/settings.jsonc`", and administrator
//! settings from `managed-settings.json` (ampcode.com/docs/cli/settings).
//! "All settings use the `amp.` prefix." The keys and types below are those
//! of the official schema, https://ampcode.com/cli-settings.schema.json
//! (21 `amp.` properties and `$schema`, `"additionalProperties": false`),
//! plus four keys the settings page names outside it (`amp.url`,
//! `amp.proxy`, `amp.workspaces`, `amp.mcpTrustedServers`). The schema also
//! has `"patternProperties": { "^amp\\.": true }`, so another `amp.` key is
//! schema-valid but undocumented (info), while a key without the prefix is
//! not (warning).
//!
//! Both file names are parsed as JSONC: Amp documents `settings.jsonc` and
//! shows `//` comments in its settings examples. Whether `settings.json`
//! accepts comments is not documented, so agnix does not report them.

use super::amp_mcp::{
    McpOrigin, Sink, check_servers, last_members, member_deletion_span, type_name,
};
use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel, Fix},
    schemas::kiro_json::{JsonMember, JsonNode, closest_name, parse_json_document},
};
use rust_i18n::t;
use std::path::Path;

/// Value shape of a documented setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Bool,
    Str,
    StrArray,
    /// `amp.keymap`: values are a string, an array of strings or null.
    Keymap,
    /// `amp.mcpServers`: server name to server entry.
    McpServers,
    /// `amp.defaultVisibility`: values are private, workspace or group.
    Visibility,
    /// `amp.mcpPermissions` (AMP-007).
    McpPermissions,
    /// `amp.updates.mode`: auto, warn or disabled.
    UpdatesMode,
    /// `amp.admin.compatibilityDate`: `YYYY-MM-DD`, managed settings only.
    CompatibilityDate,
    /// Named on the settings page without a type.
    Any,
}

/// Every documented setting with its schema type.
const SETTINGS: &[(&str, Kind)] = &[
    ("$schema", Kind::Str),
    ("amp.fuzzy.alwaysIncludePaths", Kind::StrArray),
    ("amp.showCosts", Kind::Bool),
    ("amp.git.commit.ampThread.enabled", Kind::Bool),
    ("amp.git.commit.coauthor.enabled", Kind::Bool),
    ("amp.keymap", Kind::Keymap),
    ("amp.mcpServers", Kind::McpServers),
    ("amp.defaultVisibility", Kind::Visibility),
    ("amp.notifications.enabled", Kind::Bool),
    ("amp.remoteThreadCreation.enabled", Kind::Bool),
    ("amp.runner.autoUpdate.enabled", Kind::Bool),
    ("amp.runner.env.enabled", Kind::Bool),
    ("amp.skills.disableClaudeCodeSkills", Kind::Bool),
    ("amp.skills.disableGlobalAgentsSkills", Kind::Bool),
    ("amp.skills.path", Kind::Str),
    ("amp.terminal.copyOnSelect", Kind::Bool),
    ("amp.terminal.detailsExpandedByDefault", Kind::Bool),
    ("amp.thread.autoArchiveOnQuit", Kind::Bool),
    ("amp.tools.disable", Kind::StrArray),
    ("amp.mcpPermissions", Kind::McpPermissions),
    ("amp.updates.mode", Kind::UpdatesMode),
    ("amp.admin.compatibilityDate", Kind::CompatibilityDate),
    // The settings page names these outside the schema ("You cannot set
    // these keys in the workspace document").
    ("amp.url", Kind::Any),
    ("amp.proxy", Kind::Any),
    ("amp.workspaces", Kind::Any),
    ("amp.mcpTrustedServers", Kind::Any),
];

const UPDATES_MODES: &[&str] = &["auto", "warn", "disabled"];
const VISIBILITIES: &[&str] = &["private", "workspace", "group"];
const PERMISSION_ACTIONS: &[&str] = &["allow", "reject"];
const MATCH_KEYS: &[&str] = &["command", "args", "url"];

fn known_keys() -> Vec<&'static str> {
    SETTINGS.iter().map(|(key, _)| *key).collect()
}

/// Validate an Amp settings file.
pub(crate) fn validate_amp_settings(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    if !["AMP-004", "AMP-006", "AMP-007", "AMP-010"]
        .iter()
        .any(|rule| config.is_rule_enabled(rule))
    {
        return Vec::new();
    }
    let mut sink = Sink::new(path, content, config);
    let doc = match parse_json_document(content, true) {
        Ok(doc) => doc,
        Err(error) => {
            if config.is_rule_enabled("AMP-004") {
                sink.diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        error.line,
                        error.column,
                        "AMP-004",
                        t!("rules.amp_004.parse_error", error = error.message.as_str()),
                    )
                    .with_suggestion(t!("rules.amp_004.parse_error_suggestion")),
                );
            }
            return sink.diagnostics;
        }
    };
    let Some(members) = doc.root.value.as_object() else {
        sink.push(
            DiagnosticLevel::Error,
            "AMP-004",
            (doc.root.start, doc.root.end),
            t!(
                "rules.amp_004.root_not_object",
                found = type_name(&doc.root)
            )
            .to_string(),
            t!("rules.amp_004.root_not_object_suggestion").to_string(),
        );
        return sink.diagnostics;
    };
    let managed = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("managed-settings.json"));

    for (index, member) in last_members(members) {
        match SETTINGS.iter().find(|(key, _)| *key == member.key) {
            Some((_, kind)) => check_setting(&mut sink, member, *kind, managed),
            None => report_unknown_key(&mut sink, content, members, index),
        }
    }
    sink.diagnostics
}

fn report_unknown_key(
    sink: &mut Sink<'_, '_>,
    content: &str,
    members: &[JsonMember],
    index: usize,
) {
    let member = &members[index];
    let key = member.key.as_str();
    let known = known_keys();
    let suggestion = match closest_name(key, &known) {
        Some(suggested) => t!("rules.amp_004.did_you_mean", suggested = suggested).to_string(),
        None => t!("rules.amp_004.unknown_key_suggestion").to_string(),
    };
    let (level, message) = if key.starts_with("amp.") {
        (
            DiagnosticLevel::Info,
            t!("rules.amp_004.unknown_amp_key", key = key).to_string(),
        )
    } else {
        (
            DiagnosticLevel::Warning,
            t!("rules.amp_004.unknown_key", key = key).to_string(),
        )
    };
    // Unsafe: Amp may read a key its schema leaves out (as it does the four
    // keys the settings page names), so removing it can change behavior.
    let fix = member_deletion_span(content, members, index).map(|(start, end)| {
        Fix::delete(
            start,
            end,
            t!("rules.amp_004.fix_remove", key = key).to_string(),
            false,
        )
    });
    sink.push_with_fix(
        level,
        "AMP-004",
        (member.key_start, member.key_end),
        message,
        suggestion,
        fix,
    );
}

fn wrong_type(sink: &mut Sink<'_, '_>, member: &JsonMember, expected: &str) {
    sink.push(
        DiagnosticLevel::Warning,
        "AMP-004",
        (member.value.start, member.value.end),
        t!(
            "rules.amp_004.type",
            key = member.key.as_str(),
            expected = expected,
            found = type_name(&member.value)
        )
        .to_string(),
        t!("rules.amp_004.type_suggestion").to_string(),
    );
}

fn is_string_array(node: &JsonNode) -> bool {
    node.value
        .as_array()
        .is_some_and(|items| items.iter().all(|item| item.value.as_str().is_some()))
}

fn check_setting(sink: &mut Sink<'_, '_>, member: &JsonMember, kind: Kind, managed: bool) {
    let value = &member.value;
    match kind {
        Kind::Bool if value.value.as_bool().is_none() => wrong_type(sink, member, "a boolean"),
        Kind::Str if value.value.as_str().is_none() => wrong_type(sink, member, "a string"),
        Kind::StrArray if !is_string_array(value) => {
            wrong_type(sink, member, "an array of strings");
        }
        Kind::Keymap => match value.value.as_object() {
            Some(entries) => {
                for (_, entry) in last_members(entries) {
                    let ok = entry.value.value.as_str().is_some()
                        || entry.value.value.is_null()
                        || is_string_array(&entry.value);
                    if !ok {
                        sink.push(
                            DiagnosticLevel::Warning,
                            "AMP-004",
                            (entry.value.start, entry.value.end),
                            t!(
                                "rules.amp_004.keymap_entry",
                                entry = entry.key.as_str(),
                                found = type_name(&entry.value)
                            )
                            .to_string(),
                            t!("rules.amp_004.type_suggestion").to_string(),
                        );
                    }
                }
            }
            None => wrong_type(sink, member, "an object"),
        },
        Kind::Visibility => match value.value.as_object() {
            Some(entries) => {
                for (_, entry) in last_members(entries) {
                    let found = entry.value.value.as_str();
                    if !found.is_some_and(|visibility| VISIBILITIES.contains(&visibility)) {
                        let found = found
                            .map(|v| format!("\"{v}\""))
                            .unwrap_or_else(|| type_name(&entry.value).to_string());
                        sink.push(
                            DiagnosticLevel::Warning,
                            "AMP-004",
                            (entry.value.start, entry.value.end),
                            t!(
                                "rules.amp_004.visibility",
                                entry = entry.key.as_str(),
                                found = found
                            )
                            .to_string(),
                            t!("rules.amp_004.type_suggestion").to_string(),
                        );
                    }
                }
            }
            None => wrong_type(sink, member, "an object"),
        },
        Kind::McpServers => match value.value.as_object() {
            Some(servers) => check_servers(sink, servers, McpOrigin::Settings),
            None => wrong_type(sink, member, "an object"),
        },
        Kind::McpPermissions => check_mcp_permissions(sink, member),
        Kind::UpdatesMode => match value.value.as_str() {
            Some(mode) if UPDATES_MODES.contains(&mode) => {}
            Some(mode) => sink.push(
                DiagnosticLevel::Warning,
                "AMP-004",
                (value.start, value.end),
                t!("rules.amp_004.updates_mode", value = mode).to_string(),
                t!("rules.amp_004.type_suggestion").to_string(),
            ),
            None => wrong_type(sink, member, "\"auto\", \"warn\" or \"disabled\""),
        },
        Kind::CompatibilityDate => {
            match value.value.as_str() {
                Some(date) if is_iso_date(date) => {}
                Some(date) => sink.push(
                    DiagnosticLevel::Warning,
                    "AMP-004",
                    (value.start, value.end),
                    t!("rules.amp_004.compatibility_date_format", value = date).to_string(),
                    t!("rules.amp_004.type_suggestion").to_string(),
                ),
                None => wrong_type(sink, member, "a string"),
            }
            if !managed {
                sink.push(
                    DiagnosticLevel::Info,
                    "AMP-004",
                    (member.key_start, member.key_end),
                    t!("rules.amp_004.compatibility_date_managed").to_string(),
                    t!("rules.amp_004.compatibility_date_managed_suggestion").to_string(),
                );
            }
        }
        _ => {}
    }
}

/// The schema's `^[0-9]{4}-[0-9]{2}-[0-9]{2}$`.
fn is_iso_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}

/// AMP-007: `amp.mcpPermissions` is a list of `{ "matches": {...}, "action":
/// "allow" | "reject" }` rules (schema: `required: ["matches", "action"]`,
/// `additionalProperties: false`). The settings page: "For a remote MCP
/// server, use the `url` key to match the server endpoint. For a local MCP
/// server, use the `command` and `args` keys" and "`matches` must be an
/// object, such as `{"command":"*"}`, not a string."
fn check_mcp_permissions(sink: &mut Sink<'_, '_>, member: &JsonMember) {
    let suggestion = || t!("rules.amp_007.suggestion").to_string();
    let Some(rules) = member.value.value.as_array() else {
        sink.push(
            DiagnosticLevel::Warning,
            "AMP-007",
            (member.value.start, member.value.end),
            t!("rules.amp_007.not_array", found = type_name(&member.value)).to_string(),
            suggestion(),
        );
        return;
    };
    for (index, rule) in rules.iter().enumerate() {
        let span = (rule.start, rule.end);
        let Some(fields) = rule.value.as_object() else {
            sink.push(
                DiagnosticLevel::Warning,
                "AMP-007",
                span,
                t!(
                    "rules.amp_007.rule_not_object",
                    index = index,
                    found = type_name(rule)
                )
                .to_string(),
                suggestion(),
            );
            continue;
        };
        for (_, field) in last_members(fields) {
            if !["matches", "action"].contains(&field.key.as_str()) {
                sink.push(
                    DiagnosticLevel::Warning,
                    "AMP-007",
                    (field.key_start, field.key_end),
                    t!(
                        "rules.amp_007.rule_key",
                        index = index,
                        key = field.key.as_str()
                    )
                    .to_string(),
                    suggestion(),
                );
            }
        }
        let get = |key: &str| fields.iter().rev().find(|field| field.key == key);
        match get("matches") {
            None => sink.push(
                DiagnosticLevel::Warning,
                "AMP-007",
                span,
                t!("rules.amp_007.matches_missing", index = index).to_string(),
                suggestion(),
            ),
            Some(matches) => match matches.value.value.as_object() {
                None => sink.push(
                    DiagnosticLevel::Warning,
                    "AMP-007",
                    (matches.value.start, matches.value.end),
                    t!(
                        "rules.amp_007.matches_not_object",
                        index = index,
                        found = type_name(&matches.value)
                    )
                    .to_string(),
                    suggestion(),
                ),
                Some(conditions) => {
                    for (_, condition) in last_members(conditions) {
                        if !MATCH_KEYS.contains(&condition.key.as_str()) {
                            sink.push(
                                DiagnosticLevel::Warning,
                                "AMP-007",
                                (condition.key_start, condition.key_end),
                                t!(
                                    "rules.amp_007.matches_key",
                                    index = index,
                                    key = condition.key.as_str()
                                )
                                .to_string(),
                                suggestion(),
                            );
                        }
                    }
                    let has = |key: &str| conditions.iter().any(|c| c.key == key);
                    if has("url") && (has("command") || has("args")) {
                        sink.push(
                            DiagnosticLevel::Warning,
                            "AMP-007",
                            (matches.value.start, matches.value.end),
                            t!("rules.amp_007.mixed", index = index).to_string(),
                            suggestion(),
                        );
                    }
                }
            },
        }
        match get("action") {
            None => sink.push(
                DiagnosticLevel::Warning,
                "AMP-007",
                span,
                t!("rules.amp_007.action_missing", index = index).to_string(),
                suggestion(),
            ),
            Some(action) => {
                let found = action.value.value.as_str();
                if !found.is_some_and(|value| PERMISSION_ACTIONS.contains(&value)) {
                    let found = found
                        .map(|v| format!("\"{v}\""))
                        .unwrap_or_else(|| type_name(&action.value).to_string());
                    sink.push(
                        DiagnosticLevel::Warning,
                        "AMP-007",
                        (action.value.start, action.value.end),
                        t!("rules.amp_007.action_invalid", index = index, found = found)
                            .to_string(),
                        suggestion(),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::LintConfig;
    use crate::diagnostics::{Diagnostic, DiagnosticLevel};
    use crate::rules::{Validator, amp::AmpValidator};
    use crate::schemas::kiro_json::parse_json_document;
    use std::path::Path;

    fn validate(path: &str, content: &str) -> Vec<Diagnostic> {
        AmpValidator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn findings(diagnostics: &[Diagnostic]) -> Vec<(&str, DiagnosticLevel, usize, usize)> {
        diagnostics
            .iter()
            .map(|d| (d.rule.as_str(), d.level, d.line, d.column))
            .collect()
    }

    /// Every documented key with a valid value, as on the settings page.
    const DOCUMENTED: &str = r#"{
	"$schema": "https://ampcode.com/cli-settings.schema.json",
	"amp.fuzzy.alwaysIncludePaths": ["dist/**", "node_modules/@myorg/**"],
	"amp.showCosts": true,
	"amp.git.commit.ampThread.enabled": true,
	"amp.git.commit.coauthor.enabled": false,
	"amp.keymap": {"ctrl+o": "commandPalette", "ctrl+x": ["a", "b"], "ctrl+y": null},
	"amp.mcpServers": {
		"playwright": {"command": "npx", "args": ["-y", "@playwright/mcp@latest", "--headless"]},
		"linear": {"url": "https://mcp.linear.app/sse"},
		"sourcegraph": {"url": "${SRC_ENDPOINT}/.api/mcp/v1", "headers": {"Authorization": "token ${SRC_ACCESS_TOKEN}"}},
		"internal-tools": {"url": "https://mcp.example.com/mcp", "headers": {"Authorization": "Bearer ${amp:id-token}"}}
	},
	"amp.defaultVisibility": {"github.com/org/repo": "workspace"},
	"amp.notifications.enabled": true,
	"amp.remoteThreadCreation.enabled": false,
	"amp.runner.autoUpdate.enabled": true,
	"amp.runner.env.enabled": false,
	"amp.skills.disableClaudeCodeSkills": false,
	"amp.skills.disableGlobalAgentsSkills": false,
	"amp.skills.path": "~/my-skills:/shared/team-skills",
	"amp.terminal.copyOnSelect": true,
	"amp.terminal.detailsExpandedByDefault": false,
	"amp.thread.autoArchiveOnQuit": false,
	"amp.tools.disable": ["builtin:Bash", "mcp_*"],
	"amp.mcpPermissions": [
		// Allow specific trusted MCP servers
		{ "matches": { "command": "npx", "args": "* @playwright/mcp@*" }, "action": "allow" },
		{ "matches": { "url": "https://mcp.trusted.com/mcp" }, "action": "allow" },
		// Block potentially risky MCP servers
		{ "matches": { "command": "python", "args": "*bad_command*" }, "action": "reject" },
		{ "matches": { "url": "*/malicious.com*" }, "action": "reject" }
	],
	"amp.updates.mode": "warn",
	"amp.url": "https://ampcode.com",
	"amp.proxy": "http://proxy:8080",
	"amp.workspaces": {},
	"amp.mcpTrustedServers": []
}"#;

    #[test]
    fn documented_settings_are_clean() {
        for path in [
            ".amp/settings.json",
            ".amp/settings.jsonc",
            "/home/u/.config/amp/settings.json",
        ] {
            let diagnostics = validate(path, DOCUMENTED);
            assert!(diagnostics.is_empty(), "{path}: {diagnostics:?}");
        }
        let managed = "{\"amp.admin.compatibilityDate\": \"2024-01-15\", \"amp.showCosts\": false}";
        assert!(validate("/etc/ampcode/managed-settings.json", managed).is_empty());
    }

    #[test]
    fn parse_errors_and_root() {
        let diagnostics = validate(".amp/settings.json", "{ invalid json");
        assert_eq!(findings(&diagnostics)[0].0, "AMP-004");
        assert_eq!(diagnostics[0].level, DiagnosticLevel::Error);
        let diagnostics = validate(".amp/settings.json", r#"["not", "object"]"#);
        assert_eq!(
            findings(&diagnostics),
            [("AMP-004", DiagnosticLevel::Error, 1, 1)]
        );
        assert!(diagnostics[0].message.contains("JSON object"));
    }

    #[test]
    fn unknown_keys() {
        // `model` was accepted by the old allowlist; it is not in the schema
        // (and has no `amp.` prefix), so it is a warning now. Unknown keys
        // were errors.
        let content = "{\n  \"model\": \"x\",\n  \"badKey\": true,\n  \"amp.showCost\": true,\n  \"amp.dangerouslyAllowAll\": true\n}";
        let diagnostics = validate(".amp/settings.json", content);
        assert_eq!(
            findings(&diagnostics),
            [
                ("AMP-004", DiagnosticLevel::Warning, 2, 3),
                ("AMP-004", DiagnosticLevel::Warning, 3, 3),
                ("AMP-004", DiagnosticLevel::Info, 4, 3),
                ("AMP-004", DiagnosticLevel::Info, 5, 3),
            ]
        );
        assert!(
            diagnostics[2]
                .suggestion
                .as_deref()
                .unwrap()
                .contains("amp.showCosts")
        );
        for diagnostic in &diagnostics {
            let fix = &diagnostic.fixes[0];
            assert!(!fix.safe, "AMP-004 deletion is unsafe");
            assert!(fix.is_deletion());
            let mut fixed = content.to_string();
            fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
            assert!(parse_json_document(&fixed, false).is_ok(), "{fixed:?}");
        }
    }

    #[test]
    fn value_types() {
        let content = r#"{
  "amp.showCosts": "true",
  "amp.tools.disable": "Bash",
  "amp.skills.path": ["a"],
  "amp.updates.mode": "never",
  "amp.keymap": {"ctrl+o": 1},
  "amp.defaultVisibility": {"github.com/o/r": "public"},
  "amp.mcpServers": [],
  "$schema": 1
}"#;
        let diagnostics = validate(".amp/settings.json", content);
        let lines: Vec<_> = findings(&diagnostics)
            .into_iter()
            .map(|(rule, level, line, _)| {
                assert_eq!((rule, level), ("AMP-004", DiagnosticLevel::Warning));
                line
            })
            .collect();
        assert_eq!(lines, [2, 3, 4, 5, 6, 7, 8, 9], "{diagnostics:?}");
        // At the value.
        assert_eq!(diagnostics[0].column, 20);
    }

    #[test]
    fn compatibility_date() {
        let diagnostics = validate(
            ".amp/settings.json",
            "{\"amp.admin.compatibilityDate\": \"2024-1-15\"}",
        );
        assert_eq!(
            findings(&diagnostics)
                .iter()
                .map(|(rule, level, ..)| (*rule, *level))
                .collect::<Vec<_>>(),
            [
                ("AMP-004", DiagnosticLevel::Warning),
                ("AMP-004", DiagnosticLevel::Info)
            ]
        );
        assert!(diagnostics[1].message.contains("managed settings"));
        let managed = validate(
            "C:/ProgramData/ampcode/managed-settings.json",
            "{\"amp.admin.compatibilityDate\": \"2024-01-15\"}",
        );
        assert!(managed.is_empty());
    }

    #[test]
    fn mcp_permissions_amp_007() {
        let content = r#"{
  "amp.mcpPermissions": [
    { "matches": "*", "action": "reject" },
    { "matches": { "url": "*", "command": "*" }, "action": "deny" },
    { "matches": { "name": "x" } },
    { "action": "allow", "comment": "x" },
    "allow"
  ]
}"#;
        let diagnostics = validate(".amp/settings.json", content);
        let found: Vec<_> = findings(&diagnostics)
            .into_iter()
            .map(|(rule, level, line, _)| {
                assert_eq!((rule, level), ("AMP-007", DiagnosticLevel::Warning));
                line
            })
            .collect();
        // matches string; mixed + bad action; unknown matches key + no
        // action; extra rule key + no matches; rule not an object.
        assert_eq!(found, [3, 4, 4, 5, 5, 6, 6, 7], "{diagnostics:?}");
        assert!(diagnostics[0].message.contains("must be an object"));
        let not_array = validate(".amp/settings.json", r#"{"amp.mcpPermissions": {}}"#);
        assert_eq!(findings(&not_array)[0].0, "AMP-007");
    }

    #[test]
    fn mcp_servers_regression_no_amp_004() {
        // Regression: the old allowlist reported the documented
        // `amp.mcpServers` key as unknown (error, with a delete fix).
        let content = r#"{
  "amp.mcpServers": {
    "playwright": {"command": "npx", "args": ["-y", "@playwright/mcp@latest"]},
    "linear": {"url": "https://mcp.linear.app/sse"}
  }
}"#;
        let diagnostics = validate(".amp/settings.json", content);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let diagnostics = validate(".amp/settings.json", DOCUMENTED);
        assert!(diagnostics.iter().all(|d| d.fixes.is_empty()));
    }

    #[test]
    fn mcp_servers_in_settings() {
        // AMP-006 errors and the Amp-specific AMP-010 checks in
        // amp.mcpServers; no AMP-008 (includeTools is a skill
        // recommendation). Missing command/url and field types are left to
        // the generic MCP checks in settings files.
        let content = r#"{
  "amp.mcpServers": {
    "a": {"url": "https://x.dev?t=${amp:id-token}"},
    "b": {"command": "srv", "env": {"T": "${amp:id-token}"}},
    "c": {"command": "srv", "url": "https://x.dev", "args": "x"},
    "d": {"url": "https://x.dev", "oauth": {"clientId": "c"}, "transport": "ws"},
    "e": {"args": ["x"]}
  }
}"#;
        let diagnostics = validate(".amp/settings.json", content);
        assert_eq!(
            findings(&diagnostics)
                .iter()
                .map(|(rule, level, line, _)| (*rule, *level, *line))
                .collect::<Vec<_>>(),
            [
                ("AMP-006", DiagnosticLevel::Error, 3),
                ("AMP-006", DiagnosticLevel::Error, 4),
                ("AMP-010", DiagnosticLevel::Warning, 5),
                ("AMP-010", DiagnosticLevel::Warning, 6),
                ("AMP-010", DiagnosticLevel::Warning, 6),
            ],
            "{diagnostics:?}"
        );
        assert!(diagnostics[2].message.contains("both command and url"));
        assert!(diagnostics[3].message.contains("authUrl, tokenUrl"));
        assert!(diagnostics[4].message.contains("'ws'"));
    }

    #[test]
    fn legacy_local_settings_file_is_not_validated() {
        // `.amp/settings.local.json` is no longer an Amp settings file.
        assert!(validate(".amp/settings.local.json", "{\"model\": 1}").is_empty());
    }
}
