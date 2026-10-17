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
        "tests/fixtures/kiro-permissions/valid/.kiro/settings/permissions.yaml",
        "tests/fixtures/kiro-permissions/invalid/.kiro/settings/permissions.yaml",
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
            if let Some(name) = path.file_name().and_then(|name| name.to_str())
                && name.starts_with('.')
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
        ("tests/fixtures/kiro-powers", 7_u64, 4_u64),
        // One finding per invalid fixture (two for unknown-fields.json and
        // missing-name.json, three for invalid-v3-hooks.json); valid-agent.json,
        // valid-hooks.json, valid-markdown-agent.md and minimal-agent.json are
        // clean.
        ("tests/fixtures/kiro-agents", 16_u64, 16_u64),
        // V3 hook fixtures (v3/valid, v3/invalid: one file per rule) and the
        // legacy KR-HK-016/026 and valid IDE 0.x fixtures were added.
        ("tests/fixtures/kiro-hooks", 29_u64, 26_u64),
        ("tests/fixtures/kiro-mcp", 1_u64, 0_u64),
        ("tests/fixtures/kiro-permissions", 2_u64, 4_u64),
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
    let diagnostics = output["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["rule"], "KR-PM-001");
}

#[test]
fn kiro_permissions_fixtures_cover_claimed_rule_ids() {
    let root = workspace_root();
    let valid = root.join("tests/fixtures/kiro-permissions/valid");
    let invalid = root.join("tests/fixtures/kiro-permissions/invalid");
    assert_eq!(diagnostics_len(&run_agnix_json(&valid), &valid), 0);
    let output = run_agnix_json(&invalid);
    let rules: BTreeSet<_> = output["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|diagnostic| diagnostic["rule"].as_str().unwrap())
        .collect();
    assert_eq!(
        rules,
        BTreeSet::from(["KR-PM-002", "KR-PM-003", "KR-PM-004"])
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
            || rel == "tests/fixtures/kiro-mcp/.kiro/settings/mcp.json";

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
    ];

    for (rel, expected) in expectations {
        let detected = detect_file_type(Path::new(rel));
        assert_eq!(
            detected, expected,
            "Unexpected representative file type for fixture {}",
            rel
        );
    }
}
