//! Native configuration contracts that the old generic/Claude routing missed.
use agnix_core::{LintConfig, ValidatorRegistry, validate_content};
use std::path::Path;

fn rules(path: &str, content: &str) -> Vec<String> {
    validate_content(
        Path::new(path),
        content,
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    )
    .into_iter()
    .map(|diagnostic| diagnostic.rule)
    .collect()
}

#[test]
fn pi_settings_validate_nested_numeric_constraints() {
    assert!(
        rules(
            ".pi/settings.json",
            r#"{"compaction":{"reserveTokens":-1}}"#
        )
        .contains(&"PI-CFG-002".into())
    );
}

#[test]
fn pi_project_scope_reports_settings_the_client_ignores() {
    assert!(
        rules(".pi/settings.json", r#"{"defaultProjectTrust":"always"}"#)
            .contains(&"PI-CFG-003".into())
    );
}

#[test]
fn pi_models_validate_nested_model_definitions() {
    assert!(
        rules(
            "/fixture/.pi/agent/models.json",
            r#"{"providers":{"local":{"models":[{"id":12}]}}}"#
        )
        .contains(&"PI-MDL-001".into())
    );
}

#[test]
fn pi_declared_skill_name_can_be_omitted_or_differ_from_directory() {
    for frontmatter in ["", "name: other-name\n"] {
        let content = format!(
            "---\n{frontmatter}description: Use when reviewing code\n---\nReview the changes.\n"
        );
        let found = rules(".pi/skills/review/SKILL.md", &content);
        assert!(
            !found.iter().any(|id| id.starts_with("CC-")
                || ["AS-001", "AS-002", "AS-004", "AS-017"].contains(&id.as_str())),
            "{found:?}"
        );
    }
}

#[test]
fn pi_skill_uses_native_scalar_semantics_and_ignores_other_client_fields() {
    for name in ["name: 42\n", "name: false\n", "name: []\n", "name: ''\n"] {
        let content = format!(
            "---\n{name}description: Use when reviewing code\nhooks: false\nmetadata: 42\nmodel: []\n---\nReview changes.\n"
        );
        let found = rules(".pi/skills/review/SKILL.md", &content);
        assert!(
            !found
                .iter()
                .any(|id| ["AS-004", "AS-016"].contains(&id.as_str())),
            "{found:?}"
        );
    }
    assert!(
        rules(
            ".pi/skills/review/SKILL.md",
            "---\ndescription: 42\n---\nReview changes.\n"
        )
        .contains(&"AS-003".into())
    );
}

#[test]
fn editor_buffer_metadata_replaces_stale_disk_tool_ownership() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("settings.json");
    std::fs::write(
        &file,
        r#"{"$schema":"https://pi.dev/schemas/settings.schema.json"}"#,
    )
    .unwrap();
    let content =
        r#"{"$schema":"https://antigravity.google/schemas/v1/settings.json","agentMode":"yolo"}"#;
    let fs = std::sync::Arc::new(agnix_core::fs::MockFileSystem::new());
    fs.add_file(file.clone(), content);
    let mut buffered = LintConfig::default();
    buffered.set_fs(fs);
    assert_eq!(
        agnix_core::resolve_file_type(&file, &buffered),
        agnix_core::FileType::AntigravityConfig
    );
    let found = validate_content(
        &file,
        content,
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(found.iter().any(|d| d.rule == "AGY-CFG-002"), "{found:?}");
    assert!(
        !found.iter().any(|d| d.rule.starts_with("PI-")),
        "{found:?}"
    );
    let package = temp.path().join("package.json");
    std::fs::write(&package, r#"{"pi":{"extensions":[]}}"#).unwrap();
    let found = validate_content(
        &package,
        r#"{"dependencies":{"@earendil-works/pi-coding-agent":"1.4.0"}}"#,
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(
        !found.iter().any(|d| d.rule.starts_with("PI-")),
        "{found:?}"
    );
}

#[test]
fn real_scans_discover_arbitrarily_named_bundled_schema_documents() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("custom-theme.json");
    std::fs::write(
        &file,
        r#"{"$schema":"https://pi.dev/schemas/theme.schema.json","colors":false}"#,
    )
    .unwrap();
    let result = agnix_core::validate_project(temp.path(), &LintConfig::default()).unwrap();
    assert!(
        result.diagnostics.iter().any(|d| d.rule == "PI-TH-001"),
        "{result:?}"
    );
}

#[test]
fn integrated_review_native_contract_regressions() {
    for name in ["code_review", "code review"] {
        let found = rules(
            ".grok/skills/review/SKILL.md",
            &format!(
                "---\nname: {name}\ndescription: Use when reviewing code\n---\nReview changes.\n"
            ),
        );
        assert!(!found.contains(&"AS-004".into()), "{found:?}");
    }
    let oversized = format!(
        "---\ntrigger: always_on\n---\n{} Literal marker @[",
        "x".repeat(24_001)
    );
    assert!(rules(".agents/rules/large.md", &oversized).contains(&"AGY-RL-003".into()));
    for option in [
        "command = ''",
        "url = '   '",
        "command = 'node'\nstartup_timeout_sec = -1",
        "command = 'node'\ntool_timeout_sec = 0.5",
        "command = 'node'\ntool_timeouts = {query = -2}",
    ] {
        assert!(
            rules(
                ".grok/config.toml",
                &format!("[mcp_servers.broken]\n{option}\n")
            )
            .contains(&"GRK-MCP-001".into()),
            "{option}"
        );
    }
    assert!(rules(".grok/agents/review.md", "---\nname: review\ndescription: Review changes\nmcpServers:\n  - broken:\n      command: 17\n      args: true\n---\nReview changes.\n").contains(&"GRK-MCP-001".into()));
}

#[test]
fn declared_pi_resources_preserve_native_subtypes_and_filters() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for dir in ["custom", ".pi", "external"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    std::fs::write(
        root.join("package.json"),
        r#"{"pi":{"prompts":["custom/*.md","!custom/skip.md"],"themes":["custom/*.json"]}}"#,
    )
    .unwrap();
    std::fs::write(
        root.join(".pi/settings.json"),
        r#"{"prompts":["../external/command.md"],"themes":["../external/mcp.json"]}"#,
    )
    .unwrap();
    let prompt = "---\ndescription: 42\n---\nReview changes.\n";
    let theme = r#"{"vars":{"a":"b","b":"a"},"colors":{},"mcpServers":{"ignored":{"command":"evil@latest"}}}"#;
    for (name, content) in [
        ("custom/review.md", prompt),
        ("custom/skip.md", prompt),
        ("custom/models.json", theme),
        ("external/command.md", prompt),
        ("external/mcp.json", theme),
    ] {
        std::fs::write(root.join(name), content).unwrap();
    }
    for (name, expected) in [
        ("custom/review.md", "PI-PR-002"),
        ("custom/models.json", "PI-TH-002"),
        ("external/command.md", "PI-PR-002"),
        ("external/mcp.json", "PI-TH-002"),
    ] {
        let result = agnix_core::validate_file(&root.join(name), &LintConfig::default()).unwrap();
        let printed = format!("{result:?}");
        assert!(printed.contains(expected), "{name}: {printed}");
        assert!(
            !printed.contains("PI-MDL-") && !printed.contains("MCP-027"),
            "{name}: {printed}"
        );
    }
    let result =
        agnix_core::validate_file(&root.join("custom/skip.md"), &LintConfig::default()).unwrap();
    assert!(!format!("{result:?}").contains("PI-PR-002"));
}

#[test]
fn antigravity_registry_owns_only_registered_nested_and_external_rules() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for dir in [".agents/rules/nested", "registered"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    for name in [
        ".agents/rules/nested/ignored.md",
        "registered/yes.md",
        "registered/no.md",
    ] {
        std::fs::write(root.join(name), "Missing native frontmatter.\n").unwrap();
    }
    std::fs::write(
        root.join(".agents/rules.json"),
        r#"{"inherits":[{"path":"inherited.json"}]}"#,
    )
    .unwrap();
    std::fs::write(root.join(".agents/inherited.json"), r#"{"inherits":[{"path":"rules.json"}],"entries":[{"path":"../registered","include_only":["yes.md"],"exclude":["no.md"]}]}"#).unwrap();
    let mut config = LintConfig::default();
    config.set_root_dir(root.to_path_buf());
    for (name, expected) in [
        (".agents/rules/nested/ignored.md", false),
        ("registered/yes.md", true),
        ("registered/no.md", false),
    ] {
        let result = agnix_core::validate_file(&root.join(name), &config).unwrap();
        assert_eq!(
            format!("{result:?}").contains("AGY-RL-001"),
            expected,
            "{name}: {result:?}"
        );
    }
}

#[test]
fn antigravity_size_limit_measures_expansion_and_strips_included_frontmatter() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join(".agents/rules")).unwrap();
    let path = root.join(".agents/rules/review.md");
    std::fs::write(root.join(".agents/rules/shared.md"), "x".repeat(12_000)).unwrap();
    let content = format!(
        "---\ntrigger: always_on\n---\n{}@[shared](shared.md)",
        "x".repeat(13_000)
    );
    assert!(
        validate_content(
            &path,
            &content,
            &LintConfig::default(),
            &ValidatorRegistry::with_defaults()
        )
        .iter()
        .any(|d| d.rule == "AGY-RL-003")
    );
    std::fs::write(
        root.join(".agents/rules/shared.md"),
        format!("---\ndescription: {}\n---\nsmall", "x".repeat(30_000)),
    )
    .unwrap();
    let content = "---\ntrigger: always_on\n---\n@[shared](shared.md)";
    assert!(
        !validate_content(
            &path,
            content,
            &LintConfig::default(),
            &ValidatorRegistry::with_defaults()
        )
        .iter()
        .any(|d| d.rule == "AGY-RL-003")
    );
}

#[test]
fn grok_marketplace_and_lsp_documents_use_their_native_contracts() {
    let valid = rules(
        ".grok-plugin/marketplace.json",
        r#"{"name":"My Org Plugins","plugins":[{"name":"drive","source":"./plugins/drive"}]}"#,
    );
    assert!(!valid.iter().any(|id| id.starts_with("GRK-")), "{valid:?}");
    assert!(
        rules(".grok-plugin/marketplace.json", r#"{"plugins":[]}"#).contains(&"GRK-PL-001".into())
    );
    assert!(
        rules(
            ".grok/lsp.json",
            r#"{"ts":{"command":"typescript-language-server","args":false}}"#
        )
        .contains(&"GRK-CFG-003".into())
    );
    let valid = rules(
        ".grok/lsp.json",
        r#"{"ts":{"command":"typescript-language-server","args":["--stdio"],"transport":"stdio","startupTimeout":15000,"extensionToLanguage":{".ts":"typescript"}}}"#,
    );
    assert!(!valid.iter().any(|id| id.starts_with("GRK-")), "{valid:?}");
    assert!(
        !rules(".pi/lsp.json", r#"{"broken":false}"#)
            .iter()
            .any(|id| id.starts_with("PI-"))
    );
}

#[test]
fn native_home_overrides_work_without_mutating_the_test_process_environment() {
    let temp = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "native_home_override_child",
            "--nocapture",
        ])
        .env("PI_CODING_AGENT_DIR", temp.path().join("pi-home"))
        .env("GROK_HOME", temp.path().join("grok-home"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "run by the isolated environment test"]
fn native_home_override_child() {
    let pi = std::path::PathBuf::from(std::env::var_os("PI_CODING_AGENT_DIR").unwrap());
    let grok = std::path::PathBuf::from(std::env::var_os("GROK_HOME").unwrap());
    assert_eq!(
        agnix_core::detect_file_type(&pi.join("settings.json")),
        agnix_core::FileType::PiConfig
    );
    assert_eq!(
        agnix_core::detect_file_type(&grok.join("config.toml")),
        agnix_core::FileType::GrokConfig
    );
    let found = rules(
        pi.join("settings.json").to_str().unwrap(),
        r#"{"defaultProjectTrust":"always","defaultThinkingLevel":"yolo"}"#,
    );
    assert!(
        found.contains(&"PI-CFG-002".into()) && !found.contains(&"PI-CFG-003".into()),
        "{found:?}"
    );
    let found = rules(
        grok.join("config.toml").to_str().unwrap(),
        "[ui]\nsimple_mode=true\n",
    );
    assert!(
        found.contains(&"GRK-CFG-004".into()) && !found.contains(&"GRK-CFG-002".into()),
        "{found:?}"
    );
}

#[test]
fn pi_resource_review_preserves_file_discovery_and_override_semantics() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for dir in ["skills/review/references", "prompts", "extra"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    std::fs::write(
        root.join("package.json"),
        r#"{"pi":{"skills":["skills"],"prompts":["prompts/*.md","extra/plain.md","!skip.md"]}}"#,
    )
    .unwrap();
    std::fs::write(
        root.join("skills/review/SKILL.md"),
        "---\ndescription: Use when reviewing code\n---\nReview changes.",
    )
    .unwrap();
    for name in [
        "skills/review/references/data.md",
        "prompts/skip.md",
        "extra/plain.md",
    ] {
        std::fs::write(
            root.join(name),
            "---\ndescription: 42\n---\nReview changes.",
        )
        .unwrap();
    }
    let result = agnix_core::validate_file(
        &root.join("skills/review/references/data.md"),
        &LintConfig::default(),
    )
    .unwrap();
    assert!(!format!("{result:?}").contains("AS-001"), "{result:?}");
    let result =
        agnix_core::validate_file(&root.join("prompts/skip.md"), &LintConfig::default()).unwrap();
    assert!(!format!("{result:?}").contains("PI-PR-002"), "{result:?}");
    let result =
        agnix_core::validate_file(&root.join("extra/plain.md"), &LintConfig::default()).unwrap();
    assert!(format!("{result:?}").contains("PI-PR-002"), "{result:?}");
    std::fs::write(
        root.join("prompts/SYSTEM.md"),
        "---\ndescription: [unterminated\n---\nReview changes.",
    )
    .unwrap();
    let result =
        agnix_core::validate_file(&root.join("prompts/SYSTEM.md"), &LintConfig::default()).unwrap();
    assert!(format!("{result:?}").contains("PI-PR-001"), "{result:?}");
}

#[test]
fn grok_review_covers_fallback_indexes_lsp_maps_and_project_output_caps() {
    let valid = rules(
        ".grok-plugin/plugin.json",
        r#"{"name":"Team Plugin Marketplace","plugins":[]}"#,
    );
    assert!(!valid.iter().any(|id| id.starts_with("GRK-")), "{valid:?}");
    assert!(
        rules(
            ".grok/lsp.json",
            r#"{"$schema":"https://example.test/lsp.json","rust":{"command":"rust-analyzer"}}"#
        )
        .contains(&"GRK-CFG-003".into())
    );
    let valid = rules(".grok/config.toml", "[mcp]\nmax_output_bytes=40000\n");
    assert!(!valid.iter().any(|id| id.starts_with("GRK-")), "{valid:?}");
    assert!(
        rules(
            ".grok/managed_config.toml",
            "[tools.zdr_video_output_s3]\nbucket=17\n"
        )
        .contains(&"GRK-CFG-003".into())
    );
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join(".grok-plugin")).unwrap();
    std::fs::create_dir_all(temp.path().join("agents")).unwrap();
    std::fs::write(
        temp.path().join(".grok-plugin/plugin.json"),
        r#"{"name":"review"}"#,
    )
    .unwrap();
    let path = temp.path().join("agents/review.md");
    let found = validate_content(
        &path,
        "---\nname: review\ndescription: Review code\npermissionMode: bypassPermissions\nmcpServers: [docs]\nhooks: {}\n---\nReview changes.",
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert_eq!(
        found.iter().filter(|d| d.rule == "GRK-AG-004").count(),
        3,
        "{found:?}"
    );
    std::fs::write(temp.path().join(".lsp.json"), r#"{"rust":{"command":42}}"#).unwrap();
    let result =
        agnix_core::validate_file(&temp.path().join(".lsp.json"), &LintConfig::default()).unwrap();
    assert!(format!("{result:?}").contains("GRK-CFG-003"), "{result:?}");
}

#[test]
fn antigravity_plain_rules_get_only_native_size_checks_when_scoped() {
    let content = "x".repeat(24_001);
    let mut config = LintConfig::default();
    config.set_tools(vec!["antigravity".into()]);
    let found = validate_content(
        Path::new("AGENTS.md"),
        &content,
        &config,
        &ValidatorRegistry::with_defaults(),
    );
    assert!(found.iter().any(|d| d.rule == "AGY-RL-003"), "{found:?}");
    assert!(!found.iter().any(|d| d.rule == "AGY-RL-001"), "{found:?}");
    assert!(rules("/fixture/.gemini/config/GEMINI.md", &content).contains(&"AGY-RL-003".into()));
    assert!(!rules("AGENTS.md", &content).contains(&"AGY-RL-003".into()));
}

#[test]
fn declared_plugin_components_keep_their_native_formats() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join(".grok-plugin")).unwrap();
    std::fs::create_dir_all(root.join("custom")).unwrap();
    std::fs::write(root.join(".grok-plugin/plugin.json"), r#"{"name":"review","hooks":"custom/server.json","mcpServers":"custom/remote.json","lspServers":"custom/language.json"}"#).unwrap();
    std::fs::write(
        root.join("custom/server.json"),
        r#"{"hooks":{"InvalidEvent":[]},"mcpServers":{"ignored":{"command":"evil@latest"}}}"#,
    )
    .unwrap();
    std::fs::write(
        root.join("custom/remote.json"),
        r#"{"mcpServers":{"bad":{"command":17}}}"#,
    )
    .unwrap();
    std::fs::write(
        root.join("custom/language.json"),
        r#"{"rust":{"command":17}}"#,
    )
    .unwrap();
    for (name, expected) in [
        ("server.json", "GRK-HK-002"),
        ("remote.json", "GRK-MCP-001"),
        ("language.json", "GRK-CFG-003"),
    ] {
        let result =
            agnix_core::validate_file(&root.join("custom").join(name), &LintConfig::default())
                .unwrap();
        let result = format!("{result:?}");
        assert!(result.contains(expected), "{name}: {result}");
        assert!(!result.contains("MCP-027"), "{name}: {result}");
    }
    assert!(
        rules(
            "/fixture/.gemini/config/config.json",
            r#"{"cliRemoteControlHostname":42}"#
        )
        .contains(&"AGY-CFG-002".into())
    );
    assert!(
        !rules(
            "/fixture/.gemini/config/config.json",
            r#"{"cliRemoteControlHostname":"my-workstation"}"#
        )
        .contains(&"AGY-CFG-002".into())
    );
}

#[test]
fn pi_auto_discovery_is_flat_but_declared_resource_roots_can_be_recursive() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join(".pi/prompts/nested")).unwrap();
    std::fs::create_dir_all(root.join(".pi/themes/nested")).unwrap();
    let prompt = root.join(".pi/prompts/nested/unused.md");
    let theme = root.join(".pi/themes/nested/unused.json");
    std::fs::write(&prompt, "---\ndescription: 42\n---\nReview code.").unwrap();
    std::fs::write(&theme, "{}").unwrap();
    for path in [&prompt, &theme] {
        let result = agnix_core::validate_file(path, &LintConfig::default()).unwrap();
        assert!(!format!("{result:?}").contains("PI-"), "{result:?}");
    }
    std::fs::write(
        root.join(".pi/settings.json"),
        r#"{"prompts":["prompts/nested"],"themes":["themes/nested"]}"#,
    )
    .unwrap();
    for (path, rule) in [(&prompt, "PI-PR-002"), (&theme, "PI-TH-001")] {
        let result = agnix_core::validate_file(path, &LintConfig::default()).unwrap();
        assert!(format!("{result:?}").contains(rule), "{result:?}");
    }
}

#[test]
fn grok_declared_json_components_do_not_depend_on_filename_suffixes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join(".grok-plugin")).unwrap();
    for suffix in ["conf", "toml", ""] {
        let name = |prefix: &str| {
            if suffix.is_empty() {
                prefix.to_string()
            } else {
                format!("{prefix}.{suffix}")
            }
        };
        let manifest = serde_json::json!({"name":"review","hooks":name("events"),"mcpServers":name("servers"),"lspServers":name("languages")});
        std::fs::write(root.join(".grok-plugin/plugin.json"), manifest.to_string()).unwrap();
        for (filename, content, rule) in [
            (
                name("events"),
                r#"{"hooks":{"InvalidEvent":[]}}"#,
                "GRK-HK-002",
            ),
            (
                name("servers"),
                r#"{"mcpServers":{"bad":{"command":42}}}"#,
                "GRK-MCP-001",
            ),
            (
                name("languages"),
                r#"{"bad":{"command":42}}"#,
                "GRK-CFG-003",
            ),
        ] {
            std::fs::write(root.join(&filename), content).unwrap();
            let result =
                agnix_core::validate_file(&root.join(&filename), &LintConfig::default()).unwrap();
            assert!(
                format!("{result:?}").contains(rule),
                "{filename}: {result:?}"
            );
        }
    }
}

#[test]
fn grok_plugin_helper_files_and_component_directories_are_not_config_documents() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for dir in [".grok-plugin", "agents", "hook-directory"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    std::fs::write(
        root.join(".grok-plugin/plugin.json"),
        r#"{"name":"review","hooks":"hook-directory"}"#,
    )
    .unwrap();
    std::fs::write(root.join("agents/helper.py"), "print('helper')").unwrap();
    std::fs::write(
        root.join("hook-directory/ignored.json"),
        r#"{"hooks":{"InvalidEvent":[]}}"#,
    )
    .unwrap();
    for name in ["agents/helper.py", "hook-directory/ignored.json"] {
        let result = agnix_core::validate_file(&root.join(name), &LintConfig::default()).unwrap();
        assert!(
            !format!("{result:?}").contains("GRK-"),
            "{name}: {result:?}"
        );
    }
}

#[test]
fn pi_project_mcp_partial_override_is_not_missing_a_transport() {
    let found = rules(
        ".pi/mcp.json",
        r#"{"mcpServers":{"inherited":{"enabled":false,"exposure":"codemode"}}}"#,
    );
    assert!(
        !found
            .iter()
            .any(|id| id.starts_with("PI-") || id.starts_with("MCP-")),
        "{found:?}"
    );
}

#[test]
fn pi_project_mcp_cannot_select_global_login_credentials() {
    assert!(rules(".pi/mcp.json", r#"{"mcpServers":{"api":{"url":"https://example.com/mcp","auth":{"provider":"anthropic"}}}}"#).contains(&"PI-MCP-003".into()));
}

#[test]
fn antigravity_settings_validate_native_enum_values() {
    assert!(
        rules(
            "/fixture/.gemini/antigravity-cli/settings.json",
            r#"{"agentMode":"yolo"}"#
        )
        .contains(&"AGY-CFG-002".into())
    );
}

#[test]
fn antigravity_settings_hook_block_is_ignored_by_the_client() {
    assert!(
        rules(
            "/fixture/.gemini/antigravity-cli/settings.json",
            r#"{"hooks":{"SessionStart":[]}}"#
        )
        .contains(&"AGY-CFG-003".into())
    );
}

#[test]
fn antigravity_rules_reject_camel_case_activation_trigger() {
    assert!(
        rules(
            ".agents/rules/review.md",
            "---\ntrigger: alwaysOn\n---\nReview the code.\n"
        )
        .contains(&"AGY-RL-001".into())
    );
}

#[test]
fn antigravity_native_hooks_accept_optional_command_type() {
    let found = rules(
        ".agents/hooks.json",
        r#"{"startup":{"SessionStart":[{"command":"echo ready"}]}}"#,
    );
    assert!(
        !found
            .iter()
            .any(|id| id.starts_with("CC-") || id.starts_with("AGY-")),
        "{found:?}"
    );
}

#[test]
fn antigravity_remote_mcp_rejects_unsupported_legacy_http_url() {
    assert!(
        rules(
            ".agents/mcp_config.json",
            r#"{"mcpServers":{"api":{"httpUrl":"https://example.com/sse"}}}"#
        )
        .contains(&"AGY-MCP-002".into())
    );
}

#[test]
fn grok_project_scope_reports_ignored_model_settings() {
    assert!(
        rules(".grok/config.toml", "[models]\ndefault = 'grok-build'\n")
            .contains(&"GRK-CFG-002".into())
    );
}

#[test]
fn grok_native_hooks_accept_http_handlers() {
    let found = rules(
        ".grok/hooks/audit.json",
        r#"{"hooks":{"PermissionDenied":[{"hooks":[{"type":"http","url":"https://example.com/audit"}]}]}}"#,
    );
    assert!(
        !found
            .iter()
            .any(|id| id.starts_with("CC-") || id.starts_with("GRK-")),
        "{found:?}"
    );
}

#[test]
fn grok_skill_name_and_description_fall_back_to_directory_and_body() {
    let found = rules(
        ".grok/skills/review/SKILL.md",
        "---\nuser-invocable: true\n---\nReview changes when reviewing code.\n",
    );
    assert!(
        !found.iter().any(
            |id| id.starts_with("CC-") || ["AS-001", "AS-002", "AS-003"].contains(&id.as_str())
        ),
        "{found:?}"
    );
}

#[test]
fn grok_custom_sandbox_profiles_cannot_redefine_builtins() {
    assert!(
        rules(
            ".grok/sandbox.toml",
            "[profiles.strict]\nextends = 'workspace'\n"
        )
        .contains(&"GRK-SBX-001".into())
    );
}

#[test]
fn pi_review_models_accept_line_comments_and_trailing_commas() {
    let found = rules(
        "/fixture/.pi/agent/models.json",
        "{ // native models dialect\n\"providers\": {},\n}",
    );
    assert!(!found.iter().any(|id| id.starts_with("PI-")), "{found:?}");
}

#[test]
fn pi_review_empty_and_null_prompt_headers_use_body_fallback() {
    for yaml in ["", "null\n", "description: 3\nargument-hint: false\n"] {
        let found = rules(
            ".pi/prompts/review.md",
            &format!("---\n{yaml}---\nReview this.\n"),
        );
        assert!(!found.contains(&"PI-PR-001".into()), "{yaml:?}: {found:?}");
    }
}

#[test]
fn pi_review_transport_type_must_be_a_string_when_present() {
    for value in ["null", "1", "true", "{}", "[]"] {
        let content =
            format!("{{\"mcpServers\":{{\"test\":{{\"type\":{value},\"command\":\"server\"}}}}}}");
        assert!(
            rules(".pi/mcp.json", &content).contains(&"PI-MCP-002".into()),
            "type={value}"
        );
    }
}

#[test]
fn pi_review_theme_path_wins_over_configuration_basename() {
    for name in [
        "settings.json",
        "models.json",
        "keybindings.json",
        "mcp.json",
    ] {
        let found = rules(&format!(".pi/themes/{name}"), "{}");
        assert!(found.contains(&"PI-TH-001".into()), "{name}: {found:?}");
    }
}

#[test]
fn pi_review_models_reject_unconditional_provider_runtime_errors() {
    for provider in ["{}", r#"{"oauth":"radius"}"#] {
        let content = format!("{{\"providers\":{{\"anthropic\":{provider}}}}}");
        let found = rules("/fixture/.pi/agent/models.json", &content);
        assert!(found.contains(&"PI-MDL-002".into()), "{content}: {found:?}");
    }
}

#[test]
fn agy_review_ignored_permission_syntax_is_one_advisory() {
    let found = validate_content(
        Path::new("/fixture/.gemini/antigravity-cli/settings.json"),
        r#"{"permissions":{"deny":["command (rm -rf)"]}}"#,
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(
        !found
            .iter()
            .any(|d| d.level == agnix_core::DiagnosticLevel::Error),
        "{found:?}"
    );
    assert_eq!(found.iter().filter(|d| d.rule == "AGY-CFG-004").count(), 1);
}

#[test]
fn agy_review_unsupported_string_logo_is_ignored_not_a_load_error() {
    let found = validate_content(
        Path::new(".agents/plugins/review/plugin.json"),
        r#"{"name":"review","logo":"https://example.com/logo.png"}"#,
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(
        !found
            .iter()
            .any(|d| d.level == agnix_core::DiagnosticLevel::Error),
        "{found:?}"
    );
    assert!(found.iter().any(|d| d.rule == "AGY-PL-002"), "{found:?}");
}

#[test]
fn agy_review_unknown_formatter_reports_documented_fallback() {
    let found = validate_content(
        Path::new("/fixture/.gemini/antigravity-cli/settings.json"),
        r#"{"customModelsConfig":{"customModels":{"local":{"modelName":"gemini-api://example.com/models/gemini","toolFormatterType":"future-formatter"}}}}"#,
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(
        !found
            .iter()
            .any(|d| d.level == agnix_core::DiagnosticLevel::Error),
        "{found:?}"
    );
    assert!(found.iter().any(|d| d.rule == "AGY-CFG-005"), "{found:?}");
}

#[test]
fn native_json_parser_handles_stack_overflow_regression_without_aborting() {
    let content = format!(
        "{{\"futureSetting\":{}0{}}}",
        "[".repeat(10_000),
        "]".repeat(10_000)
    );
    for path in [
        "/fixture/.gemini/antigravity-cli/settings.json",
        ".pi/settings.json",
    ] {
        let found = rules(path, &content);
        assert!(
            found
                .iter()
                .any(|id| matches!(id.as_str(), "AGY-CFG-001" | "PI-CFG-001")),
            "{path}: {found:?}"
        );
    }
}

#[test]
fn grok_review_unrelated_ancestor_names_do_not_choose_persona_schema() {
    let found = rules(
        "/projects/roles/repo/.grok/config.toml",
        "[models]\ndefault=12\n",
    );
    assert!(found.contains(&"GRK-CFG-003".into()), "{found:?}");
    assert!(found.contains(&"GRK-CFG-002".into()), "{found:?}");
}

#[test]
fn grok_review_unrelated_plugin_ancestor_does_not_restrict_native_agent() {
    let content = "---\nname: review\ndescription: Review changes\npermissionMode: bypassPermissions\nmcpServers: [docs]\nhooks: {}\n---\nReview code.\n";
    let found = rules("/projects/plugins/repo/.grok/agents/review.md", content);
    assert!(!found.contains(&"GRK-AG-004".into()), "{found:?}");
}

#[test]
fn grok_review_native_yaml_string_coercion_is_preserved() {
    let found = rules(
        ".grok/agents/review.md",
        "---\nname: 123\ndescription: 456\n---\nReview changes.\n",
    );
    assert!(
        !found.iter().any(|id| id.starts_with("GRK-AG")),
        "{found:?}"
    );
}

#[test]
fn grok_review_incomplete_recovery_policy_cannot_drop_agent_silently() {
    let found = rules(
        ".grok/agents/review.md",
        "---\nname: review\ndescription: Review changes\ncompletionRequirement:\n  tool: done\n  reminder: Finish\n  recovery: {}\n---\nReview code.\n",
    );
    assert!(found.contains(&"GRK-AG-002".into()), "{found:?}");
}

#[test]
fn grok_review_absolute_component_within_plugin_root_is_valid() {
    let found = rules(
        "/projects/test/.grok/plugins/review/plugin.json",
        r#"{"name":"review","skills":"/projects/test/.grok/plugins/review/skills"}"#,
    );
    assert!(!found.contains(&"GRK-PL-002".into()), "{found:?}");
}

#[test]
fn grok_review_all_declared_hook_field_types_are_checked() {
    for handler in [
        r#"{"type":"command","command":"echo ready","url":3}"#,
        r#"{"type":"http","url":"https://example.com/hook","command":3}"#,
    ] {
        let content = format!("{{\"hooks\":{{\"Stop\":[{{\"hooks\":[{handler}]}}]}}}}");
        assert!(
            rules(".grok/hooks/review.json", &content).contains(&"GRK-HK-001".into()),
            "{handler}"
        );
    }
}

#[test]
fn pi_package_metadata_is_discovered_without_a_client_directory() {
    let found = rules(
        "package.json",
        r#"{"name":"test-pi","pi":{"skills":"skills"}}"#,
    );
    assert!(found.contains(&"PI-PKG-001".into()), "{found:?}");
    let ordinary = rules(
        "package.json",
        r#"{"name":"ordinary","scripts":{"pi":"echo test"}}"#,
    );
    assert!(ordinary.is_empty(), "{ordinary:?}");
}

#[test]
fn pi_gallery_package_without_resource_manifest_still_checks_host_dependencies() {
    let found = rules(
        "package.json",
        r#"{"name":"test-pi","keywords":["pi-package"],"dependencies":{"@earendil-works/pi-coding-agent":"1.4.0"}}"#,
    );
    assert!(found.contains(&"PI-PKG-002".into()), "{found:?}");
}

#[test]
fn recognized_editor_schema_uses_bundled_contract_but_files_exclusions_win() {
    let content = r#"{"$schema":"https://pi.dev/schemas/settings.schema.json","defaultThinkingLevel":"yolo"}"#;
    let found = rules("custom-settings.json", content);
    assert!(found.contains(&"PI-CFG-002".into()), "{found:?}");
    let mut config = LintConfig::default();
    config.files_mut().exclude = vec!["custom-settings.json".into()];
    assert!(
        validate_content(
            Path::new("custom-settings.json"),
            content,
            &config,
            &ValidatorRegistry::with_defaults()
        )
        .is_empty()
    );
}

#[test]
fn real_file_and_project_scans_discover_pi_package_manifest() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("package.json");
    std::fs::write(&file, r#"{"pi":{"extensions":false}}"#).unwrap();
    let config = LintConfig::default();
    assert_eq!(
        agnix_core::resolve_file_type(&file, &config),
        agnix_core::FileType::PiConfig
    );
    let outcome = agnix_core::validate_file(&file, &config).unwrap();
    assert!(format!("{outcome:?}").contains("PI-PKG-001"));
    let result = agnix_core::validate_project(temp.path(), &config).unwrap();
    assert!(
        result.diagnostics.iter().any(|d| d.rule == "PI-PKG-001"),
        "{result:?}"
    );
}

#[test]
fn all_new_targets_and_aliases_filter_other_native_tool_rules() {
    let registry = ValidatorRegistry::with_defaults();
    for (alias, path, content, expected) in [
        (
            "pi",
            ".pi/settings.json",
            r#"{"compaction":{"reserveTokens":-1}}"#,
            "PI-CFG-002",
        ),
        (
            "agy",
            ".gemini/antigravity-cli/settings.json",
            r#"{"agentMode":"yolo"}"#,
            "AGY-CFG-002",
        ),
        (
            "grok",
            ".grok/sandbox.toml",
            "[profiles.strict]\nextends='workspace'\n",
            "GRK-SBX-001",
        ),
    ] {
        let config = LintConfig::builder()
            .tools(vec![alias.into()])
            .build()
            .unwrap();
        assert!(
            !config
                .validate()
                .iter()
                .any(|warning| warning.field == "tools"),
            "{alias}"
        );
        let found = validate_content(Path::new(path), content, &config, &registry);
        assert!(
            found.iter().any(|d| d.rule == expected),
            "{alias}: {found:?}"
        );
        let other = LintConfig::builder()
            .tools(vec!["codex".into()])
            .build()
            .unwrap();
        let found = validate_content(Path::new(path), content, &other, &registry);
        assert!(!found.iter().any(|d| d.rule == expected), "{found:?}");
    }
}

#[test]
fn native_mcp_files_receive_shared_supply_chain_checks() {
    for (path, content) in [
        (
            ".pi/mcp.json",
            r#"{"mcpServers":{"tools":{"command":"npx","args":["-y","example-mcp"]}}}"#,
        ),
        (
            ".agents/mcp_config.json",
            r#"{"mcpServers":{"tools":{"command":"npx","args":["-y","example-mcp"]}}}"#,
        ),
        (
            ".grok/config.toml",
            "[mcp_servers.tools]\ncommand='npx'\nargs=['-y','example-mcp']\n",
        ),
    ] {
        assert!(rules(path, content).contains(&"MCP-027".into()), "{path}");
    }
}

#[test]
fn pi_credential_command_references_are_not_executed_or_reported_as_literals() {
    let temp = tempfile::tempdir().unwrap();
    let marker = temp.path().join("must-not-exist");
    let content=serde_json::json!({"mcpServers":{"tools":{"command":"server","env":{"API_KEY":format!("!touch {}",marker.display())}}}}).to_string();
    let found = rules(".pi/mcp.json", &content);
    assert!(!marker.exists());
    assert!(!found.contains(&"MCP-018".into()), "{found:?}");
}

#[test]
fn ignored_settings_mcp_blocks_do_not_receive_active_server_diagnostics() {
    let content = r#"{"mcpServers":{"ignored":{"command":"npx","args":["-y","example-mcp"]}}}"#;
    for path in [".pi/settings.json", ".gemini/antigravity-cli/settings.json"] {
        assert!(!rules(path, content).contains(&"MCP-027".into()), "{path}");
    }
}

#[test]
fn pi_theme_literals_follow_native_case_exponent_and_channel_rules() {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../src/schemas/vendor/pi-theme.json")).unwrap();
    let roles = schema["properties"]["colors"]["required"]
        .as_array()
        .unwrap();
    for (color, valid) in [
        ("#abc", true),
        ("#ABCDEF", true),
        ("OKLCH(62% 0.1 -200deg)", true),
        ("okhsl(2e2deg 60% 55%)", true),
        ("okhsl(+.5 1 0)", true),
        ("#ggg", false),
        ("#abcd", false),
        ("oklch(101% .1 20)", false),
        ("okhsl(20 101% 20%)", false),
        ("OKLCH(.5 -1 10)", false),
        ("oklch(.5 1 1e400)", false),
    ] {
        let mut colors = serde_json::Map::new();
        for role in roles {
            colors.insert(role.as_str().unwrap().into(), serde_json::json!(""));
        }
        colors.insert("text".into(), serde_json::json!(color));
        let content = serde_json::json!({"name":"custom","colors":colors}).to_string();
        let found = rules(".pi/themes/custom.json", &content);
        assert_eq!(
            !found.contains(&"PI-TH-002".into()),
            valid,
            "{color}: {found:?}"
        );
    }
}

#[test]
fn every_native_catalog_rule_is_exercised_by_a_contract_fixture() {
    #[derive(serde::Deserialize)]
    struct Case {
        rule: String,
        path: String,
        content: String,
    }
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/native-tool-contracts/cases.json"
    ))
    .unwrap();
    let catalog: serde_json::Value = serde_json::from_str(agnix_rules::rules_json()).unwrap();
    let expected: std::collections::BTreeSet<_> = catalog["rules"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|rule| rule["id"].as_str())
        .filter(|id| {
            ["PI-", "AGY-", "GRK-"]
                .iter()
                .any(|prefix| id.starts_with(prefix))
        })
        .map(str::to_owned)
        .collect();
    let covered = cases.iter().map(|case| case.rule.clone()).collect();
    assert_eq!(
        expected, covered,
        "Each native rule needs a functional contract fixture."
    );
    for case in cases {
        let found = rules(&case.path, &case.content);
        assert!(
            found.contains(&case.rule),
            "{} at {}: {found:?}",
            case.rule,
            case.path
        );
    }
}
