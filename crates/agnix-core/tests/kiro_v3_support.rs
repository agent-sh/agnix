//! Kiro V3 policy dispatch, precise fix roundtrips, and legacy loading advice.
use agnix_core::{
    Diagnostic, DiagnosticLevel, FileSystem, FileType, LintConfig, MockFileSystem,
    ValidatorRegistry, apply_fixes_with_fs, detect_file_type,
};
use std::{path::Path, sync::Arc};

fn validate(path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
    let content = agnix_core::normalize_line_endings(content);
    ValidatorRegistry::with_defaults()
        .validators_for(detect_file_type(path))
        .iter()
        .flat_map(|v| v.validate(path, &content, config))
        .collect()
}

#[test]
fn kiro_permissions_fixtures_and_category_switch() {
    let path = Path::new(".kiro/settings/permissions.yaml");
    let good = include_str!(
        "../../../tests/fixtures/kiro-permissions/valid/.kiro/settings/permissions.yaml"
    );
    let bad = include_str!(
        "../../../tests/fixtures/kiro-permissions/invalid/.kiro/settings/permissions.yaml"
    );
    let mut config = LintConfig::default();
    assert!(validate(path, good, &config).is_empty());
    let diagnostics = validate(path, bad, &config);
    for id in ["KR-PM-002", "KR-PM-003", "KR-PM-004"] {
        assert!(diagnostics.iter().any(|d| d.rule == id));
    }
    config.rules_mut().kiro_steering = false;
    assert!(validate(path, bad, &config).is_empty());
    assert!(validate(Path::new(".kiro/hooks/aliases.json"), r#"{"version":"1","hooks":[{"name":"a","trigger":"agentSpawn","action":{"type":"command","command":"echo"}}]}"#, &config).is_empty());
}

#[test]
fn kiro_permissions_detection_and_dispatch() {
    for name in [
        ".kiro/settings/permissions.yaml",
        "/home/me/.kiro/settings/permissions.yaml",
        "/home/me/.kiro/workspace-roots/abc/permissions.yaml",
    ] {
        let path = Path::new(name);
        assert_eq!(detect_file_type(path), FileType::KiroPermissions);
        let diagnostics = validate(
            path,
            "rules: [{capability: nope, effect: nope, match: false}]",
            &LintConfig::default(),
        );
        for id in ["KR-PM-002", "KR-PM-003", "KR-PM-004"] {
            assert!(diagnostics.iter().any(|d| d.rule == id), "{diagnostics:?}");
        }
        assert!(
            validate(
                path,
                "rules: [{capability: fs_read, effect: allow}]",
                &LintConfig::default()
            )
            .is_empty()
        );
    }
    for name in [
        "permissions.yaml",
        "settings/permissions.yaml",
        ".kiro/permissions.yaml",
        ".kiro/settings/nested/permissions.yaml",
        ".kiro/workspace-roots/permissions.yaml",
        ".kiro/settings/permissions.yml",
    ] {
        assert_eq!(
            detect_file_type(Path::new(name)),
            FileType::Unknown,
            "{name}"
        );
    }
}

#[test]
fn kiro_version_fix_exact_span_and_unsafe_roundtrip() {
    let path = Path::new(".kiro/hooks/version.json");
    for version in ["1", "1.0", "\\u0031"] {
        let content = format!(
            "{{\r\n  \"nested\": {{\"version\": \"1\"}}, \"note\": \"中文\",\r\n  \"version\": \"{version}\",\r\n  \"hooks\": []\r\n}}"
        );
        let diagnostics = validate(path, &content, &LintConfig::default());
        let diag = diagnostics.iter().find(|d| d.rule == "KR-HK-011").unwrap();
        assert_eq!(diag.line, 3);
        assert_eq!(diag.fixes.len(), 1, "{diag:?}");
        let fix = &diag.fixes[0];
        assert!(!fix.is_safe());
        assert_eq!(
            &agnix_core::normalize_line_endings(&content)[fix.start_byte..fix.end_byte],
            format!("\"{version}\"")
        );
        let fs = Arc::new(MockFileSystem::new());
        fs.add_file(path, &content);
        apply_fixes_with_fs(&diagnostics, false, true, Some(fs.clone())).unwrap();
        assert_eq!(
            fs.read_to_string(path).unwrap(),
            content,
            "safe-only must not activate invalid hook"
        );
        apply_fixes_with_fs(&diagnostics, false, false, Some(fs.clone())).unwrap();
        let fixed = fs.read_to_string(path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&fixed).unwrap();
        assert_eq!(value["version"], "v1");
        assert_eq!(value["nested"]["version"], "1");
        assert!(
            validate(path, &fixed, &LintConfig::default())
                .iter()
                .all(|d| d.rule != "KR-HK-011")
        );
    }
}

#[test]
fn kiro_unknown_missing_numeric_and_duplicate_versions_never_fixed() {
    let path = Path::new(".kiro/hooks/version.json");
    for content in [
        r#"{"hooks":[]}"#,
        r#"{"version":"v2","hooks":[]}"#,
        r#"{"version":1,"hooks":[]}"#,
        r#"{"version":"1","version":"1.0","hooks":[]}"#,
    ] {
        let diagnostics = validate(path, content, &LintConfig::default());
        assert!(!diagnostics.is_empty());
        assert!(
            diagnostics.iter().all(|d| d.fixes.is_empty()),
            "{diagnostics:?}"
        );
    }
}

#[test]
fn kiro_alias_fixes_each_entry_without_touching_payloads() {
    let path = Path::new(".kiro/hooks/aliases.json");
    let pairs = [
        ("agentSpawn", "SessionStart"),
        ("AgentSpawn", "SessionStart"),
        ("userPromptSubmit", "UserPromptSubmit"),
        ("preToolUse", "PreToolUse"),
        ("postToolUse", "PostToolUse"),
        ("stop", "Stop"),
        ("agentSpawn", "SessionStart"),
    ];
    let hooks: Vec<_> = pairs.iter().enumerate().map(|(i, (old, _))| serde_json::json!({"name":format!("hook-{i}"), "trigger":old, "action":{"type":"command", "command":old, "trigger":old}})).collect();
    let content =
        serde_json::json!({"version":"v1", "hooks":hooks, "trigger":"agentSpawn"}).to_string();
    let diagnostics = validate(path, &content, &LintConfig::default());
    assert!(diagnostics.iter().all(|d| d.rule != "KR-HK-012"));
    let hints: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "KR-HK-014")
        .collect();
    assert_eq!(hints.len(), pairs.len());
    assert!(
        hints.iter().all(|d| d.level == DiagnosticLevel::Info
            && d.fixes.len() == 1
            && d.fixes[0].is_safe())
    );
    let fs = Arc::new(MockFileSystem::new());
    fs.add_file(path, &content);
    apply_fixes_with_fs(&diagnostics, false, true, Some(fs.clone())).unwrap();
    let fixed = fs.read_to_string(path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&fixed).unwrap();
    for (i, (old, canonical)) in pairs.iter().enumerate() {
        assert_eq!(value["hooks"][i]["trigger"], *canonical);
        assert_eq!(value["hooks"][i]["action"]["command"], *old);
        assert_eq!(value["hooks"][i]["action"]["trigger"], *old);
    }
    assert_eq!(value["trigger"], "agentSpawn");
    assert!(validate(path, &fixed, &LintConfig::default()).is_empty());
}

#[test]
fn kiro_alias_escaped_keys_and_values_have_exact_safe_fixes() {
    let path = Path::new(".kiro/hooks/escaped.json");
    let content = r#"{
      "note": "é🙂",
      "ho\u006fks": [{"name":"é🙂", "trig\u0067er":"agent\u0053pawn", "action":{"type":"command","command":"agentSpawn", "trigger":"stop"}}],
      "ver\u0073ion": "1"
    }"#;
    let diagnostics = validate(path, content, &LintConfig::default());
    assert_eq!(diagnostics.len(), 2);
    let hint = diagnostics.iter().find(|d| d.rule == "KR-HK-014").unwrap();
    assert_eq!(hint.line, 3);
    assert_eq!(hint.fixes.len(), 1);
    let fix = &hint.fixes[0];
    assert_eq!(
        &content[fix.start_byte..fix.end_byte],
        r#""agent\u0053pawn""#
    );
    let prefix = &content[..fix.start_byte];
    assert_eq!(
        hint.column,
        prefix.rsplit('\n').next().unwrap().chars().count() + 1
    );
    let fs = Arc::new(MockFileSystem::new());
    fs.add_file(path, content);
    apply_fixes_with_fs(&diagnostics, false, true, Some(fs.clone())).unwrap();
    assert_eq!(
        fs.read_to_string(path).unwrap(),
        content.replace(r#""agent\u0053pawn""#, r#""SessionStart""#)
    );
}

#[test]
fn kiro_alias_ambiguous_decoded_paths_never_fixed() {
    let path = Path::new(".kiro/hooks/duplicates.json");
    let hook =
        r#"{"name":"a","trigger":"agentSpawn","action":{"type":"command","command":"echo"}}"#;
    for content in [
        format!(r#"{{"version":"v1","hooks":[],"hooks":[{hook}]}}"#),
        format!(r#"{{"version":"v1","hooks":[{hook}],"ho\u006fks":[{hook}]}}"#),
        format!(r#"{{"version":"v1","ho\u006fks":[],"hooks":[],"hooks":[{hook}]}}"#),
        format!(
            r#"{{"version":"v1","hooks":[{}]}}"#,
            hook.replace(
                r#""trigger":"agentSpawn""#,
                r#""trigger":"stop","trig\u0067er":"agentSpawn""#
            )
        ),
    ] {
        let diagnostics = validate(path, &content, &LintConfig::default());
        assert_eq!(diagnostics.len(), 1, "{content}: {diagnostics:?}");
        assert_eq!(diagnostics[0].rule, "KR-HK-014");
        assert!(diagnostics[0].fixes.is_empty(), "{content}");
    }
}

#[test]
fn kiro_manual_and_session_end_are_valid_v3_triggers() {
    let path = Path::new(".kiro/hooks/cli.json");
    for trigger in ["Manual", "SessionEnd"] {
        let content = serde_json::json!({"version":"v1","hooks":[{"name":"a","trigger":trigger,"action":{"type":"command","command":"echo ok"}}]}).to_string();
        assert!(validate(path, &content, &LintConfig::default()).is_empty());
    }
}

#[test]
fn kiro_legacy_cli_hint_requires_absent_matching_v3_hook() {
    let path = Path::new("/project/.kiro/hooks/end.kiro.hook");
    let legacy =
        r#"{"when":{"type":"SessionEnd"},"then":{"type":"command","command":"echo done"}}"#;
    let fs = Arc::new(MockFileSystem::new());
    fs.add_dir("/project/.kiro/hooks");
    fs.add_file(path, legacy);
    let config = LintConfig::builder().fs(fs.clone()).build().unwrap();
    let diagnostics = validate(path, legacy, &config);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].rule, "KR-HK-014");
    assert_eq!(diagnostics[0].level, DiagnosticLevel::Info);
    assert!(diagnostics[0].fixes.is_empty());
    let sibling = Path::new("/project/.kiro/hooks/renamed.json");
    for content in [
        "{}",
        "invalid",
        r#"{"version":"v1","hooks":[{"name":"a","trigger":"Stop","action":{"type":"command","command":"echo done"}}]}"#,
        r#"{"version":"v1","hooks":[{"name":"a","trigger":"SessionEnd","enabled":false,"action":{"type":"command","command":"echo done"}}]}"#,
    ] {
        fs.add_file(sibling, content);
        assert!(
            validate(path, legacy, &config)
                .iter()
                .any(|d| d.rule == "KR-HK-014")
        );
    }
    fs.add_file(sibling, r#"{"version":"v1","hooks":[{"name":"a","trigger":"SessionEnd","action":{"type":"command","command":"echo done"}}]}"#);
    assert!(validate(path, legacy, &config).is_empty());
}

#[test]
fn kiro_legacy_ide_events_and_manual_hooks_do_not_get_cli_hint() {
    let path = Path::new("/project/.kiro/hooks/ide.kiro.hook");
    let fs = Arc::new(MockFileSystem::new());
    fs.add_dir("/project/.kiro/hooks");
    fs.add_file(path, "{}");
    let config = LintConfig::builder().fs(fs).build().unwrap();
    // agentSpawn is listed in the IDE migration mapping, so do not classify it
    // as CLI-only despite the narrower hook-types compatibility table.
    for trigger in [
        "agentSpawn",
        "sessionStart",
        "userTriggered",
        "promptSubmit",
        "agentStop",
        "preTaskExecution",
        "postTaskExecution",
        "fileCreated",
        "fileEdited",
        "fileDeleted",
    ] {
        let content = serde_json::json!({"when":{"type":trigger,"patterns":[".*"]},"then":{"type":"agent","prompt":"Review"}}).to_string();
        assert!(validate(path, &content, &config).is_empty(), "{trigger}");
    }
}

#[test]
fn kiro_new_diagnostics_respect_disable_rules() {
    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = [
        "KR-HK-011",
        "KR-HK-014",
        "KR-PM-001",
        "KR-PM-002",
        "KR-PM-003",
        "KR-PM-004",
    ]
    .map(str::to_string)
    .to_vec();
    assert!(validate(Path::new(".kiro/hooks/a.json"), r#"{"version":"1","hooks":[{"name":"a","trigger":"agentSpawn","action":{"type":"command","command":"echo"}}]}"#, &config).is_empty());
    assert!(
        validate(
            Path::new(".kiro/settings/permissions.yaml"),
            "rules: [",
            &config
        )
        .is_empty()
    );
}

// ----- Kiro agents (KR-AG-*, KR-HK-005, KR-HK-006) ---------------------------

fn kiro_config(kiro: Option<&str>, fs: Option<Arc<MockFileSystem>>) -> LintConfig {
    let mut builder = LintConfig::builder();
    builder.tool_versions(agnix_core::config::ToolVersions {
        kiro: kiro.map(str::to_string),
        ..Default::default()
    });
    if let Some(fs) = fs {
        builder.fs(fs);
    }
    builder.build().unwrap()
}

fn agent_rules(diagnostics: &[Diagnostic]) -> Vec<(String, DiagnosticLevel)> {
    let mut rules: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("KR-AG-") || d.rule == "KR-HK-005" || d.rule == "KR-HK-006")
        .map(|d| (d.rule.clone(), d.level))
        .collect();
    rules.sort_by(|a, b| a.0.cmp(&b.0));
    rules
}

#[test]
fn kiro_agent_fixtures_report_what_kiro_rejects() {
    use DiagnosticLevel::{Error, Warning};
    type Case<'a> = (&'a str, &'a str, &'a [(&'a str, DiagnosticLevel)]);
    let cases: &[Case<'_>] = &[
        (
            "valid-agent.json",
            include_str!("../../../tests/fixtures/kiro-agents/.kiro/agents/valid-agent.json"),
            &[],
        ),
        (
            "valid-hooks.json",
            include_str!("../../../tests/fixtures/kiro-agents/.kiro/agents/valid-hooks.json"),
            &[],
        ),
        (
            "valid-markdown-agent.md",
            include_str!(
                "../../../tests/fixtures/kiro-agents/.kiro/agents/valid-markdown-agent.md"
            ),
            &[],
        ),
        (
            "minimal-agent.json",
            include_str!("../../../tests/fixtures/kiro-agents/.kiro/agents/minimal-agent.json"),
            &[],
        ),
        (
            "jsonc-agent.json",
            include_str!("../../../tests/fixtures/kiro-agents/.kiro/agents/jsonc-agent.json"),
            &[("KR-AG-016", Warning)],
        ),
        (
            "missing-name.json",
            include_str!("../../../tests/fixtures/kiro-agents/.kiro/agents/missing-name.json"),
            &[("KR-AG-015", Warning), ("KR-AG-019", Error)],
        ),
        (
            "invalid-hook-event.json",
            include_str!(
                "../../../tests/fixtures/kiro-agents/.kiro/agents/invalid-hook-event.json"
            ),
            &[("KR-HK-005", Error)],
        ),
        (
            "missing-hook-command.json",
            include_str!(
                "../../../tests/fixtures/kiro-agents/.kiro/agents/missing-hook-command.json"
            ),
            &[("KR-HK-006", Error)],
        ),
        (
            "invalid-v3-hooks.json",
            include_str!("../../../tests/fixtures/kiro-agents/.kiro/agents/invalid-v3-hooks.json"),
            &[
                ("KR-AG-025", Warning),
                ("KR-HK-005", Warning),
                ("KR-HK-006", Error),
            ],
        ),
        (
            "notes.md",
            include_str!("../../../tests/fixtures/kiro-agents/.kiro/agents/notes.md"),
            &[("KR-AG-017", Warning)],
        ),
    ];
    for (name, content, expected) in cases {
        let path = format!("/project/.kiro/agents/{name}");
        let diagnostics = validate(Path::new(&path), content, &kiro_config(None, None));
        let expected: Vec<_> = expected
            .iter()
            .map(|(rule, level)| (rule.to_string(), *level))
            .collect();
        assert_eq!(
            agent_rules(&diagnostics),
            expected,
            "{name}: {diagnostics:#?}"
        );
    }
}

#[test]
fn kiro_agent_missing_prompt_file_uses_the_configured_file_system() {
    let path = Path::new("/project/.kiro/agents/a.json");
    let content = r#"{"name": "a", "description": "Reviews code changes for the team", "prompt": "file://./prompts/a.md"}"#;
    let fs = Arc::new(MockFileSystem::new());
    fs.add_file(path, content);
    let missing = validate(path, content, &kiro_config(None, Some(fs.clone())));
    assert_eq!(
        agent_rules(&missing),
        [("KR-AG-021".to_string(), DiagnosticLevel::Error)]
    );
    fs.add_file("/project/.kiro/agents/prompts/a.md", "You review code.");
    assert!(agent_rules(&validate(path, content, &kiro_config(None, Some(fs)))).is_empty());
}

#[test]
fn kiro_agent_pinned_engine_changes_levels() {
    let path = Path::new("/project/.kiro/agents/a.json");
    // Missing name: fatal in CLI 2.x only.
    let neutral = r#"{"description": "Reviews code changes for the team"}"#;
    let level = |kiro| agent_rules(&validate(path, neutral, &kiro_config(kiro, None)));
    assert_eq!(
        level(None),
        [("KR-AG-019".to_string(), DiagnosticLevel::Warning)]
    );
    assert_eq!(
        level(Some("2.32.1")),
        [("KR-AG-019".to_string(), DiagnosticLevel::Error)]
    );
    assert!(level(Some("3.0.0")).is_empty());
    // A V3 profile with tools "*": valid in V3, rejected by CLI 2.x.
    let v3 = r#"{"name": "a", "description": "Reviews code changes for the team", "tools": "*", "permissions": {"rules": []}}"#;
    assert!(agent_rules(&validate(path, v3, &kiro_config(None, None))).is_empty());
    assert_eq!(
        agent_rules(&validate(path, v3, &kiro_config(Some("2.x"), None))),
        [
            // CLI 2.x ignores the V3 permissions block.
            ("KR-AG-001".to_string(), DiagnosticLevel::Info),
            ("KR-AG-018".to_string(), DiagnosticLevel::Error),
        ]
    );
    // Markdown agents are not loaded by CLI 2.x.
    let md = Path::new("/project/.kiro/agents/a.md");
    assert_eq!(
        agent_rules(&validate(
            md,
            "---\ndescription: Reviews code changes for the team\n---\nBody",
            &kiro_config(Some("2.x"), None)
        )),
        [("KR-AG-028".to_string(), DiagnosticLevel::Warning)]
    );
}

#[test]
fn kiro_agent_hook_key_fix_is_unsafe_exact_and_reparses() {
    let path = Path::new("/project/.kiro/agents/hooks.json");
    let content = "{\n  \"name\": \"a\",\n  \"note\": \"PreToolUse stays\",\n  \"hooks\": {\n    \"PreToolUse\": [{\"command\": \"echo PreToolUse\"}]\n  }\n}";
    let config = kiro_config(None, None);
    let diagnostics = validate(path, content, &config);
    let hint = diagnostics.iter().find(|d| d.rule == "KR-HK-005").unwrap();
    assert_eq!((hint.line, hint.column), (5, 5));
    assert_eq!(hint.fixes.len(), 1);
    let fix = &hint.fixes[0];
    assert!(!fix.is_safe());
    assert_eq!(&content[fix.start_byte..fix.end_byte], "\"PreToolUse\"");
    let fs = Arc::new(MockFileSystem::new());
    fs.add_file(path, content);
    apply_fixes_with_fs(&diagnostics, false, true, Some(fs.clone())).unwrap();
    assert_eq!(
        fs.read_to_string(path).unwrap(),
        content,
        "safe-only must not rename"
    );
    apply_fixes_with_fs(&diagnostics, false, false, Some(fs.clone())).unwrap();
    let fixed = fs.read_to_string(path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&fixed).unwrap();
    assert_eq!(
        value["hooks"]["preToolUse"][0]["command"],
        "echo PreToolUse"
    );
    assert_eq!(value["note"], "PreToolUse stays");
    assert!(
        validate(path, &fixed, &config)
            .iter()
            .all(|d| d.rule != "KR-HK-005")
    );
}

#[test]
fn kiro_agent_empty_trust_fields_fix_reparses_and_loads_in_v3() {
    let path = Path::new("/project/.kiro/agents/pk.json");
    let content = "{\n  \"name\": \"pk\",\n  \"tools\": [\"read\"],\n  \"toolsSettings\": {},\n  \"includePowers\": true\n}";
    let config = kiro_config(None, None);
    let diagnostics = validate(path, content, &config);
    let finding = diagnostics.iter().find(|d| d.rule == "KR-AG-015").unwrap();
    assert_eq!(finding.line, 4);
    assert_eq!(finding.fixes.len(), 1);
    assert!(!finding.fixes[0].is_safe());
    let fs = Arc::new(MockFileSystem::new());
    fs.add_file(path, content);
    apply_fixes_with_fs(&diagnostics, false, false, Some(fs.clone())).unwrap();
    let fixed = fs.read_to_string(path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&fixed).unwrap();
    assert_eq!(
        value,
        serde_json::json!({"name": "pk", "tools": ["read"], "includePowers": true})
    );
    assert!(
        validate(path, &fixed, &config)
            .iter()
            .all(|d| d.rule != "KR-AG-015")
    );
    // Non-empty trust fields are never removed automatically.
    let real = r#"{"name": "pk", "toolsSettings": {"shell": {"allowedCommands": ["git status"]}}}"#;
    let diagnostics = validate(path, real, &config);
    assert!(
        diagnostics
            .iter()
            .find(|d| d.rule == "KR-AG-015")
            .unwrap()
            .fixes
            .is_empty()
    );
}

#[test]
fn kiro_agent_array_trigger_fix_targets_the_trigger_value() {
    let path = Path::new("/project/.kiro/agents/v3.json");
    let content = r#"{"name": "v3", "hooks": [{"name": "AgentSpawn", "trigger": "AgentSpawn", "action": {"type": "command", "command": "AgentSpawn"}}]}"#;
    let config = kiro_config(None, None);
    let diagnostics = validate(path, content, &config);
    let fix = &diagnostics
        .iter()
        .find(|d| d.rule == "KR-HK-005")
        .unwrap()
        .fixes[0];
    assert_eq!(
        content[..fix.start_byte].matches("AgentSpawn").count(),
        1,
        "must skip the name value"
    );
    let fs = Arc::new(MockFileSystem::new());
    fs.add_file(path, content);
    apply_fixes_with_fs(&diagnostics, false, false, Some(fs.clone())).unwrap();
    let value: serde_json::Value = serde_json::from_str(&fs.read_to_string(path).unwrap()).unwrap();
    assert_eq!(value["hooks"][0]["trigger"], "SessionStart");
    assert_eq!(value["hooks"][0]["name"], "AgentSpawn");
    assert_eq!(value["hooks"][0]["action"]["command"], "AgentSpawn");
}
