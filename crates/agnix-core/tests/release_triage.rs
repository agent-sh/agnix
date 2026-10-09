use agnix_core::{LintConfig, ValidatorRegistry, validate_content};
use std::path::Path;

#[test]
fn codex_0161_accepts_daybreak_opt_in() {
    let diagnostics = validate_content(
        Path::new(".codex/config.toml"),
        "daybreak = true\n[features]\ncli_daybreak = true\n",
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn kiro_228_hook_matchers_accept_categories_and_mcp_patterns() {
    for matcher in ["shell", "read", "write", "mcp__github__*", "execute_bash"] {
        let content = serde_json::json!({
            "name": "demo",
            "description": "Demonstrate hook matching",
            "prompt": "Perform the task",
            "tools": ["*"],
            "hooks": { "preToolUse": [{ "matcher": matcher, "command": "echo demo" }] }
        });
        let diagnostics = validate_content(
            Path::new(".kiro/agents/demo.json"),
            &content.to_string(),
            &LintConfig::default(),
            &ValidatorRegistry::with_defaults(),
        );
        assert!(diagnostics.is_empty(), "{matcher}: {diagnostics:?}");
    }
}

#[test]
fn plugin_names_distinguish_style_from_forbidden_characters() {
    for (name, expected) in [
        ("Demo_plugin", agnix_core::DiagnosticLevel::Warning),
        ("demo:bad", agnix_core::DiagnosticLevel::Error),
        ("demo bad", agnix_core::DiagnosticLevel::Error),
        ("demo\u{202e}bad", agnix_core::DiagnosticLevel::Error),
    ] {
        let content = serde_json::json!({"name": name, "version": "1.0.0", "description": "Demo"});
        let diagnostics = validate_content(
            Path::new(".claude-plugin/plugin.json"),
            &content.to_string(),
            &LintConfig::default(),
            &ValidatorRegistry::with_defaults(),
        );
        let diagnostic = diagnostics.iter().find(|d| d.rule == "CC-PL-016").unwrap();
        assert_eq!(diagnostic.level, expected, "{name}");
    }
}

#[cfg(feature = "filesystem")]
#[test]
fn user_skill_resolution_uses_the_platform_home() {
    let filesystem = std::sync::Arc::new(agnix_core::MockFileSystem::new());
    let home = dirs::home_dir().expect("test runner has a user home");
    filesystem.add_file(home.join(".claude/skills/platform-demo/SKILL.md"), "demo");
    let config = LintConfig::builder().fs(filesystem).build().unwrap();
    let diagnostics = validate_content(
        Path::new("project/.claude/agents/demo.md"),
        "---\nname: demo\ndescription: Demonstrate user skills\nskills: [platform-demo]\n---\nPerform the task\n",
        &config,
        &ValidatorRegistry::with_defaults(),
    );
    assert!(
        diagnostics.iter().all(|d| d.rule != "CC-AG-005"),
        "{diagnostics:?}"
    );
}

#[test]
fn codex_0162_accepts_new_tui_and_root_keys() {
    let diagnostics = validate_content(
        Path::new(".codex/config.toml"),
        "goals = { enabled = true }\nmcp_optional_startup_grace_ms = 500\nmodel_post_turn_compact_threshold_percent = 80\n[tui]\nmouse_scroll_speed = 3\nagents_overview_grouping = \"project\"\n[tui.keymap.agents]\ntoggle_pin = \"p\"\n",
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}
