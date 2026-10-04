//! Configuration regressions from the 2026-10-04 primary-source review (#1615).

use agnix_core::{DiagnosticLevel, LintConfig, validate_project};
use std::fs;

#[test]
fn agent_experimental_cache_ttl_loads_through_project_validation() {
    let temp = tempfile::TempDir::new().unwrap();
    let agents = temp.path().join(".claude/agents");
    fs::create_dir_all(&agents).unwrap();
    // Both recognized TTLs work. Other TTLs are ignored by Claude Code, so
    // they must not turn a loadable agent into a parse or unknown-field error.
    for (name, experimental) in [
        ("short-cache", "{cacheTtl: 5m}"),
        ("long-cache", "{cacheTtl: 1h}"),
        ("ignored-cache", "{cacheTtl: 10m}"),
        ("malformed-cache", "5m"),
        ("typo-cache", "{cacheTtl: 5m}"),
    ] {
        let key = if name == "typo-cache" {
            "experimantal"
        } else {
            "experimental"
        };
        fs::write(
            agents.join(format!("{name}.md")),
            format!(
                "---\nname: {name}\ndescription: Review code for cache behavior\n{key}: {experimental}\n---\nReview code.\n"
            ),
        )
        .unwrap();
    }
    let result = validate_project(temp.path(), &LintConfig::default()).unwrap();
    for name in ["short-cache", "long-cache", "ignored-cache"] {
        assert!(
            result.diagnostics.iter().all(|d| {
                !d.file.ends_with(format!("{name}.md"))
                    || !["CC-AG-007", "CC-AG-019"].contains(&d.rule.as_str())
            }),
            "loadable agent {name}: {:?}",
            result.diagnostics
        );
    }
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| { d.file.ends_with("malformed-cache.md") && d.rule == "CC-AG-007" })
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| { d.file.ends_with("typo-cache.md") && d.rule == "CC-AG-019" })
    );
}

#[test]
fn plugin_bundle_urls_and_version_advice_follow_manifest_contract() {
    for (field, bundle, valid) in [
        ("mcpServers", "https://example.com/server.mcpb", true),
        ("mcpServers", "https://example.com/server.dxt", true),
        ("mcpServers", "https://example.com/server.json", false),
        ("mcpServers", "http://example.com/server.mcpb", false),
        (
            "mcpServers",
            "https://example.com/server.mcpb?download=1",
            false,
        ),
        (
            "mcpServers",
            "https://example.com/server.mcpb#bundle",
            false,
        ),
        ("mcpServers", "https://bad host/server.mcpb", false),
        ("commands", "https://example.com/server.mcpb", false),
        ("mcpServers", "../server.mcpb", false),
    ] {
        let temp = tempfile::TempDir::new().unwrap();
        let manifest = temp.path().join(".claude-plugin/plugin.json");
        fs::create_dir_all(manifest.parent().unwrap()).unwrap();
        let mut config = serde_json::json!({
            "name": "bundle-example", "description": "Bundle example",
            "version": "release-2026-10"
        });
        config[field] = bundle.into();
        fs::write(&manifest, config.to_string()).unwrap();
        let result = validate_project(temp.path(), &LintConfig::default()).unwrap();
        assert_eq!(
            result.diagnostics.iter().any(|d| d.rule == "CC-PL-007"),
            !valid,
            "{field}: {bundle}: {:?}",
            result.diagnostics
        );
        let version = result
            .diagnostics
            .iter()
            .find(|d| d.rule == "CC-PL-003")
            .unwrap();
        assert_eq!(version.level, DiagnosticLevel::Warning);
    }
}

#[test]
fn hook_matchers_accept_current_event_vocabulary_and_still_catch_typos() {
    let temp = tempfile::TempDir::new().unwrap();
    let settings = temp.path().join(".claude/settings.json");
    fs::create_dir_all(settings.parent().unwrap()).unwrap();
    let mut hooks = serde_json::Map::new();
    for (event, matchers) in [
        (
            "DirectoryAdded",
            &["slash_command", "register_repo_root"][..],
        ),
        (
            "Notification",
            &[
                "quota_auto_resume_fired",
                "quota_auto_resume_stale",
                "quota_auto_resume_disabled",
            ][..],
        ),
        (
            "StopFailure",
            &["account_on_hold", "cloud_credential_error"][..],
        ),
    ] {
        hooks.insert(event.into(), serde_json::Value::Array(matchers.iter().map(|matcher| {
            serde_json::json!({"matcher": matcher, "hooks": [{"type": "command", "command": "echo ready", "timeout": 10}]})
        }).collect()));
    }
    fs::write(&settings, serde_json::json!({"hooks": hooks}).to_string()).unwrap();
    let result = validate_project(temp.path(), &LintConfig::default()).unwrap();
    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| !["CC-HK-004", "CC-HK-018", "CC-HK-025"].contains(&d.rule.as_str())),
        "{:?}",
        result.diagnostics
    );
    fs::write(&settings, serde_json::json!({"hooks": {"DirectoryAdded": [{"matcher": "slash_comand", "hooks": [{"type": "command", "command": "echo ready", "timeout": 10}]}]}}).to_string()).unwrap();
    let result = validate_project(temp.path(), &LintConfig::default()).unwrap();
    assert!(result.diagnostics.iter().any(|d| d.rule == "CC-HK-025"));
}

#[test]
fn once_only_has_effect_in_skill_hooks() {
    let temp = tempfile::TempDir::new().unwrap();
    // macOS temporary roots can be symlinks; diagnostics use canonical paths.
    let root = temp.path().canonicalize().unwrap();
    let skill = root.join(".claude/skills/cache-review/SKILL.md");
    let agent = root.join(".claude/agents/cache-review.md");
    let settings = root.join(".claude/settings.json");
    let body = "---\nname: cache-review\ndescription: Review code for cache behavior\nhooks:\n  Stop:\n    - hooks:\n        - type: command\n          command: echo ready\n          timeout: 10\n          once: true\n---\nReview code.\n";
    for file in [&skill, &agent] {
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, body).unwrap();
    }
    fs::write(&settings, serde_json::json!({"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "echo ready", "timeout": 10, "once": true}]}]}}).to_string()).unwrap();
    let result = validate_project(&root, &LintConfig::default()).unwrap();
    let once: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.rule == "CC-HK-014")
        .collect();
    assert_eq!(once.len(), 2, "{:?}", result.diagnostics);
    assert!(once.iter().any(|d| d.file == agent));
    assert!(once.iter().any(|d| d.file == settings));
    assert!(once.iter().all(|d| d.file != skill));
    let config = LintConfig::builder()
        .disable_rule("CC-HK-014")
        .build()
        .unwrap();
    let result = validate_project(&root, &config).unwrap();
    assert!(result.diagnostics.iter().all(|d| d.rule != "CC-HK-014"));
}

#[test]
fn hook_and_plugin_fix_metadata_match_emitted_fixes() {
    let rules: serde_json::Value = serde_json::from_str(agnix_rules::rules_json()).unwrap();
    let rule = |id: &str| {
        rules["rules"]
            .as_array()
            .unwrap()
            .iter()
            .find(|rule| rule["id"] == id)
            .unwrap()
    };
    assert_eq!(rule("CC-HK-014")["fix"]["autofix"], false);
    assert_eq!(rule("CC-PL-003")["fix"]["fix_safety"], "unsafe");

    let project = tempfile::TempDir::new().unwrap();
    let manifest = project.path().join(".claude-plugin/plugin.json");
    fs::create_dir_all(manifest.parent().unwrap()).unwrap();
    fs::write(
        &manifest,
        r#"{"name":"partial-version","description":"Plugin example","version":"1.0"}"#,
    )
    .unwrap();
    let result = validate_project(project.path(), &LintConfig::default()).unwrap();
    let version = result
        .diagnostics
        .iter()
        .find(|d| d.rule == "CC-PL-003")
        .unwrap();
    assert!(!version.fixes.is_empty());
    assert!(version.fixes.iter().all(|fix| !fix.safe));
}
