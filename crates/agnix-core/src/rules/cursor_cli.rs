//! Cursor CLI configuration and allowlists: CUR-025 (broad allow entries),
//! CUR-026 (CLI settings that have no effect in a project file), CUR-027
//! (ignored permissions.json entries) and CUR-039 (invalid CLI config).
//!
//! The Cursor CLI reads `~/.cursor/cli-config.json` (global) and
//! `<project>/.cursor/cli.json` ("Only permissions can be configured at the
//! project level"), in "Pure JSON format (no comments)". The editor and CLI
//! allowlists live in `permissions.json` (`~/.cursor/permissions.json` and
//! `<workspace>/.cursor/permissions.json`, JSONC).

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    rules::config_json::{JsonCx, key_span, node_span},
    schemas::kiro_json::{JsonDialect, JsonNode, closest_name, parse_json_document},
};
use rust_i18n::t;
use std::path::Path;

/// Permission token types of the Cursor CLI (`Shell(...)`, `Read(...)`,
/// `Write(...)`, `WebFetch(...)`, `Mcp(server:tool)`).
const PERMISSION_TYPES: &[&str] = &["Shell", "Read", "Write", "WebFetch", "Mcp"];

/// CLI settings that "must be set globally" (configuration reference,
/// "Optional fields" and "Required fields" other than `version` and
/// `permissions`).
const GLOBAL_ONLY_KEYS: &[&str] = &[
    "editor",
    "channel",
    "model",
    "maxMode",
    "hasChangedDefaultModel",
    "notifications",
    "hints",
    "rewind",
    "suggestNextPrompt",
    "display",
    "approvalMode",
    "sandbox",
    "network",
    "attribution",
];

/// permissions.json top-level fields ("All fields are optional. Unknown keys
/// are ignored.").
const PERMISSIONS_JSON_FIELDS: &[&str] = &["mcpAllowlist", "terminalAllowlist", "autoRun"];

/// Validate `.cursor/cli.json` or `~/.cursor/cli-config.json`.
pub(crate) fn validate_cli_config(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let mut cx = JsonCx::new(path, content, config);
    let project = path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("cli.json"));

    let document = match parse_json_document(content, true) {
        Ok(document) => document,
        Err(error) => {
            if cx.enabled("CUR-039") {
                cx.push_at(
                    DiagnosticLevel::Error,
                    "CUR-039",
                    (error.line, error.column),
                    t!("rules.cur_039.parse_error", error = error.message.as_str()),
                    t!("rules.cur_039.suggestion"),
                );
            }
            return cx.diagnostics;
        }
    };
    if let JsonDialect::Jsonc { offset } = document.dialect
        && cx.enabled("CUR-039")
    {
        cx.push(
            DiagnosticLevel::Error,
            "CUR-039",
            (offset, offset + 1),
            t!("rules.cur_039.comments"),
            t!("rules.cur_039.suggestion"),
        );
    }
    let root = &document.root;
    let Some(members) = root.value.as_object() else {
        if cx.enabled("CUR-039") {
            cx.push(
                DiagnosticLevel::Error,
                "CUR-039",
                node_span(root),
                t!("rules.cur_039.not_object"),
                t!("rules.cur_039.suggestion"),
            );
        }
        return cx.diagnostics;
    };

    if project && cx.enabled("CUR-026") {
        for member in members {
            if GLOBAL_ONLY_KEYS.contains(&member.key.as_str()) {
                cx.push(
                    DiagnosticLevel::Warning,
                    "CUR-026",
                    key_span(member),
                    t!("rules.cur_026.message", key = member.key.as_str()),
                    t!("rules.cur_026.suggestion", key = member.key.as_str()),
                );
            }
        }
    }

    let Some(permissions) = root.value.get("permissions") else {
        return cx.diagnostics;
    };
    if permissions.value.value.as_object().is_none() {
        if cx.enabled("CUR-039") {
            cx.push(
                DiagnosticLevel::Error,
                "CUR-039",
                node_span(&permissions.value),
                t!(
                    "rules.cur_039.field_type",
                    field = "permissions",
                    expected = "an object",
                    got = permissions.value.value.type_name()
                ),
                t!("rules.cur_039.suggestion"),
            );
        }
        return cx.diagnostics;
    }
    for list in ["allow", "deny"] {
        let Some(member) = permissions.value.value.get(list) else {
            continue;
        };
        let field = format!("permissions.{list}");
        let Some(entries) = member.value.value.as_array() else {
            if cx.enabled("CUR-039") {
                cx.push(
                    DiagnosticLevel::Error,
                    "CUR-039",
                    node_span(&member.value),
                    t!(
                        "rules.cur_039.field_type",
                        field = field.as_str(),
                        expected = "an array of strings",
                        got = member.value.value.type_name()
                    ),
                    t!("rules.cur_039.suggestion"),
                );
            }
            continue;
        };
        for entry in entries {
            check_permission_entry(&mut cx, entry, list);
        }
    }
    cx.diagnostics
}

fn check_permission_entry(cx: &mut JsonCx<'_>, entry: &JsonNode, list: &str) {
    let Some(text) = entry.value.as_str() else {
        if cx.enabled("CUR-039") {
            cx.push(
                DiagnosticLevel::Error,
                "CUR-039",
                node_span(entry),
                t!(
                    "rules.cur_039.entry_type",
                    list = list,
                    got = entry.value.type_name()
                ),
                t!("rules.cur_039.suggestion"),
            );
        }
        return;
    };
    let parsed = text
        .split_once('(')
        .and_then(|(kind, rest)| rest.strip_suffix(')').map(|arg| (kind, arg)));
    let Some((kind, argument)) = parsed.filter(|(kind, arg)| {
        PERMISSION_TYPES.contains(kind) && !arg.is_empty() && (*kind != "Mcp" || arg.contains(':'))
    }) else {
        if cx.enabled("CUR-039") {
            let kind = text.split('(').next().unwrap_or(text);
            let suggestion = match closest_name(kind, PERMISSION_TYPES) {
                Some(close) if close != kind => {
                    t!("rules.cur_039.entry_suggestion_close", kind = close)
                }
                _ => t!("rules.cur_039.entry_suggestion"),
            };
            cx.push(
                DiagnosticLevel::Warning,
                "CUR-039",
                node_span(entry),
                t!("rules.cur_039.entry_format", entry = text),
                suggestion,
            );
        }
        return;
    };
    // CUR-025: "Allow fetches from any domain (use with caution)" and "Allow
    // all MCP tools (use with caution)".
    if list == "allow"
        && cx.enabled("CUR-025")
        && ((kind == "WebFetch" && argument.trim() == "*")
            || (kind == "Mcp" && argument.trim() == "*:*"))
    {
        cx.push(
            DiagnosticLevel::Warning,
            "CUR-025",
            node_span(entry),
            t!("rules.cur_025.message", entry = text),
            t!("rules.cur_025.suggestion"),
        );
    }
}

/// Validate `.cursor/permissions.json` or `~/.cursor/permissions.json`.
pub(crate) fn validate_permissions_json(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let mut cx = JsonCx::new(path, content, config);
    if !cx.enabled("CUR-027") && !cx.enabled("CUR-025") {
        return cx.diagnostics;
    }
    // "If neither file is present, both are unparseable, or no file contains a
    // given key, Cursor falls back to the IDE allowlist for that key."
    let document = match parse_json_document(content, true) {
        Ok(document) => document,
        Err(error) => {
            if cx.enabled("CUR-027") {
                cx.push_at(
                    DiagnosticLevel::Warning,
                    "CUR-027",
                    (error.line, error.column),
                    t!("rules.cur_027.parse_error", error = error.message.as_str()),
                    t!("rules.cur_027.parse_error_suggestion"),
                );
            }
            return cx.diagnostics;
        }
    };
    let root = &document.root;
    let Some(members) = root.value.as_object() else {
        if cx.enabled("CUR-027") {
            cx.push(
                DiagnosticLevel::Warning,
                "CUR-027",
                node_span(root),
                t!("rules.cur_027.not_object"),
                t!("rules.cur_027.parse_error_suggestion"),
            );
        }
        return cx.diagnostics;
    };

    for member in members {
        let key = member.key.as_str();
        if !PERMISSIONS_JSON_FIELDS.contains(&key) {
            if cx.enabled("CUR-027") {
                let suggestion = match closest_name(key, PERMISSIONS_JSON_FIELDS) {
                    Some(close) => t!("rules.cur_027.unknown_key_suggestion", key = close),
                    None => t!("rules.cur_027.unknown_key_suggestion_none"),
                };
                cx.push(
                    DiagnosticLevel::Warning,
                    "CUR-027",
                    key_span(member),
                    t!("rules.cur_027.unknown_key", key = key),
                    suggestion,
                );
            }
            continue;
        }
        if key == "autoRun" {
            continue;
        }
        let Some(entries) = member.value.value.as_array() else {
            if cx.enabled("CUR-027") {
                cx.push(
                    DiagnosticLevel::Warning,
                    "CUR-027",
                    node_span(&member.value),
                    t!(
                        "rules.cur_027.not_array",
                        key = key,
                        got = member.value.value.type_name()
                    ),
                    t!("rules.cur_027.not_array_suggestion"),
                );
            }
            continue;
        };
        for entry in entries {
            let Some(text) = entry.value.as_str() else {
                if cx.enabled("CUR-027") {
                    cx.push(
                        DiagnosticLevel::Warning,
                        "CUR-027",
                        node_span(entry),
                        t!(
                            "rules.cur_027.non_string",
                            key = key,
                            got = entry.value.type_name()
                        ),
                        t!("rules.cur_027.non_string_suggestion"),
                    );
                }
                continue;
            };
            if key != "mcpAllowlist" {
                continue;
            }
            // "Entries that do not contain a `:` are ignored."
            if !text.contains(':') {
                if cx.enabled("CUR-027") {
                    cx.push(
                        DiagnosticLevel::Warning,
                        "CUR-027",
                        node_span(entry),
                        t!("rules.cur_027.missing_colon", entry = text),
                        t!("rules.cur_027.missing_colon_suggestion", entry = text),
                    );
                }
            } else if text.trim() == "*:*" && cx.enabled("CUR-025") {
                cx.push(
                    DiagnosticLevel::Info,
                    "CUR-025",
                    node_span(entry),
                    t!("rules.cur_025.all_mcp_tools"),
                    t!("rules.cur_025.suggestion"),
                );
            }
        }
    }
    cx.diagnostics
}

#[cfg(test)]
mod tests {
    use crate::config::LintConfig;
    use crate::diagnostics::{Diagnostic, DiagnosticLevel};
    use crate::rules::Validator;
    use crate::rules::cursor::CursorValidator;
    use std::path::Path;

    fn validate(path: &str, content: &str) -> Vec<Diagnostic> {
        CursorValidator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn rule<'a>(diagnostics: &'a [Diagnostic], id: &str) -> Vec<&'a Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == id).collect()
    }

    #[test]
    fn detection() {
        for (path, expected) in [
            (".cursor/cli.json", crate::FileType::CursorCliConfig),
            (
                "/home/u/.cursor/cli-config.json",
                crate::FileType::CursorCliConfig,
            ),
            (
                ".cursor/permissions.json",
                crate::FileType::CursorPermissions,
            ),
            (
                "/home/u/.cursor/permissions.json",
                crate::FileType::CursorPermissions,
            ),
        ] {
            assert_eq!(crate::detect_file_type(Path::new(path)), expected, "{path}");
        }
    }

    #[test]
    fn documented_cli_config_is_clean() {
        let global = r#"{
  "version": 1,
  "editor": { "vimMode": false },
  "approvalMode": "allowlist",
  "permissions": {
    "allow": ["Shell(ls)", "Shell(curl:*)", "Read(src/**/*.ts)", "Write(package.json)", "WebFetch(*.github.com)", "Mcp(datadog:*)"],
    "deny": ["Shell(rm)", "Read(.env*)"]
  }
}"#;
        assert!(validate("/home/u/.cursor/cli-config.json", global).is_empty());
        let project = r#"{ "version": 1, "permissions": { "allow": ["Shell(git)"], "deny": [] } }"#;
        assert!(validate(".cursor/cli.json", project).is_empty());
    }

    #[test]
    fn cur_025_broad_allow_entries() {
        let content = r#"{ "permissions": { "allow": ["WebFetch(*)", "Mcp(*:*)", "Mcp(github:*)"], "deny": ["WebFetch(*)"] } }"#;
        let diagnostics = validate(".cursor/cli.json", content);
        let cur_025 = rule(&diagnostics, "CUR-025");
        assert_eq!(cur_025.len(), 2, "{cur_025:#?}");
        assert!(cur_025.iter().all(|d| d.level == DiagnosticLevel::Warning));

        let permissions = r#"{ "mcpAllowlist": ["*:*", "github:*"] }"#;
        let diagnostics = validate(".cursor/permissions.json", permissions);
        let cur_025 = rule(&diagnostics, "CUR-025");
        assert_eq!(cur_025.len(), 1);
        assert_eq!(cur_025[0].level, DiagnosticLevel::Info);
    }

    #[test]
    fn cur_026_global_settings_in_project_file() {
        let content = "{\n  \"version\": 1,\n  \"editor\": { \"vimMode\": true },\n  \"approvalMode\": \"unrestricted\",\n  \"permissions\": { \"allow\": [] }\n}";
        let diagnostics = validate(".cursor/cli.json", content);
        let cur_026 = rule(&diagnostics, "CUR-026");
        assert_eq!(cur_026.len(), 2);
        assert_eq!(cur_026[0].line, 3);
        // The same keys are fine in the global file.
        assert!(validate("/home/u/.cursor/cli-config.json", content).is_empty());
    }

    #[test]
    fn cur_039_invalid_cli_config() {
        let comments = "{\n  // allow git\n  \"permissions\": { \"allow\": [\"Shell(git)\"] }\n}";
        let diagnostics = validate(".cursor/cli.json", comments);
        let cur_039 = rule(&diagnostics, "CUR-039");
        assert_eq!(cur_039.len(), 1);
        assert_eq!(cur_039[0].line, 2);

        let broken = validate(".cursor/cli.json", "{ \"permissions\": ");
        assert_eq!(rule(&broken, "CUR-039").len(), 1);

        let entries = r#"{ "permissions": { "allow": ["Bash(ls)", "Mcp(github)", 3, "Shell(ls)"], "deny": "Shell(rm)" } }"#;
        let diagnostics = validate(".cursor/cli.json", entries);
        let cur_039 = rule(&diagnostics, "CUR-039");
        assert_eq!(cur_039.len(), 4, "{cur_039:#?}");
        assert!(
            cur_039
                .iter()
                .any(|d| d.suggestion.as_deref().unwrap().contains("Shell"))
        );
    }

    #[test]
    fn cur_027_ignored_permissions_entries() {
        let content = "{\n  // shared allowlist\n  \"mcpAllowlist\": [\"github:*\", \"linear\", 7],\n  \"terminalAllowlist\": [\"git\", \"npm\",],\n  \"mcpAllowList\": [\"x:y\"],\n  \"autoRun\": { \"allow_instructions\": [\"Reads are fine.\"] }\n}";
        let diagnostics = validate(".cursor/permissions.json", content);
        let cur_027 = rule(&diagnostics, "CUR-027");
        assert_eq!(cur_027.len(), 3, "{cur_027:#?}");
        assert!(cur_027.iter().all(|d| d.line == 3 || d.line == 5));
        let typo = cur_027.iter().find(|d| d.line == 5).unwrap();
        assert!(typo.suggestion.as_deref().unwrap().contains("mcpAllowlist"));
        assert_eq!(
            rule(
                &validate(".cursor/permissions.json", "{ \"mcpAllowlist\": "),
                "CUR-027"
            )
            .len(),
            1
        );
    }
}
