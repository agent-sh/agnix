use super::*;
use crate::config::LintConfig;
use crate::diagnostics::DiagnosticLevel;

const PATH: &str = ".kiro/settings/cli.json";

fn validate(content: &str) -> Vec<Diagnostic> {
    KiroSettingsValidator.validate(Path::new(PATH), content, &LintConfig::default())
}

fn hits(diagnostics: &[Diagnostic], rule: &str) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|d| d.rule == rule)
        .cloned()
        .collect()
}

fn only(diagnostics: &[Diagnostic], rule: &str) -> Diagnostic {
    let found = hits(diagnostics, rule);
    assert_eq!(found.len(), 1, "expected one {rule}: {diagnostics:#?}");
    found.into_iter().next().unwrap()
}

/// Apply the only fix of the only `rule` diagnostic, assert the result still
/// parses as JSON, and return it.
fn apply_fix(content: &str, rule: &str) -> String {
    let diagnostic = only(&validate(content), rule);
    assert_eq!(diagnostic.fixes.len(), 1, "{diagnostic:#?}");
    let fix = &diagnostic.fixes[0];
    let mut out = content.to_string();
    out.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
    serde_json::from_str::<serde_json::Value>(&out)
        .unwrap_or_else(|e| panic!("fixed file must parse: {e}: {out}"));
    out
}

// ===== KR-SET-001 =====

#[test]
fn test_kr_set_001_absent_or_boolean_is_fine() {
    for content in [
        r#"{"chat.defaultAgent": "x"}"#,
        r#"{"toolSearch.enabled": true}"#,
        r#"{"toolSearch.enabled": false}"#,
    ] {
        assert!(validate(content).is_empty(), "{content}");
    }
}

#[test]
fn test_kr_set_001_non_boolean_values() {
    // A wrongly typed value is ignored (Kiro keeps the default), so this is a
    // warning; it used to be an error.
    for value in [r#""true""#, "1", "null", "[]", r#""yes""#] {
        let content = format!(r#"{{"toolSearch.enabled": {value}}}"#);
        let d = only(&validate(&content), "KR-SET-001");
        assert_eq!(d.level, DiagnosticLevel::Warning, "{value}");
    }
}

#[test]
fn test_kr_set_001_quoted_boolean_fix_is_unsafe_and_exact() {
    let content = "{\n  \"chat.defaultAgent\": \"x\",\n  \"toolSearch.enabled\": \"TRUE\"\n}";
    let d = only(&validate(content), "KR-SET-001");
    assert_eq!((d.line, d.column), (3, 25));
    let fix = &d.fixes[0];
    assert!(
        !fix.safe,
        "quoted booleans must not be converted by plain --fix"
    );
    assert!(
        fix.description
            .contains("may change behavior or security policy")
    );
    assert_eq!(&content[fix.start_byte..fix.end_byte], "\"TRUE\"");
    let fixed = apply_fix(content, "KR-SET-001");
    assert!(fixed.contains("\"toolSearch.enabled\": true"));
    assert!(validate(&fixed).is_empty());
    let fixed = apply_fix(r#"{"toolSearch.enabled": "false"}"#, "KR-SET-001");
    assert_eq!(fixed, r#"{"toolSearch.enabled": false}"#);
}

#[test]
fn test_kr_set_001_no_fix_for_ambiguous_values() {
    for value in [r#""yes""#, "1", "0", "null", "[]"] {
        let content = format!(r#"{{"toolSearch.enabled": {value}}}"#);
        assert!(
            only(&validate(&content), "KR-SET-001").fixes.is_empty(),
            "{value}"
        );
    }
}

#[test]
fn test_kr_set_001_nested_decoy_is_not_the_target() {
    // The old fix edited the first textual "toolSearch.enabled" (a nested copy).
    let content =
        r#"{"profiles": {"a": {"toolSearch.enabled": "yes"}}, "toolSearch.enabled": "TRUE"}"#;
    let fixed = apply_fix(content, "KR-SET-001");
    let value: serde_json::Value = serde_json::from_str(&fixed).unwrap();
    assert_eq!(value["toolSearch.enabled"], true);
    assert_eq!(value["profiles"]["a"]["toolSearch.enabled"], "yes");
}

#[test]
fn test_kr_set_001_duplicate_keys_use_the_last_value() {
    // serde_json (and Kiro) keep the last value of a repeated key.
    let content = r#"{"toolSearch.enabled": "true", "toolSearch.enabled": false}"#;
    let diagnostics = validate(content);
    assert!(
        hits(&diagnostics, "KR-SET-001").is_empty(),
        "{diagnostics:#?}"
    );
    only(&diagnostics, "KR-SET-010");

    let content = r#"{"toolSearch.enabled": false, "toolSearch.enabled": "true"}"#;
    let fixed = apply_fix(content, "KR-SET-001");
    assert_eq!(
        fixed,
        r#"{"toolSearch.enabled": false, "toolSearch.enabled": true}"#
    );
    // Converges: the effective value is now valid.
    assert!(hits(&validate(&fixed), "KR-SET-001").is_empty());
}

// ===== KR-SET-002 =====

#[test]
fn test_kr_set_002_values() {
    for value in ["5", "0", "2.5", "100"] {
        let content = format!(r#"{{"toolSearch.minPct": {value}}}"#);
        assert!(validate(&content).is_empty(), "{value}");
    }
    for value in ["-1", "150", r#""5""#, r#""five""#] {
        let content = format!(r#"{{"toolSearch.minPct": {value}}}"#);
        let d = only(&validate(&content), "KR-SET-002");
        assert_eq!(d.level, DiagnosticLevel::Warning, "{value}");
    }
}

#[test]
fn test_kr_set_002_number_fix() {
    let fixed = apply_fix(r#"{"toolSearch.minPct": "5"}"#, "KR-SET-002");
    assert_eq!(fixed, r#"{"toolSearch.minPct": 5}"#);
    let fixed = apply_fix(r#"{"toolSearch.minPct": "2.5"}"#, "KR-SET-002");
    assert_eq!(fixed, r#"{"toolSearch.minPct": 2.5}"#);
    let d = only(&validate(r#"{"toolSearch.minPct": "5"}"#), "KR-SET-002");
    assert!(
        !d.fixes[0].safe,
        "a quoted number is ignored today; converting it changes behavior"
    );
}

#[test]
fn test_kr_set_002_no_fix_for_unrepresentable_or_unusual_numbers() {
    // "1e400" used to become a bare 1e400, which no JSON parser accepts.
    for value in ["1e400", "5e2", "05", "-5", "+5", ".5", "5.", "1.5e-3"] {
        let content = format!(r#"{{"toolSearch.minPct": "{value}"}}"#);
        let d = only(&validate(&content), "KR-SET-002");
        assert!(d.fixes.is_empty(), "{value}: {:?}", d.fixes);
    }
}

// ===== KR-SET-003 =====

#[test]
fn test_kr_set_003_values() {
    for value in ["50000", "0"] {
        let content = format!(r#"{{"toolSearch.minTokens": {value}}}"#);
        assert!(validate(&content).is_empty(), "{value}");
    }
    for value in ["-10", "100.5", r#""50000""#, r#""lots""#] {
        let content = format!(r#"{{"toolSearch.minTokens": {value}}}"#);
        let d = only(&validate(&content), "KR-SET-003");
        assert_eq!(d.level, DiagnosticLevel::Warning, "{value}");
    }
}

#[test]
fn test_kr_set_003_integer_fix_only() {
    let fixed = apply_fix(r#"{"toolSearch.minTokens": "50000"}"#, "KR-SET-003");
    assert_eq!(fixed, r#"{"toolSearch.minTokens": 50000}"#);
    for value in ["50000.5", "1e10", "99999999999999999999"] {
        let content = format!(r#"{{"toolSearch.minTokens": "{value}"}}"#);
        assert!(
            only(&validate(&content), "KR-SET-003").fixes.is_empty(),
            "{value}"
        );
    }
}

// ===== KR-SET-004 =====

#[test]
fn test_kr_set_004_unparseable_files() {
    // Previously silent. kiro-cli 2.32.1 refuses to start with each of these.
    for content in [
        r#"{"toolSearch.enabled": tr"#,
        "// comment\n{\"chat.defaultAgent\": \"x\"}",
        r#"{"chat.defaultAgent": "x",}"#,
        "",
        "[]",
        "\"x\"",
        r#"{"n": 1e400}"#,
    ] {
        let d = only(&validate(content), "KR-SET-004");
        assert_eq!(d.level, DiagnosticLevel::Error, "{content}");
    }
    let d = only(&validate("{\n  \"a\": 1,\n}"), "KR-SET-004");
    assert_eq!(d.line, 3, "{d:?}");
}

// ===== KR-SET-005 / 006 / 007 =====

#[test]
fn test_kr_set_005_unknown_keys() {
    let diagnostics = validate(
        r#"{"chat.autoCompact": true, "kirocrew.skillDiscovery.inheritFiles": true, "chat.defaultAgnet": "x"}"#,
    );
    let found = hits(&diagnostics, "KR-SET-005");
    assert_eq!(found.len(), 3, "{diagnostics:#?}");
    assert!(found.iter().all(|d| d.level == DiagnosticLevel::Info));
    let suggestion = |key: &str| {
        found
            .iter()
            .find(|d| d.message.contains(key))
            .and_then(|d| d.suggestion.clone())
            .unwrap()
    };
    assert!(suggestion("chat.autoCompact").contains("chat.disableAutoCompaction"));
    assert!(suggestion("chat.defaultAgnet").contains("'chat.defaultAgent'"));
}

#[test]
fn test_kr_set_006_misplaced_sections() {
    let content = r#"{"chat.defaultAgent": "oma-hooks", "mcpServers": {"serena": {"command": "oma"}}, "hooks": {"stop": []}}"#;
    let found = hits(&validate(content), "KR-SET-006");
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|d| d.level == DiagnosticLevel::Warning));
    assert!(found.iter().any(|d| d.message.contains("mcp.json")));
}

#[test]
fn test_kr_set_007_nested_objects_are_ignored() {
    let d = only(
        &validate(r#"{"toolSearch": {"enabled": true, "minPct": 5}}"#),
        "KR-SET-007",
    );
    assert!(
        d.message.contains("toolSearch.enabled, toolSearch.minPct"),
        "{d:?}"
    );
    let d = only(
        &validate(r#"{"chat": {"keybindings": {"quit": "ctrl+q"}}}"#),
        "KR-SET-007",
    );
    assert!(d.message.contains("chat.keybindings.quit"), "{d:?}");
    // Object-valued settings are not nested settings.
    assert!(
        validate(
            r#"{"chat.modelDefaults": {"claude-opus-5.5": {"output_config": {"effort": "high"}}}}"#
        )
        .is_empty()
    );
}

// ===== KR-SET-008 / 009 =====

#[test]
fn test_kr_set_008_value_types() {
    for (key, value) in [
        ("chat.enableKnowledge", r#""yes""#),
        ("cleanup.periodDays", r#""7d""#),
        ("knowledge.defaultIncludePatterns", r#""**/*.ts""#),
        ("knowledge.defaultExcludePatterns", "[1]"),
        ("chat.defaultModel", "5"),
        ("chat.modelDefaults", "[]"),
        ("chat.showThinking", r#""collapsed""#),
    ] {
        let content = format!(r#"{{"{key}": {value}}}"#);
        let d = only(&validate(&content), "KR-SET-008");
        assert_eq!(d.level, DiagnosticLevel::Warning, "{key}");
        assert!(d.message.contains(key));
    }
}

#[test]
fn test_kr_set_008_quoted_value_fixes_are_unsafe() {
    let fixed = apply_fix(r#"{"chat.enableKnowledge": "True"}"#, "KR-SET-008");
    assert_eq!(fixed, r#"{"chat.enableKnowledge": true}"#);
    let fixed = apply_fix(r#"{"api.timeout": "600"}"#, "KR-SET-008");
    assert_eq!(fixed, r#"{"api.timeout": 600}"#);
    let d = only(
        &validate(r#"{"chat.enableKnowledge": "true"}"#),
        "KR-SET-008",
    );
    assert!(!d.fixes[0].safe);
    for content in [
        r#"{"cleanup.periodDays": "7d"}"#,
        // Fractional values of numeric settings are not converted.
        r#"{"knowledge.maxFiles": "2.5"}"#,
    ] {
        assert!(
            only(&validate(content), "KR-SET-008").fixes.is_empty(),
            "{content}"
        );
    }
}

#[test]
fn test_kr_set_009_documented_values() {
    for (key, value) in [
        ("chat.uiMode", r#""compact""#),
        ("chat.agentEngine", r#""v4""#),
        ("memory.mode", r#""rw""#),
        ("compaction.excludeContextWindowPercent", "150"),
        ("chat.fullscreenWheelRows", "4"),
    ] {
        let content = format!(r#"{{"{key}": {value}}}"#);
        let d = only(&validate(&content), "KR-SET-009");
        assert_eq!(d.level, DiagnosticLevel::Warning, "{key}");
    }
    for content in [
        r#"{"chat.uiMode": "tui", "chat.agentEngine": "v3", "chat.ui": "classic"}"#,
        r#"{"knowledge.indexType": "Best", "chat.fullscreenWheelRows": 2}"#,
    ] {
        assert!(
            hits(&validate(content), "KR-SET-009").is_empty(),
            "{content}"
        );
    }
}

#[test]
fn test_corpus_style_files_are_clean() {
    for content in [
        r#"{"chat.modelPicker.lastUsed": {"claude-opus-5.5": 1792087625064}, "chat.modelDefaults": {"claude-opus-5.5": {"output_config": {"effort": "high"}}}}"#,
        r#"{"chat.defaultAgent": "kiro-dev", "chat.enableKnowledge": true, "chat.enableTodoList": true, "toolSearch.enabled": true, "toolSearch.minPct": 0, "toolSearch.minTokens": 0}"#,
        r#"{"chat.defaultAgent": "vibeflow-dev", "knowledge.defaultIncludePatterns": ["**/*.ts"], "knowledge.defaultExcludePatterns": ["dist/**"]}"#,
        "{}",
    ] {
        // KR-SET-011 depends on whether the test path is under the real home.
        let diagnostics: Vec<_> = validate(content)
            .into_iter()
            .filter(|d| d.rule != "KR-SET-011")
            .collect();
        assert!(diagnostics.is_empty(), "{content}: {diagnostics:#?}");
    }
}

// ===== KR-SET-010 / 011 =====

#[test]
fn test_kr_set_010_duplicate_key_points_at_the_ignored_one() {
    let content = "{\n  \"chat.defaultAgent\": \"a\",\n  \"chat.defaultAgent\": \"b\"\n}";
    let d = only(&validate(content), "KR-SET-010");
    assert_eq!(d.line, 2);
    assert!(d.message.contains("line 3"));
}

#[test]
fn test_workspace_settings_decision() {
    let home = Path::new("/home/u");
    let decide = |p: &str, kiro_home: Option<&str>| {
        workspace_settings_decision(Path::new(p), Some(home), kiro_home.map(Path::new))
    };
    assert_eq!(decide("/home/u/.kiro/settings/cli.json", None), Some(false));
    assert_eq!(
        decide("/home/u/proj/.kiro/settings/cli.json", None),
        Some(true)
    );
    assert_eq!(
        decide("/opt/kiro/settings/cli.json", Some("/opt/kiro")),
        Some(false)
    );
    // KIRO_HOME set to $HOME is ignored by Kiro.
    assert_eq!(
        decide("/home/u/settings/cli.json", Some("/home/u")),
        Some(true)
    );
    assert_eq!(
        workspace_settings_decision(Path::new("/x/.kiro/settings/cli.json"), None, None),
        None
    );
}

#[test]
fn test_kr_set_011_global_only_key_in_workspace_file() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join(".kiro/settings/cli.json");
    let content = r#"{"chat.uiMode": "tui", "chat.defaultAgent": "x"}"#;
    let diagnostics = KiroSettingsValidator.validate(&path, content, &LintConfig::default());
    if is_workspace_settings_file(&path) == Some(true) {
        let d = only(&diagnostics, "KR-SET-011");
        assert_eq!(d.level, DiagnosticLevel::Info);
        assert!(d.message.contains("chat.uiMode"));
    }
    // A copy of the global file (it carries keys only Kiro writes, globally)
    // is not treated as a workspace file.
    let copy = r#"{"chat.uiMode": "tui", "mcp.loadedBefore": true}"#;
    let diagnostics = KiroSettingsValidator.validate(&path, copy, &LintConfig::default());
    assert!(
        hits(&diagnostics, "KR-SET-011").is_empty(),
        "{diagnostics:#?}"
    );
}

#[test]
fn test_kr_set_012_legacy_settings_json() {
    let path = Path::new(".kiro/settings.json");
    let config = LintConfig::default();
    // Another tool's settings.json is not a Kiro file: nothing to report.
    for content in [
        r#"{"editor.tabSize": 2, "statusLine": {"enabled": true}}"#,
        "{not json",
        "[]",
    ] {
        assert!(
            KiroSettingsValidator
                .validate(path, content, &config)
                .is_empty(),
            "{content}"
        );
    }
    let diagnostics = KiroSettingsValidator.validate(
        path,
        r#"{"chat.defaultModel": "claude-sonnet-4.6", "toolSearch.enabled": "true"}"#,
        &config,
    );
    let d = only(&diagnostics, "KR-SET-012");
    assert_eq!(d.level, DiagnosticLevel::Warning);
    only(&diagnostics, "KR-SET-001");
}

// ===== Toggles and metadata =====

#[test]
fn test_rules_are_independently_disableable() {
    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = vec!["KR-SET-002".to_string()];
    let content = r#"{"toolSearch.enabled": "true", "toolSearch.minPct": -5}"#;
    let diagnostics = KiroSettingsValidator.validate(Path::new(PATH), content, &config);
    assert!(diagnostics.iter().any(|d| d.rule == "KR-SET-001"));
    assert!(!diagnostics.iter().any(|d| d.rule == "KR-SET-002"));

    config.rules_mut().disabled_rules = RULE_IDS.iter().map(|id| id.to_string()).collect();
    for content in [
        r#"{"toolSearch.enabled": "x", "toolSearch.minPct": "x", "toolSearch.minTokens": -1, "chat.ui": 1, "chat.uiMode": "x", "zz": 1, "hooks": {}, "chat": {"defaultAgent": "x"}, "chat.ui": "tui"}"#,
        "{",
    ] {
        let diagnostics = KiroSettingsValidator.validate(Path::new(PATH), content, &config);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }
}

#[test]
fn test_does_not_panic_on_non_ascii_content() {
    let content = "{\n  \"chat.defaultAgent\": \"\u{1F525}prose\u{4e2d}\u{6587}\",\n  \"toolSearch.enabled\": 42\n}";
    let d = only(&validate(content), "KR-SET-001");
    assert_eq!(d.line, 3);
}

#[test]
fn test_settings_table_matches_kiro_cli_2_32_1() {
    assert_eq!(SETTINGS.len(), 140);
    let mut keys: Vec<_> = SETTINGS.iter().map(|s| s.key).collect();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), 140, "duplicate keys in SETTINGS");
    assert!(setting("chat.uiMode").unwrap().global_only);
    assert!(!setting("chat.defaultAgent").unwrap().global_only);
}

#[test]
fn test_metadata() {
    let metadata = KiroSettingsValidator.metadata();
    assert_eq!(metadata.name, "KiroSettingsValidator");
    assert_eq!(metadata.rule_ids, RULE_IDS);
}
