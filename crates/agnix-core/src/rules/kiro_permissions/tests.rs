use super::*;
use crate::config::LintConfig;
use crate::diagnostics::DiagnosticLevel;

const PATH: &str = ".kiro/settings/permissions.yaml";

/// Diagnostics for `content`, without the KR-PM-009 location note (it
/// depends on whether the test path is under the real home directory).
fn validate_at(path: &str, content: &str) -> Vec<Diagnostic> {
    KiroPermissionsValidator
        .validate(Path::new(path), content, &LintConfig::default())
        .into_iter()
        .filter(|d| d.rule != "KR-PM-009")
        .collect()
}

fn validate(content: &str) -> Vec<Diagnostic> {
    validate_at(PATH, content)
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

#[test]
fn kiro_permissions_valid_vendor_examples_and_all_capabilities() {
    for capability in VALID_PERMISSION_CAPABILITIES {
        for effect in VALID_PERMISSION_EFFECTS {
            let content = format!("rules:\n  - capability: {capability}\n    effect: {effect}\n");
            // KR-PM-010 advises against unrestricted allows of write-capable
            // capabilities; the rule itself is valid.
            let diagnostics: Vec<_> = validate(&content)
                .into_iter()
                .filter(|d| d.rule != "KR-PM-010")
                .collect();
            assert!(diagnostics.is_empty(), "{content}");
        }
    }
    assert!(validate("rules: []").is_empty());
    assert!(validate("rules:\n  - capability: mcp\n    match: [\"my-server/*\"]\n    exclude: [\"my-server/dangerous-tool\"]\n    effect: allow\n").is_empty());
    assert!(
        validate("rules: [{capability: shell, effect: ask, match: [\"git *\"], exclude: []}]")
            .is_empty()
    );
    assert!(validate("policies: [read-only-shell, dev-shell]\nrules: []\n").is_empty());
}

#[test]
fn kiro_permissions_invalid_document_shapes() {
    for content in [
        "rules: [",
        "",
        "# only a comment\n",
        "[]",
        "rules: null",
        "rules: {}",
        "rules: [false]",
        "rules: [{capability: shell, capability: mcp}]",
    ] {
        let d = only(&validate(content), "KR-PM-001");
        assert_eq!(d.level, DiagnosticLevel::Error, "{content}");
    }
    let d = only(
        &validate("capability: fs_write\nmatch: /**\neffect: allow"),
        "KR-PM-001",
    );
    assert!(d.message.contains("fails closed"), "{d:?}");
}

#[test]
fn kiro_permissions_missing_capability_is_fatal_and_other_fields_still_reported() {
    for content in [
        "rules: [{}]",
        "rules: [{capability: 1, effect: true}]",
        "rules: [{capability: null, effect: null}]",
    ] {
        let diagnostics = validate(content);
        let d = only(&diagnostics, "KR-PM-002");
        assert!(d.message.contains("rejects the whole file"), "{content}");
        only(&diagnostics, "KR-PM-003");
        assert!(diagnostics.iter().all(|d| d.fixes.is_empty()));
    }
}

#[test]
fn kiro_permissions_unknown_capability_skips_only_that_rule() {
    // Kiro checks the capability first and skips a rule with an unknown one
    // before reading its other fields. Those fields are still reported, as
    // warnings, because they become fatal once the capability is fixed.
    let diagnostics = validate("rules: [{capability: SHELL, effect: Allow, bogus: 1}]");
    let d = only(&diagnostics, "KR-PM-002");
    assert_eq!(d.level, DiagnosticLevel::Error);
    assert!(d.message.contains("skips this rule"), "{d:?}");
    for rule in ["KR-PM-003", "KR-PM-005"] {
        let d = only(&diagnostics, rule);
        assert_eq!(d.level, DiagnosticLevel::Warning, "{rule}");
        assert!(d.message.contains("once the capability is fixed"), "{d:?}");
    }
    // Corpus: capability "*" is not a capability (the meta-capability is all).
    let d = only(
        &validate("rules:\n  - capability: \"*\"\n    match: [\"*\"]\n    effect: allow\n"),
        "KR-PM-002",
    );
    assert_eq!((d.line, d.column), (2, 5));
}

#[test]
fn kiro_permissions_workflow_capability_is_accepted() {
    assert!(validate("rules: [{capability: workflow, effect: ask}]").is_empty());
}

#[test]
fn kiro_permissions_effects() {
    let d = only(
        &validate("rules:\n  - capability: shell\n    match: [\"git *\"]\n"),
        "KR-PM-003",
    );
    assert_eq!(d.level, DiagnosticLevel::Error);
    let d = only(
        &validate("rules: [{capability: shell, effect: evaluate}]"),
        "KR-PM-003",
    );
    assert_eq!(d.level, DiagnosticLevel::Warning);
    assert!(d.message.contains("skipped"));
    for effect in ["permit", "Allow", "1"] {
        let content = format!("rules: [{{capability: shell, effect: {effect}}}]");
        assert_eq!(
            only(&validate(&content), "KR-PM-003").level,
            DiagnosticLevel::Error,
            "{effect}"
        );
    }
}

#[test]
fn kiro_permissions_admin_scope_allows_only_deny_and_ask() {
    let content = r#"{"rules": [{"capability": "shell", "effect": "allow"}, {"capability": "mcp", "effect": "deny"}]}"#;
    let diagnostics = validate_at("it/managed-settings.json", content);
    let d = only(&diagnostics, "KR-PM-003");
    assert!(d.message.contains("managed-settings.json"), "{d:?}");
    let presets = r#"{"policies": ["allow-all"], "rules": []}"#;
    let d = only(
        &validate_at("it/managed-settings.json", presets),
        "KR-PM-006",
    );
    assert_eq!(d.level, DiagnosticLevel::Warning);
}

#[test]
fn kiro_permissions_json_files_are_strict_json() {
    let path = "/home/u/.kiro/settings/permissions.json";
    assert!(
        validate_at(
            path,
            r#"{"rules": [{"capability": "shell", "effect": "ask"}]}"#
        )
        .is_empty()
    );
    let d = only(&validate_at(path, "{\"rules\": [],}"), "KR-PM-001");
    assert_eq!(d.level, DiagnosticLevel::Error);
    // YAML syntax in a .json file is not read as YAML.
    only(&validate_at(path, "rules: []\n"), "KR-PM-001");
}

#[test]
fn kiro_permissions_patterns_require_string_arrays() {
    for field in ["match", "exclude"] {
        for value in [
            "null",
            "true",
            "'git *'",
            "{}",
            "[12]",
            "[false]",
            "['git *', null]",
        ] {
            let content =
                format!("rules: [{{capability: shell, effect: allow, {field}: {value}}}]");
            let d = only(&validate(&content), "KR-PM-004");
            assert!(d.message.contains("fails closed"), "{content}");
        }
    }
}

#[test]
fn kiro_permissions_unknown_rule_fields_are_fatal() {
    // Previously accepted on purpose; the V3 parser rejects the whole file
    // ("Unknown field(s) ... in rule N").
    let content = "rules:\n  - capability: shell\n    effect: ask\n    future: true\n    note: x\n";
    let d = only(&validate(content), "KR-PM-005");
    assert_eq!(d.level, DiagnosticLevel::Error);
    assert!(d.message.contains("future, note"), "{d:?}");
    assert_eq!((d.line, d.column), (4, 5));
}

#[test]
fn kiro_permissions_policies() {
    let d = only(
        &validate("policies: read-only-shell\nrules: []\n"),
        "KR-PM-006",
    );
    assert_eq!(d.level, DiagnosticLevel::Error);
    let d = only(
        &validate("policies: [read-only-shell, yolo]\nrules: []\n"),
        "KR-PM-006",
    );
    assert_eq!(d.level, DiagnosticLevel::Warning);
    assert!(d.message.contains("yolo"));
    assert_eq!(d.line, 1);
    let d = only(&validate("policies: [1]\nrules: []\n"), "KR-PM-006");
    assert_eq!(d.level, DiagnosticLevel::Error);
}

#[test]
fn kiro_permissions_patterns_that_never_match() {
    let content = "rules:\n  - capability: mcp\n    match: [\"@github\", \"git/*\"]\n    effect: allow\n  - capability: web_fetch\n    match: [\"https://docs.aws.amazon.com/*\", \"*.amazonaws.com\"]\n    effect: allow\n";
    let found = hits(&validate(content), "KR-PM-007");
    assert_eq!(found.len(), 2, "{found:#?}");
    assert!(found[0].message.contains("'github'"), "{:?}", found[0]);
    assert_eq!(found[0].line, 3);
    assert_eq!(found[1].line, 6);
    assert!(found.iter().all(|d| d.level == DiagnosticLevel::Warning));
    // Host-name patterns and fs paths are fine.
    let ok = "rules:\n  - capability: web_fetch\n    match: [\"kiro.dev\"]\n    effect: allow\n  - capability: fs_read\n    match: [\"/home/u/**\"]\n    effect: allow\n";
    assert!(validate(ok).is_empty());
}

#[test]
fn kiro_permissions_no_effect_patterns() {
    let content = "rules:\n  - capability: web_search\n    match: [\"google.com\"]\n    effect: allow\n  - capability: fs_write\n    match: [\".kiro/hooks/**\", \"./.git/**\", \"**/*.code-workspace\", \"src/**\"]\n    effect: allow\n";
    let found = hits(&validate(content), "KR-PM-007");
    assert_eq!(found.len(), 4, "{found:#?}");
    assert!(found[0].message.contains("no resource"), "{:?}", found[0]);
    assert!(found[1..].iter().all(|d| d.message.contains("always asks")));
    // A deny on a protected path is fine (and redundant only for .kiroignore).
    assert!(
        hits(
            &validate("rules: [{capability: fs_write, match: [\".git/**\"], effect: deny}]"),
            "KR-PM-007"
        )
        .is_empty()
    );
}

#[test]
fn kiro_permissions_unrestricted_write_allow() {
    for rule in [
        "{capability: shell, effect: allow}",
        "{capability: fs_write, match: [\"**\"], effect: allow}",
        "{capability: all, match: [\"*\"], effect: allow}",
        "{capability: mcp, effect: allow}",
    ] {
        let content = format!("rules: [{rule}]");
        let d = only(&validate(&content), "KR-PM-010");
        assert_eq!(d.level, DiagnosticLevel::Warning, "{rule}");
    }
    for rule in [
        "{capability: shell, match: [\"git *\"], effect: allow}",
        "{capability: shell, effect: allow, exclude: [\"rm *\"]}",
        "{capability: fs_read, effect: allow}",
        "{capability: shell, effect: ask}",
        "{capability: shell, effect: allow, match: []}",
    ] {
        let content = format!("rules: [{rule}]");
        assert!(hits(&validate(&content), "KR-PM-010").is_empty(), "{rule}");
    }
}

#[test]
fn kiro_permissions_unsupported_pattern_syntax() {
    let content = "rules:\n  - capability: shell\n    match: [\"git **\", \"ls ?\", \"cat [ab]\", \"git *\"]\n    effect: allow\n";
    let found = hits(&validate(content), "KR-PM-011");
    assert_eq!(found.len(), 3, "{found:#?}");
    // Globs are supported for file paths.
    assert!(hits(&validate("rules: [{capability: fs_read, match: [\"src/**/*.rs\", \"a?.txt\"], effect: allow}]"), "KR-PM-011").is_empty());
}

#[test]
fn kiro_permissions_sandbox_network_order() {
    let content = "rules:\n  - capability: sandbox_network\n    match: [\"*.example.com\"]\n    effect: allow\n  - capability: sandbox_network\n    match: [\"evil.example.com\"]\n    effect: deny\n  - capability: sandbox_network\n    match: [\"bad.org\"]\n    effect: deny\n";
    let d = only(&validate(content), "KR-PM-012");
    assert_eq!(d.line, 6, "{d:?}");
    assert!(d.message.contains("evil.example.com") && d.message.contains("index 0"));
    // Deny first is the documented order.
    let ordered = "rules:\n  - capability: sandbox_network\n    match: [\"evil.example.com\"]\n    effect: deny\n  - capability: sandbox_network\n    match: [\"*.example.com\"]\n    effect: allow\n";
    assert!(hits(&validate(ordered), "KR-PM-012").is_empty());
    // An exclude on the allow keeps the deny reachable.
    let excluded = "rules:\n  - capability: sandbox_network\n    match: [\"*\"]\n    exclude: [\"evil.example.com\"]\n    effect: allow\n  - capability: sandbox_network\n    match: [\"evil.example.com\"]\n    effect: deny\n";
    assert!(hits(&validate(excluded), "KR-PM-012").is_empty());
}

#[test]
fn like_match_semantics() {
    assert!(like_match("*.example.com", "evil.example.com"));
    assert!(like_match("*", "anything"));
    assert!(like_match("git *", "git status"));
    assert!(!like_match("git *", "gitk"));
    assert!(like_match("a*b*c", "aXbYc"));
    assert!(!like_match("a*b*c", "aXcYb"));
    assert!(like_match("exact", "exact"));
}

#[test]
fn kiro_permissions_empty_match_matches_everything() {
    let d = only(
        &validate("rules: [{capability: shell, effect: allow, match: []}]"),
        "KR-PM-008",
    );
    assert_eq!(d.level, DiagnosticLevel::Warning);
    assert!(d.message.contains("every shell resource"), "{d:?}");
}

#[test]
fn kiro_permissions_corpus_true_positives_keep_their_rules() {
    let cases = [
        (
            "capability: fs_write\nmatch: /**\neffect: allow",
            "KR-PM-001",
        ),
        (
            "file:\n  - pattern: \".env*\"\n    allow: false\n",
            "KR-PM-001",
        ),
        (
            "rules:\n  - capability: \"*\"\n    match: [\"*\"]\n    effect: allow\n",
            "KR-PM-002",
        ),
        (
            "rules:\n  - capability: shell\n    match:\n      - git log *\n",
            "KR-PM-003",
        ),
    ];
    for (content, rule) in cases {
        only(&validate(content), rule);
    }
}

#[test]
fn kiro_permissions_location_decision() {
    let home = std::path::absolute("/home/u").unwrap();
    let loaded = |p: &str, kiro_home: Option<&str>| {
        let kiro_home = kiro_home.map(|p| std::path::absolute(p).unwrap());
        permissions_file_is_loaded(Path::new(p), Some(&home), kiro_home.as_deref())
    };
    assert_eq!(
        loaded("/home/u/.kiro/settings/permissions.yaml", None),
        Some(true)
    );
    assert_eq!(
        loaded(
            "/home/u/.kiro/workspace-roots/0123456789abcdef/permissions.yaml",
            None
        ),
        Some(true)
    );
    assert_eq!(
        loaded("/home/u/proj/.kiro/settings/permissions.yaml", None),
        Some(false)
    );
    assert_eq!(
        loaded(
            "/home/u/proj/.kiro/workspace-roots/x/permissions.yaml",
            None
        ),
        Some(false)
    );
    assert_eq!(
        loaded("/opt/k/settings/permissions.json", Some("/opt/k")),
        Some(true)
    );
    #[cfg(not(windows))]
    assert_eq!(loaded("/etc/kiro/managed-settings.json", None), Some(true));
    #[cfg(windows)]
    assert_eq!(
        loaded(r"C:\ProgramData\Kiro\managed-settings.json", None),
        Some(true)
    );
    assert_eq!(
        loaded("/home/u/it/managed-settings.json", None),
        Some(false)
    );
    // Another account's home directory counts as a user Kiro directory.
    assert_eq!(
        loaded("/home/me/.kiro/settings/permissions.yaml", None),
        Some(true)
    );
    assert_eq!(
        loaded("/Users/me/.kiro/workspace-roots/abc/permissions.yaml", None),
        Some(true)
    );
    assert_eq!(
        loaded("/Users/me/src/app/.kiro/settings/permissions.yaml", None),
        Some(false)
    );
    assert_eq!(
        permissions_file_is_loaded(Path::new("/x/.kiro/settings/permissions.yaml"), None, None),
        None
    );
}

#[test]
fn kiro_permissions_repo_local_file_gets_location_note() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join(".kiro/settings/permissions.yaml");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let diagnostics =
        KiroPermissionsValidator.validate(&path, "rules: []\n", &LintConfig::default());
    if permissions_file_is_loaded(&path, user_home().as_deref(), None) == Some(false) {
        let d = only(&diagnostics, "KR-PM-009");
        assert_eq!(d.level, DiagnosticLevel::Info);
    }
    // permissions.json next to permissions.yaml is never read.
    std::fs::write(&path, "rules: []\n").unwrap();
    let json = path.with_file_name("permissions.json");
    let diagnostics =
        KiroPermissionsValidator.validate(&json, "{\"rules\": []}", &LintConfig::default());
    let d = only(&diagnostics, "KR-PM-009");
    assert!(d.message.contains("permissions.yaml exists"), "{d:?}");
}

#[test]
fn kiro_permissions_locations_point_at_rule_fields() {
    let content = "# header\nrules:\n  - capability: shell\n    effect: allow\n  - capability: fs_read\n    effect: nope\n";
    let d = only(&validate(content), "KR-PM-003");
    assert_eq!((d.line, d.column), (6, 5));
    let content = "rules:\n  - effect: allow\n    capability: 7\n";
    let d = only(&validate(content), "KR-PM-002");
    assert_eq!((d.line, d.column), (3, 5));
    // Flow style falls back to the rules key.
    let d = only(
        &validate("rules: [{capability: nope, effect: allow}]"),
        "KR-PM-002",
    );
    assert_eq!(d.line, 1);
}

#[test]
fn kiro_permissions_disabled_rule_is_respected() {
    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = RULE_IDS.iter().map(|id| id.to_string()).collect();
    for content in [
        "rules: [{capability: nope, effect: nope, match: true}]",
        "rules: [{capability: shell, effect: evaluate, x: 1, match: []}]",
        "policies: 1\nrules: [",
        "rules: [",
    ] {
        assert!(
            KiroPermissionsValidator
                .validate(Path::new(PATH), content, &config)
                .is_empty(),
            "{content}"
        );
    }
}

#[test]
fn kiro_permissions_metadata() {
    assert_eq!(KiroPermissionsValidator.metadata().rule_ids, RULE_IDS);
}
