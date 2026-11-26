use agnix_core::{LintConfig, ValidatorRegistry, validate_content};
use std::path::Path;

#[test]
fn codex_0166_accepts_tagged_features_in_root_and_profiles() {
    let features = serde_json::json!({
        "app_server_websocket_compression": true,
        "critical_path_timing": true,
        "executor_capability_discovery_v2": true,
        "guardian_native_multi_agent_history": false,
        "mcp_connection_sharing": true,
        "mcp_tool_list_changed": true,
        "multi_env_copy_file": true,
        "omit_skill_descriptions": true,
        "plugin_discovery_prefetch": false,
        "runtime_agent_instances": {"enabled": true},
        "simplified_selection_hints": true,
        "skill_catalog_deltas": true,
        "subagent_thread_environments": true,
        "untrusted_as_tool_output": true,
        "wake_on_agent_messages": true
    });
    let value =
        serde_json::json!({"features":features,"profiles":{"review":{"features":features}}});
    for (file, content) in [
        (".codex/config.json", value.to_string()),
        (".codex/config.yaml", serde_yaml::to_string(&value).unwrap()),
        (".codex/config.toml", toml::to_string(&value).unwrap()),
    ] {
        let diagnostics = validate_content(
            Path::new(file),
            &content,
            &LintConfig::default(),
            &ValidatorRegistry::with_defaults(),
        );
        // The schema retains inline profiles for compatibility, while
        // CDX-APP-003 still advises migrating them to standalone files.
        assert!(
            diagnostics.iter().all(|d| d.rule == "CDX-APP-003"),
            "{file}: {diagnostics:?}"
        );
    }
}

#[test]
fn codex_0167_accepts_tagged_feature_flags_in_all_config_formats() {
    // rust-v0.167.0 added seven of these flags to config.schema.json and
    // rust-v0.167.2 added incremental_tools_legacy_gcp (the flat-bool opt-out
    // for incremental tool updates on Google Cloud Vertex AI); the 0.167
    // releases never name the other eight in their notes, so the known-flag
    // set behind CDX-CFG-011 is reconciled against the tagged schemas.
    let features = serde_json::json!({
        "apps_early_resource_discovery": true,
        "code_mode_host_output_truncation": true,
        "gcp_setup_wizard": false,
        "incremental_exec_tools": true,
        "incremental_tools_legacy": false,
        "incremental_tools_legacy_gcp": false,
        "interrupt_on_cancel": true,
        "suppress_model_change_announcements": true,
        "tui_extensions_statusline": true
    });
    let value = serde_json::json!({"features": features});
    for (file, content) in [
        (".codex/config.json", value.to_string()),
        (".codex/config.yaml", serde_yaml::to_string(&value).unwrap()),
        (".codex/config.toml", toml::to_string(&value).unwrap()),
    ] {
        let diagnostics = validate_content(
            Path::new(file),
            &content,
            &LintConfig::default(),
            &ValidatorRegistry::with_defaults(),
        );
        assert!(diagnostics.is_empty(), "{file}: {diagnostics:?}");
    }
}

#[test]
fn codex_0169_accepts_tagged_feature_flags_and_tui_keys() {
    // The tagged rust-v0.168.0 schema adds nine [features] flags and removes
    // both incremental_tools_legacy names; rust-v0.169.0 adds three more,
    // restores both legacy names, and the Tui definition gains
    // collapse_turn_details, default_hosts and sidebar_width_percent. None are
    // named in the release notes, so the allowlists behind CDX-CFG-011 and
    // CDX-CFG-006 are reconciled against the tagged schemas.
    let features = serde_json::json!({
        "agent_message_board_mcp": true,
        "async_compaction_root_only": true,
        "cli_resume_account_check": true,
        "command_sessions_world_state": true,
        "daybreak_program_propagation": true,
        "gateway_setup_wizard": false,
        "guardian_denial_reconsideration": true,
        "guardian_include_additional_context": true,
        "hosted_search_policy": true,
        "image_generation_animation": true,
        "incremental_tools_legacy": false,
        "incremental_tools_legacy_gcp": false,
        "mcp_lazy_stdio_startup": true,
        "tui_workspace": true
    });
    let tui = serde_json::json!({
        "collapse_turn_details": true,
        "default_hosts": "example.internal",
        "sidebar_width_percent": 25
    });
    let value = serde_json::json!({"features": features, "tui": tui});
    for (file, content) in [
        (".codex/config.json", value.to_string()),
        (".codex/config.yaml", serde_yaml::to_string(&value).unwrap()),
        (".codex/config.toml", toml::to_string(&value).unwrap()),
    ] {
        let diagnostics = validate_content(
            Path::new(file),
            &content,
            &LintConfig::default(),
            &ValidatorRegistry::with_defaults(),
        );
        assert!(diagnostics.is_empty(), "{file}: {diagnostics:?}");
    }
}

#[test]
fn codex_0171_accepts_tagged_feature_flags_and_tui_key() {
    // The tagged rust-v0.170.0 schema adds multi_environment_mcp to
    // [features]; rust-v0.171.0 adds eight more flags plus the
    // tui.status_line_provider preference. None are named in the release
    // notes, so the allowlists behind CDX-CFG-011 and CDX-CFG-006 are
    // reconciled against the tagged schemas.
    let features = serde_json::json!({
        "catalog_apps_instructions": true,
        "file_backed_tool_audio": true,
        "guardian_nested_approval_context": true,
        "mcp_shared_repl_cwd": true,
        "multi_environment_mcp": true,
        "per_environment_context": true,
        "restored_history_prewarm": true,
        "safe_image_decoder": true,
        "utc_environment_date": true
    });
    let tui = serde_json::json!({"status_line_provider": "builtin"});
    let value = serde_json::json!({"features": features, "tui": tui});
    for (file, content) in [
        (".codex/config.json", value.to_string()),
        (".codex/config.yaml", serde_yaml::to_string(&value).unwrap()),
        (".codex/config.toml", toml::to_string(&value).unwrap()),
    ] {
        let diagnostics = validate_content(
            Path::new(file),
            &content,
            &LintConfig::default(),
            &ValidatorRegistry::with_defaults(),
        );
        assert!(diagnostics.is_empty(), "{file}: {diagnostics:?}");
    }
}

#[test]
fn grok_1_0_89_accepts_changelog_reconciled_config_surfaces() {
    // 1.0.79 adds `/vim-mode all` (keys.vim = "all"), 1.0.82 the `minimal`
    // built-in sandbox profile, 1.0.88 the MCP `headers_helper` command
    // (x.ai changelog). The reconciled contract accepts all three.
    let diagnostics = validate_content(
        Path::new(".grok/config.toml"),
        "[mcp_servers.helper]\nurl = \"https://mcp.example.com/mcp\"\nheaders_helper = \"get-headers.sh\"\n",
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    // keys.vim = "all" no longer violates the enum (GRK-CFG-003); the
    // project-file warning for the [keys] section (GRK-CFG-002) is expected.
    for vim in ["all", "nav", "off"] {
        let diagnostics = validate_content(
            Path::new(".grok/config.toml"),
            &format!("[keys]\nvim = \"{vim}\"\n"),
            &LintConfig::default(),
            &ValidatorRegistry::with_defaults(),
        );
        assert!(
            !diagnostics.iter().any(|d| d.rule == "GRK-CFG-003"),
            "{vim}: {diagnostics:?}"
        );
    }
    let diagnostics = validate_content(
        Path::new(".grok/config.toml"),
        "[keys]\nvim = \"bogus\"\n",
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(
        diagnostics.iter().any(|d| d.rule == "GRK-CFG-003"),
        "{diagnostics:?}"
    );

    // A wrong-typed headers_helper is GRK-MCP-001's finding, and a custom
    // sandbox profile can no longer redefine the 1.0.82 `minimal` built-in.
    let diagnostics = validate_content(
        Path::new(".grok/config.toml"),
        "[mcp_servers.helper]\nurl = \"https://mcp.example.com/mcp\"\nheaders_helper = 42\n",
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(
        diagnostics.iter().any(|d| d.rule == "GRK-MCP-001"),
        "{diagnostics:?}"
    );
    let diagnostics = validate_content(
        Path::new(".grok/sandbox.toml"),
        "[profiles.minimal]\nextends = \"workspace\"\n",
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(
        diagnostics.iter().any(|d| d.rule == "GRK-SBX-001"),
        "{diagnostics:?}"
    );
}

#[test]
fn grok_version_pins_gate_the_1_0_82_minimal_sandbox_builtin() {
    // GRK-SBX-001: `minimal` is a builtin only from 1.0.82 (x.ai
    // changelog). A pin below 1.0.82 may still define a custom
    // `profiles.minimal`; 1.0.82 and later (and unpinned files, checked
    // against the latest release) reserve the name.
    fn pinned(version: &str) -> LintConfig {
        LintConfig::builder()
            .tool_versions(agnix_core::config::ToolVersions {
                grok_code: Some(version.to_string()),
                ..Default::default()
            })
            .build()
            .unwrap()
    }
    let sandbox = "[profiles.minimal]\nextends = \"workspace\"\n";
    for version in ["1.0.81", "1.0.77"] {
        let diagnostics = validate_content(
            Path::new(".grok/sandbox.toml"),
            sandbox,
            &pinned(version),
            &ValidatorRegistry::with_defaults(),
        );
        assert!(
            !diagnostics.iter().any(|d| d.rule == "GRK-SBX-001"),
            "{version}: {diagnostics:?}"
        );
    }
    for version in ["1.0.82", "1.0.89"] {
        let diagnostics = validate_content(
            Path::new(".grok/sandbox.toml"),
            sandbox,
            &pinned(version),
            &ValidatorRegistry::with_defaults(),
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "GRK-SBX-001"),
            "{version}: {diagnostics:?}"
        );
    }
    // The pre-1.0.82 builtins stay reserved whatever the pin.
    for version in ["1.0.77", "1.0.82"] {
        let diagnostics = validate_content(
            Path::new(".grok/sandbox.toml"),
            "[profiles.strict]\nextends = \"workspace\"\n",
            &pinned(version),
            &ValidatorRegistry::with_defaults(),
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "GRK-SBX-001"),
            "{version}: {diagnostics:?}"
        );
    }
}

#[test]
fn grok_version_pins_gate_the_1_0_88_headers_helper_field() {
    // GRK-MCP-001: `headers_helper` is read only from 1.0.88 (x.ai
    // changelog). A pin below 1.0.88 does not read the field, so any
    // value keeps the unknown-field treatment; 1.0.88 and later (and
    // unpinned files) require the documented string type.
    fn pinned(version: &str) -> LintConfig {
        LintConfig::builder()
            .tool_versions(agnix_core::config::ToolVersions {
                grok_code: Some(version.to_string()),
                ..Default::default()
            })
            .build()
            .unwrap()
    }
    let wrong_type =
        "[mcp_servers.helper]\nurl = \"https://mcp.example.com/mcp\"\nheaders_helper = 42\n";
    for version in ["1.0.87", "1.0.82"] {
        let diagnostics = validate_content(
            Path::new(".grok/config.toml"),
            wrong_type,
            &pinned(version),
            &ValidatorRegistry::with_defaults(),
        );
        assert!(
            !diagnostics.iter().any(|d| d.rule == "GRK-MCP-001"),
            "{version}: {diagnostics:?}"
        );
    }
    for version in ["1.0.88", "1.0.89"] {
        let diagnostics = validate_content(
            Path::new(".grok/config.toml"),
            wrong_type,
            &pinned(version),
            &ValidatorRegistry::with_defaults(),
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "GRK-MCP-001"),
            "{version}: {diagnostics:?}"
        );
    }
    // The documented string form passes on both sides of the boundary.
    for version in ["1.0.87", "1.0.88"] {
        let diagnostics = validate_content(
            Path::new(".grok/config.toml"),
            "[mcp_servers.helper]\nurl = \"https://mcp.example.com/mcp\"\nheaders_helper = \"get-headers.sh\"\n",
            &pinned(version),
            &ValidatorRegistry::with_defaults(),
        );
        assert!(diagnostics.is_empty(), "{version}: {diagnostics:?}");
    }
}

#[test]
fn opencode_223_accepts_separate_managed_policy_actions() {
    for version in [None, Some("2.2.3")] {
        let config = LintConfig::builder()
            .tool_versions(agnix_core::config::ToolVersions {
                opencode: version.map(str::to_string),
                ..Default::default()
            })
            .build()
            .unwrap();
        for field in ["policies", "experimental"] {
            let policies = serde_json::json!([
                {"action":"integration.connect","resource":"github","effect":"allow"},
                {"action":"integration.use","resource":"github#work","effect":"deny"},
                {"action":"mcp.use","resource":"docs","effect":"deny"},
                {"action":"skill.use","resource":"review","effect":"allow"},
                {"action":"plugin.use","resource":"npm:plugin","effect":"deny"}
            ]);
            let content = if field == "policies" {
                serde_json::json!({"policies": policies})
            } else {
                serde_json::json!({"experimental":{"policies":policies}})
            };
            let diagnostics = validate_content(
                Path::new("opencode.jsonc"),
                &content.to_string(),
                &config,
                &ValidatorRegistry::with_defaults(),
            );
            assert!(
                diagnostics.is_empty(),
                "{version:?}/{field}: {diagnostics:?}"
            );
        }
    }
}

#[test]
fn opencode_223_policy_actions_preserve_old_version_and_invalid_action_checks() {
    for (version, action) in [
        ("2.2.2", "mcp.use"),
        ("2.2.3", "mcp.ues"),
        ("1.19.5", "skill.use"),
    ] {
        let config = LintConfig::builder()
            .tool_versions(agnix_core::config::ToolVersions {
                opencode: Some(version.into()),
                ..Default::default()
            })
            .build()
            .unwrap();
        let content = serde_json::json!({"experimental":{"policies":[{"action":action,"resource":"docs","effect":"deny"}]}});
        let diagnostics = validate_content(
            Path::new("opencode.json"),
            &content.to_string(),
            &config,
            &ValidatorRegistry::with_defaults(),
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "OC-POL-001"),
            "{version}: {diagnostics:?}"
        );
    }
}

#[test]
fn claude_21314_accepts_write_boundary_and_monitor_hook_conditions() {
    let content = r#"{"permissions":{"blockWritesOutsideWorkingDirectories":true},"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"echo check","if":"Monitor(tail *)"}]}]}}"#;
    let diagnostics = validate_content(
        Path::new(".claude/settings.json"),
        content,
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn opencode_223_shadow_checks_normalize_legacy_actions_without_crossing_integrations() {
    for prefix in ["mcp", "skill", "plugin"] {
        let content = serde_json::json!({"policies":[
            {"action":"integration.use","resource":format!("{prefix}:docs"),"effect":"deny"},
            {"action":format!("{prefix}.use"),"resource":"*","effect":"allow"}
        ]});
        let diagnostics = validate_content(
            Path::new("opencode.json"),
            &content.to_string(),
            &LintConfig::default(),
            &ValidatorRegistry::with_defaults(),
        );
        assert_eq!(
            diagnostics.iter().filter(|d| d.rule == "OC-PM-003").count(),
            1,
            "{prefix}: {diagnostics:?}"
        );
        assert!(
            !diagnostics.iter().any(|d| d.rule == "OC-POL-001"),
            "{diagnostics:?}"
        );
    }
    let content = r#"{"policies":[{"action":"integration.use","resource":"mcp:docs","effect":"deny"},{"action":"integration.use","resource":"*","effect":"allow"}]}"#;
    let diagnostics = validate_content(
        Path::new("opencode.json"),
        content,
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn opencode_221_accepts_timezone_and_released_experimental_settings() {
    let registry = ValidatorRegistry::with_defaults();
    let pinned = LintConfig::builder()
        .tool_versions(agnix_core::config::ToolVersions {
            opencode: Some("2.2.1".into()),
            ..Default::default()
        })
        .build()
        .unwrap();
    for image_generate in [
        serde_json::json!(true),
        serde_json::json!({"model":"openai/gpt-image-2"}),
    ] {
        let content = serde_json::json!({
            "timezone":"Asia/Jerusalem",
            "experimental":{"image_generate":image_generate,"schedules":true}
        })
        .to_string();
        for config in [&pinned, &LintConfig::default()] {
            let diagnostics =
                validate_content(Path::new("opencode.jsonc"), &content, config, &registry);
            assert!(diagnostics.is_empty(), "{diagnostics:?}");
        }
    }
}

#[test]
fn opencode_221_timezone_keeps_v1_and_unknown_key_checks() {
    let registry = ValidatorRegistry::with_defaults();
    let pinned_v1 = LintConfig::builder()
        .tool_versions(agnix_core::config::ToolVersions {
            opencode: Some("1.19.5".into()),
            ..Default::default()
        })
        .build()
        .unwrap();
    let diagnostics = validate_content(
        Path::new("opencode.json"),
        r#"{"timezone":"UTC"}"#,
        &pinned_v1,
        &registry,
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| d.rule == "OC-004" && d.message.contains("OpenCode 2")),
        "{diagnostics:?}"
    );
    let diagnostics = validate_content(
        Path::new("opencode.json"),
        r#"{"timzone":"UTC"}"#,
        &LintConfig::default(),
        &registry,
    );
    assert!(
        diagnostics.iter().any(|d| d.rule == "OC-004"),
        "{diagnostics:?}"
    );
}

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

#[test]
fn claude_code_021332_accepts_pre_skill_shell_command_hook() {
    // v2.1.332: "Added a `PreSkillShellCommand` hook event that can refuse a
    // skill or command's inline shell commands as it loads". The hooks
    // reference evaluates `if` on the event and matches it on the tool that
    // runs the command.
    let registry = ValidatorRegistry::with_defaults();
    let settings = serde_json::json!({"hooks": {"PreSkillShellCommand": [
        {
            "matcher": "Bash|PowerShell",
            "hooks": [{"type": "command", "command": "echo x", "if": "Bash(git *)", "onFailure": "block"}]
        },
        {"hooks": [{"type": "command", "command": "echo y"}]}
    ]}})
    .to_string();
    let hook_file = serde_json::json!({
        "PreSkillShellCommand": [{"hooks": [{"type": "command", "command": "echo y"}]}]
    })
    .to_string();
    for (path, content) in [
        (".claude/settings.json", settings.as_str()),
        (".claude/hooks/hooks.json", hook_file.as_str()),
    ] {
        let diagnostics =
            validate_content(Path::new(path), content, &LintConfig::default(), &registry);
        assert!(
            diagnostics
                .iter()
                .all(|d| d.rule != "CC-HK-001" && d.rule != "CC-HK-021"),
            "{path}: {diagnostics:?}"
        );
    }
}

#[test]
fn claude_code_pins_below_2_1_332_keep_the_error_for_pre_skill_shell_command() {
    // "on 2.1.331 and older, a settings file naming it opens a dialog at
    // start" (v2.1.332 release notes): the hook never runs there.
    fn pinned(version: &str) -> LintConfig {
        LintConfig::builder()
            .tool_versions(agnix_core::config::ToolVersions {
                claude_code: Some(version.to_string()),
                ..Default::default()
            })
            .build()
            .unwrap()
    }
    let registry = ValidatorRegistry::with_defaults();
    let settings =
        r#"{"hooks":{"PreSkillShellCommand":[{"hooks":[{"type":"command","command":"echo x"}]}]}}"#;
    for version in ["2.1.330", "2.1.331"] {
        let diagnostics = validate_content(
            Path::new(".claude/settings.json"),
            settings,
            &pinned(version),
            &registry,
        );
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-HK-001")
            .collect();
        assert_eq!(errors.len(), 1, "{version}: {diagnostics:?}");
        assert!(
            errors[0].message.contains("2.1.332"),
            "{version}: {diagnostics:?}"
        );
    }
    for version in ["2.1.332", "2.2.0"] {
        let diagnostics = validate_content(
            Path::new(".claude/settings.json"),
            settings,
            &pinned(version),
            &registry,
        );
        assert!(
            diagnostics.iter().all(|d| d.rule != "CC-HK-001"),
            "{version}: {diagnostics:?}"
        );
    }
}

#[test]
fn claude_code_021331_accepts_optional_plugin_dependencies() {
    // v2.1.331: "Added `optionalDependencies` to plugin manifests: plugins
    // that install with the plugin listing them and can be disabled or
    // uninstalled on their own". A marketplace entry carries the same key.
    let registry = ValidatorRegistry::with_defaults();
    let manifest = r#"{"name":"my-plugin","description":"demo plugin","version":"1.0.0","dependencies":[{"name":"required-helper","version":"^1.0.0"}],"optionalDependencies":[{"name":"optional-helper","version":"^1.0.0"}]}"#;
    let diagnostics = validate_content(
        Path::new("my-plugin/.claude-plugin/plugin.json"),
        manifest,
        &LintConfig::default(),
        &registry,
    );
    assert!(
        diagnostics.iter().all(|d| d.rule != "CC-PL-017"),
        "{diagnostics:?}"
    );

    // A misspelling is still unknown to Claude Code.
    let misspelled = manifest.replace("\"optionalDependencies\"", "\"optionalDependencies2\"");
    let diagnostics = validate_content(
        Path::new("my-plugin/.claude-plugin/plugin.json"),
        &misspelled,
        &LintConfig::default(),
        &registry,
    );
    assert!(
        diagnostics.iter().any(|d| d.rule == "CC-PL-017"),
        "{diagnostics:?}"
    );

    let marketplace = r#"{"name":"acme","description":"Acme plugins","owner":{"name":"acme"},"plugins":[{"name":"with-optional","source":"./plugins/with-optional","description":"d","version":"1.0.0","optionalDependencies":[{"name":"optional-helper","version":"^1.0.0"}]}]}"#;
    let diagnostics = validate_content(
        Path::new(".claude-plugin/marketplace.json"),
        marketplace,
        &LintConfig::default(),
        &registry,
    );
    assert!(
        diagnostics
            .iter()
            .all(|d| !(d.rule == "CC-MP-005" && d.message.contains("optionalDependencies"))),
        "{diagnostics:?}"
    );
}

#[test]
fn codex_0172_accepts_tagged_feature_flags_and_per_tool_reviewer() {
    // The tagged rust-v0.172.0 schema adds connector_tool_result_passthrough
    // and realtime_frames_local to [features], approvals_reviewer to
    // apps.<app>.tools.<tool>, and the tui.keymap.agents bindings. None are
    // named in the release notes, so the allowlists behind CDX-CFG-011 and
    // CDX-CFG-006 are reconciled against the tagged schema.
    let value = serde_json::json!({
        "features": {
            "connector_tool_result_passthrough": true,
            "realtime_frames_local": true,
            "guardianv3": {
                "enabled": true,
                "message_board_in_memory": true,
                "max_instruction_tokens": 4096,
                "housekeeper": {"batch_timeout_ms": 30000, "routing_instructions": "x"},
                "reviewer": {"retrieval_instructions": "y"}
            }
        },
        "apps": {
            "browser": {"tools": {"search": {"enabled": true, "approvals_reviewer": "user"}}}
        },
        "tui": {"keymap": {"agents": {"next_group": "n", "previous_group": "p"}}}
    });
    for (file, content) in [
        (".codex/config.json", value.to_string()),
        (".codex/config.yaml", serde_yaml::to_string(&value).unwrap()),
        (".codex/config.toml", toml::to_string(&value).unwrap()),
    ] {
        let diagnostics = validate_content(
            Path::new(file),
            &content,
            &LintConfig::default(),
            &ValidatorRegistry::with_defaults(),
        );
        assert!(diagnostics.is_empty(), "{file}: {diagnostics:?}");
    }
}

#[test]
fn codex_0172_locates_each_app_tool_reviewer() {
    let content = "approvals_reviewer = \"user\"\n[apps.\"a.b\".tools.one]\napprovals_reviewer = \"typo\"\n[apps.\"a.b\".tools.two]\napprovals_reviewer = 42\n";
    let diagnostics = validate_content(
        Path::new(".codex/config.toml"),
        content,
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    let positions: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CDX-CFG-024")
        .map(|d| (d.line, d.column))
        .collect();
    assert_eq!(positions, [(3, 1), (5, 1)], "{diagnostics:?}");
}

#[test]
fn opencode_2_4_1_accepts_root_automations() {
    // v2.4.0: "Set `\"automations\": false` in your config if you don't want
    // them": a native OpenCode 2 root key.
    let registry = ValidatorRegistry::with_defaults();
    let content = r#"{"$schema":"https://opencode.ai/config.json","automations":false}"#;
    for version in [None, Some("2.4.0"), Some("2.4.1")] {
        let config = LintConfig::builder()
            .tool_versions(agnix_core::config::ToolVersions {
                opencode: version.map(str::to_string),
                ..Default::default()
            })
            .build()
            .unwrap();
        let diagnostics = validate_content(Path::new("opencode.json"), content, &config, &registry);
        assert!(
            diagnostics.iter().all(|d| d.rule != "OC-004"),
            "{version:?}: {diagnostics:?}"
        );
    }

    let older = LintConfig::builder()
        .tool_versions(agnix_core::config::ToolVersions {
            opencode: Some("2.3.2".into()),
            ..Default::default()
        })
        .build()
        .unwrap();
    let diagnostics = validate_content(Path::new("opencode.json"), content, &older, &registry);
    assert!(
        diagnostics.iter().any(|d| d.rule == "OC-004"),
        "{diagnostics:?}"
    );

    for bad in ["null", "0", r#""false""#, "{}", "[]"] {
        let content = format!(r#"{{"automations":{bad}}}"#);
        let diagnostics = validate_content(
            Path::new("opencode.json"),
            &content,
            &LintConfig::default(),
            &registry,
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "OC-004"
                && d.message.contains("automations")
                && d.message.contains("boolean")),
            "{bad}: {diagnostics:?}"
        );
    }

    // OpenCode 1 does not read the key.
    let pinned_v1 = LintConfig::builder()
        .tool_versions(agnix_core::config::ToolVersions {
            opencode: Some("1.19.6".into()),
            ..Default::default()
        })
        .build()
        .unwrap();
    let diagnostics = validate_content(Path::new("opencode.json"), content, &pinned_v1, &registry);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.rule == "OC-004" && d.message.contains("OpenCode 2")),
        "{diagnostics:?}"
    );
}

#[test]
fn cursor_environment_accepts_clone_root_field() {
    // The published environment.schema.json carries a root `clone` object
    // (`depth`, `shallowSince`, `singleBranch`, `tags`, `filter`,
    // `bundleUri`, `extraArgs`), re-checked 2026-11-25.
    let registry = ValidatorRegistry::with_defaults();
    let content =
        serde_json::json!({"install": "npm ci", "clone": {"depth": 1, "singleBranch": true}})
            .to_string();
    let diagnostics = validate_content(
        Path::new(".cursor/environment.json"),
        &content,
        &LintConfig::default(),
        &registry,
    );
    assert!(
        diagnostics.iter().all(|d| d.rule != "CUR-016"),
        "{diagnostics:?}"
    );

    use serde_json::json;
    for clone in [
        json!({}),
        json!({"tags": false, "shallowSince": "2.weeks.ago", "singleBranch": true, "filter": "blob:none", "extraArgs": []}),
        json!({"bundleUri": true}),
        json!({"bundleUri": "https://example.com/bundle", "extraArgs": ["--no-tags"]}),
    ] {
        let diagnostics = validate_content(
            Path::new(".cursor/environment.json"),
            &json!({"clone":clone}).to_string(),
            &LintConfig::default(),
            &registry,
        );
        assert!(diagnostics.is_empty(), "{clone}: {diagnostics:?}");
    }
    for clone in [
        json!(null),
        json!(false),
        json!({"typo":1}),
        json!({"depth":"1"}),
        json!({"depth":0}),
        json!({"depth":2147483648_i64}),
        json!({"tags":1}),
        json!({"singleBranch":"true"}),
        json!({"bundleUri":false}),
        json!({"bundleUri":"http://example.com"}),
        json!({"filter":1}),
        json!({"filter":"none"}),
        json!({"shallowSince":false}),
        json!({"shallowSince":"yesterday"}),
        json!({"extraArgs":"--no-tags"}),
        json!({"extraArgs":[3]}),
        json!({"extraArgs":["--bare"]}),
    ] {
        let diagnostics = validate_content(
            Path::new(".cursor/environment.json"),
            &json!({"clone":clone}).to_string(),
            &LintConfig::default(),
            &registry,
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule == "CUR-016" && d.message.contains("clone")),
            "{clone}: {diagnostics:?}"
        );
    }

    let unknown = r#"{"install":"npm ci","clon":{}}"#;
    let diagnostics = validate_content(
        Path::new(".cursor/environment.json"),
        unknown,
        &LintConfig::default(),
        &registry,
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| d.rule == "CUR-016" && d.message.contains("clon")),
        "{diagnostics:?}"
    );
}

#[test]
fn grok_1_0_92_reports_unknown_sandbox_keys_from_1_0_90() {
    // 1.0.90: "unknown keys in sandbox.toml are reported as errors". Grok
    // then ignores the whole file, so every profile it defined is missing.
    // A pin below 1.0.90 keeps the open shape; unpinned files take the
    // latest release.
    fn pinned(version: &str) -> LintConfig {
        LintConfig::builder()
            .tool_versions(agnix_core::config::ToolVersions {
                grok_code: Some(version.to_string()),
                ..Default::default()
            })
            .build()
            .unwrap()
    }
    let registry = ValidatorRegistry::with_defaults();
    let sandbox = "profile = \"workspace\"\n\n[profiles.typo]\nextends = \"workspace\"\nread_onlys = [\"/data\"]\nworkspace_writ = true\nworkspace_write = true\n";
    for config in [&LintConfig::default(), &pinned("1.0.90"), &pinned("1.0.92")] {
        let diagnostics =
            validate_content(Path::new(".grok/sandbox.toml"), sandbox, config, &registry);
        let found: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "GRK-SBX-002")
            .collect();
        assert_eq!(found.len(), 3, "{diagnostics:?}");
    }
    for version in ["1.0.89", "1.0.82"] {
        let diagnostics = validate_content(
            Path::new(".grok/sandbox.toml"),
            sandbox,
            &pinned(version),
            &registry,
        );
        assert!(
            diagnostics.iter().all(|d| d.rule != "GRK-SBX-002"),
            "{version}: {diagnostics:?}"
        );
    }
}

#[test]
fn grok_1_0_82_types_workspace_write_and_accepts_extends_minimal() {
    // 1.0.82: "New `minimal` sandbox profile and `workspace_write` option
    // let custom profiles control launch-folder writes". The bundled
    // configuration reference types `workspace_write` as a boolean.
    fn pinned(version: &str) -> LintConfig {
        LintConfig::builder()
            .tool_versions(agnix_core::config::ToolVersions {
                grok_code: Some(version.to_string()),
                ..Default::default()
            })
            .build()
            .unwrap()
    }
    let registry = ValidatorRegistry::with_defaults();
    let sandbox = "[profiles.tiny]\nextends = \"minimal\"\nread_only = [\"/usr\"]\nread_write = [\"~/.cache\"]\nworkspace_write = true\n";
    let diagnostics = validate_content(
        Path::new(".grok/sandbox.toml"),
        sandbox,
        &pinned("1.0.81"),
        &registry,
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| d.rule == "GRK-SBX-002" && d.line == 2),
        "{diagnostics:?}"
    );
    let custom = "[profiles.minimal]\nextends = \"workspace\"\n";
    let diagnostics = validate_content(
        Path::new(".grok/sandbox.toml"),
        custom,
        &pinned("1.0.81"),
        &registry,
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    for version in ["1.0.82", "1.0.92"] {
        let diagnostics = validate_content(
            Path::new(".grok/sandbox.toml"),
            sandbox,
            &pinned(version),
            &registry,
        );
        assert!(diagnostics.is_empty(), "{version}: {diagnostics:?}");
    }
    let diagnostics = validate_content(
        Path::new(".grok/sandbox.toml"),
        sandbox,
        &LintConfig::default(),
        &registry,
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    let wrong = "[profiles.tiny]\nextends = \"workspace\"\nworkspace_write = \"yes\"\n";
    for version in ["1.0.81", "1.0.77"] {
        let diagnostics = validate_content(
            Path::new(".grok/sandbox.toml"),
            wrong,
            &pinned(version),
            &registry,
        );
        assert!(
            diagnostics.iter().all(|d| d.rule != "GRK-SBX-002"),
            "{version}: {diagnostics:?}"
        );
    }
    for version in ["1.0.82", "1.0.92"] {
        let diagnostics = validate_content(
            Path::new(".grok/sandbox.toml"),
            wrong,
            &pinned(version),
            &registry,
        );
        assert_eq!(diagnostics.len(), 1, "{version}: {diagnostics:?}");
        assert_eq!(
            diagnostics[0].rule, "GRK-SBX-002",
            "{version}: {diagnostics:?}"
        );
    }
    let diagnostics = validate_content(
        Path::new(".grok/sandbox.toml"),
        wrong,
        &LintConfig::default(),
        &registry,
    );
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].rule, "GRK-SBX-002", "{diagnostics:?}");
}

#[test]
fn kiro_360_accepts_subagent_stop_hook() {
    // kiro-cli 3.6.0 ("Subagent Stop Hooks and Guided Compaction"): "In a V3
    // session, a SubagentStop Hook runs when a dispatched subagent finishes".
    let registry = ValidatorRegistry::with_defaults();
    let pinned = LintConfig::builder()
        .tool_versions(agnix_core::config::ToolVersions {
            kiro: Some("3.6.0".into()),
            ..Default::default()
        })
        .build()
        .unwrap();
    let content = serde_json::json!({
        "version": "v1",
        "hooks": [
            {"name": "subagent-stop", "trigger": "SubagentStop", "action": {"type": "command", "command": "echo done"}}
        ]
    })
    .to_string();
    for config in [&LintConfig::default(), &pinned] {
        let diagnostics = validate_content(
            Path::new(".kiro/hooks/subagent-stop.json"),
            &content,
            config,
            &registry,
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    // The trigger name is still exact and case-sensitive.
    let wrong = content.replace("\"SubagentStop\"", "\"subagentStop\"");
    let diagnostics = validate_content(
        Path::new(".kiro/hooks/subagent-stop.json"),
        &wrong,
        &LintConfig::default(),
        &registry,
    );
    assert!(
        diagnostics.iter().any(|d| d.rule == "KR-HK-012"),
        "{diagnostics:?}"
    );
}
