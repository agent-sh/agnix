use agnix_core::{LintConfig, ValidatorRegistry, validate_content};
use std::path::Path;

#[test]
fn kiro_234_accepts_documented_workflow_launch_choices() {
    for launch in ["when-asked", "large-tasks", "delegate-all"] {
        let content = serde_json::json!({"chat.workflowLaunch": launch});
        let diagnostics = validate_content(
            Path::new(".kiro/settings/cli.json"),
            &content.to_string(),
            &LintConfig::default(),
            &ValidatorRegistry::with_defaults(),
        );
        assert!(diagnostics.is_empty(), "{launch}: {diagnostics:?}");
    }
    for (value, rule) in [
        (serde_json::json!(true), "KR-SET-008"),
        (serde_json::json!("automatic"), "KR-SET-009"),
    ] {
        let diagnostics = validate_content(
            Path::new(".kiro/settings/cli.json"),
            &serde_json::json!({"chat.workflowLaunch":value}).to_string(),
            &LintConfig::default(),
            &ValidatorRegistry::with_defaults(),
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == rule),
            "{diagnostics:?}"
        );
    }
}

#[test]
fn gemini_066_accepts_updated_skill_delimiters_and_private_network_oauth_option() {
    let registry = ValidatorRegistry::with_defaults();
    for content in [
        "---\nname: review\ndescription: Use when reviewing code\n---\nReview code.\n",
        "\u{feff}--- \t\nname: review\ndescription: Use when reviewing code\n--- \t\nReview code.\n",
        "\u{feff}--- \t\r\nname: review\r\ndescription: Use when reviewing code\r\n--- \t\r\nReview code.\r\n",
    ] {
        for (path, config) in [
            (".gemini/skills/review/SKILL.md", LintConfig::default()),
            (
                ".agents/skills/review/SKILL.md",
                LintConfig::builder()
                    .tools(vec!["gemini-cli".into()])
                    .build()
                    .unwrap(),
            ),
        ] {
            let diagnostics = validate_content(Path::new(path), content, &config, &registry);
            assert!(
                diagnostics
                    .iter()
                    .all(|d| !["AS-001", "AS-003", "AS-016"].contains(&d.rule.as_str())),
                "{path}: {diagnostics:?}"
            );
        }
    }
    let diagnostics = validate_content(
        Path::new(".gemini/settings.json"),
        r#"{"mcpServers":{"enterprise":{"httpUrl":"https://mcp.example.com/mcp","oauth":{"enabled":true,"allowPrivateNetwork":true}}}}"#,
        &LintConfig::default(),
        &registry,
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn gemini_skills_do_not_inherit_claude_only_rules() {
    let content = "---\nname: claude-review\ndescription: Use when reviewing code\ncontext: not-a-claude-context\n---\nReview code.\n";
    for (path, config) in [
        (
            ".gemini/skills/claude-review/SKILL.md",
            LintConfig::default(),
        ),
        (
            ".agents/skills/claude-review/SKILL.md",
            LintConfig::builder()
                .tools(vec!["gemini-cli".into()])
                .build()
                .unwrap(),
        ),
    ] {
        let diagnostics = validate_content(
            Path::new(path),
            content,
            &config,
            &ValidatorRegistry::with_defaults(),
        );
        assert!(
            diagnostics.iter().all(|d| !d.rule.starts_with("CC-SK-")),
            "{diagnostics:?}"
        );
    }
}

#[test]
fn cline_4126_persisted_rule_toggle_is_accepted() {
    let diagnostics = validate_content(
        Path::new(".cline/rules/review.md"),
        "---\nname: review\ndisabled: true\npaths:\n  - \"**/*.rs\"\n---\nCheck unsafe blocks.\n",
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn codex_0165_accepts_tagged_feature_flags_in_all_config_formats() {
    for (file, content) in [
        (
            ".codex/config.toml",
            "[auto_review]\ntimeout_ms = 0\n[features]\nguardian_recorded_items = true\nprediction_preserve_parent_tools = true\ntui_multi_backend = true\n[features.guardianv3]\nenabled = true\n",
        ),
        (
            ".codex/config.json",
            r#"{"auto_review":{"timeout_ms":0},"features":{"guardian_recorded_items":true,"prediction_preserve_parent_tools":true,"tui_multi_backend":true,"guardianv3":{"enabled":true}}}"#,
        ),
        (
            ".codex/config.yaml",
            "auto_review:\n  timeout_ms: 0\nfeatures:\n  guardian_recorded_items: true\n  prediction_preserve_parent_tools: true\n  tui_multi_backend: true\n  guardianv3:\n    enabled: true\n",
        ),
    ] {
        let diagnostics = validate_content(
            Path::new(file),
            content,
            &LintConfig::default(),
            &ValidatorRegistry::with_defaults(),
        );
        assert!(diagnostics.is_empty(), "{file}: {diagnostics:?}");
    }
}

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

#[test]
fn codex_0163_accepts_new_root_keys_and_feature_flags() {
    let diagnostics = validate_content(
        Path::new(".codex/config.toml"),
        "include_environment_context_time = true\njs_repl_node_module_dirs = [\"node_modules\"]\njs_repl_node_path = \"/usr/bin/node\"\n[features]\ninstant_interrupt = true\nrecommended_plugins = true\nsubagent_default_context_limits = true\nultrafast_mode = true\nwrite_stdin_approval = true\n",
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn claude_code_021299_accepts_idle_compaction_setting() {
    let diagnostics = validate_content(
        Path::new(".claude/settings.json"),
        "{\"idleCompaction\": false}",
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(
        diagnostics.iter().all(|d| d.rule != "CC-SET-033"),
        "{diagnostics:?}"
    );

    let diagnostics = validate_content(
        Path::new(".claude/settings.json"),
        "{\"idleCompaction\": \"false\"}",
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(
        diagnostics.iter().any(|d| d.rule == "CC-SET-033"),
        "{diagnostics:?}"
    );
}

#[test]
fn kiro_v3_standalone_hook_file_validates() {
    let content = serde_json::json!({
        "version": "v1",
        "hooks": [
            {
                "name": "lint-on-save",
                "trigger": "PostFileSave",
                "matcher": "\\.(ts|tsx)$",
                "action": { "type": "command", "command": "npx eslint --fix" },
                "timeout": 30,
                "enabled": true
            },
            {
                "name": "startup-context",
                "trigger": "SessionStart",
                "action": { "type": "command", "command": "git status" }
            }
        ]
    })
    .to_string();
    let diagnostics = validate_content(
        Path::new(".kiro/hooks/lint-on-save.json"),
        &content,
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn kiro_v3_standalone_hook_file_flags_bad_trigger() {
    let content = serde_json::json!({
        "version": "v1",
        "hooks": [
            { "name": "bad", "trigger": "onFileChange", "action": { "type": "command", "command": "echo" } }
        ]
    })
    .to_string();
    let diagnostics = validate_content(
        Path::new(".kiro/hooks/bad.json"),
        &content,
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(
        diagnostics.iter().any(|d| d.rule == "KR-HK-012"),
        "{diagnostics:?}"
    );
}

#[test]
fn kiro_v3_agent_profile_fields_and_migration_rule() {
    let upgraded = serde_json::json!({
        "name": "backend-dev",
        "tools": ["read", "write", "shell", "web"],
        "excludedTools": ["knowledge"],
        "includeMcpJson": true,
        "includePowers": false,
        "resources": ["file://./ARCHITECTURE.md", "skill://backend-patterns"],
        "permissions": {
            "rules": [
                { "capability": "shell", "match": ["npm *", "node *"], "effect": "allow" },
                { "capability": "web_fetch", "effect": "allow" },
                { "capability": "web_search", "effect": "deny" }
            ]
        },
        "prompt": "Backend work."
    })
    .to_string();
    let diagnostics = validate_content(
        Path::new(".kiro/agents/backend-dev.json"),
        &upgraded,
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    let legacy = serde_json::json!({
        "name": "legacy",
        "tools": ["fs_read"],
        "allowedTools": ["fs_read"]
    })
    .to_string();
    let diagnostics = validate_content(
        Path::new(".kiro/agents/legacy.json"),
        &legacy,
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(
        diagnostics.iter().any(|d| d.rule == "KR-AG-015"),
        "{diagnostics:?}"
    );
}
