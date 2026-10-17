use super::*;
use crate::config::LintConfig;
use crate::fs::MockFileSystem;
use std::sync::Arc;

const SETTINGS: &str = "/proj/.gemini/settings.json";
const EXTENSION_HOOKS: &str = "/ext/hooks/hooks.json";

fn config_with(files: &[&str]) -> LintConfig {
    let fs = MockFileSystem::new();
    for file in files {
        fs.add_file(file, "#!/bin/sh\n");
    }
    let mut config = LintConfig::default();
    config.set_fs(Arc::new(fs));
    config
}

fn run(path: &str, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
    GeminiHooksValidator.validate(Path::new(path), content, config)
}

fn settings(content: &str) -> Vec<Diagnostic> {
    run(SETTINGS, content, &config_with(&[]))
}

fn ids(diagnostics: &[Diagnostic]) -> Vec<&str> {
    diagnostics.iter().map(|d| d.rule.as_str()).collect()
}

/// `{"hooks": {"<event>": [<group>]}}`
fn one_group(event: &str, group: &str) -> String {
    format!(r#"{{"hooks": {{"{event}": [{group}]}}}}"#)
}

// ===== GM-004: structure =====

#[test]
fn documented_settings_hooks_are_clean() {
    // The configuration-schema example of geminicli.com/docs/hooks.
    let content = r#"{
  "hooksConfig": {"enabled": true, "disabled": ["slow-hook"], "notifications": false},
  "hooks": {
    "BeforeTool": [
      {
        "matcher": "write_file|replace",
        "sequential": true,
        "hooks": [
          {
            "name": "security-check",
            "type": "command",
            "command": "echo check",
            "timeout": 5000,
            "description": "Scans code changes"
          }
        ]
      }
    ],
    "SessionStart": [{"matcher": "startup", "hooks": [{"type": "command", "command": "echo hi"}]}],
    "Notification": [{"hooks": [{"type": "command", "command": "echo n"}]}],
    "PreCompress": []
  }
}"#;
    let diagnostics = settings(content);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn hooks_config_used_as_an_event_map_is_reported() {
    // agnix 0.58 read hooksConfig as `{Event: [{type, command}]}` and warned
    // on the documented `enabled` / `disabled` keys instead.
    let content = r#"{"hooksConfig": {"BeforeAgent": [{"type": "command", "command": "echo"}], "PreToolUse": [], "verbose": true}}"#;
    let diagnostics = settings(content);
    assert_eq!(ids(&diagnostics), ["GM-004", "GM-004", "GM-004"]);
    assert!(
        diagnostics[0]
            .message
            .contains("belong under the top-level \"hooks\"")
    );
    assert!(diagnostics[1].message.contains("hooksConfig.PreToolUse"));
    assert!(
        diagnostics[2]
            .message
            .contains("Unknown hooksConfig key 'verbose'")
    );
    // Points at the key: `"BeforeAgent"` starts at column 18.
    assert_eq!((diagnostics[0].line, diagnostics[0].column), (1, 18));
    assert!(
        diagnostics
            .iter()
            .all(|d| d.level == DiagnosticLevel::Warning)
    );
}

#[test]
fn hooks_config_value_types() {
    let content =
        r#"{"hooksConfig": {"enabled": "yes", "notifications": 1, "disabled": ["a", 2]}}"#;
    let diagnostics = settings(content);
    assert_eq!(ids(&diagnostics), ["GM-004", "GM-004", "GM-004"]);
    assert!(
        diagnostics[0]
            .message
            .contains("hooksConfig.enabled must be a boolean")
    );
    assert!(diagnostics[2].message.contains("hooksConfig.disabled[1]"));
    let not_object = settings(r#"{"hooksConfig": []}"#);
    assert_eq!(ids(&not_object), ["GM-004"]);
}

#[test]
fn unknown_events_get_hints() {
    let content =
        r#"{"hooks": {"PreToolUse": [], "beforetool": [], "Whatever": [], "enabled": true}}"#;
    let diagnostics = settings(content);
    assert_eq!(ids(&diagnostics), ["GM-004"; 4]);
    assert!(diagnostics[0].message.contains("did you mean 'BeforeTool'"));
    assert!(diagnostics[1].message.contains("did you mean 'BeforeTool'"));
    assert!(diagnostics[2].message.contains("fires only SessionStart"));
    assert!(
        diagnostics[3]
            .message
            .contains("hooksConfig.enabled setting")
    );
}

#[test]
fn group_and_hook_shapes() {
    for (group, needle) in [
        (r#""echo""#, "must be a hook group object"),
        (r#"{"matcher": "x"}"#, "has no 'hooks' array"),
        (
            r#"{"type": "command", "command": "echo"}"#,
            "has 'command' but no 'hooks' array",
        ),
        (r#"{"matcher": 5, "hooks": []}"#, "matcher must be a string"),
        (
            r#"{"sequential": "yes", "hooks": []}"#,
            "sequential must be a boolean",
        ),
        (r#"{"hooks": {}}"#, "hooks must be an array"),
        (r#"{"hooks": ["echo"]}"#, "must be a hook object"),
        (
            r#"{"hooks": [{"command": "echo"}]}"#,
            "missing required field 'type'",
        ),
        (
            r#"{"hooks": [{"type": "prompt", "command": "echo"}]}"#,
            "supports only \"command\"",
        ),
        (
            r#"{"hooks": [{"type": 1, "command": "echo"}]}"#,
            "type must be a string",
        ),
        (
            r#"{"hooks": [{"type": "command"}]}"#,
            "missing required field 'command'",
        ),
        (
            r#"{"hooks": [{"type": "command", "command": " "}]}"#,
            "command is empty",
        ),
        (
            r#"{"hooks": [{"type": "command", "command": ["a"]}]}"#,
            "command must be a string",
        ),
        (
            r#"{"hooks": [{"type": "command", "command": "x", "name": 1}]}"#,
            "name must be a string",
        ),
        (
            r#"{"hooks": [{"type": "command", "command": "x", "timeout": "5000"}]}"#,
            "timeout must be a number",
        ),
        (
            r#"{"hooks": [{"type": "command", "command": "x", "timeout": 0}]}"#,
            "positive number",
        ),
        (
            r#"{"hooks": [{"type": "command", "command": "x", "timeout": -5}]}"#,
            "positive number",
        ),
    ] {
        let diagnostics = settings(&one_group("SessionStart", group));
        assert_eq!(ids(&diagnostics), ["GM-004"], "{group}: {diagnostics:?}");
        assert!(
            diagnostics[0].message.contains(needle),
            "{group}: {}",
            diagnostics[0].message
        );
    }
    let not_array = settings(r#"{"hooks": {"SessionStart": {}}}"#);
    assert!(
        not_array[0]
            .message
            .contains("must be an array of hook groups")
    );
    let not_object = settings(r#"{"hooks": []}"#);
    assert!(not_object[0].message.contains("'hooks' must be an object"));
}

#[test]
fn unknown_hook_keys_are_allowed() {
    // hookRunner.ts reads `env` from a hook config; keys beyond the
    // documented ones are not reported.
    let content = one_group(
        "SessionStart",
        r#"{"hooks": [{"type": "command", "command": "echo", "env": {"A": "1"}}]}"#,
    );
    assert!(settings(&content).is_empty());
}

#[test]
fn settings_parse_errors_belong_to_gm_009() {
    assert!(settings("{ invalid").is_empty());
}

#[test]
fn jsonc_settings_keep_exact_positions() {
    let content = "{\n  // comment\n  \"hooks\": {\"PreToolUse\": []}\n}";
    let diagnostics = settings(content);
    assert_eq!(ids(&diagnostics), ["GM-004"]);
    assert_eq!((diagnostics[0].line, diagnostics[0].column), (3, 13));
}

// ===== GM-004: extension hooks/hooks.json =====

fn extension(content: &str) -> Vec<Diagnostic> {
    run(EXTENSION_HOOKS, content, &config_with(&[]))
}

#[test]
fn extension_hooks_file_shape() {
    assert!(extension(r#"{"hooks": {"SessionStart": []}}"#).is_empty());
    for content in ["{}", "[]", r#"{"SessionStart": []}"#] {
        let diagnostics = extension(content);
        assert_eq!(ids(&diagnostics), ["GM-004"], "{content}");
        assert!(diagnostics[0].message.contains("has no \"hooks\" object"));
    }
    let diagnostics = extension(r#"{"hooks": []}"#);
    assert!(diagnostics[0].message.contains("'hooks' must be an object"));
}

#[test]
fn extension_hooks_are_strict_json() {
    // loadExtensionHooks uses JSON.parse; a comment makes Gemini skip the file.
    let diagnostics = extension("{\n  // note\n  \"hooks\": {}\n}");
    assert_eq!(ids(&diagnostics), ["GM-004"]);
    assert_eq!(diagnostics[0].line, 2);
    assert!(diagnostics[0].message.contains("not valid JSON"));
}

#[test]
fn extension_hooks_do_not_check_hooks_config() {
    // hooksConfig is a settings.json key; extension files only have `hooks`.
    let content = r#"{"hooks": {}, "hooksConfig": {"BeforeTool": []}}"#;
    assert!(extension(content).is_empty());
}

// ===== GM-018 =====

#[test]
fn gm_018_seconds_like_timeouts() {
    for (value, shown, suggested) in [
        ("30", "30", "30000"),
        ("1", "1", "1000"),
        ("999", "999", "999000"),
        ("1.5", "1.5", "1500"),
    ] {
        let content = one_group(
            "SessionStart",
            &format!(r#"{{"hooks": [{{"type": "command", "command": "x", "timeout": {value}}}]}}"#),
        );
        for path in [SETTINGS, EXTENSION_HOOKS] {
            let diagnostics = run(path, &content, &config_with(&[]));
            assert_eq!(ids(&diagnostics), ["GM-018"], "{value} {path}");
            assert_eq!(diagnostics[0].level, DiagnosticLevel::Warning);
            assert_eq!(
                diagnostics[0].message,
                format!(
                    "Gemini hook timeouts are milliseconds (default 60000); {shown} means {shown} ms. Did you mean {suggested}?"
                )
            );
        }
    }
    for value in ["1000", "5000", "60000", "0.5"] {
        let content = one_group(
            "SessionStart",
            &format!(r#"{{"hooks": [{{"type": "command", "command": "x", "timeout": {value}}}]}}"#),
        );
        assert!(!ids(&settings(&content)).contains(&"GM-018"), "{value}");
    }
}

#[test]
fn gm_018_points_at_the_timeout_value() {
    let content = "{\"hooks\": {\"SessionStart\": [{\"hooks\": [\n  {\"type\": \"command\", \"command\": \"x\", \"timeout\": 30}]}]}}";
    let diagnostics = settings(content);
    assert_eq!((diagnostics[0].line, diagnostics[0].column), (2, 50));
}

// ===== GM-026 =====

#[test]
fn gm_026_catch_all_tool_matchers() {
    for group in [
        r#"{"hooks": [{"type": "command", "command": "x"}]}"#,
        r#"{"matcher": "*", "hooks": [{"type": "command", "command": "x"}]}"#,
        r#"{"matcher": "", "hooks": [{"type": "command", "command": "x"}]}"#,
        r#"{"matcher": " * ", "hooks": [{"type": "command", "command": "x"}]}"#,
        r#"{"matcher": ".*", "hooks": [{"type": "command", "command": "x"}]}"#,
    ] {
        for event in TOOL_EVENTS {
            let diagnostics = settings(&one_group(event, group));
            assert_eq!(ids(&diagnostics), ["GM-026"], "{event} {group}");
            assert_eq!(diagnostics[0].level, DiagnosticLevel::Info);
            assert!(
                diagnostics[0]
                    .message
                    .contains("specify only the tools you need")
            );
        }
        // Lifecycle matchers are exact strings; no advice there.
        assert!(
            settings(&one_group("SessionStart", group)).is_empty(),
            "{group}"
        );
    }
    let specific = one_group(
        "BeforeTool",
        r#"{"matcher": "write_file|replace", "hooks": [{"type": "command", "command": "x"}]}"#,
    );
    assert!(settings(&specific).is_empty());
}

#[test]
fn gm_026_invalid_tool_regex_falls_back_to_exact_name() {
    // `*` is special-cased by hookPlanner.ts before `new RegExp`, so it is
    // not an invalid regex; `write_file(` is, and Gemini then compares it
    // with the tool name as plain text.
    let content = one_group(
        "AfterTool",
        r#"{"matcher": "write_file(", "hooks": [{"type": "command", "command": "x"}]}"#,
    );
    let diagnostics = settings(&content);
    assert_eq!(ids(&diagnostics), ["GM-026"]);
    assert!(
        diagnostics[0]
            .message
            .contains("not a valid regular expression")
    );
    assert!(
        diagnostics[0]
            .message
            .contains("named exactly 'write_file('")
    );
    // Not a GM-004 problem: the hook still loads.
    let lifecycle = one_group(
        "SessionStart",
        r#"{"matcher": "write_file(", "hooks": [{"type": "command", "command": "x"}]}"#,
    );
    assert!(settings(&lifecycle).is_empty());
}

// ===== GM-019 =====

fn hook_command(command: &str) -> String {
    let command = serde_json::to_string(command).unwrap();
    one_group(
        "SessionStart",
        &format!(r#"{{"hooks": [{{"type": "command", "command": {command}}}]}}"#),
    )
}

#[test]
fn gm_019_project_settings_paths() {
    let config = config_with(&[
        "/proj/.gemini/hooks/check.sh",
        "/proj/scripts/run.py",
        "/proj/hook.js",
    ]);
    let clean = [
        "$GEMINI_PROJECT_DIR/.gemini/hooks/check.sh",
        "\"$GEMINI_PROJECT_DIR\"/.gemini/hooks/check.sh",
        "${GEMINI_PROJECT_DIR}/.gemini/hooks/check.sh --fast",
        "$CLAUDE_PROJECT_DIR/.gemini/hooks/check.sh",
        "${CLAUDE_PROJECT_DIR}/.gemini/hooks/check.sh",
        ".gemini/hooks/check.sh",
        "./.gemini/hooks/../hooks/check.sh",
        "python3 scripts/run.py --flag",
        "bash -x .gemini/hooks/check.sh",
        "node hook.js",
        "/proj/hook.js",
        // Not checked: no path, PATH lookups, inline code, other variables,
        // home paths, system paths, assignments, dependencies.
        "echo hi",
        "npx eslint .",
        "bash -c './missing.sh'",
        "node -e 'require(\"./missing.js\")'",
        "python3 -m tools.missing",
        "$GEMINI_PLANS_DIR/missing.sh",
        "$HOME/missing.sh",
        "~/missing.sh",
        "/usr/local/bin/missing",
        "FOO=1 ./missing.sh",
        "node_modules/.bin/missing",
        "./scripts/*.sh",
    ];
    for command in clean {
        let diagnostics = run(SETTINGS, &hook_command(command), &config);
        assert!(diagnostics.is_empty(), "{command}: {diagnostics:?}");
    }
    // (command, file name in the resolved path). Paths are compared by file
    // name so the test also passes with Windows separators.
    let mut missing = vec![
        ("$GEMINI_PROJECT_DIR/.gemini/hooks/missing.sh", "missing.sh"),
        ("${GEMINI_PROJECT_DIR}/hooks/missing.sh", "missing.sh"),
        ("$CLAUDE_PROJECT_DIR/missing.sh", "missing.sh"),
        ("./missing.sh arg", "missing.sh"),
        (
            "bash \"$GEMINI_PROJECT_DIR/scripts/missing.sh\"",
            "missing.sh",
        ),
        ("python3 -W ignore scripts/missing.py", "missing.py"),
        ("node --require ./setup.js scripts/missing.js", "missing.js"),
        ("./missing.sh && echo done", "missing.sh"),
    ];
    if cfg!(unix) {
        // A literal absolute path inside the (absolute) project root.
        missing.push(("/proj/scripts/missing.sh", "missing.sh"));
    }
    for (command, file) in missing {
        let diagnostics = run(SETTINGS, &hook_command(command), &config);
        assert_eq!(ids(&diagnostics), ["GM-019"], "{command}");
        assert_eq!(diagnostics[0].level, DiagnosticLevel::Warning);
        let resolved = diagnostics[0]
            .message
            .split("resolved to ")
            .nth(1)
            .unwrap_or_default();
        assert!(
            resolved.contains(file) && resolved.contains("proj"),
            "{command}: {}",
            diagnostics[0].message
        );
    }
}

#[test]
fn gm_019_extension_hooks_use_extension_path() {
    let config = config_with(&["/ext/scripts/start.sh", "/repo/scripts/up.sh"]);
    for command in [
        "${extensionPath}/scripts/start.sh",
        "${extensionPath}${/}scripts${/}start.sh",
        "bash \"${extensionPath}/scripts/start.sh\"",
        // Relative paths run in the open project, not the extension.
        "./scripts/missing.sh",
        "$GEMINI_PROJECT_DIR/missing.sh",
        "${workspacePath}/missing.sh",
    ] {
        let diagnostics = run(EXTENSION_HOOKS, &hook_command(command), &config);
        assert!(diagnostics.is_empty(), "{command}: {diagnostics:?}");
    }
    let diagnostics = run(
        EXTENSION_HOOKS,
        &hook_command("bash \"${extensionPath}/../scripts/missing.sh\""),
        &config,
    );
    assert_eq!(ids(&diagnostics), ["GM-019"]);
    assert!(diagnostics[0].message.contains("/scripts/missing.sh"));
}

#[cfg(feature = "filesystem")]
#[test]
fn gm_019_user_settings_skip_project_variables() {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let path = home.join(".gemini").join("settings.json");
    let content = hook_command("$GEMINI_PROJECT_DIR/.gemini/hooks/missing.sh");
    let diagnostics = GeminiHooksValidator.validate(&path, &content, &config_with(&[]));
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let relative = hook_command("./missing.sh");
    assert!(
        GeminiHooksValidator
            .validate(&path, &relative, &config_with(&[]))
            .is_empty()
    );
}

#[cfg(unix)]
#[test]
fn gm_019_reports_missing_exec_bit_for_the_command_only() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path();
    std::fs::create_dir_all(project.join(".gemini/hooks")).unwrap();
    let script = project.join(".gemini/hooks/check.sh");
    std::fs::write(&script, "#!/bin/sh\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o644)).unwrap();
    let settings_path = project.join(".gemini/settings.json");
    let config = LintConfig::default();

    let direct = hook_command("$GEMINI_PROJECT_DIR/.gemini/hooks/check.sh");
    let diagnostics = GeminiHooksValidator.validate(&settings_path, &direct, &config);
    assert_eq!(ids(&diagnostics), ["GM-019"]);
    assert!(diagnostics[0].message.contains("not executable"));

    // `bash script.sh` does not need the bit.
    let via_bash = hook_command("bash $GEMINI_PROJECT_DIR/.gemini/hooks/check.sh");
    assert!(
        GeminiHooksValidator
            .validate(&settings_path, &via_bash, &config)
            .is_empty()
    );

    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        GeminiHooksValidator
            .validate(&settings_path, &direct, &config)
            .is_empty()
    );
}

// ===== Helpers =====

#[test]
fn shell_words_split_like_a_shell() {
    let words = shell_words(r#"bash "${extensionPath}/a b.sh" 'x y' c\ d; echo # rest"#);
    let texts: Vec<_> = words
        .iter()
        .map(|w| (w.text.as_str(), w.operator))
        .collect();
    assert_eq!(
        texts,
        [
            ("bash", false),
            ("${extensionPath}/a b.sh", false),
            ("x y", false),
            ("c d", false),
            (";", true),
            ("echo", false),
        ]
    );
}

#[test]
fn normalize_resolves_dots_lexically() {
    assert_eq!(
        normalize(Path::new("/a/b/../c/./d")),
        PathBuf::from("/a/c/d")
    );
    assert_eq!(normalize(Path::new("a/../../b")), PathBuf::from("../b"));
    assert_eq!(normalize(Path::new("/../a")), PathBuf::from("/a"));
}

#[test]
fn rules_can_be_disabled() {
    let mut config = config_with(&[]);
    config.rules_mut().disabled_rules = vec![
        "GM-004".to_string(),
        "GM-018".to_string(),
        "GM-019".to_string(),
        "GM-026".to_string(),
    ];
    let content = r#"{"hooks": {"PreToolUse": [], "BeforeTool": [{"hooks": [{"type": "command", "command": "./missing.sh", "timeout": 5}]}]}}"#;
    assert!(run(SETTINGS, content, &config).is_empty());
    let mut category_off = config_with(&[]);
    category_off.rules_mut().gemini_md = false;
    assert!(run(SETTINGS, content, &category_off).is_empty());
    let all = run(SETTINGS, content, &config_with(&[]));
    let mut found = ids(&all);
    found.sort_unstable();
    assert_eq!(found, ["GM-004", "GM-018", "GM-019", "GM-026"]);
}
