use std::path::Path;

fn validate(root: &Path, args: &[&str]) -> serde_json::Value {
    std::fs::create_dir_all(root.join(".git")).unwrap();
    let output = assert_cmd::cargo::cargo_bin_cmd!("agnix")
        .current_dir(root)
        .args(["--target", "claude-code", "--format", "json"])
        .args(args)
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}

fn count(result: &serde_json::Value, rule: &str) -> usize {
    result["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["rule"] == rule)
        .count()
}

#[test]
fn community_regression_multi_directory_project_checks_once() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("a")).unwrap();
    std::fs::create_dir_all(root.join("b")).unwrap();
    std::fs::write(root.join(".agnix.toml"), "").unwrap();
    let result = validate(root, &["a", "b"]);
    assert_eq!(count(&result, "VER-001"), 1);
}

#[test]
fn community_regression_explicit_config_diagnostic_location() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("target-tree")).unwrap();
    std::fs::write(root.join("outside.toml"), "").unwrap();
    std::fs::write(
        root.join("target-tree/.agnix.toml"),
        "[tool_versions]\nclaude_code = \"2.1.285\"\n",
    )
    .unwrap();
    let result = validate(root, &["--config", "outside.toml", "target-tree"]);
    let diagnostic = result["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["rule"] == "VER-001")
        .unwrap();
    assert_eq!(
        root.join(Path::new(diagnostic["file"].as_str().unwrap()))
            .canonicalize()
            .unwrap(),
        root.join("outside.toml").canonicalize().unwrap()
    );
}

#[test]
fn community_regression_prose_does_not_suppress() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join(".claude/agents")).unwrap();
    std::fs::write(root.join(".agnix.toml"), "").unwrap();
    std::fs::write(root.join(".claude/agents/helper.md"), "---\nname: helper\ndescription: Helps with demos.\nmodel: gpt-9\n---\nagnix-disable\nDo the demo.\n").unwrap();
    assert_eq!(count(&validate(root, &["."]), "CC-AG-003"), 1);
}

#[test]
fn community_regression_skill_dependencies_honor_excludes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join(".claude/skills/demo/node_modules/pkg")).unwrap();
    std::fs::write(
        root.join(".agnix.toml"),
        "[files]\nexclude = [\"**/node_modules/**\"]\n",
    )
    .unwrap();
    std::fs::write(
        root.join(".claude/skills/demo/SKILL.md"),
        "---\nname: demo\ndescription: Demonstrates a task.\n---\nPerform the task.\n",
    )
    .unwrap();
    std::fs::write(root.join(".claude/skills/demo/node_modules/pkg/index.d.ts"), "/**\n * Example: isPathInside(\"/Users/jdoe42/project\", \"/Users/jdoe42\")\n */\nexport default function f(): void;\n").unwrap();
    let result = validate(root, &["."]);
    assert_eq!(result["files_checked"], 1, "{result}");
    assert_eq!(count(&result, "CC-SK-021"), 0, "{result}");
}

#[test]
fn community_regression_plugin_unknown_key() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join(".claude-plugin")).unwrap();
    std::fs::write(root.join(".agnix.toml"), "").unwrap();
    std::fs::write(
        root.join(".claude-plugin/plugin.json"),
        r#"{"name":"demo","version":"1.0.0","description":"Demo plugin","bogus":true}"#,
    )
    .unwrap();
    assert_eq!(count(&validate(root, &["."]), "CC-PL-017"), 1);
}

#[test]
fn community_regression_agent_name_skip_conditions() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join(".claude/agents")).unwrap();
    std::fs::write(root.join(".agnix.toml"), "").unwrap();
    for (index, name) in ["-helper".to_string(), "a".repeat(257)].iter().enumerate() {
        std::fs::write(
            root.join(format!(".claude/agents/{index}.md")),
            format!("---\nname: {name}\ndescription: Helps with demos.\n---\nDo the demo.\n"),
        )
        .unwrap();
    }
    std::fs::write(
        root.join(".claude/agents/offset.md"),
        "\n---\nname: helper\ndescription: Helps with demos.\n---\nDo the demo.\n",
    )
    .unwrap();
    let result = validate(root, &["."]);
    assert_eq!(count(&result, "CC-AG-020"), 2);
    assert_eq!(count(&result, "CC-AG-007"), 1);
}

#[test]
fn community_regression_plugin_agent_filename_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("agents")).unwrap();
    std::fs::create_dir_all(root.join(".claude-plugin")).unwrap();
    std::fs::write(root.join(".agnix.toml"), "").unwrap();
    std::fs::write(
        root.join(".claude-plugin/plugin.json"),
        r#"{"name":"demo"}"#,
    )
    .unwrap();
    std::fs::write(
        root.join("agents/helper.md"),
        "---\ndescription: Helps with demos.\n---\nDo the demo.\n",
    )
    .unwrap();
    assert_eq!(count(&validate(root, &["."]), "CC-AG-001"), 0);
}

#[test]
fn community_regression_inline_suppressions_can_be_ignored() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join(".claude/agents")).unwrap();
    std::fs::write(root.join(".agnix.toml"), "").unwrap();
    std::fs::write(root.join(".claude/agents/helper.md"), "---\nname: helper\ndescription: Helps with demos.\nmodel: gpt-9\n---\n<!-- agnix-disable -->\nDo the demo.\n").unwrap();
    assert_eq!(count(&validate(root, &["."]), "CC-AG-003"), 0);
    assert_eq!(
        count(
            &validate(root, &["--ignore-inline-suppressions", "."]),
            "CC-AG-003"
        ),
        1
    );
    std::fs::write(
        root.join(".agnix.toml"),
        "ignore_inline_suppressions = true\n",
    )
    .unwrap();
    assert_eq!(count(&validate(root, &["."]), "CC-AG-003"), 1);
}

#[test]
fn community_regression_documented_skill_argument_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join(".claude/skills/demo")).unwrap();
    std::fs::write(root.join(".agnix.toml"), "").unwrap();
    std::fs::write(root.join(".claude/skills/demo/SKILL.md"), "---\nname: demo\ndescription: Demonstrates a task.\nargument-hint: <task>\n---\nPerform the task.\n!`date` !`pwd` !`whoami` !`git status`\n").unwrap();
    let result = validate(root, &["."]);
    assert_eq!(count(&result, "CC-SK-012"), 0);
    assert_eq!(count(&result, "CC-SK-009"), 0);
}

#[test]
fn community_regression_documented_plugin_fields_are_known() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join(".claude-plugin")).unwrap();
    std::fs::write(root.join(".agnix.toml"), "").unwrap();
    let manifest = serde_json::json!({
        "name": "demo", "version": "1.0.0", "description": "Demo",
        "displayName": "Demo", "defaultEnabled": true, "dependencies": [],
        "termsOfServiceUrl": "https://example.com/terms", "types": "./types.d.ts",
        "channels": [], "experimental": {}
    });
    std::fs::write(
        root.join(".claude-plugin/plugin.json"),
        manifest.to_string(),
    )
    .unwrap();
    assert_eq!(count(&validate(root, &["."]), "CC-PL-017"), 0);
}

#[test]
fn community_regression_manifestless_and_cached_plugin_agent_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::write(root.join(".agnix.toml"), "").unwrap();
    for layout in ["manifestless/agents", ".claude/plugins/cache/demo/1/agents"] {
        let directory = root.join(layout);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("missing.md"),
            "---\ndescription: Helps with demos.\n---\nDo the demo.\n",
        )
        .unwrap();
        std::fs::write(directory.join("plain.md"), "Do the demo.\n").unwrap();
        std::fs::write(
            directory.join("malformed.md"),
            "---\nname: [invalid\n---\nDo the demo.\n",
        )
        .unwrap();
    }
    let result = validate(root, &["."]);
    assert_eq!(count(&result, "CC-AG-001"), 0);
    assert_eq!(count(&result, "CC-AG-007"), 0);
}

#[cfg(unix)]
#[test]
fn community_regression_logical_instruction_symlink_is_not_deduplicated_as_unknown() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::write(root.join(".agnix.toml"), "").unwrap();
    std::fs::write(root.join("raw.txt"), "# Instructions\n<unclosed>\n").unwrap();
    std::os::unix::fs::symlink("raw.txt", root.join("AGENTS.md")).unwrap();
    let result = validate(root, &["raw.txt", "."]);
    assert_eq!(result["files_checked"], 1);
    assert!(result["diagnostics"].as_array().unwrap().iter().any(|d| {
        d["file"]
            .as_str()
            .is_some_and(|file| file.ends_with("AGENTS.md"))
    }));
}
