//! Gemini CLI settings validation rules
//!
//! Validates:
//! - GM-009: Settings.json parse error (HIGH) and unknown top-level keys
//!   (warning; the documented categories of geminicli.com/docs/reference/configuration)
//! - GM-010: `experimental.memoryManager`, which is not a setting since v0.40.0
//! - GM-011, GM-014, GM-015: `mcpServers` names, transports and `trust`
//!   (shared with the extension manifest, `rules::gemini_mcp`)
//! - GM-013: the deprecated `tools.exclude`
//! - GM-016: `general.defaultApprovalMode` outside its enum
//! - GM-017: a `tools.core` allowlist with only shell entries
//! - GM-024: a project `tools.allowed` that auto-approves every shell command
//! - GM-025: deprecated settings keys
//!
//! The `hooks` and `hooksConfig` blocks (GM-004, GM-018, GM-019, GM-026) are
//! checked by `rules::gemini_hooks`, which also runs on this file.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel, Fix},
    rules::{
        Validator, ValidatorMetadata,
        gemini_hooks::is_home_dir,
        gemini_mcp::{JsonSink, McpSource, check_mcp_servers},
    },
    schemas::{
        gemini_settings::parse_gemini_settings,
        kiro_json::{JsonMember, JsonValue, LineIndex, closest_name, parse_json_document},
    },
};
use rust_i18n::t;
use std::path::Path;

/// Rules this validator owns, including the MCP server rules that
/// GeminiExtensionValidator also emits (GM-011, GM-014, GM-015).
const RULE_IDS: &[&str] = &[
    "GM-009", "GM-010", "GM-011", "GM-013", "GM-014", "GM-015", "GM-016", "GM-017", "GM-024",
    "GM-025",
];

/// `general.defaultApprovalMode` options in settingsSchema.ts: "**Values:**
/// `"default"`, `"auto_edit"`, `"plan"`".
const APPROVAL_MODES: &[&str] = &["default", "auto_edit", "plan"];

/// Deprecated settings in settingsSchema.ts, with their replacement:
/// "@deprecated Use ui.loadingPhrases instead." and "Deprecated: Use
/// general.topicUpdateNarration instead."
const DEPRECATED_SETTINGS: &[(&str, &str, &str)] = &[
    (
        "ui.accessibility",
        "enableLoadingPhrases",
        "ui.loadingPhrases",
    ),
    (
        "experimental",
        "topicUpdateNarration",
        "general.topicUpdateNarration",
    ),
];

pub struct GeminiSettingsValidator;

impl Validator for GeminiSettingsValidator {
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
        let path_buf = path.to_path_buf();

        let parsed = parse_gemini_settings(content);

        // GM-009: Parse error (ERROR)
        if let Some(ref error) = parsed.parse_error {
            if config.is_rule_enabled("GM-009") {
                diagnostics.push(
                    Diagnostic::error(
                        path_buf.clone(),
                        error.line,
                        error.column,
                        "GM-009",
                        t!("rules.gm_009.message", error = error.message.as_str()),
                    )
                    .with_suggestion(t!("rules.gm_009.suggestion")),
                );
            }
            return diagnostics;
        }

        let schema = match parsed.schema {
            Some(s) => s,
            None => return diagnostics,
        };

        // Key positions. The document parsed after comment stripping, so the
        // JSONC tree (comments blanked in place) has the same keys.
        let doc = parse_json_document(content, true).ok();
        let lines = LineIndex::new(content);
        let top_level = doc.as_ref().and_then(|doc| doc.root.value.as_object());
        let key_position = |members: Option<&[JsonMember]>, key: &str| {
            members
                .and_then(|members| members.iter().find(|m| m.key == key))
                .map(|m| {
                    let (line, column) = lines.line_col(m.key_start);
                    let (end_line, end_column) = lines.line_col(m.key_end);
                    (line, column, end_line, end_column)
                })
                .unwrap_or((1, 1, 1, 1))
        };

        // GM-009: Unknown top-level keys (WARNING)
        if config.is_rule_enabled("GM-009") {
            for key in &parsed.unknown_top_keys {
                let (line, column, end_line, end_column) = key_position(top_level, key);
                let mut diagnostic = Diagnostic::warning(
                    path_buf.clone(),
                    line,
                    column,
                    "GM-009",
                    t!("rules.gm_009.unknown_key", key = key.as_str()),
                )
                .with_span(end_line, end_column)
                .with_suggestion(t!("rules.gm_009.unknown_key_suggestion"));

                // Unsafe: the key may be a mistyped or misplaced setting
                // the user wants, or one a newer Gemini CLI reads.
                if let Some(members) = top_level
                    && let Some((start, end)) = member_deletion_span(content, members, key)
                {
                    diagnostic = diagnostic.with_fix(Fix::delete(
                        start,
                        end,
                        format!("Remove unknown settings key '{key}'"),
                        false,
                    ));
                }

                diagnostics.push(diagnostic);
            }
        }

        // GM-010: experimental.memoryManager is not a setting since v0.40.0
        // (WARNING). settingsSchema.ts has `memoryManager` up to v0.39.0 and
        // not from v0.40.0 on; `experimental.autoMemory` now drives
        // background extraction and the /memory inbox. Any value is
        // reported: Gemini ignores the key either way. agnix has no
        // gemini-cli version pin, so the rule is not version-gated.
        if config.is_rule_enabled("GM-010")
            && schema
                .experimental
                .as_ref()
                .is_some_and(|exp| exp.has_memory_manager)
        {
            let experimental = top_level
                .and_then(|members| members.iter().rev().find(|m| m.key == "experimental"))
                .and_then(|m| m.value.value.as_object());
            let (line, column, end_line, end_column) = key_position(experimental, "memoryManager");
            diagnostics.push(
                Diagnostic::warning(
                    path_buf.clone(),
                    line,
                    column,
                    "GM-010",
                    t!("rules.gm_010.message"),
                )
                .with_span(end_line, end_column)
                .with_suggestion(t!("rules.gm_010.suggestion")),
            );
        }

        if let Some(doc) = doc.as_ref() {
            let mut sink = JsonSink::new(path, content, config);
            check_settings_values(&mut sink, content, &doc.root.value, is_user_settings(path));
            diagnostics.append(&mut sink.diagnostics);
        }

        diagnostics
    }
}

/// `~/.gemini/settings.json`: the `.gemini` folder is in the home directory.
fn is_user_settings(path: &Path) -> bool {
    path.parent()
        .and_then(Path::parent)
        .is_some_and(is_home_dir)
}

/// The value of `a.b.c` (the last member of each key, as `JSON.parse` keeps).
fn lookup<'v>(root: &'v JsonValue, dotted: &str) -> Option<&'v JsonMember> {
    let mut parts = dotted.split('.');
    let mut member = root.get(parts.next()?)?;
    for part in parts {
        member = member.value.value.get(part)?;
    }
    Some(member)
}

/// A `tools.core` / `tools.allowed` entry for the shell tool: the tool name
/// `run_shell_command` (or the class name `ShellTool` the enterprise guide
/// uses), bare or with a `(<command prefix>)`.
fn shell_entry(entry: &str) -> Option<bool> {
    ["run_shell_command", "ShellTool"].iter().find_map(|tool| {
        let rest = entry.strip_prefix(tool)?;
        if rest.is_empty() {
            Some(true)
        } else {
            rest.starts_with('(').then_some(false)
        }
    })
}

fn check_settings_values(sink: &mut JsonSink<'_, '_>, content: &str, root: &JsonValue, user: bool) {
    let server_command = lookup(root, "mcp.serverCommand")
        .and_then(|m| m.value.value.as_str())
        .is_some_and(|command| !command.is_empty());
    if let Some(mcp_servers) = root.get("mcpServers") {
        let source = if user {
            McpSource::UserSettings { server_command }
        } else {
            McpSource::ProjectSettings { server_command }
        };
        check_mcp_servers(sink, mcp_servers, &source);
    }

    // GM-013: "The legacy `tools.exclude` setting in `settings.json` is
    // deprecated in favor of policy rules with a `deny` decision."
    if let Some(exclude) = lookup(root, "tools.exclude") {
        sink.push(
            DiagnosticLevel::Info,
            "GM-013",
            (exclude.key_start, exclude.key_end),
            t!("rules.gm_013.message"),
            t!("rules.gm_013.suggestion"),
        );
    }

    // GM-016: the enum of settingsSchema.ts, and "YOLO mode (auto-approve
    // all actions) can only be enabled via command line".
    if let Some(mode) = lookup(root, "general.defaultApprovalMode") {
        let node = &mode.value;
        let value = node.value.as_str();
        if !value.is_some_and(|v| APPROVAL_MODES.contains(&v)) {
            let shown = value.map_or_else(
                || content.get(node.start..node.end).unwrap_or("").to_string(),
                ToString::to_string,
            );
            let message = if value.is_some_and(|v| v.eq_ignore_ascii_case("yolo")) {
                t!("rules.gm_016.yolo")
            } else {
                t!(
                    "rules.gm_016.message",
                    value = shown.as_str(),
                    valid = APPROVAL_MODES.join(", ")
                )
            };
            let suggestion = match value.and_then(|v| closest_name(v, APPROVAL_MODES)) {
                Some(closest) if !value.is_some_and(|v| v.eq_ignore_ascii_case("yolo")) => {
                    t!("rules.gm_016.did_you_mean", closest = closest)
                }
                _ => t!("rules.gm_016.suggestion"),
            };
            sink.push(
                DiagnosticLevel::Warning,
                "GM-016",
                (node.start, node.end),
                message,
                suggestion,
            );
        }
    }

    // GM-017: "When you set `tools.core` to any value, _only_ the tools
    // explicitly listed will be enabled."
    if let Some(core) = lookup(root, "tools.core")
        && let Some(entries) = core.value.value.as_array()
        && !entries.is_empty()
        && entries.iter().all(|entry| {
            entry
                .value
                .as_str()
                .is_some_and(|text| shell_entry(text.trim()).is_some())
        })
    {
        sink.push(
            DiagnosticLevel::Warning,
            "GM-017",
            (core.key_start, core.key_end),
            t!("rules.gm_017.message"),
            t!("rules.gm_017.suggestion"),
        );
    }

    // GM-024: auto-approving every shell command in a project's settings,
    // which the trust dialog flags as "potentially dangerous".
    if !user
        && let Some(allowed) = lookup(root, "tools.allowed")
        && let Some(entries) = allowed.value.value.as_array()
    {
        for entry in entries {
            let Some(text) = entry.value.as_str() else {
                continue;
            };
            let trimmed = text.trim();
            if trimmed == "*" || shell_entry(trimmed) == Some(true) {
                sink.push(
                    DiagnosticLevel::Warning,
                    "GM-024",
                    (entry.start, entry.end),
                    t!("rules.gm_024.message", entry = text),
                    t!("rules.gm_024.suggestion"),
                );
            }
        }
    }

    // GM-025: deprecated keys, with their replacement.
    for (parent, key, replacement) in DEPRECATED_SETTINGS {
        if let Some(member) = lookup(root, parent).and_then(|m| m.value.value.get(key)) {
            let setting = format!("{parent}.{key}");
            sink.push(
                DiagnosticLevel::Info,
                "GM-025",
                (member.key_start, member.key_end),
                t!(
                    "rules.gm_025.message",
                    key = setting.as_str(),
                    replacement = *replacement
                ),
                t!("rules.gm_025.suggestion", replacement = *replacement),
            );
        }
    }
}

/// The bytes to delete to remove member `key` (and one separating comma)
/// from an object, so the result still parses. `None` when the key repeats.
fn member_deletion_span(
    content: &str,
    members: &[JsonMember],
    key: &str,
) -> Option<(usize, usize)> {
    let mut matching = members.iter().enumerate().filter(|(_, m)| m.key == key);
    let (index, member) = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    let bytes = content.as_bytes();
    // Start of the line when only blanks precede `pos` on it.
    let line_start = |pos: usize| {
        let mut start = pos;
        while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
            start -= 1;
        }
        if start == 0 || bytes[start - 1] == b'\n' {
            start
        } else {
            pos
        }
    };
    if let Some(next) = members.get(index + 1) {
        // `"key": value,` up to the next member.
        let start = line_start(member.key_start);
        let end = if start < member.key_start {
            line_start(next.key_start)
        } else {
            next.key_start
        };
        Some((start, end))
    } else if let Some(previous) = index.checked_sub(1).and_then(|i| members.get(i)) {
        // Last member: also remove the comma after the previous value.
        Some((previous.value.end, member.value.end))
    } else {
        Some((member.key_start, member.value.end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = GeminiSettingsValidator;
        validator.validate(
            Path::new(".gemini/settings.json"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = GeminiSettingsValidator;
        validator.validate(Path::new(".gemini/settings.json"), content, config)
    }

    fn rule<'d>(diagnostics: &'d [Diagnostic], id: &str) -> Vec<&'d Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == id).collect()
    }

    // ===== GM-009: Parse Error =====

    #[test]
    fn test_gm_009_invalid_json() {
        let diagnostics = validate("{ invalid }");
        let gm_009 = rule(&diagnostics, "GM-009");
        assert_eq!(gm_009.len(), 1);
        assert_eq!(gm_009[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_gm_009_unterminated_jsonc_comment_is_invalid() {
        let diagnostics = validate(r#"{"general": {}} /* "memoryManager": true"#);
        assert!(diagnostics.iter().any(|d| d.rule == "GM-009"));
        assert!(diagnostics.iter().all(|d| d.rule == "GM-009"));
    }

    #[test]
    fn test_gm_009_comment_removal_does_not_join_tokens() {
        let diagnostics = validate(r#"{"experimental":{"autoMemory":tr/*x*/ue}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "GM-009"));
        assert!(diagnostics.iter().all(|d| d.rule == "GM-009"));
    }

    #[test]
    fn test_gm_009_empty_content() {
        let diagnostics = validate("");
        assert_eq!(rule(&diagnostics, "GM-009").len(), 1);
    }

    #[test]
    fn test_gm_009_unknown_top_key() {
        let content = "{\"general\": {}, \"badKey\": true}";
        let diagnostics = validate(content);
        let gm_009 = rule(&diagnostics, "GM-009");
        assert_eq!(gm_009.len(), 1);
        assert_eq!(gm_009[0].level, DiagnosticLevel::Warning);
        // Points at the key.
        assert_eq!((gm_009[0].line, gm_009[0].column), (1, 17));
    }

    #[test]
    fn test_gm_009_documented_keys_are_known() {
        // agnix 0.58 warned on `hooks` and `mcpServers`, which the
        // configuration reference documents.
        let content = r#"{
  "$schema": "https://raw.githubusercontent.com/google-gemini/gemini-cli/main/schemas/settings.schema.json",
  "hooks": {},
  "mcpServers": {},
  "policyPaths": [],
  "privacy": {},
  "billing": {},
  "modelConfigs": {},
  "agents": {},
  "mcp": {},
  "useWriteTodos": true,
  "contextManagement": {},
  "admin": {},
  "telemetry": {},
  "extensions": {}
}"#;
        assert!(rule(&validate(content), "GM-009").is_empty());
    }

    #[test]
    fn test_gm_009_valid_json_no_issues() {
        assert!(rule(&validate(r#"{"general": {}}"#), "GM-009").is_empty());
    }

    #[test]
    fn test_gm_009_blocks_further_rules() {
        let diagnostics = validate("{ invalid }");
        assert!(diagnostics.iter().all(|d| d.rule == "GM-009"));
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_gemini_md_category() {
        let mut config = LintConfig::default();
        config.rules_mut().gemini_md = false;

        let diagnostics = validate_with_config("{ invalid }", &config);
        assert!(diagnostics.iter().all(|d| !d.rule.starts_with("GM-")));
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["GM-009".to_string()];

        let diagnostics = validate_with_config("{ invalid }", &config);
        assert!(rule(&diagnostics, "GM-009").is_empty());
    }

    // ===== Valid Settings =====

    #[test]
    fn test_valid_settings_no_issues() {
        let content = r#"{
  "general": {},
  "model": {},
  "hooksConfig": {"enabled": true},
  "hooks": {
    "SessionStart": [
      {"hooks": [{"type": "command", "command": "echo hello"}]}
    ]
  }
}"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got: {:?}",
            diagnostics
        );
    }

    // ===== GM-010: experimental.memoryManager =====

    #[test]
    fn test_gm_010_fires_for_any_memory_manager_value() {
        // The rule used to fire only for `true` without autoMemory; since
        // v0.40.0 the key is not a setting at all, so every value is stale.
        for value in ["true", "false", "null", "\"on\""] {
            for auto_memory in ["", ", \"autoMemory\": true", ", \"autoMemory\": false"] {
                let content = format!(
                    "{{\n  \"experimental\": {{\n    \"memoryManager\": {value}{auto_memory}\n  }}\n}}"
                );
                let diagnostics = validate(&content);
                let gm_010 = rule(&diagnostics, "GM-010");
                assert_eq!(gm_010.len(), 1, "{content}");
                assert_eq!(gm_010[0].level, DiagnosticLevel::Warning);
                assert_eq!((gm_010[0].line, gm_010[0].column), (3, 5), "{content}");
                assert!(
                    gm_010[0]
                        .message
                        .contains("not a Gemini CLI setting since v0.40.0")
                );
            }
        }
    }

    #[test]
    fn test_gm_010_auto_memory_only_no_diagnostic() {
        let content = r#"{"experimental": {"autoMemory": true}}"#;
        assert!(rule(&validate(content), "GM-010").is_empty());
    }

    #[test]
    fn test_gm_010_no_experimental_block_no_diagnostic() {
        assert!(rule(&validate(r#"{"general": {}}"#), "GM-010").is_empty());
    }

    #[test]
    fn test_gm_010_ignores_key_inside_block_comment() {
        // The string "memoryManager" appears only inside a block comment.
        let content = r#"{
  /* note: experimental.memoryManager was removed in v0.40,
     see https://github.com/google-gemini/gemini-cli/pull/25601 */
  "experimental": {}
}"#;
        assert!(rule(&validate(content), "GM-010").is_empty());
    }

    #[test]
    fn test_gm_010_ignores_key_inside_line_comment() {
        let content = r#"{
  // doc: https://example.com/gemini-cli/experimental.memoryManager
  "experimental": {}
}"#;
        assert!(rule(&validate(content), "GM-010").is_empty());
    }

    #[test]
    fn test_gm_010_memory_manager_outside_experimental_is_gm_009_only() {
        let content = r#"{"memoryManager": true}"#;
        let diagnostics = validate(content);
        assert!(rule(&diagnostics, "GM-010").is_empty());
        assert_eq!(rule(&diagnostics, "GM-009").len(), 1);
    }

    #[test]
    fn test_gm_010_disabled_by_config() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["GM-010".to_string()];

        let content = r#"{"experimental": {"memoryManager": true}}"#;
        let diagnostics = validate_with_config(content, &config);
        assert!(rule(&diagnostics, "GM-010").is_empty());
    }

    // ===== Autofix Tests =====

    #[test]
    fn test_gm_009_unknown_key_fix_is_unsafe_exact_and_reparses() {
        // The former line-based fix left a trailing comma when the unknown
        // key was the last member, which Gemini cannot parse.
        for (content, expected) in [
            (
                "{\n  \"general\": {},\n  \"badKey\": true\n}",
                "{\n  \"general\": {}\n}",
            ),
            (
                "{\n  \"badKey\": {\"a\": 1},\n  \"general\": {}\n}",
                "{\n  \"general\": {}\n}",
            ),
            (
                "{\"ui\": {}, \"badKey\": [1], \"general\": {}}",
                "{\"ui\": {}, \"general\": {}}",
            ),
            ("{\"badKey\": 1}", "{}"),
            (
                "{\n  // keep\n  \"general\": {}, // tail\n  \"badKey\": true\n}",
                "{\n  // keep\n  \"general\": {}\n}",
            ),
        ] {
            let diagnostics = validate(content);
            let gm_009 = rule(&diagnostics, "GM-009");
            assert_eq!(gm_009.len(), 1, "{content}");
            let fix = &gm_009[0].fixes[0];
            assert!(!fix.safe, "GM-009 fix should be unsafe");
            assert!(fix.is_deletion());
            let mut fixed = content.to_string();
            fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
            assert_eq!(fixed, expected, "{content}");
            assert!(
                crate::schemas::gemini_settings::parse_gemini_settings(&fixed)
                    .parse_error
                    .is_none(),
                "{fixed}"
            );
            assert!(validate(&fixed).is_empty(), "{fixed}");
        }
    }

    // ===== MCP servers (GM-011, GM-014, GM-015) =====

    fn levels(diagnostics: &[Diagnostic], id: &str) -> Vec<(DiagnosticLevel, usize, usize)> {
        rule(diagnostics, id)
            .iter()
            .map(|d| (d.level, d.line, d.column))
            .collect()
    }

    #[test]
    fn test_documented_mcp_examples_are_clean() {
        // configuration structure and override examples of tools/mcp-server.
        let content = r#"{
  "mcp": {"allowed": ["my-trusted-server"], "excluded": ["experimental-server"]},
  "mcpServers": {
    "serverName": {
      "command": "path/to/server",
      "args": ["--arg1", "value1"],
      "env": {"API_KEY": "$MY_API_TOKEN"},
      "cwd": "./server-directory",
      "timeout": 30000,
      "trust": false
    },
    "httpServer": {"httpUrl": "http://localhost:3000/mcp", "headers": {"Authorization": "Bearer $T"}},
    "google-workspace": {"excludeTools": ["gmail.send"]}
  }
}"#;
        assert!(validate(content).is_empty(), "{:?}", validate(content));
    }

    #[test]
    fn test_gm_011_underscore_server_name() {
        let content = "{\n  \"mcpServers\": {\n    \"my_server\": {\"command\": \"node\"}\n  }\n}";
        let diagnostics = validate(content);
        assert_eq!(
            levels(&diagnostics, "GM-011"),
            [(DiagnosticLevel::Warning, 3, 5)]
        );
        assert!(diagnostics[0].message.contains("'my-server'"));
    }

    #[test]
    fn test_gm_014_transport() {
        let content = r#"{"mcpServers": {
  "none": {"args": ["server.js"]},
  "empty-command": {"command": "", "cwd": "."},
  "override": {"excludeTools": ["x"], "env": {"A": "1"}, "trust": false},
  "both": {"httpUrl": "https://a/mcp", "url": "https://a/sse"},
  "tcp": {"tcp": "localhost:9000"}
}}"#;
        let diagnostics = validate(content);
        assert_eq!(
            levels(&diagnostics, "GM-014"),
            [
                (DiagnosticLevel::Error, 2, 3),
                (DiagnosticLevel::Error, 3, 3),
                (DiagnosticLevel::Info, 5, 3)
            ]
        );
        let both = rule(&diagnostics, "GM-014")[2];
        assert!(both.message.contains("only httpUrl"), "{}", both.message);

        // mcp.serverCommand may serve entries without a transport.
        let content =
            r#"{"mcp": {"serverCommand": "node s.js"}, "mcpServers": {"none": {"args": ["x"]}}}"#;
        assert!(rule(&validate(content), "GM-014").is_empty());
    }

    #[test]
    fn test_gm_015_trust_in_project_settings() {
        let content = r#"{"mcpServers": {"mine": {"command": "node", "trust": true}}}"#;
        let diagnostics = validate(content);
        assert_eq!(
            levels(&diagnostics, "GM-015"),
            [(DiagnosticLevel::Warning, 1, 54)]
        );
    }

    // ===== tools and general settings (GM-013, GM-016, GM-017, GM-024, GM-025) =====

    #[test]
    fn test_gm_013_tools_exclude_is_deprecated() {
        let content = "{\n  \"tools\": {\n    \"exclude\": [\"run_shell_command(rm)\"]\n  }\n}";
        assert_eq!(
            levels(&validate(content), "GM-013"),
            [(DiagnosticLevel::Info, 3, 5)]
        );
    }

    #[test]
    fn test_gm_016_approval_mode() {
        for valid in ["default", "auto_edit", "plan"] {
            let content = format!(r#"{{"general": {{"defaultApprovalMode": "{valid}"}}}}"#);
            assert!(rule(&validate(&content), "GM-016").is_empty(), "{valid}");
        }
        let diagnostics = validate(r#"{"general": {"defaultApprovalMode": "yolo"}}"#);
        assert_eq!(
            levels(&diagnostics, "GM-016"),
            [(DiagnosticLevel::Warning, 1, 37)]
        );
        assert!(diagnostics[0].message.contains("command line"));
        let diagnostics = validate(r#"{"general": {"defaultApprovalMode": "autoEdit"}}"#);
        assert_eq!(rule(&diagnostics, "GM-016").len(), 1);
        assert!(
            diagnostics[0]
                .suggestion
                .as_deref()
                .unwrap_or("")
                .contains("auto_edit")
        );
        assert_eq!(
            rule(
                &validate(r#"{"general": {"defaultApprovalMode": 1}}"#),
                "GM-016"
            )
            .len(),
            1
        );
    }

    #[test]
    fn test_gm_017_shell_only_core() {
        let content = r#"{"tools": {"core": ["run_shell_command(git)", "ShellTool(ls)"]}}"#;
        assert_eq!(
            levels(&validate(content), "GM-017"),
            [(DiagnosticLevel::Warning, 1, 12)]
        );
        // The enterprise guide's mixed allowlist, and the generic shell tool
        // next to other tools.
        for content in [
            r#"{"tools": {"core": ["ReadFileTool", "GlobTool", "ShellTool(ls)"]}}"#,
            r#"{"tools": {"core": ["read_file", "run_shell_command"]}}"#,
            r#"{"tools": {"core": []}}"#,
        ] {
            assert!(rule(&validate(content), "GM-017").is_empty(), "{content}");
        }
    }

    #[test]
    fn test_gm_024_broad_shell_auto_approval() {
        let content = r#"{"tools": {"allowed": ["run_shell_command(git)", "run_shell_command", "*", "ShellTool"]}}"#;
        let diagnostics = validate(content);
        assert_eq!(rule(&diagnostics, "GM-024").len(), 3);
        // The configuration reference example.
        let content =
            r#"{"tools": {"allowed": ["run_shell_command(git)", "run_shell_command(npm test)"]}}"#;
        assert!(rule(&validate(content), "GM-024").is_empty());
    }

    #[test]
    fn test_gm_025_deprecated_keys() {
        let content = r#"{
  "ui": {"accessibility": {"enableLoadingPhrases": false, "screenReader": true}},
  "experimental": {"topicUpdateNarration": true}
}"#;
        let diagnostics = validate(content);
        assert_eq!(
            levels(&diagnostics, "GM-025"),
            [
                (DiagnosticLevel::Info, 2, 28),
                (DiagnosticLevel::Info, 3, 20)
            ]
        );
        assert!(diagnostics[0].message.contains("ui.loadingPhrases"));
        assert!(
            diagnostics[1]
                .message
                .contains("general.topicUpdateNarration")
        );
    }

    #[cfg(feature = "filesystem")]
    #[test]
    fn test_user_settings_tone_down_trust_and_skip_allowed() {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let path = home.join(".gemini/settings.json");
        let content = r#"{"tools": {"allowed": ["run_shell_command"]}, "mcpServers": {"mine": {"command": "node", "trust": true}}}"#;
        let diagnostics = GeminiSettingsValidator.validate(&path, content, &LintConfig::default());
        assert!(rule(&diagnostics, "GM-024").is_empty());
        assert_eq!(
            rule(&diagnostics, "GM-015")
                .iter()
                .map(|d| d.level)
                .collect::<Vec<_>>(),
            [DiagnosticLevel::Info]
        );
    }

    #[test]
    fn test_gm_009_duplicate_unknown_key_has_no_fix() {
        let diagnostics = validate(r#"{"badKey": 1, "badKey": 2}"#);
        let gm_009 = rule(&diagnostics, "GM-009");
        assert_eq!(gm_009.len(), 1);
        assert!(gm_009[0].fixes.is_empty());
    }
}
