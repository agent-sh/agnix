use assert_cmd::Command;

fn agnix() -> Command {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("agnix");
    cmd.current_dir(workspace_root());
    cmd
}

fn workspace_root() -> &'static std::path::Path {
    use std::sync::OnceLock;

    static ROOT: OnceLock<std::path::PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for ancestor in manifest_dir.ancestors() {
            let cargo_toml = ancestor.join("Cargo.toml");
            if let Ok(content) = std::fs::read_to_string(&cargo_toml)
                && (content.contains("[workspace]") || content.contains("[workspace."))
            {
                return ancestor.to_path_buf();
            }
        }
        panic!(
            "Failed to locate workspace root from CARGO_MANIFEST_DIR={}",
            manifest_dir.display()
        );
    })
    .as_path()
}

fn run_json(path: &std::path::Path) -> serde_json::Value {
    // These fixtures are intentionally excluded by the repository self-lint
    // config. Use a default config so the fixture validators actually run.
    let config = tempfile::NamedTempFile::new().unwrap();
    let output = agnix()
        .arg("--config")
        .arg(config.path())
        .arg(path)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).unwrap_or_else(|_| {
        panic!(
            "Expected valid JSON output for {}, got: {}",
            path.display(),
            stdout
        )
    })
}

fn assert_has_rule(json: &serde_json::Value, rule: &str) {
    let diagnostics = json["diagnostics"]
        .as_array()
        .unwrap_or_else(|| panic!("diagnostics missing in JSON output"));
    let found = diagnostics.iter().any(|d| d["rule"].as_str() == Some(rule));
    assert!(found, "Expected {} in diagnostics", rule);
}

fn count_rule(json: &serde_json::Value, rule: &str) -> usize {
    json["diagnostics"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|d| d["rule"].as_str() == Some(rule))
                .count()
        })
        .unwrap_or(0)
}

macro_rules! make_cli_test {
    ($name:ident, $path:expr, [$($rule:expr),+ $(,)?]) => {
        #[test]
        fn $name() {
            let path = workspace_root().join("tests/fixtures").join($path);
            let json = run_json(&path);
            $(assert_has_rule(&json, $rule);)+
        }
    };
}

make_cli_test!(
    test_cli_reports_xml_fixtures,
    "xml",
    ["XML-001", "XML-002", "XML-003"]
);
// REF-001 still fires on generic markdown (tests @import syntax)
make_cli_test!(test_cli_reports_ref_001_fixtures, "refs", ["REF-001"]);

// REF-002 only fires on agent config files, so we need a CLAUDE.md with broken links
#[test]
fn test_cli_reports_ref_002_fixtures() {
    let temp = tempfile::TempDir::new().unwrap();
    let claude_path = temp.path().join("CLAUDE.md");
    std::fs::write(
        &claude_path,
        "# Project\n\nSee [guide](missing-guide.md) for more details.\n",
    )
    .unwrap();

    let json = run_json(temp.path());
    assert_has_rule(&json, "REF-002");
}
make_cli_test!(test_cli_reports_mcp_fixtures, "mcp", ["MCP-001", "MCP-006"]);
make_cli_test!(
    test_cli_reports_agm_fixtures,
    "agents_md/no-headers",
    ["AGM-002"]
);
make_cli_test!(
    test_cli_reports_xp_fixtures,
    "cross_platform/hard-coded",
    ["XP-003"]
);
make_cli_test!(
    test_cli_reports_gemini_md_fixtures,
    "gemini_md-invalid",
    ["GM-001", "GM-002", "GM-003"]
);
make_cli_test!(
    test_cli_reports_gemini_hooks_fixtures,
    "gemini_hooks/invalid",
    [
        "GM-004", "GM-008", "GM-010", "GM-018", "GM-019", "GM-026", "GM-028"
    ]
);

#[test]
fn test_cli_gemini_extension_hooks_are_not_claude_hooks() {
    let path = workspace_root().join("tests/fixtures/gemini_hooks");
    let json = run_json(&path);
    let claude_hook_rules = json["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["rule"].as_str().is_some_and(|r| r.starts_with("CC-HK-")))
        .count();
    assert_eq!(claude_hook_rules, 0);
}

// CDX-001, CDX-002 and CDX-003 were retired (the TypeScript CLI keys are now
// CDX-004 unknown keys; committed AGENTS.override.md files are a documented
// pattern), so the fixture now covers the current Codex rules instead.
make_cli_test!(
    test_cli_reports_codex_invalid_fixtures,
    "codex-invalid",
    [
        "CDX-004",
        "CDX-AG-008",
        "CDX-CFG-007",
        "CDX-CFG-031",
        "CDX-CFG-035",
        "CDX-HK-000",
        "CDX-HK-005",
        "CDX-SUB-001",
        "CDX-RL-002",
        "CDX-DEP-001",
        "CDX-REQ-002",
        "CDX-PL-019",
        "CX-SK-006"
    ]
);

#[test]
fn test_cli_codex_invalid_fixture_counts() {
    let path = workspace_root().join("tests/fixtures/codex-invalid");
    let json = run_json(&path);
    // The fixture has exactly one approvalMode key (CDX-004), one
    // AGENTS.md shadowed by AGENTS.override.md (CDX-AG-008), and two
    // invalid permission-profile names (CDX-CFG-035: default_permissions
    // and permissions.dev.extends).
    assert_eq!(
        count_rule(&json, "CDX-004"),
        1,
        "Expected exactly 1 CDX-004"
    );
    assert_eq!(
        count_rule(&json, "CDX-AG-008"),
        1,
        "Expected exactly 1 CDX-AG-008"
    );
    assert_eq!(
        count_rule(&json, "CDX-CFG-035"),
        2,
        "Expected exactly 2 CDX-CFG-035"
    );
    for retired in ["CDX-001", "CDX-002", "CDX-003"] {
        assert_eq!(count_rule(&json, retired), 0, "{retired} is retired");
    }
}

// OpenCode: an OpenCode 1 shaped root config, an OpenCode 2 shaped nested
// config, Markdown agents and commands, a custom tool, instruction files and
// a skill defined in two sources.
make_cli_test!(
    test_cli_reports_opencode_invalid_fixtures,
    "opencode-invalid",
    [
        "OC-001",
        "OC-004",
        "OC-PM-003",
        "OC-CFG-015",
        "OC-DEP-009",
        "OC-PM-005",
        "OC-POL-001",
        "OC-AG-010",
        "OC-CMD-001",
        "OC-TOOL-001",
        "OC-AGM-002",
        "OC-AGM-003",
        "OC-SK-002"
    ]
);

#[test]
fn test_cli_reports_pe_fixtures() {
    // PE-001 is retired; PE-004 is reported for the ambiguous-term fixture.
    let source = workspace_root().join("tests/fixtures/prompt/pe-004-ambiguous.md");
    let content = std::fs::read_to_string(&source)
        .unwrap_or_else(|_| panic!("Failed to read {}", source.display()));

    let temp = tempfile::TempDir::new().unwrap();
    let claude_path = temp.path().join("CLAUDE.md");
    std::fs::write(&claude_path, content).unwrap();

    let json = run_json(temp.path());
    assert_has_rule(&json, "PE-004");
}

// ===== 0.59 vendor survey (cross-tool rules): every new rule fires through
// the real pipeline (detection, registry, validator) =====

fn run_json_with_config(path: &std::path::Path, config_toml: &str) -> serde_json::Value {
    let config = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(config.path(), config_toml).unwrap();
    let output = agnix()
        .arg("--config")
        .arg(config.path())
        .arg(path)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("Expected JSON for {}, got: {}", path.display(), stdout))
}

fn rules_in_file(json: &serde_json::Value, file_part: &str) -> Vec<String> {
    json["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["file"].as_str().is_some_and(|f| f.contains(file_part)))
        .filter_map(|d| d["rule"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn test_cli_reports_portable_skill_rules() {
    let json = run_json(&workspace_root().join("tests/fixtures/skills/portable-rules"));
    for rule in [
        "AS-020",
        "AS-022",
        "AS-023",
        "AS-024",
        "AS-026",
        "AS-027",
        "XP-SK-003",
        "XP-SK-004",
    ] {
        assert_has_rule(&json, rule);
    }
    let json = run_json(&workspace_root().join("tests/fixtures/skills/long-body"));
    assert_has_rule(&json, "AS-021");
    let json = run_json(&workspace_root().join("tests/fixtures/skills/deep-reference"));
    assert_has_rule(&json, "AS-013");
}

#[test]
fn test_cli_reports_duplicate_skill_names_across_roots() {
    let json = run_json(&workspace_root().join("tests/fixtures/skills-duplicate"));
    assert_has_rule(&json, "AS-025");
    // Claude Code and Codex never read both roots.
    let json = run_json_with_config(
        &workspace_root().join("tests/fixtures/skills-duplicate"),
        "tools = [\"claude-code\", \"codex\"]\n",
    );
    assert_eq!(count_rule(&json, "AS-025"), 0);
}

#[test]
fn test_cli_reports_agents_md_layout_rules() {
    let json = run_json(&workspace_root().join("tests/fixtures/agents_md/legacy-agent"));
    assert_has_rule(&json, "AGM-008");
    // AGM-007 applies to a repository-root AGENTS.md of a buildable project;
    // fixtures inside this repository are nested, so the project is built at
    // run time.
    let project = tempfile::TempDir::new().unwrap();
    std::fs::write(
        project.path().join("AGENTS.md"),
        "# Project\n\nA date parser.\n\n## Style\n\nUse tabs.\n",
    )
    .unwrap();
    std::fs::write(project.path().join("package.json"), "{}\n").unwrap();
    let json = run_json(project.path());
    assert_has_rule(&json, "AGM-007");

    // AGM-009 needs a repository root (`.git`), which a fixture in this
    // repository cannot carry, so the project is built at run time.
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::create_dir(temp.path().join(".git")).unwrap();
    std::fs::create_dir(temp.path().join("pkg")).unwrap();
    std::fs::write(
        temp.path().join("pkg/AGENTS.md"),
        "# Pkg\n\nRun `cargo test`.\n",
    )
    .unwrap();
    let json = run_json(temp.path());
    assert_has_rule(&json, "AGM-009");
}

#[test]
fn test_cli_reports_mcp_server_and_tool_rules() {
    let mcp = workspace_root().join("tests/fixtures/mcp");
    let json = run_json(&mcp.join("server-hygiene.mcp.json"));
    for rule in ["MCP-027", "MCP-028", "MCP-029"] {
        assert_has_rule(&json, rule);
    }
    let json = run_json(&mcp.join("tool-hygiene.mcp.json"));
    for rule in ["MCP-030", "MCP-031", "MCP-033"] {
        assert_has_rule(&json, rule);
    }
    let json = run_json(&mcp.join("deprecated-capabilities.mcp.json"));
    assert_has_rule(&json, "MCP-032");
}

#[test]
fn test_cli_reports_mcp_rules_in_client_configs() {
    let json = run_json(&workspace_root().join("tests/fixtures/mcp-clients"));
    let gemini = rules_in_file(&json, ".gemini/settings.json");
    assert!(gemini.contains(&"MCP-028".to_string()), "{gemini:?}");
    let opencode = rules_in_file(&json, "opencode.json");
    assert!(opencode.contains(&"MCP-018".to_string()), "{opencode:?}");
    assert!(opencode.contains(&"MCP-027".to_string()), "{opencode:?}");
    let amp = rules_in_file(&json, ".amp/settings.json");
    assert!(amp.contains(&"MCP-028".to_string()), "{amp:?}");
    let codex = rules_in_file(&json, ".codex/config.toml");
    assert!(codex.contains(&"MCP-027".to_string()), "{codex:?}");
    assert!(codex.contains(&"MCP-028".to_string()), "{codex:?}");
    // MCP-018/028 also run on Gemini agent frontmatter and on Amp skill
    // servers (SKILL.md `mcpServers` and the skill's `mcp.json`).
    let gemini_agent = rules_in_file(&json, ".gemini/agents/researcher.md");
    assert_eq!(gemini_agent, ["MCP-018"], "{gemini_agent:?}");
    let skill = rules_in_file(&json, ".agents/skills/tracker/SKILL.md");
    assert!(skill.contains(&"MCP-028".to_string()), "{skill:?}");
    let skill_mcp = rules_in_file(&json, ".agents/skills/browser/mcp.json");
    assert_eq!(skill_mcp, ["MCP-018"], "{skill_mcp:?}");
    // Each secret is reported once: OC-CFG-005 leaves the MCP server's
    // environment secret to MCP-018, and CDX-CFG-010 leaves the Codex
    // server header to MCP-028.
    assert!(
        !opencode.contains(&"OC-CFG-005".to_string()),
        "{opencode:?}"
    );
    assert!(!codex.contains(&"CDX-CFG-010".to_string()), "{codex:?}");

    // `.vscode/` is gitignored here, so this file is created at run time.
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::create_dir(temp.path().join(".vscode")).unwrap();
    std::fs::write(
        temp.path().join(".vscode/mcp.json"),
        "{\n  // JSON with comments\n  \"servers\": {\n    \"gh\": { \"type\": \"stdio\", \"command\": \"docker\", \"args\": [\"run\", \"-i\", \"--rm\", \"ghcr.io/github/github-mcp-server\"] },\n  }\n}\n",
    )
    .unwrap();
    let json = run_json(temp.path());
    assert_has_rule(&json, "MCP-027");
    assert_eq!(
        count_rule(&json, "MCP-007"),
        0,
        "VS Code mcp.json allows comments"
    );
}

#[test]
fn test_cli_shared_agents_skills_follow_tools() {
    let fixture =
        workspace_root().join("tests/fixtures/per_client_skills/.agents/skills/test-skill");
    let unscoped = run_json(&fixture);
    assert_eq!(count_rule(&unscoped, "CX-SK-001"), 0);
    assert_eq!(count_rule(&unscoped, "AMP-SK-001"), 0);
    assert_has_rule(&unscoped, "XP-SK-001");

    let amp = run_json_with_config(&fixture, "tools = [\"amp\"]\n");
    // `hooks` only: `mcpServers` is an Amp field.
    assert_eq!(count_rule(&amp, "AMP-SK-001"), 1, "{amp}");
    let codex = run_json_with_config(&fixture, "tools = [\"codex\"]\n");
    assert_eq!(count_rule(&codex, "CX-SK-001"), 2, "{codex}");

    let cx = run_json(
        &workspace_root().join("tests/fixtures/per_client_skills/.codex/skills/test-skill"),
    );
    assert_has_rule(&cx, "CX-SK-001");
}

#[test]
fn test_cli_vendor_rules_gated_by_tools() {
    // WS-011 (Windsurf, which replaced AGM-003) and AS-009 (Codex) do not run
    // for a Kiro-only scope.
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::create_dir(temp.path().join(".windsurf")).unwrap();
    std::fs::write(
        temp.path().join("AGENTS.md"),
        format!("# Project\n\nRun `cargo test`.\n\n{}\n", "x".repeat(40_000)),
    )
    .unwrap();
    let skill = temp.path().join(".codex/skills/notes");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: notes\ndescription: Use when writing <release> notes\n---\nWrite the notes.\n",
    )
    .unwrap();
    let unscoped = run_json(temp.path());
    assert_has_rule(&unscoped, "WS-011");
    assert_eq!(count_rule(&unscoped, "AGM-003"), 0);
    assert_has_rule(&unscoped, "AS-009");
    let kiro = run_json_with_config(temp.path(), "tools = [\"kiro\"]\n");
    assert_eq!(count_rule(&kiro, "WS-011"), 0);
    assert_eq!(count_rule(&kiro, "AS-009"), 0);
    let windsurf = run_json_with_config(temp.path(), "tools = [\"windsurf\"]\n");
    assert_has_rule(&windsurf, "WS-011");
    assert_eq!(count_rule(&windsurf, "AS-009"), 0);
}

// ===== Plugin manifests: CC-PL-001 on root manifests, inline mcpServers =====

fn diagnostics_in<'a>(json: &'a serde_json::Value, file_part: &str) -> Vec<&'a serde_json::Value> {
    json["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["file"].as_str().is_some_and(|f| f.contains(file_part)))
        .collect()
}

#[test]
fn test_cli_plugin_manifest_locations() {
    let json = run_json(&workspace_root().join("tests/fixtures/plugin-manifests"));
    // Agent Plugins manifests and a root manifest beside .claude-plugin/
    // belong to other tools: no Claude Code plugin rule.
    for dir in [
        "agent-plugins/",
        "agent-plugins-no-schema/",
        "dual-manifest/",
    ] {
        let found: Vec<_> = diagnostics_in(&json, dir)
            .into_iter()
            .filter(|d| d["rule"].as_str().unwrap().starts_with("CC-PL-"))
            .collect();
        assert!(found.is_empty(), "{dir}: {found:?}");
    }
    // The Copilot legacy example: Claude Code ignores it, so CC-PL-001 is a
    // warning and Claude's path rules (`./` prefix) do not run.
    let legacy = diagnostics_in(&json, "copilot-legacy/plugin.json");
    assert_eq!(legacy.len(), 1, "{legacy:?}");
    assert_eq!(legacy[0]["rule"], "CC-PL-001");
    assert_eq!(legacy[0]["level"], "warning");

    // A Claude Code manifest (it has `userConfig`) in the wrong place.
    let json =
        run_json(&workspace_root().join("tests/fixtures/invalid/plugins/misplaced-manifest"));
    let misplaced = diagnostics_in(&json, "misplaced-manifest/plugin.json");
    assert!(
        misplaced
            .iter()
            .any(|d| d["rule"] == "CC-PL-001" && d["level"] == "error"),
        "{misplaced:?}"
    );
}

#[test]
fn test_cli_inline_mcp_servers_in_plugin_manifests() {
    let json = run_json(&workspace_root().join("tests/fixtures/plugin-manifests"));
    let claude = rules_in_file(&json, "claude-inline-mcp/.claude-plugin/plugin.json");
    for rule in [
        "MCP-009",
        "MCP-018",
        "MCP-027",
        "MCP-028",
        "CC-MCP-001",
        "CC-MCP-004",
    ] {
        assert!(claude.contains(&rule.to_string()), "{rule}: {claude:?}");
    }
    let copilot = rules_in_file(&json, "copilot-inline-mcp/.github/plugin/plugin.json");
    assert_eq!(copilot, vec!["MCP-018".to_string()], "{copilot:?}");
    // Cursor reads a url-only entry as remote: no MCP-009.
    let cursor = rules_in_file(&json, "cursor-inline-mcp/.cursor-plugin/plugin.json");
    assert_eq!(cursor, vec!["MCP-027".to_string()], "{cursor:?}");
}

// ===== COP-019, COP-020, COP-039 on the root and .claude-plugin/ manifests =====

/// `(rule, level)` pairs of the diagnostics on one file, sorted.
fn findings_in(json: &serde_json::Value, file_part: &str) -> Vec<(String, String)> {
    let mut found: Vec<_> = diagnostics_in(json, file_part)
        .into_iter()
        .map(|d| {
            (
                d["rule"].as_str().unwrap().to_string(),
                d["level"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    found.sort();
    found
}

fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected
        .iter()
        .map(|(rule, level)| (rule.to_string(), level.to_string()))
        .collect()
}

#[test]
fn test_cli_copilot_rules_on_shared_plugin_manifests() {
    let fixtures = workspace_root().join("tests/fixtures/plugin-manifests");
    let unscoped = run_json(&fixtures);
    // A legacy root manifest with Copilot-only fields (`category`, `tags`).
    assert_eq!(
        findings_in(&unscoped, "copilot-legacy-invalid/plugin.json"),
        pairs(&[("COP-020", "warning"), ("COP-039", "warning")])
    );
    // An Agent Plugins root manifest: its name rule and closed schema.
    // CDX-PL-004 reports the same name for Codex.
    let agent_plugins: Vec<_> = findings_in(&unscoped, "agent-plugins-invalid/plugin.json")
        .into_iter()
        .filter(|(rule, _)| rule.starts_with("COP-"))
        .collect();
    assert_eq!(
        agent_plugins,
        pairs(&[("COP-039", "info"), ("COP-039", "warning")])
    );
    // An Agent Plugins layout (mcp.json) whose manifest has no $schema.
    assert_eq!(
        findings_in(&unscoped, "agent-plugins-no-schema/plugin.json"),
        pairs(&[("COP-019", "warning")])
    );
    // The valid examples stay clean, and no Claude Code manifest gets a
    // Copilot rule in an unscoped run.
    for clean in [
        "agent-plugins/plugin.json",
        "dual-manifest/plugin.json",
        "claude-copilot-limits/.claude-plugin/plugin.json",
    ] {
        assert!(findings_in(&unscoped, clean).is_empty(), "{clean}");
    }
    let claude_cop: Vec<_> = diagnostics_in(&unscoped, ".claude-plugin/plugin.json")
        .into_iter()
        .filter(|d| d["rule"].as_str().unwrap().starts_with("COP-"))
        .collect();
    assert!(claude_cop.is_empty(), "{claude_cop:?}");
    // The documented legacy example has no Copilot signal: only CC-PL-001.
    assert_eq!(
        findings_in(&unscoped, "copilot-legacy/plugin.json"),
        pairs(&[("CC-PL-001", "warning")])
    );

    // A run that selects Copilot reads .claude-plugin/plugin.json as a
    // legacy manifest, unless an earlier location exists (dual-manifest).
    let copilot = run_json_with_config(&fixtures, "tools = [\"copilot\"]\n");
    assert_eq!(
        findings_in(&copilot, "claude-copilot-limits/.claude-plugin/plugin.json"),
        pairs(&[("COP-039", "warning"), ("COP-039", "warning")])
    );
    for clean in [
        "dual-manifest/.claude-plugin/plugin.json",
        "claude-inline-mcp/.claude-plugin/plugin.json",
    ] {
        let cop: Vec<_> = findings_in(&copilot, clean)
            .into_iter()
            .filter(|(rule, _)| rule.starts_with("COP-"))
            .collect();
        assert!(cop.is_empty(), "{clean}: {cop:?}");
    }
    // Without Copilot in scope, no Copilot rule runs at all.
    let claude = run_json_with_config(&fixtures, "tools = [\"claude-code\"]\n");
    assert!(
        diagnostics_in(&claude, "plugin.json")
            .iter()
            .all(|d| !d["rule"].as_str().unwrap().starts_with("COP-"))
    );
}
