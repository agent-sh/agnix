//! Tests for CC-HK-010 (SessionEnd), CC-HK-002 (http), CC-HK-021, CC-HK-025
//! (case), CC-HK-029..035, the retired CC-HK-017, and the other-vendor
//! hooks.json guard.

use super::*;
use crate::config::LintConfig;
use crate::diagnostics::DiagnosticLevel;
use std::fs;
use tempfile::TempDir;

fn validate(content: &str) -> Vec<Diagnostic> {
    HooksValidator.validate(Path::new("settings.json"), content, &LintConfig::default())
}

fn rule<'a>(diagnostics: &'a [Diagnostic], id: &str) -> Vec<&'a Diagnostic> {
    diagnostics.iter().filter(|d| d.rule == id).collect()
}

fn settings(event: &str, matcher: Option<&str>, hook: serde_json::Value) -> String {
    let mut group = serde_json::json!({ "hooks": [hook] });
    if let Some(matcher) = matcher {
        group["matcher"] = serde_json::json!(matcher);
    }
    serde_json::to_string_pretty(&serde_json::json!({ "hooks": { event: [group] } })).unwrap()
}

fn write(root: &Path, rel: &str, content: &str) -> std::path::PathBuf {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
    path
}

fn validate_file(path: &Path) -> Vec<Diagnostic> {
    let content = fs::read_to_string(path).unwrap();
    HooksValidator.validate(path, &content, &LintConfig::default())
}

// ===== CC-HK-010: SessionEnd budget =====

#[test]
fn cc_hk_010_session_end_timeout_above_cap_warns() {
    for (timeout, warns) in [(1, false), (5, false), (60, false), (61, true), (120, true)] {
        let content = settings(
            "SessionEnd",
            None,
            serde_json::json!({"type": "command", "command": "echo bye", "timeout": timeout}),
        );
        let diagnostics = validate(&content);
        let hits = rule(&diagnostics, "CC-HK-010");
        assert_eq!(!hits.is_empty(), warns, "{timeout}: {diagnostics:?}");
        if warns {
            assert_eq!(hits[0].level, DiagnosticLevel::Warning);
            assert!(hits[0].message.contains("60 seconds"));
            // Points at the timeout value, not line 1.
            let line = content
                .lines()
                .position(|l| l.contains("\"timeout\""))
                .unwrap()
                + 1;
            assert_eq!(hits[0].line, line);
        }
    }
}

#[test]
fn cc_hk_010_no_warning_for_missing_or_long_timeouts_elsewhere() {
    // The hooks reference documents defaults only and uses `"timeout": 120`
    // on its own agent-hook example, so neither case is reported.
    for (event, hook) in [
        (
            "PreToolUse",
            serde_json::json!({"type": "command", "command": "echo"}),
        ),
        (
            "PreToolUse",
            serde_json::json!({"type": "command", "command": "echo", "timeout": 900}),
        ),
        (
            "Stop",
            serde_json::json!({"type": "agent", "prompt": "Check tests", "timeout": 120}),
        ),
        (
            "Stop",
            serde_json::json!({"type": "prompt", "prompt": "Done?"}),
        ),
        (
            "SessionEnd",
            serde_json::json!({"type": "command", "command": "echo"}),
        ),
    ] {
        let diagnostics = validate(&settings(event, Some("Bash"), hook));
        assert!(
            rule(&diagnostics, "CC-HK-010").is_empty(),
            "{diagnostics:?}"
        );
    }
}

#[test]
fn cc_hk_010_plugin_session_end_timeout_is_info() {
    let temp = TempDir::new().unwrap();
    write(
        temp.path(),
        ".claude-plugin/plugin.json",
        r#"{"name": "p"}"#,
    );
    let path = write(
        temp.path(),
        "hooks/hooks.json",
        &settings(
            "SessionEnd",
            None,
            serde_json::json!({"type": "command", "command": "echo bye", "timeout": 10}),
        ),
    );
    let diagnostics = validate_file(&path);
    let hits = rule(&diagnostics, "CC-HK-010");
    assert_eq!(hits.len(), 1, "{diagnostics:?}");
    assert_eq!(hits[0].level, DiagnosticLevel::Info);
    assert!(hits[0].message.contains("1.5-second"));
}

// ===== CC-HK-017 retired =====

#[test]
fn cc_hk_017_retired_prompt_without_arguments_is_fine() {
    // "If `$ARGUMENTS` is not present, input JSON is appended to the prompt"
    let diagnostics = validate(&settings(
        "Stop",
        None,
        serde_json::json!({"type": "prompt", "prompt": "Should Claude stop?"}),
    ));
    assert!(rule(&diagnostics, "CC-HK-017").is_empty());
    assert!(!RULE_IDS.contains(&"CC-HK-017"));
}

// ===== CC-HK-002: http on SessionStart and Setup =====

#[test]
fn cc_hk_002_http_hook_on_session_start_and_setup() {
    for (event, fires) in [("SessionStart", true), ("Setup", true), ("Stop", false)] {
        let diagnostics = validate(&settings(
            event,
            None,
            serde_json::json!({"type": "http", "url": "http://localhost:8080/h"}),
        ));
        assert_eq!(
            !rule(&diagnostics, "CC-HK-002").is_empty(),
            fires,
            "{event}: {diagnostics:?}"
        );
    }
}

// ===== CC-HK-021: `if` =====

#[test]
fn cc_hk_021_if_on_non_tool_event_is_an_error() {
    let content = settings(
        "Stop",
        None,
        serde_json::json!({"type": "command", "command": "echo", "if": "Bash(git *)"}),
    );
    let diagnostics = validate(&content);
    let hits = rule(&diagnostics, "CC-HK-021");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].level, DiagnosticLevel::Error);
    assert!(hits[0].message.contains("never runs"));
    let line = content.lines().position(|l| l.contains("\"if\"")).unwrap() + 1;
    assert_eq!(hits[0].line, line);
}

#[test]
fn cc_hk_021_combined_rules() {
    for (rule_text, op) in [
        ("Bash(git *) && Edit(*.ts)", Some("&&")),
        ("Bash(git *) || Bash(npm *)", Some("||")),
        ("Bash(git *), Edit(*.ts)", Some(",")),
        ("Bash(npm test && npm run lint)", None),
        ("Edit(src/**)", None),
    ] {
        let diagnostics = validate(&settings(
            "PreToolUse",
            Some("Bash"),
            serde_json::json!({"type": "command", "command": "echo", "if": rule_text}),
        ));
        let hits = rule(&diagnostics, "CC-HK-021");
        match op {
            Some(op) => {
                assert_eq!(hits.len(), 1, "{rule_text}: {diagnostics:?}");
                assert_eq!(hits[0].level, DiagnosticLevel::Warning);
                assert!(hits[0].message.contains(&format!("'{op}'")));
            }
            None => assert!(hits.is_empty(), "{rule_text}: {diagnostics:?}"),
        }
    }
}

// ===== CC-HK-025: case-sensitive tool matchers =====

#[test]
fn cc_hk_025_tool_matcher_case() {
    for (matcher, fires) in [
        ("bash", true),
        ("Edit|write", true),
        ("Bash", false),
        ("Edit|Write", false),
        ("^bash$", false),
        ("my_tool", false),
    ] {
        let diagnostics = validate(&settings(
            "PreToolUse",
            Some(matcher),
            serde_json::json!({"type": "command", "command": "echo"}),
        ));
        assert_eq!(
            !rule(&diagnostics, "CC-HK-025").is_empty(),
            fires,
            "{matcher}: {diagnostics:?}"
        );
    }
}

// ===== CC-HK-029: onFailure =====

#[test]
fn cc_hk_029_on_failure_cases() {
    let cases = [
        (
            "PreToolUse",
            serde_json::json!({"type": "command", "command": "x", "onFailure": "block"}),
            None,
        ),
        (
            "PreToolUse",
            serde_json::json!({"type": "http", "url": "https://x", "onFailure": "continue"}),
            None,
        ),
        (
            "PreToolUse",
            serde_json::json!({"type": "command", "command": "x", "onFailure": "deny"}),
            Some(DiagnosticLevel::Warning),
        ),
        (
            "Stop",
            serde_json::json!({"type": "command", "command": "x", "onFailure": "block"}),
            Some(DiagnosticLevel::Info),
        ),
        (
            "PreToolUse",
            serde_json::json!({"type": "prompt", "prompt": "x", "onFailure": "block"}),
            Some(DiagnosticLevel::Info),
        ),
        (
            "PreToolUse",
            serde_json::json!({"type": "command", "command": "x", "async": true, "onFailure": "block"}),
            Some(DiagnosticLevel::Info),
        ),
    ];
    for (event, hook, expected) in cases {
        let diagnostics = validate(&settings(event, Some("Bash"), hook.clone()));
        let hits = rule(&diagnostics, "CC-HK-029");
        match expected {
            None => assert!(hits.is_empty(), "{hook}: {diagnostics:?}"),
            Some(level) => {
                assert_eq!(hits.len(), 1, "{hook}: {diagnostics:?}");
                assert_eq!(hits[0].level, level, "{hook}");
            }
        }
    }
}

#[test]
fn cc_hk_029_version_pinned_below_2_1_295() {
    let mut config = LintConfig::default();
    config.tool_versions_mut().claude_code = Some("2.1.290".to_string());
    let diagnostics = HooksValidator.validate(
        Path::new("settings.json"),
        &settings(
            "PreToolUse",
            Some("Bash"),
            serde_json::json!({"type": "command", "command": "x", "onFailure": "block"}),
        ),
        &config,
    );
    let hits = rule(&diagnostics, "CC-HK-029");
    assert_eq!(hits.len(), 1, "{diagnostics:?}");
    assert!(hits[0].message.contains("2.1.295"));
}

// ===== CC-HK-030: unquoted placeholders, relative scripts =====

#[test]
fn cc_hk_030_unquoted_placeholder_in_shell_form() {
    let cases = [
        ("${CLAUDE_PLUGIN_ROOT}/scripts/format.sh", None, true),
        ("node $CLAUDE_PROJECT_DIR/.claude/hooks/x.js", None, true),
        ("\"${CLAUDE_PLUGIN_ROOT}\"/scripts/format.sh", None, false),
        ("bash \"${CLAUDE_PLUGIN_ROOT}/hooks/x.sh\"", None, false),
        // Exec form: no shell, so no quoting is needed.
        (
            "${CLAUDE_PLUGIN_ROOT}/scripts/format.sh",
            Some(serde_json::json!([])),
            false,
        ),
    ];
    for (command, args, fires) in cases {
        let mut hook = serde_json::json!({"type": "command", "command": command});
        if let Some(args) = args {
            hook["args"] = args;
        }
        let diagnostics = validate(&settings("PostToolUse", Some("Write"), hook));
        let hits: Vec<_> = rule(&diagnostics, "CC-HK-030")
            .into_iter()
            .filter(|d| d.level == DiagnosticLevel::Warning)
            .collect();
        assert_eq!(!hits.is_empty(), fires, "{command}: {diagnostics:?}");
    }
}

#[test]
fn cc_hk_030_powershell_hooks_are_left_to_cc_hk_032() {
    let diagnostics = validate(&settings(
        "PostToolUse",
        Some("Write"),
        serde_json::json!({
            "type": "command",
            "shell": "powershell",
            "command": "& ${CLAUDE_PROJECT_DIR}\\x.ps1"
        }),
    ));
    assert!(
        rule(&diagnostics, "CC-HK-030").is_empty(),
        "{diagnostics:?}"
    );
}

#[test]
fn cc_hk_030_relative_script_is_info() {
    let diagnostics = validate(&settings(
        "PostToolUse",
        Some("Write"),
        serde_json::json!({"type": "command", "command": "python3 .claude/hooks/check.py"}),
    ));
    let hits = rule(&diagnostics, "CC-HK-030");
    assert_eq!(hits.len(), 1, "{diagnostics:?}");
    assert_eq!(hits[0].level, DiagnosticLevel::Info);
    assert!(hits[0].message.contains(".claude/hooks/check.py"));
}

// ===== CC-HK-031: exec form =====

#[test]
fn cc_hk_031_exec_form_command_with_arguments() {
    let cases = [
        (
            serde_json::json!({"type": "command", "command": "node script.js", "args": ["--fix"]}),
            Some(DiagnosticLevel::Error),
        ),
        (
            serde_json::json!({"type": "command", "command": "C:\\Program Files\\nodejs\\node.exe", "args": ["x.js"]}),
            None,
        ),
        (
            serde_json::json!({"type": "command", "command": "node", "args": ["x.js"]}),
            None,
        ),
        (
            serde_json::json!({"type": "command", "command": "node", "args": ["x.js"], "shell": "bash"}),
            Some(DiagnosticLevel::Info),
        ),
        // Shell form with spaces is normal.
        (
            serde_json::json!({"type": "command", "command": "node script.js"}),
            None,
        ),
    ];
    for (hook, expected) in cases {
        let diagnostics = validate(&settings("PostToolUse", Some("Write"), hook.clone()));
        let hits = rule(&diagnostics, "CC-HK-031");
        match expected {
            None => assert!(hits.is_empty(), "{hook}: {diagnostics:?}"),
            Some(level) => {
                assert_eq!(hits.len(), 1, "{hook}: {diagnostics:?}");
                assert_eq!(hits[0].level, level);
            }
        }
    }
}

// ===== CC-HK-032: PowerShell =====

#[test]
fn cc_hk_032_powershell_placeholder_spelling() {
    for (command, fires) in [
        ("& \"$CLAUDE_PROJECT_DIR\\.claude\\hooks\\check.ps1\"", true),
        ("& '${CLAUDE_PROJECT_DIR}\\.claude\\hooks\\check.ps1'", true),
        (
            "& \"$env:CLAUDE_PROJECT_DIR\\.claude\\hooks\\check.ps1\"",
            false,
        ),
        (
            "& \"${CLAUDE_PROJECT_DIR}\\.claude\\hooks\\check.ps1\"",
            false,
        ),
    ] {
        let diagnostics = validate(&settings(
            "PostToolUse",
            Some("Write"),
            serde_json::json!({"type": "command", "shell": "powershell", "command": command}),
        ));
        let hits = rule(&diagnostics, "CC-HK-032");
        assert_eq!(!hits.is_empty(), fires, "{command}: {diagnostics:?}");
        if fires {
            assert_eq!(hits[0].level, DiagnosticLevel::Error);
        }
    }
}

// ===== CC-HK-033: MCP matcher without wildcard =====

#[test]
fn cc_hk_033_bare_server_matcher() {
    for (matcher, fires) in [
        ("mcp__memory", true),
        ("mcp__brave-search", true),
        ("Bash|mcp__memory", true),
        ("mcp__memory__.*", false),
        ("mcp__memory__create_entities", false),
        ("mcp__.*", false),
    ] {
        let diagnostics = validate(&settings(
            "PreToolUse",
            Some(matcher),
            serde_json::json!({"type": "command", "command": "echo"}),
        ));
        let hits = rule(&diagnostics, "CC-HK-033");
        assert_eq!(!hits.is_empty(), fires, "{matcher}: {diagnostics:?}");
    }
    // Non-tool events (Elicitation matches server names) are not checked.
    let diagnostics = validate(&settings(
        "Elicitation",
        Some("mcp__memory"),
        serde_json::json!({"type": "command", "command": "echo"}),
    ));
    assert!(rule(&diagnostics, "CC-HK-033").is_empty());
}

#[test]
fn cc_hk_033_fix_keeps_json_valid() {
    let content = settings(
        "PreToolUse",
        Some("mcp__memory"),
        serde_json::json!({"type": "command", "command": "echo"}),
    );
    let diagnostics = validate(&content);
    let hit = rule(&diagnostics, "CC-HK-033")[0];
    assert_eq!(hit.fixes.len(), 1);
    let fix = &hit.fixes[0];
    assert!(!fix.safe);
    let mut fixed = content.clone();
    fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
    let value: serde_json::Value = serde_json::from_str(&fixed).unwrap();
    assert_eq!(
        value["hooks"]["PreToolUse"][0]["matcher"],
        serde_json::json!("mcp__memory__.*")
    );
    assert!(rule(&validate(&fixed), "CC-HK-033").is_empty());
    // A multi-alternative matcher gets no fix.
    let content = settings(
        "PreToolUse",
        Some("Bash|mcp__memory"),
        serde_json::json!({"type": "command", "command": "echo"}),
    );
    assert!(rule(&validate(&content), "CC-HK-033")[0].fixes.is_empty());
}

// ===== CC-HK-034: plugin's own MCP server by bare key =====

#[test]
fn cc_hk_034_plugin_hooks_name_own_server_by_bare_key() {
    let temp = TempDir::new().unwrap();
    write(
        temp.path(),
        ".claude-plugin/plugin.json",
        r#"{"name": "my-plugin"}"#,
    );
    write(
        temp.path(),
        ".mcp.json",
        r#"{"mcpServers": {"db": {"command": "node", "args": ["s.js"]}}}"#,
    );
    let hooks = serde_json::json!({
        "hooks": {
            "PreToolUse": [
                {"matcher": "mcp__db__.*", "hooks": [{"type": "command", "command": "echo"}]},
                {"matcher": "mcp__plugin_my-plugin_db__.*", "hooks": [{"type": "command", "command": "echo"}]},
                {"matcher": "mcp__github__.*", "hooks": [{"type": "command", "command": "echo"}]}
            ],
            "PostToolUse": [
                {"matcher": "Write", "hooks": [{"type": "mcp_tool", "server": "db", "tool": "scan"}]},
                {"matcher": "Write", "hooks": [{"type": "mcp_tool", "server": "plugin:my-plugin:db", "tool": "scan"}]}
            ]
        }
    });
    let path = write(
        temp.path(),
        "hooks/hooks.json",
        &serde_json::to_string_pretty(&hooks).unwrap(),
    );
    let diagnostics = validate_file(&path);
    let hits = rule(&diagnostics, "CC-HK-034");
    assert_eq!(hits.len(), 2, "{diagnostics:?}");
    assert!(hits.iter().all(|d| d.level == DiagnosticLevel::Error));
    assert!(hits.iter().any(|d| {
        d.suggestion
            .as_deref()
            .unwrap()
            .contains("mcp__plugin_my-plugin_db__")
    }));
}

#[test]
fn cc_hk_034_settings_files_are_not_checked() {
    // Outside a plugin, `mcp__db__.*` is the right name for a configured server.
    let diagnostics = validate(&settings(
        "PreToolUse",
        Some("mcp__db__.*"),
        serde_json::json!({"type": "command", "command": "echo"}),
    ));
    assert!(rule(&diagnostics, "CC-HK-034").is_empty());
}

// ===== CC-HK-035: broad PermissionRequest matcher =====

#[test]
fn cc_hk_035_permission_request_matchers() {
    for (matcher, fires) in [
        (None, true),
        (Some(""), true),
        (Some("*"), true),
        (Some(".*"), true),
        (Some("Bash"), false),
    ] {
        let diagnostics = validate(&settings(
            "PermissionRequest",
            matcher,
            serde_json::json!({"type": "command", "command": "approve.sh"}),
        ));
        assert_eq!(
            !rule(&diagnostics, "CC-HK-035").is_empty(),
            fires,
            "{matcher:?}: {diagnostics:?}"
        );
        assert!(
            rule(&diagnostics, "CC-HK-003").is_empty(),
            "{diagnostics:?}"
        );
    }
    // Async hooks can't approve, so they're exempt.
    let diagnostics = validate(&settings(
        "PermissionRequest",
        None,
        serde_json::json!({"type": "command", "command": "log.sh", "async": true}),
    ));
    assert!(rule(&diagnostics, "CC-HK-035").is_empty());
}

// ===== Other vendors' hooks/hooks.json =====

#[test]
fn gemini_extension_and_codex_plugin_hooks_are_skipped() {
    let content = r#"{"hooks": {"BeforeTool": [{"hooks": [{"type": "command", "command": "x"}]}], "Interrupt": []}}"#;
    for marker in ["gemini-extension.json", ".codex-plugin/plugin.json"] {
        let temp = TempDir::new().unwrap();
        write(temp.path(), marker, "{}");
        let path = write(temp.path(), "hooks/hooks.json", content);
        assert!(validate_file(&path).is_empty(), "{marker}");
    }
    // A plugin that ships both a Claude and a Codex manifest is still checked.
    let temp = TempDir::new().unwrap();
    write(temp.path(), ".codex-plugin/plugin.json", "{}");
    write(
        temp.path(),
        ".claude-plugin/plugin.json",
        r#"{"name": "p"}"#,
    );
    let path = write(temp.path(), "hooks/hooks.json", content);
    assert!(
        validate_file(&path).iter().any(|d| d.rule == "CC-HK-001"),
        "Claude plugin hooks keep CC-HK-001"
    );
}

// ===== CC-PL-021: plugin root settings.json =====

#[test]
fn cc_pl_021_plugin_settings_keys_are_dropped() {
    let temp = TempDir::new().unwrap();
    write(
        temp.path(),
        ".claude-plugin/plugin.json",
        r#"{"name": "p"}"#,
    );
    let path = write(
        temp.path(),
        "settings.json",
        r#"{"agent": "reviewer", "permissions": {"allow": ["Bash"]}, "hooks": {"Stop": []}}"#,
    );
    let diagnostics = validate_file(&path);
    let hits = rule(&diagnostics, "CC-PL-021");
    assert_eq!(hits.len(), 2, "{diagnostics:?}");
    assert!(diagnostics.iter().all(|d| !d.rule.starts_with("CC-HK")));
}
