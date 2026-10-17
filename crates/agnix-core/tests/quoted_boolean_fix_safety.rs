//! Quoted booleans can be ignored (or interpreted differently) by consumers.
//! Exact token conversion is available, but must never activate behavior in safe mode.

use agnix_core::{
    FileSystem, FileType, FixApplyMode, FixApplyOptions, LintConfig, MockFileSystem,
    ValidatorRegistry, apply_fixes_with_fs_options, detect_file_type, validate_content,
};
use std::{path::Path, sync::Arc};

fn assert_requires_opt_in(path: &Path, content: &str, expected: &str, rule: &str) {
    let config = LintConfig::default();
    let registry = ValidatorRegistry::with_defaults();
    let diagnostics: Vec<_> = validate_content(path, content, &config, &registry)
        .into_iter()
        .filter(|d| d.rule == rule)
        .collect();
    assert_eq!(diagnostics.len(), 1, "{rule}: {content}");
    assert_eq!(diagnostics[0].fixes.len(), 1, "{rule}: fix remains offered");
    assert!(!diagnostics[0].fixes[0].safe, "{rule}: requires opt-in");
    assert!(
        diagnostics[0].fixes[0]
            .description
            .contains("may change behavior or security policy"),
        "{rule}: the localized description must warn about semantic changes"
    );

    let fs = Arc::new(MockFileSystem::new());
    fs.add_file(path, content);
    let results = apply_fixes_with_fs_options(
        &diagnostics,
        FixApplyOptions::new(false, FixApplyMode::SafeOnly),
        Some(fs.clone()),
    )
    .unwrap();
    assert!(results.is_empty(), "{rule}: plain --fix must not convert");
    assert_eq!(fs.read_to_string(path).unwrap(), content, "{rule}");
    assert!(
        validate_content(path, &fs.read_to_string(path).unwrap(), &config, &registry)
            .iter()
            .any(|d| d.rule == rule),
        "{rule}: safe mode must leave the diagnostic unresolved"
    );

    let results = apply_fixes_with_fs_options(
        &diagnostics,
        FixApplyOptions::new(false, FixApplyMode::All),
        Some(fs.clone()),
    )
    .unwrap();
    assert_eq!(results.len(), 1, "{rule}");
    assert_eq!(results[0].applied.len(), 1, "{rule}");
    // The engine normalizes CRLF only when applying the opted-in fix.
    let fixed = fs.read_to_string(path).unwrap();
    assert_eq!(fixed, expected.replace("\r\n", "\n"), "{rule}");
    assert!(
        !validate_content(path, &fixed, &config, &registry)
            .iter()
            .any(|d| d.rule == rule),
        "{rule}: unsafe conversion must resolve the type diagnostic"
    );
}

#[test]
fn all_settings_quoted_booleans_require_opt_in() {
    let cases = [
        ("CC-SET-002", "/channelsEnabled"),
        ("CC-SET-006", "/disableBundledSkills"),
        ("CC-SET-007", "/enforceAvailableModels"),
        ("CC-SET-008", "/sandbox/allowAppleEvents"),
        ("CC-SET-009", "/attribution/sessionUrl"),
        ("CC-SET-011", "/respondToBashCommands"),
        ("CC-SET-013", "/autoMode/classifyAllShell"),
        ("CC-SET-017", "/sandbox/filesystem/disabled"),
        ("CC-SET-018", "/emojiCompletionEnabled"),
        ("CC-SET-019", "/sandbox/network/strictAllowlist"),
        ("CC-SET-033", "/idleCompaction"),
    ];
    for (rule, pointer) in cases {
        for quoted in ["true", "false"] {
            let mut value = serde_json::Value::String(quoted.into());
            for key in pointer
                .split('/')
                .skip(1)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
            {
                value = serde_json::json!({key: value});
            }
            // Unicode and quoted decoys must not move or become the target span.
            value["note"] = serde_json::json!(format!("é decoy: \"{quoted}\""));
            let content = serde_json::to_string_pretty(&value).unwrap();
            *value.pointer_mut(pointer).unwrap() = serde_json::Value::Bool(quoted == "true");
            let expected = serde_json::to_string_pretty(&value).unwrap();
            // Several of these keys are read only from user or managed
            // settings; in .claude/settings.json CC-SET-034 reports them and
            // the type check does not run. The managed file reads them all.
            for newline in ["\n", "\r\n"] {
                assert_requires_opt_in(
                    Path::new(".claude/managed-settings.json"),
                    &content.replace('\n', newline),
                    &expected.replace('\n', newline),
                    rule,
                );
            }
        }
    }
}

#[test]
fn hook_quoted_booleans_require_opt_in_and_target_only_the_intended_hook() {
    for quoted in ["true", "false"] {
        let content = format!(
            r#"{{"hooks":{{"PreToolUse":[{{"matcher":"Bash","hooks":[{{"type":"command","command":"echo é","once":true}},{{"type":"command","command":"echo second","once":"{quoted}"}}]}}]}}}}"#
        );
        let expected = content.replace(
            &format!("\"once\":\"{quoted}\""),
            &format!("\"once\":{quoted}"),
        );
        assert_requires_opt_in(
            Path::new(".claude/settings.json"),
            &content,
            &expected,
            "CC-HK-023",
        );
        let fixed: serde_json::Value = serde_json::from_str(&expected).unwrap();
        assert_eq!(fixed["hooks"]["PreToolUse"][0]["hooks"][0]["once"], true);
        assert_eq!(
            fixed["hooks"]["PreToolUse"][0]["hooks"][1]["once"],
            quoted == "true"
        );
    }
}

#[test]
fn mcp_quoted_booleans_require_opt_in_and_target_only_the_intended_server() {
    for quoted in ["true", "false"] {
        let content = format!(
            r#"{{"mcpServers":{{"first":{{"command":"node","args":["é"],"alwaysLoad":true}},"second":{{"command":"node","alwaysLoad":"{quoted}"}}}}}}"#
        );
        let expected = content.replace(
            &format!("\"alwaysLoad\":\"{quoted}\""),
            &format!("\"alwaysLoad\":{quoted}"),
        );
        assert_requires_opt_in(Path::new(".mcp.json"), &content, &expected, "CC-MCP-004");
        let fixed: serde_json::Value = serde_json::from_str(&expected).unwrap();
        assert_eq!(fixed["mcpServers"]["first"]["alwaysLoad"], true);
        assert_eq!(
            fixed["mcpServers"]["second"]["alwaysLoad"],
            quoted == "true"
        );
    }
}

#[test]
fn output_style_quoted_booleans_require_opt_in_and_preserve_comments_and_body() {
    for quoted in ["true", "false"] {
        for quote in ['\'', '"'] {
            let content = format!(
                "---\nname: é\ndescription: style\nkeep-coding-instructions: {quote}{quoted}{quote} # keep\n---\nBody with {quote}{quoted}{quote}\n"
            );
            let expected = content.replacen(
                &format!("keep-coding-instructions: {quote}{quoted}{quote}"),
                &format!("keep-coding-instructions: {quoted}"),
                1,
            );
            for newline in ["\n", "\r\n"] {
                assert_requires_opt_in(
                    Path::new(".claude/output-styles/test.md"),
                    &content.replace('\n', newline),
                    &expected.replace('\n', newline),
                    "CC-OS-002",
                );
            }
        }
    }
}

#[test]
fn kiro_settings_quoted_booleans_require_opt_in_and_target_the_top_level_key() {
    // Kiro CLI settings live in .kiro/settings/cli.json; older agnix builds
    // routed .kiro/settings.json to the same validator.
    let path = [".kiro/settings/cli.json", ".kiro/settings.json"]
        .into_iter()
        .map(Path::new)
        .find(|path| detect_file_type(path) == FileType::KiroSettings)
        .expect("a Kiro CLI settings path is detected");
    for quoted in ["true", "false"] {
        for (rule, key) in [
            ("KR-SET-001", "toolSearch.enabled"),
            ("KR-SET-008", "chat.enableKnowledge"),
        ] {
            // A nested copy of the key and a quoted decoy must not be edited.
            let content = format!(
                "{{\n  \"profiles\": {{\"{key}\": \"{quoted}\"}},\n  \"chat.defaultAgent\": \"é \\\"{quoted}\\\"\",\n  \"{key}\": \"{quoted}\"\n}}"
            );
            let expected = format!(
                "{{\n  \"profiles\": {{\"{key}\": \"{quoted}\"}},\n  \"chat.defaultAgent\": \"é \\\"{quoted}\\\"\",\n  \"{key}\": {quoted}\n}}"
            );
            for newline in ["\n", "\r\n"] {
                assert_requires_opt_in(
                    path,
                    &content.replace('\n', newline),
                    &expected.replace('\n', newline),
                    rule,
                );
            }
            let fixed: serde_json::Value = serde_json::from_str(&expected).unwrap();
            assert_eq!(fixed[key], quoted == "true");
            assert_eq!(fixed["profiles"][key], quoted);
        }
    }
}
