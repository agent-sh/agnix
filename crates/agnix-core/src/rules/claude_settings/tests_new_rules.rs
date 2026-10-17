//! Tests for CC-SET-016 and CC-SET-034..040 and CC-OS-007.

use super::*;
use crate::config::LintConfig;
use crate::diagnostics::DiagnosticLevel;
use std::path::PathBuf;

fn at(path: &str, content: &str) -> Vec<Diagnostic> {
    ClaudeSettingsValidator.validate(&PathBuf::from(path), content, &LintConfig::default())
}

fn project(content: &str) -> Vec<Diagnostic> {
    at(".claude/settings.json", content)
}

fn hits<'a>(diagnostics: &'a [Diagnostic], rule: &str) -> Vec<&'a Diagnostic> {
    diagnostics.iter().filter(|d| d.rule == rule).collect()
}

// ===== CC-SET-034 =====

#[test]
fn cc_set_034_reports_keys_by_scope_column() {
    // (key json, file, expected hits)
    let cases = [
        // "User or managed": ignored in both repository files
        (
            r#"{"autoMode": {"environment": ["x"]}}"#,
            ".claude/settings.json",
            1,
        ),
        (
            r#"{"autoMode": {"environment": ["x"]}}"#,
            ".claude/settings.local.json",
            1,
        ),
        (
            r#"{"autoMode": {"environment": ["x"]}}"#,
            ".claude/managed-settings.json",
            0,
        ),
        (
            r#"{"pluginConfigs": {"p": {}}}"#,
            ".claude/settings.json",
            1,
        ),
        (
            r#"{"pluginConfigs": {"p": {}}}"#,
            ".claude/settings.local.json",
            1,
        ),
        (
            r#"{"sandbox": {"ripgrep": "/opt/rg"}}"#,
            ".claude/settings.json",
            1,
        ),
        // "Managed": ignored outside managed settings
        (
            r#"{"allowedProviders": ["firstParty"]}"#,
            ".claude/settings.json",
            1,
        ),
        (
            r#"{"allowedProviders": ["firstParty"]}"#,
            ".claude/managed-settings.json",
            0,
        ),
        (
            r#"{"claudeMd": "Run make lint."}"#,
            ".claude/settings.local.json",
            1,
        ),
        // "User, local, or managed": ignored only in the shared file
        (
            r#"{"useAutoModeDuringPlan": false}"#,
            ".claude/settings.json",
            1,
        ),
        (
            r#"{"useAutoModeDuringPlan": false}"#,
            ".claude/settings.local.json",
            0,
        ),
        // "User": only ~/.claude/settings.json
        (
            r#"{"worktree": {"location": "/tmp/wt"}}"#,
            ".claude/settings.json",
            1,
        ),
        // "Global config": never from a settings file
        (
            r#"{"diffTool": "auto"}"#,
            ".claude/managed-settings.json",
            1,
        ),
        // "Any file" keys are fine everywhere
        (
            r#"{"model": "opus", "outputStyle": "Explanatory"}"#,
            ".claude/settings.json",
            0,
        ),
        // null is treated as absent
        (r#"{"autoMode": null}"#, ".claude/settings.json", 0),
    ];
    for (content, path, expected) in cases {
        assert_eq!(
            hits(&at(path, content), "CC-SET-034").len(),
            expected,
            "{content} in {path}"
        );
    }
}

#[test]
fn cc_set_034_points_at_nested_key_and_reports_parent_once() {
    let content = "{\n  \"autoMode\": {\n    \"classifyAllShell\": \"true\"\n  },\n  \"sandbox\": {\n    \"ripgrep\": \"/opt/rg\"\n  }\n}";
    let diagnostics = project(content);
    let found = hits(&diagnostics, "CC-SET-034");
    assert_eq!(found.len(), 2, "{diagnostics:?}");
    assert_eq!((found[0].line, found[0].column), (2, 3));
    assert_eq!((found[1].line, found[1].column), (6, 5));
    assert!(found[1].message.contains("'sandbox.ripgrep'"));
    // The type check on an ignored value is not reported.
    assert!(hits(&diagnostics, "CC-SET-013").is_empty());
}

#[test]
fn cc_set_034_replaces_type_errors_on_managed_only_keys() {
    // channelsEnabled is Managed: in a project file the scope finding is
    // reported and the boolean type check is skipped.
    let diagnostics = project(r#"{"channelsEnabled": "true"}"#);
    assert_eq!(hits(&diagnostics, "CC-SET-034").len(), 1);
    assert!(hits(&diagnostics, "CC-SET-002").is_empty());
    // In managed settings the type check applies.
    let managed = at(
        ".claude/managed-settings.json",
        r#"{"channelsEnabled": "true"}"#,
    );
    assert_eq!(hits(&managed, "CC-SET-002").len(), 1);
    assert!(hits(&managed, "CC-SET-034").is_empty());
}

#[test]
fn cc_set_034_model_picker_scope_moved_from_cc_set_025() {
    let content = r#"{"modelPicker": {"options": [{"model": "opus"}]}}"#;
    let diagnostics = project(content);
    assert_eq!(hits(&diagnostics, "CC-SET-034").len(), 1);
    assert!(hits(&diagnostics, "CC-SET-025").is_empty());
}

#[test]
fn cc_set_034_value_level_exceptions() {
    // "a project or local `true` is ignored"; false still applies.
    assert_eq!(
        hits(
            &project(r#"{"remoteControlAtStartup": true}"#),
            "CC-SET-034"
        )
        .len(),
        1
    );
    assert!(
        hits(
            &project(r#"{"remoteControlAtStartup": false}"#),
            "CC-SET-034"
        )
        .is_empty()
    );
    assert!(
        hits(
            &at(
                ".claude/managed-settings.json",
                r#"{"remoteControlAtStartup": true}"#
            ),
            "CC-SET-034"
        )
        .is_empty()
    );
    assert_eq!(
        hits(
            &at(
                ".claude/settings.local.json",
                r#"{"bashEditDiffEnabled": true}"#
            ),
            "CC-SET-034"
        )
        .len(),
        1
    );
    // "`auto` and `bypassPermissions` don't take effect from project or
    // local settings"
    for mode in ["auto", "bypassPermissions"] {
        let content = format!(r#"{{"permissions": {{"defaultMode": "{mode}"}}}}"#);
        assert_eq!(hits(&project(&content), "CC-SET-034").len(), 1, "{mode}");
    }
    assert!(
        hits(
            &project(r#"{"permissions": {"defaultMode": "acceptEdits"}}"#),
            "CC-SET-034"
        )
        .is_empty()
    );
    // autoContinueAtUsageLimit "turns the feature off rather than being
    // ignored" in a project file.
    assert!(
        hits(
            &project(r#"{"autoContinueAtUsageLimit": false}"#),
            "CC-SET-034"
        )
        .is_empty()
    );
}

#[test]
fn cc_set_034_env_variables() {
    let content = r#"{"env": {
        "CLAUDE_CONFIG_DIR": "/tmp/c",
        "XDG_CONFIG_HOME": "/tmp/x",
        "OTEL_EXPORTER_OTLP_ENDPOINT": "https://otel",
        "OTEL_METRICS_EXPORTER": "none",
        "OTEL_LOG_USER_PROMPTS": "0",
        "NODE_ENV": "test",
        "CLAUDE_CODE_REMOTE": "1"
    }}"#;
    let diagnostics = project(content);
    let found = hits(&diagnostics, "CC-SET-034");
    let names: Vec<&str> = found
        .iter()
        .map(|d| d.message.split('\'').nth(1).unwrap_or_default())
        .collect();
    assert_eq!(
        names,
        vec![
            "CLAUDE_CONFIG_DIR",
            "XDG_CONFIG_HOME",
            "OTEL_EXPORTER_OTLP_ENDPOINT",
            "CLAUDE_CODE_REMOTE"
        ],
        "{diagnostics:?}"
    );
    // In managed settings only the variables ignored everywhere remain.
    let managed = at(".claude/managed-settings.json", content);
    assert_eq!(hits(&managed, "CC-SET-034").len(), 1);
}

#[test]
fn cc_set_034_can_be_disabled_without_reviving_type_checks() {
    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = vec!["CC-SET-034".to_string()];
    let diagnostics = ClaudeSettingsValidator.validate(
        &PathBuf::from(".claude/settings.json"),
        r#"{"channelsEnabled": "true"}"#,
        &config,
    );
    // The value has no effect in this file either way.
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

// ===== CC-SET-016 =====

#[test]
fn cc_set_016_covers_multiedit_and_points_at_the_entry() {
    let content =
        "{\n  \"permissions\": {\n    \"deny\": [\"Read(./x)\", \"MultiEdit(docs/**)\"]\n  }\n}";
    let diagnostics = project(content);
    let found = hits(&diagnostics, "CC-SET-016");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].level, DiagnosticLevel::Error);
    assert_eq!((found[0].line, found[0].column), (3, 27));
    let fix = &found[0].fixes[0];
    assert!(!fix.safe);
    let mut fixed = content.to_string();
    fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
    assert!(fixed.contains("\"Edit(docs/**)\""));
    serde_json::from_str::<serde_json::Value>(&fixed).expect("fixed JSON parses");
    assert!(hits(&project(&fixed), "CC-SET-016").is_empty());
}

// ===== CC-SET-035 =====

#[test]
fn cc_set_035_cases() {
    let cases = [
        ("allow", "Bash(ls) x", DiagnosticLevel::Error, "malformed"),
        ("deny", "Bash(ls", DiagnosticLevel::Error, "malformed"),
        (
            "allow",
            "mcp__github__get_issue(owner:me)",
            DiagnosticLevel::Error,
            "mcp__",
        ),
        (
            "deny",
            "Bash(command:rm *)",
            DiagnosticLevel::Error,
            "primary field",
        ),
        (
            "deny",
            "WebFetch(url : https://x)",
            DiagnosticLevel::Error,
            "primary field",
        ),
        (
            "allow",
            "mcp__*",
            DiagnosticLevel::Error,
            "approves nothing",
        ),
        ("allow", "B*", DiagnosticLevel::Error, "approves nothing"),
        (
            "allow",
            "Bash(git:* push)",
            DiagnosticLevel::Warning,
            "literal",
        ),
        (
            "allow",
            "Bash(git * main)",
            DiagnosticLevel::Warning,
            "before the subcommand",
        ),
        (
            "deny",
            "Read(/Users/alice/secrets/**)",
            DiagnosticLevel::Warning,
            "one slash",
        ),
        (
            "deny",
            "bash(rm *)",
            DiagnosticLevel::Warning,
            "matches no Claude Code tool",
        ),
        (
            "ask",
            "Stop Task",
            DiagnosticLevel::Warning,
            "matches no Claude Code tool",
        ),
    ];
    for (list, rule, level, needle) in cases {
        let content = format!(
            r#"{{"permissions": {{"{list}": [{}]}}}}"#,
            serde_json::to_string(rule).unwrap()
        );
        let diagnostics = project(&content);
        let found = hits(&diagnostics, "CC-SET-035");
        assert_eq!(found.len(), 1, "{rule}: {diagnostics:?}");
        assert_eq!(found[0].level, level, "{rule}");
        assert!(
            found[0].message.contains(needle),
            "{rule}: {}",
            found[0].message
        );
    }
}

#[test]
fn cc_set_035_valid_rules_are_clean() {
    let content = r#"{"permissions": {
        "allow": ["Bash(npm run *)", "Bash(git log * main)", "Bash(ls:*)", "mcp__puppeteer__*",
                  "mcp__github__get_*", "Edit(./Finance (2024)/**)", "WebFetch(domain:example.com)",
                  "Read(//Users/alice/notes/**)", "Read(~/notes/**)"],
        "deny": ["Agent(model:opus)", "Bash(run_in_background:true)", "mcp__*", "*", "TaskOutput",
                 "mcp__memory__create_entities", "Edit(/src/generated/**)"],
        "ask": ["Bash(git push *)", "Skill(skill:deploy)"]
    }}"#;
    let diagnostics = project(content);
    assert!(
        hits(&diagnostics, "CC-SET-035").is_empty(),
        "{diagnostics:?}"
    );
}

// ===== CC-SET-036 =====

#[test]
fn cc_set_036_broad_allow_rules() {
    let content = r#"{"permissions": {"allow": [
        "Bash", "Bash(python*)", "Bash(devbox run *)", "Bash(npm run *)", "Agent",
        "Bash(npm run test)", "Bash(devbox run npm test)"
    ]}}"#;
    let diagnostics = project(content);
    let found = hits(&diagnostics, "CC-SET-036");
    let levels: Vec<DiagnosticLevel> = found.iter().map(|d| d.level).collect();
    assert_eq!(
        levels,
        vec![
            DiagnosticLevel::Warning,
            DiagnosticLevel::Warning,
            DiagnosticLevel::Warning,
            DiagnosticLevel::Info,
            DiagnosticLevel::Info
        ],
        "{diagnostics:?}"
    );
    // Deny lists are not about approving code.
    assert!(
        hits(
            &project(r#"{"permissions": {"deny": ["Bash"]}}"#),
            "CC-SET-036"
        )
        .is_empty()
    );
}

// ===== CC-SET-037 =====

#[test]
fn cc_set_037_project_sandbox_weakening() {
    let content = r#"{"sandbox": {
        "enableWeakerNestedSandbox": true,
        "enableWeakerNetworkIsolation": false,
        "network": {"allowUnixSockets": ["/var/run/docker.sock"], "allowLocalBinding": true},
        "excludedCommands": ["docker *", "docker compose *"],
        "filesystem": {"allowWrite": ["~/.zshrc", "~/.kube"]}
    }}"#;
    let diagnostics = project(content);
    assert_eq!(hits(&diagnostics, "CC-SET-037").len(), 5, "{diagnostics:?}");
    // A developer's own settings are their choice.
    assert!(hits(&at(".claude/settings.local.json", content), "CC-SET-037").is_empty());
    assert!(hits(&at(".claude/managed-settings.json", content), "CC-SET-037").is_empty());
}

// ===== CC-SET-039 =====

#[test]
fn cc_set_039_secret_files_without_read_deny() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
    std::fs::write(dir.path().join(".env"), "KEY=1\n").unwrap();
    std::fs::write(dir.path().join(".env.example"), "KEY=\n").unwrap();
    let settings = dir.path().join(".claude/settings.json");
    let validate = |content: &str| {
        ClaudeSettingsValidator.validate(&settings, content, &LintConfig::default())
    };

    let diagnostics = validate(r#"{"permissions": {"allow": ["Bash(ls)"]}}"#);
    let found = hits(&diagnostics, "CC-SET-039");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].level, DiagnosticLevel::Info);
    assert!(found[0].message.contains(".env"));
    assert!(!found[0].message.contains(".env.example"));

    assert!(
        hits(
            &validate(r#"{"permissions": {"deny": ["Read(./.env)"]}}"#),
            "CC-SET-039"
        )
        .is_empty()
    );
    // A deny rule in the local file counts too.
    std::fs::write(
        dir.path().join(".claude/settings.local.json"),
        r#"{"permissions": {"deny": ["Read(**/.env)"]}}"#,
    )
    .unwrap();
    assert!(hits(&validate("{}"), "CC-SET-039").is_empty());
}

// ===== CC-SET-040 =====

#[test]
fn cc_set_040_memory_paths() {
    let content = r#"{
      "claudeMdExcludes": ["packages/web/CLAUDE.md", "**/vendor/**/CLAUDE.md", "/home/me/x/CLAUDE.md"],
      "autoMemoryDirectory": "memory"
    }"#;
    let diagnostics = project(content);
    let found = hits(&diagnostics, "CC-SET-040");
    assert_eq!(found.len(), 2, "{diagnostics:?}");
    assert!(found[0].message.contains("packages/web/CLAUDE.md"));
    assert!(found[1].message.contains("autoMemoryDirectory"));
    assert!(
        hits(
            &project(r#"{"autoMemoryDirectory": "~/my-memory"}"#),
            "CC-SET-040"
        )
        .is_empty()
    );
}

// ===== CC-OS-007 =====

#[test]
fn cc_os_007_output_style_case() {
    let content = "{\n  \"outputStyle\": \"explanatory\"\n}";
    let diagnostics = project(content);
    let found = hits(&diagnostics, "CC-OS-007");
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].line, found[0].column), (2, 18));
    let fix = &found[0].fixes[0];
    assert!(!fix.safe, "selecting a style changes behavior");
    let mut fixed = content.to_string();
    fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
    assert_eq!(fixed, "{\n  \"outputStyle\": \"Explanatory\"\n}");
    assert!(hits(&project(&fixed), "CC-OS-007").is_empty());

    for clean in [
        r#"{"outputStyle": "Concise"}"#,
        r#"{"outputStyle": "default"}"#,
        r#"{"outputStyle": "My Team Style"}"#,
    ] {
        assert!(hits(&project(clean), "CC-OS-007").is_empty(), "{clean}");
    }
}

#[test]
fn cc_os_007_custom_style_case() {
    let dir = tempfile::TempDir::new().unwrap();
    let styles = dir.path().join(".claude/output-styles");
    std::fs::create_dir_all(&styles).unwrap();
    std::fs::write(
        styles.join("diagrams.md"),
        "---\nname: Diagrams First\n---\nLead with a diagram.\n",
    )
    .unwrap();
    let settings = dir.path().join(".claude/settings.json");
    let diagnostics = ClaudeSettingsValidator.validate(
        &settings,
        r#"{"outputStyle": "diagrams first"}"#,
        &LintConfig::default(),
    );
    let found = hits(&diagnostics, "CC-OS-007");
    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains("'Diagrams First'"));
}

#[test]
fn cc_set_034_partial_scopes() {
    // "Claude Code honors `mask` entries ... only from user settings, managed
    // settings, and the `--settings` flag."
    let content = r#"{"sandbox": {"credentials": {
        "files": [{"path": "~/.netrc", "mode": "mask", "extract": 42}, {"path": "~/.ssh", "mode": "deny"}]
    }}}"#;
    let diagnostics = project(content);
    assert_eq!(hits(&diagnostics, "CC-SET-034").len(), 1, "{diagnostics:?}");
    // The mask entry's options are not type-checked where it is ignored.
    assert!(
        hits(&diagnostics, "CC-SET-012").is_empty(),
        "{diagnostics:?}"
    );
    let managed = at(".claude/managed-settings.json", content);
    assert!(hits(&managed, "CC-SET-034").is_empty());
    assert!(!hits(&managed, "CC-SET-012").is_empty());

    // "from project and local settings it reads plain string tips only"
    let tips = r#"{"spinnerTipsOverride": {"tips": ["plain", {"id": "a", "text": "obj"}], "label": "Team"}}"#;
    assert_eq!(hits(&project(tips), "CC-SET-034").len(), 2);
    assert!(
        hits(
            &project(r#"{"spinnerTipsOverride": {"tips": ["plain"]}}"#),
            "CC-SET-034"
        )
        .is_empty()
    );

    // Self-hosted environment IDs: user or managed settings only.
    assert_eq!(
        hits(
            &project(r#"{"remote": {"defaultEnvironmentId": "ccpool_abc"}}"#),
            "CC-SET-034"
        )
        .len(),
        1
    );
    assert!(
        hits(
            &project(r#"{"remote": {"defaultEnvironmentId": "env_abc"}}"#),
            "CC-SET-034"
        )
        .is_empty()
    );
}
