use agnix_core::{FileType, detect_file_type};
use assert_cmd::Command;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn agnix() -> Command {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("agnix");
    cmd.current_dir(workspace_root());
    cmd
}

fn workspace_root() -> &'static Path {
    use std::sync::OnceLock;

    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for ancestor in manifest_dir.ancestors() {
            let cargo_toml = ancestor.join("Cargo.toml");
            if let Ok(content) = fs::read_to_string(&cargo_toml)
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

fn required_paths() -> Vec<&'static str> {
    vec![
        "tests/fixtures/kiro-powers/.kiro/powers/valid-power/POWER.md",
        "tests/fixtures/kiro-powers/.kiro/powers/valid-power/mcp.json",
        "tests/fixtures/kiro-powers/.kiro/powers/missing-frontmatter/POWER.md",
        "tests/fixtures/kiro-powers/.kiro/powers/empty-keywords/POWER.md",
        "tests/fixtures/kiro-powers/.kiro/powers/empty-body/POWER.md",
        "tests/fixtures/kiro-powers/.kiro/powers/bad-mcp/POWER.md",
        "tests/fixtures/kiro-powers/.kiro/powers/bad-mcp/mcp.json",
        "tests/fixtures/kiro-powers/.kiro/powers/steps-and-samples/POWER.md",
        "tests/fixtures/kiro-agents/.kiro/agents/valid-agent.json",
        "tests/fixtures/kiro-agents/.kiro/agents/minimal-agent.json",
        "tests/fixtures/kiro-agents/.kiro/agents/invalid-resource.json",
        "tests/fixtures/kiro-agents/.kiro/agents/invalid-model.json",
        "tests/fixtures/kiro-agents/.kiro/agents/mismatched-tools.json",
        "tests/fixtures/kiro-agents/.kiro/agents/unknown-fields.json",
        "tests/fixtures/kiro-agents/.kiro/agents/no-mcp-access.json",
        "tests/fixtures/kiro-agents/.kiro/agents/valid-hooks.json",
        "tests/fixtures/kiro-agents/.kiro/agents/invalid-hook-event.json",
        "tests/fixtures/kiro-agents/.kiro/agents/missing-hook-command.json",
        "tests/fixtures/kiro-agents/.kiro/agents/valid-markdown-agent.md",
        "tests/fixtures/kiro-agents/.kiro/agents/jsonc-agent.json",
        "tests/fixtures/kiro-agents/.kiro/agents/missing-name.json",
        "tests/fixtures/kiro-agents/.kiro/agents/missing-prompt-file.json",
        "tests/fixtures/kiro-agents/.kiro/agents/invalid-v3-hooks.json",
        "tests/fixtures/kiro-agents/.kiro/agents/notes.md",
        "tests/fixtures/kiro-hooks/.kiro/hooks/valid-file-save.kiro.hook",
        "tests/fixtures/kiro-hooks/.kiro/hooks/valid-prompt-submit.kiro.hook",
        "tests/fixtures/kiro-hooks/.kiro/hooks/valid-pre-tool.kiro.hook",
        "tests/fixtures/kiro-hooks/.kiro/hooks/invalid-event.kiro.hook",
        "tests/fixtures/kiro-hooks/.kiro/hooks/missing-patterns.kiro.hook",
        "tests/fixtures/kiro-hooks/.kiro/hooks/missing-action.kiro.hook",
        "tests/fixtures/kiro-hooks/.kiro/hooks/missing-tool-types.kiro.hook",
        "tests/fixtures/kiro-hooks/.kiro/hooks/kr-hk-001-undocumented-manual.kiro.hook",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-027-tool-hook-without-matcher.json",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-028-broad-file-hook.json",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-029-unanchored-matcher.json",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-030-Lint_On_Save.json",
        "tests/fixtures/kiro-hooks/.kiro/hooks/kr-hk-016-v1-format.kiro.hook",
        "tests/fixtures/kiro-hooks/.kiro/hooks/kr-hk-026-field-types.kiro.hook",
        "tests/fixtures/kiro-hooks/.kiro/hooks/valid-env-reference.kiro.hook",
        "tests/fixtures/kiro-hooks/.kiro/hooks/valid-ide-0x-format.kiro.hook",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-011-version.json",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-012-missing-fields.json",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-012-unknown-trigger.json",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-013-action-type.json",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-014-trigger-alias.json",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-015-legacy-when-then.json",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-017-field-types.json",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-018-confirm.json",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-019-glob-matcher.json",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-020-ignored-matcher.json",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-021-claude-tool-names.json",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-022-misplaced-timeout.json",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-023-placeholders.json",
        "tests/fixtures/kiro-hooks/v3/invalid/.kiro/hooks/kr-hk-024-never-runs.json",
        "tests/fixtures/kiro-hooks/v3/valid/.kiro/hooks/guard-writes.json",
        "tests/fixtures/kiro-hooks/v3/valid/.kiro/hooks/lint-on-save.json",
        "tests/fixtures/kiro-hooks/v3/valid/.kiro/hooks/session-context.json",
        "tests/fixtures/kiro-hooks/v3/valid/.kiro/hooks/stop-confirm.json",
        "tests/fixtures/kiro-mcp/.kiro/settings/mcp.json",
        "tests/fixtures/kiro-mcp/.kiro/settings/valid-local-mcp.json",
        "tests/fixtures/kiro-mcp/.kiro/settings/valid-remote-mcp.json",
        "tests/fixtures/kiro-mcp/.kiro/settings/missing-command-url.json",
        "tests/fixtures/kiro-mcp/.kiro/settings/hardcoded-secrets.json",
        "tests/fixtures/kiro-mcp/invalid/.kiro/settings/mcp.json",
        "tests/fixtures/kiro-mcp/jsonc/.kiro/settings/mcp.json",
        "tests/fixtures/kiro-permissions/valid/.kiro/settings/permissions.yaml",
        "tests/fixtures/kiro-permissions/invalid/.kiro/settings/permissions.yaml",
        "tests/fixtures/kiro-detect/.kiroignore",
        "tests/fixtures/kiro-detect/.kiro/agents/reviewer.JSON",
        "tests/fixtures/kiro-detect/.kiro/hooks/team/lint.json",
        "tests/fixtures/kiro-detect/.kiro/mcp.json",
        "tests/fixtures/kiro-detect/.kiro/powers/installed/demo/dev.kiro/steering/usage.md",
        "tests/fixtures/kiro-detect/.kiro/powers/installed/demo/plugin.json",
        "tests/fixtures/kiro-detect/.kiro/settings.json",
        "tests/fixtures/kiro-detect/.kiro/settings/cli.json",
        "tests/fixtures/kiro-detect/.kiro/settings/permissions.yml",
        "tests/fixtures/kiro-detect/.kiro/settings/settings.json",
        "tests/fixtures/kiro-detect/.kiro/workflows/broken.workflow.yaml",
        "tests/fixtures/kiro-detect/.kiro/workflows/notes.json",
        "tests/fixtures/kiro-detect/.kiro/workflows/plan-and-implement.workflow.json",
        "tests/fixtures/kiro-detect/power/POWER.md",
        "tests/fixtures/kiro-detect/power/steering/patterns.md",
        "tests/fixtures/kiro-permissions/strict/.kiro/settings/permissions.yaml",
        // kiro-reach: one minimal project per rule that no other family
        // triggers (see kiro_rule_reachability.rs); two pin a Kiro version.
        "tests/fixtures/kiro-reach/kiro-005/.kiro/steering/empty-body.md",
        "tests/fixtures/kiro-reach/kiro-011/.kiro/steering/everything.md",
        "tests/fixtures/kiro-reach/kiro-016/.kiro/steering/project.md",
        "tests/fixtures/kiro-reach/kiro-016/AGENTS.md",
        "tests/fixtures/kiro-reach/kiro-017/.kiro/steering/project.md",
        "tests/fixtures/kiro-reach/kiro-017/AGENTS.md",
        "tests/fixtures/kiro-reach/kiro-018/.kiro/steering/api-design.md",
        "tests/fixtures/kiro-reach/kr-ag-006/.kiro/agents/lead.json",
        "tests/fixtures/kiro-reach/kr-ag-008/.kiro/agents/review-agent.json",
        "tests/fixtures/kiro-reach/kr-ag-009/.kiro/agents/review-agent.json",
        "tests/fixtures/kiro-reach/kr-ag-010/.kiro/agents/review-agent.json",
        "tests/fixtures/kiro-reach/kr-ag-011/.kiro/agents/review-agent.json",
        "tests/fixtures/kiro-reach/kr-ag-012/.kiro/agents/review-agent.json",
        "tests/fixtures/kiro-reach/kr-ag-013/.kiro/agents/review-agent.json",
        "tests/fixtures/kiro-reach/kr-ag-014/.kiro/agents/review-agent.json",
        "tests/fixtures/kiro-reach/kr-ag-018/.kiro/agents/review-agent.json",
        "tests/fixtures/kiro-reach/kr-ag-020/.kiro/agents/review-agent.json.attribution.json",
        "tests/fixtures/kiro-reach/kr-ag-022/.kiro/agents/plan.md",
        "tests/fixtures/kiro-reach/kr-ag-023/.kiro/agents/review-agent.json",
        "tests/fixtures/kiro-reach/kr-ag-024/.kiro/agents/review-agent.json",
        "tests/fixtures/kiro-reach/kr-ag-025/.kiro/agents/review-agent.json",
        "tests/fixtures/kiro-reach/kr-ag-026/.kiro/agents/review-agent.json",
        "tests/fixtures/kiro-reach/kr-ag-027/.kiro/agents/review-agent.json",
        "tests/fixtures/kiro-reach/kr-ag-028/.agnix.toml",
        "tests/fixtures/kiro-reach/kr-ag-028/.kiro/agents/reviewer.md",
        "tests/fixtures/kiro-reach/kr-ag-029/.kiro/agents/review-agent.json",
        "tests/fixtures/kiro-reach/kr-ag-030/.kiro/agents/lead.json",
        "tests/fixtures/kiro-reach/kr-ag-030/.kiro/agents/review-agent.json",
        "tests/fixtures/kiro-reach/kr-hk-009/.kiro/hooks/run-tests.kiro.hook",
        "tests/fixtures/kiro-reach/kr-hk-010/.kiro/hooks/deploy-notify.json",
        "tests/fixtures/kiro-reach/kr-hk-010/.kiro/hooks/notify.kiro.hook",
        "tests/fixtures/kiro-reach/kr-hk-025/.agnix.toml",
        "tests/fixtures/kiro-reach/kr-hk-025/.kiro/hooks/lint.json",
        "tests/fixtures/kiro-reach/kr-mcp-006/.kiro/settings/mcp.json",
        "tests/fixtures/kiro-reach/kr-mcp-012/.kiro/powers/installed/docs/mcp.json",
        "tests/fixtures/kiro-reach/kr-mcp-012/.kiro/powers/installed/docs/plugin.json",
        "tests/fixtures/kiro-reach/kr-mcp-013/.kiro/powers/installed/docs/mcp.json",
        "tests/fixtures/kiro-reach/kr-mcp-013/.kiro/powers/installed/docs/plugin.json",
        "tests/fixtures/kiro-reach/kr-mcp-014/.kiro/powers/installed/docs/mcp.json",
        "tests/fixtures/kiro-reach/kr-mcp-014/.kiro/powers/installed/docs/plugin.json",
        "tests/fixtures/kiro-reach/kr-pm-001/.kiro/settings/permissions.yaml",
        "tests/fixtures/kiro-reach/kr-pw-007/.kiro/powers/my-power/POWER.md",
        "tests/fixtures/kiro-reach/kr-pw-008/.kiro/powers/deploy/POWER.md",
        "tests/fixtures/kiro-reach/kr-pw-009/.kiro/powers/installed/supabase/plugin.json",
        "tests/fixtures/kiro-reach/kr-pw-010/.kiro/powers/installed/supabase/plugin.json",
        "tests/fixtures/kiro-reach/kr-pw-011/.kiro/powers/installed/supabase/plugin.json",
        "tests/fixtures/kiro-reach/kr-pw-012/.kiro/powers/installed/supabase/plugin.json",
        "tests/fixtures/kiro-reach/kr-pw-013/.kiro/powers/installed/supabase/plugin.json",
        "tests/fixtures/kiro-reach/kr-pw-014/.kiro/powers/installed/supabase/plugin.json",
        "tests/fixtures/kiro-reach/kr-pw-014/.kiro/powers/installed/supabase/skills/Setup_DB/SKILL.md",
        "tests/fixtures/kiro-reach/kr-pw-015/.kiro/powers/installed/supabase/plugin.json",
        "tests/fixtures/kiro-reach/kr-pw-017/.kiro/powers/installed/supabase/plugin.json",
        "tests/fixtures/kiro-reach/kr-pw-017/.kiro/powers/installed/supabase/POWER.md",
        "tests/fixtures/kiro-reach/kr-pw-018/.kiro/powers/installed/supabase/plugin.json",
        "tests/fixtures/kiro-reach/kr-pw-018/.kiro/powers/installed/supabase/steering/setup.md",
        "tests/fixtures/kiro-reach/kr-sk-001/.kiro/skills/my-skill/SKILL.md",
        "tests/fixtures/kiro-reach/kr-sk-002/.kiro/skills/pr-review/SKILL.md",
        "tests/fixtures/kiro-reach/kr-wf-003/.kiro/workflows/plan.workflow.json",
        "tests/fixtures/kiro-reach/kr-wf-003/.kiro/workflows/plan.workflow.yaml",
        "tests/fixtures/kiro-reach/kr-wf-005/.kiro/workflows/review.workflow.json",
        "tests/fixtures/kiro-reach/kr-wf-008/.kiro/workflows/review.workflow.json",
        "tests/fixtures/kiro-reach/kr-wf-009/.kiro/workflows/loop.workflow.json",
    ]
}

/// Detection baseline of the `kiro-detect` family: where Kiro loads each file
/// (and the near misses it never loads).
fn kiro_detect_expectations() -> Vec<(&'static str, FileType)> {
    vec![
        (
            "tests/fixtures/kiro-detect/.kiro/agents/reviewer.JSON",
            FileType::KiroNotLoaded,
        ),
        (
            "tests/fixtures/kiro-detect/.kiro/hooks/team/lint.json",
            FileType::KiroNotLoaded,
        ),
        (
            "tests/fixtures/kiro-detect/.kiro/mcp.json",
            FileType::KiroNotLoaded,
        ),
        (
            "tests/fixtures/kiro-detect/.kiro/powers/installed/demo/dev.kiro/steering/usage.md",
            FileType::KiroPowerSteering,
        ),
        (
            "tests/fixtures/kiro-detect/.kiro/powers/installed/demo/plugin.json",
            FileType::KiroPowerPlugin,
        ),
        (
            "tests/fixtures/kiro-detect/.kiro/settings.json",
            FileType::KiroNotLoaded,
        ),
        (
            "tests/fixtures/kiro-detect/.kiro/settings/cli.json",
            FileType::KiroSettings,
        ),
        (
            "tests/fixtures/kiro-detect/.kiro/settings/permissions.yml",
            FileType::KiroNotLoaded,
        ),
        (
            "tests/fixtures/kiro-detect/.kiro/settings/settings.json",
            FileType::Unknown,
        ),
        (
            "tests/fixtures/kiro-detect/.kiroignore",
            FileType::KiroIgnore,
        ),
        (
            "tests/fixtures/kiro-detect/.kiro/workflows/broken.workflow.yaml",
            FileType::KiroWorkflow,
        ),
        (
            "tests/fixtures/kiro-detect/.kiro/workflows/notes.json",
            FileType::KiroNotLoaded,
        ),
        (
            "tests/fixtures/kiro-detect/.kiro/workflows/plan-and-implement.workflow.json",
            FileType::KiroWorkflow,
        ),
        (
            "tests/fixtures/kiro-detect/power/POWER.md",
            FileType::KiroPower,
        ),
        (
            "tests/fixtures/kiro-detect/power/steering/patterns.md",
            FileType::KiroPowerSteering,
        ),
    ]
}

fn is_allowed_hidden_fixture_file(name: &str) -> bool {
    matches!(name, ".gitkeep" | ".DS_Store")
}

fn collect_relative_files(root: &Path, dir: &Path, out: &mut BTreeSet<String>) {
    for entry in fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("Failed to read fixture directory {}: {}", dir.display(), e))
    {
        let entry =
            entry.unwrap_or_else(|e| panic!("Failed to read entry under {}: {}", dir.display(), e));
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .unwrap_or_else(|e| panic!("Failed to stat {}: {}", path.display(), e));

        if metadata.file_type().is_symlink() {
            panic!(
                "Fixture inventory must not traverse symlinks: {}",
                path.display()
            );
        }

        if metadata.is_dir() {
            collect_relative_files(root, &path, out);
        } else if metadata.is_file() {
            // `.kiroignore` is a Kiro configuration file and `.agnix.toml` pins
            // a kiro-reach fixture's Kiro version; both are inventoried like
            // the rest.
            if let Some(name) = path.file_name().and_then(|name| name.to_str())
                && name.starts_with('.')
                && name != ".kiroignore"
                && name != ".agnix.toml"
            {
                if is_allowed_hidden_fixture_file(name) {
                    continue;
                }
                panic!(
                    "Unexpected hidden fixture file {}; add an allowlist entry if intentional",
                    path.display()
                );
            }

            let relative = path
                .strip_prefix(root)
                .unwrap_or_else(|_| panic!("{} should be under workspace root", path.display()));
            out.insert(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn run_agnix_json(target: &Path) -> Value {
    // Fixture baselines are independent of repository self-lint exclusions
    // and omit the version-pinning info diagnostic.
    let config = tempfile::NamedTempFile::new().unwrap();
    fs::write(config.path(), "[rules]\ndisabled_rules = [\"VER-001\"]\n").unwrap();
    let output = agnix()
        .arg("--config")
        .arg(config.path())
        .arg(target)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap_or_else(|e| panic!("Failed to run agnix on {}: {}", target.display(), e));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() || output.status.code() == Some(1),
        "agnix exited with unexpected status {} for {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        target.display(),
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_str(&stdout).unwrap_or_else(|_| {
        panic!(
            "Expected valid JSON output for {}, got stdout:\n{}\nstderr:\n{}",
            target.display(),
            stdout,
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn json_u64(json: &Value, key: &str, fixture_path: &Path) -> u64 {
    json.get(key).and_then(Value::as_u64).unwrap_or_else(|| {
        panic!(
            "Expected numeric {} in JSON output for {}",
            key,
            fixture_path.display()
        )
    })
}

fn diagnostics_len(json: &Value, fixture_path: &Path) -> u64 {
    json.get("diagnostics")
        .and_then(Value::as_array)
        .unwrap_or_else(|| {
            panic!(
                "Expected diagnostics array in JSON output for {}",
                fixture_path.display()
            )
        })
        .len() as u64
}

#[test]
fn kiro_fixture_families_exist_with_required_cases() {
    let root = workspace_root();

    let fixture_roots = [
        "tests/fixtures/kiro-powers",
        "tests/fixtures/kiro-agents",
        "tests/fixtures/kiro-hooks",
        "tests/fixtures/kiro-mcp",
        "tests/fixtures/kiro-permissions",
        "tests/fixtures/kiro-detect",
        "tests/fixtures/kiro-reach",
    ];

    for rel in fixture_roots {
        let path = root.join(rel);
        assert!(
            path.exists(),
            "Expected fixture directory {}",
            path.display()
        );
    }

    for rel in required_paths() {
        let path = root.join(rel);
        assert!(path.exists(), "Expected fixture file {}", path.display());
    }

    let expected: BTreeSet<String> = required_paths()
        .into_iter()
        .map(ToString::to_string)
        .collect();

    let mut actual: BTreeSet<String> = BTreeSet::new();
    for rel in fixture_roots {
        let dir = root.join(rel);
        collect_relative_files(root, &dir, &mut actual);
    }

    assert_eq!(
        actual, expected,
        "Kiro fixture inventory drift detected. Update required_paths() intentionally when fixture corpus changes."
    );
}

#[test]
fn kiro_fixture_families_are_cli_runnable() {
    let root = workspace_root();
    let fixture_roots = [
        // Every legacy POWER.md now gets the KR-PW-019 format info (6), and
        // steps-and-samples/POWER.md adds a KR-PW-006 info; its Rust code
        // sample no longer trips KR-PW-008.
        ("tests/fixtures/kiro-powers", 8_u64, 11_u64),
        // One finding per invalid fixture (two for unknown-fields.json and
        // missing-name.json, three for invalid-v3-hooks.json); valid-agent.json,
        // valid-hooks.json, valid-markdown-agent.md and minimal-agent.json are
        // clean.
        ("tests/fixtures/kiro-agents", 16_u64, 16_u64),
        // V3 hook fixtures (v3/valid, v3/invalid: one file per rule) and the
        // legacy KR-HK-001/016/026 and valid IDE 0.x fixtures were added.
        ("tests/fixtures/kiro-hooks", 34_u64, 31_u64),
        // invalid/ and jsonc/ exercise KR-MCP-001..011 and 015.
        ("tests/fixtures/kiro-mcp", 3_u64, 13_u64),
        // strict/ adds KR-PM-003 (evaluate), 005..008 and 010..012; each
        // repository-local permissions file also gets the KR-PM-009 note.
        ("tests/fixtures/kiro-permissions", 3_u64, 16_u64),
        // Six KIRO-026 near misses, five KR-WF findings in broken.workflow.yaml
        // one KR-IGN-001, the KR-PW-019 legacy-format note on power/POWER.md and
        // the KR-PW-016 author note on the power plugin.json;
        // settings/settings.json is not validated.
        ("tests/fixtures/kiro-detect", 14_u64, 14_u64),
        // One run over every kiro-reach fixture (the two .agnix.toml files
        // are not linted). `--config` replaces their Kiro 2.x pins, so
        // KR-AG-028 and KR-HK-025 do not fire here; the reachability test
        // lints each fixture with its own config. The two AGENTS.md files add
        // AGM-003, XP-006 and XP-007, and several fixtures also trip related
        // Kiro rules (KR-AG-001, KR-AG-015, KR-PW-015, KR-PW-016, ...).
        ("tests/fixtures/kiro-reach", 60_u64, 67_u64),
    ];

    for (rel, expected_files_checked, expected_diagnostics) in fixture_roots {
        let fixture_path = root.join(rel);
        let parsed = run_agnix_json(&fixture_path);

        assert!(
            parsed.get("summary").and_then(Value::as_object).is_some(),
            "Expected summary in JSON output for {}",
            fixture_path.display()
        );

        let files_checked = json_u64(&parsed, "files_checked", &fixture_path);
        assert_eq!(
            files_checked,
            expected_files_checked,
            "Unexpected files_checked baseline for {}",
            fixture_path.display()
        );

        let diagnostics = diagnostics_len(&parsed, &fixture_path);
        assert_eq!(
            diagnostics,
            expected_diagnostics,
            "Unexpected diagnostics baseline for {}",
            fixture_path.display()
        );
    }
}

#[test]
fn kiro_permissions_invalid_shape_is_reported_by_cli() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".kiro/settings/permissions.yaml");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "rules: {}\n").unwrap();
    let output = run_agnix_json(dir.path());
    // KR-PM-009 notes that Kiro does not read a repository-local policy.
    let diagnostics: Vec<_> = output["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["rule"] != "KR-PM-009")
        .collect();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["rule"], "KR-PM-001");
}

#[test]
fn kiro_permissions_fixtures_cover_claimed_rule_ids() {
    let root = workspace_root();
    let valid = root.join("tests/fixtures/kiro-permissions/valid");
    let invalid = root.join("tests/fixtures/kiro-permissions/invalid");
    let rules_of = |dir: &Path| -> BTreeSet<String> {
        run_agnix_json(dir)["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|diagnostic| diagnostic["rule"].as_str().unwrap().to_string())
            .collect()
    };
    // The valid policy only gets the KR-PM-009 note: Kiro does not read a
    // repository-local .kiro/settings/permissions.yaml.
    assert_eq!(rules_of(&valid), BTreeSet::from(["KR-PM-009".to_string()]));
    assert_eq!(
        rules_of(&invalid),
        ["KR-PM-002", "KR-PM-003", "KR-PM-004", "KR-PM-009"]
            .map(str::to_string)
            .into()
    );
    let strict = root.join("tests/fixtures/kiro-permissions/strict");
    assert_eq!(
        rules_of(&strict),
        [
            "KR-PM-003",
            "KR-PM-005",
            "KR-PM-006",
            "KR-PM-007",
            "KR-PM-008",
            "KR-PM-009",
            "KR-PM-010",
            "KR-PM-011",
            "KR-PM-012"
        ]
        .map(str::to_string)
        .into()
    );
    // KR-PM-001 shape failures are covered by unit/integration tests, not this fixture.
}

#[test]
fn kiro_fixture_files_have_explicit_detection_baselines() {
    for rel in required_paths() {
        let is_detected = detect_file_type(Path::new(rel)).is_validatable();
        let should_be_detected = rel.starts_with("tests/fixtures/kiro-powers/")
            || rel.starts_with("tests/fixtures/kiro-agents/.kiro/agents/")
            || rel.starts_with("tests/fixtures/kiro-hooks/")
            || rel.starts_with("tests/fixtures/kiro-permissions/")
            || (rel.starts_with("tests/fixtures/kiro-mcp/")
                && rel.ends_with("/.kiro/settings/mcp.json"))
            || (rel.starts_with("tests/fixtures/kiro-detect/")
                && rel != "tests/fixtures/kiro-detect/.kiro/settings/settings.json")
            || (rel.starts_with("tests/fixtures/kiro-reach/") && !rel.ends_with("/.agnix.toml"));

        assert_eq!(
            is_detected, should_be_detected,
            "Unexpected file-type detection baseline for fixture {}",
            rel
        );
    }
}

#[test]
fn kiro_fixture_representatives_map_to_expected_file_types() {
    let expectations = [
        (
            "tests/fixtures/kiro-permissions/valid/.kiro/settings/permissions.yaml",
            FileType::KiroPermissions,
        ),
        (
            "tests/fixtures/kiro-powers/.kiro/powers/valid-power/POWER.md",
            FileType::KiroPower,
        ),
        (
            "tests/fixtures/kiro-agents/.kiro/agents/valid-agent.json",
            FileType::KiroAgent,
        ),
        (
            "tests/fixtures/kiro-hooks/.kiro/hooks/valid-file-save.kiro.hook",
            FileType::KiroHook,
        ),
        (
            "tests/fixtures/kiro-mcp/.kiro/settings/mcp.json",
            FileType::KiroMcp,
        ),
        (
            "tests/fixtures/kiro-powers/.kiro/powers/valid-power/mcp.json",
            FileType::KiroMcp,
        ),
    ];

    for (rel, expected) in expectations.into_iter().chain(kiro_detect_expectations()) {
        let detected = detect_file_type(Path::new(rel));
        assert_eq!(
            detected, expected,
            "Unexpected representative file type for fixture {}",
            rel
        );
    }
}

#[test]
fn kiro_detect_fixtures_report_exactly_the_expected_findings() {
    let root = workspace_root();
    let fixture = root.join("tests/fixtures/kiro-detect");
    let output = run_agnix_json(&fixture);
    let diagnostics = output["diagnostics"].as_array().unwrap();

    let found: BTreeSet<(String, String)> = diagnostics
        .iter()
        .map(|diagnostic| {
            let file = diagnostic["file"].as_str().unwrap().replace('\\', "/");
            let start = file.find("tests/fixtures/").unwrap_or(0);
            (
                diagnostic["rule"].as_str().unwrap().to_string(),
                file[start..].to_string(),
            )
        })
        .collect();
    let mut expected: BTreeSet<(String, String)> = kiro_detect_expectations()
        .into_iter()
        .filter(|(_, file_type)| *file_type == FileType::KiroNotLoaded)
        .map(|(rel, _)| ("KIRO-026".to_string(), rel.to_string()))
        .collect();
    let broken = "tests/fixtures/kiro-detect/.kiro/workflows/broken.workflow.yaml";
    for rule in [
        "KR-WF-001",
        "KR-WF-002",
        "KR-WF-004",
        "KR-WF-006",
        "KR-WF-007",
    ] {
        expected.insert((rule.to_string(), broken.to_string()));
    }
    expected.insert((
        "KR-IGN-001".to_string(),
        "tests/fixtures/kiro-detect/.kiroignore".to_string(),
    ));
    // A published POWER.md is the legacy power format (KR-PW-019 info).
    expected.insert((
        "KR-PW-019".to_string(),
        "tests/fixtures/kiro-detect/power/POWER.md".to_string(),
    ));
    // The power plugin.json has no author.name (KR-PW-016 info).
    expected.insert((
        "KR-PW-016".to_string(),
        "tests/fixtures/kiro-detect/.kiro/powers/installed/demo/plugin.json".to_string(),
    ));
    // No Claude import, cross-platform or hook rules on Kiro files: power
    // steering has `### @app.entrypoint`, POWER.md documents OS paths, and
    // .kiro/settings/settings.json looks like Claude settings. The valid
    // recipe gets nothing.
    assert_eq!(found, expected);
}
