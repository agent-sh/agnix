//! Every Kiro rule must fire through the real pipeline: path detection, the
//! validator registry and the validator, as the `agnix` binary runs them.
//!
//! Unit tests call validators directly, which hides routing bugs: a rule whose
//! file is detected as another type (or as a file Kiro never loads) passes its
//! unit test but can never fire for users. This test runs the CLI over the Kiro
//! fixture families and requires each Kiro rule in `knowledge-base/rules.json`
//! (IDs starting with `KIRO-` or `KR-`) to fire at least once, so a new rule
//! cannot ship unreachable.
//!
//! `tests/fixtures/kiro-reach/<rule-id>/` holds one minimal project per rule
//! that no other fixture family triggers. Each one is linted on its own and
//! must produce its rule. A fixture that needs a pinned Kiro version carries
//! its own `.agnix.toml`, which the CLI finds the way it does for users.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const REACH_ROOT: &str = "tests/fixtures/kiro-reach";

/// Kiro fixture families outside `tests/fixtures/kiro-*`.
const EXTRA_FAMILY_ROOTS: &[&str] = &[
    "tests/fixtures/invalid/kiro-settings",
    "tests/fixtures/valid/kiro-settings",
];

/// Neutral config for roots without their own `.agnix.toml`: every rule on,
/// without the repository's `tests/fixtures/**` exclusion and the
/// version-pinning note.
const NEUTRAL_CONFIG: &str = "[rules]\ndisabled_rules = [\"VER-001\"]\n";

fn workspace_root() -> &'static Path {
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

/// Kiro rule IDs from `knowledge-base/rules.json`.
fn kiro_rule_ids() -> BTreeSet<String> {
    let path = workspace_root().join("knowledge-base/rules.json");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
    let json: Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path.display(), e));
    let ids: BTreeSet<String> = json["rules"]
        .as_array()
        .expect("rules.json has a rules array")
        .iter()
        .filter_map(|rule| rule["id"].as_str())
        .filter(|id| id.starts_with("KIRO-") || id.starts_with("KR-"))
        .map(str::to_string)
        .collect();
    assert!(
        ids.len() > 100,
        "Expected the Kiro rules in rules.json, found {}",
        ids.len()
    );
    ids
}

fn sorted_dirs(dir: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", dir.display(), e))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    dirs
}

/// The per-rule fixtures under `tests/fixtures/kiro-reach`.
fn reach_fixtures() -> Vec<PathBuf> {
    let fixtures = sorted_dirs(&workspace_root().join(REACH_ROOT));
    assert!(!fixtures.is_empty(), "Expected fixtures under {REACH_ROOT}");
    fixtures
}

/// Every root the pipeline runs on: each `tests/fixtures/kiro-*` family (one
/// run per family), the extra families, and each kiro-reach fixture.
fn fixture_roots() -> Vec<PathBuf> {
    let root = workspace_root();
    let mut roots: Vec<PathBuf> = sorted_dirs(&root.join("tests/fixtures"))
        .into_iter()
        .filter(|dir| {
            let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.starts_with("kiro-") && name != "kiro-reach"
        })
        .collect();
    roots.extend(EXTRA_FAMILY_ROOTS.iter().map(|rel| root.join(rel)));
    roots.extend(reach_fixtures());
    for path in &roots {
        assert!(path.is_dir(), "Expected fixture root {}", path.display());
    }
    roots
}

/// Rule IDs the `agnix` binary reports for `target`. A root with its own
/// `.agnix.toml` runs without `--config`, so the CLI discovers that file;
/// other roots get the neutral config. `tools` (for example `["kiro"]`)
/// selects tools the way a project's `.agnix.toml` does.
fn fired_rules(target: &Path, tools: Option<&str>) -> BTreeSet<String> {
    let own_config = target.join(".agnix.toml");
    let base = if own_config.is_file() {
        Some(fs::read_to_string(&own_config).unwrap())
    } else {
        None
    };
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("agnix");
    cmd.current_dir(workspace_root());
    // Kept alive until the command has run.
    let config_file = tempfile::NamedTempFile::new().unwrap();
    if base.is_none() || tools.is_some() {
        // Top-level keys come before the config's tables.
        let tools_line = tools.map_or(String::new(), |tools| format!("tools = {tools}\n"));
        let body = base.as_deref().unwrap_or(NEUTRAL_CONFIG);
        fs::write(config_file.path(), format!("{tools_line}{body}")).unwrap();
        cmd.arg("--config").arg(config_file.path());
    }
    let output = cmd
        .arg(target)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap_or_else(|e| panic!("Failed to run agnix on {}: {}", target.display(), e));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() || output.status.code() == Some(1),
        "agnix exited with status {} for {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        target.display(),
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| {
        panic!(
            "Expected JSON output for {}, got:\n{}\nstderr:\n{}",
            target.display(),
            stdout,
            String::from_utf8_lossy(&output.stderr)
        )
    });
    json["diagnostics"]
        .as_array()
        .unwrap_or_else(|| panic!("diagnostics missing for {}", target.display()))
        .iter()
        .filter_map(|diagnostic| diagnostic["rule"].as_str())
        .map(str::to_string)
        .collect()
}

/// Fired rules per fixture root, running the roots in parallel.
fn run_all(tools: Option<&'static str>) -> BTreeMap<PathBuf, BTreeSet<String>> {
    let roots = fixture_roots();
    std::thread::scope(|scope| {
        let handles: Vec<_> = roots
            .chunks(roots.len().div_ceil(4))
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|root| (root.clone(), fired_rules(root, tools)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("fixture run panicked"))
            .collect()
    })
}

fn default_runs() -> &'static BTreeMap<PathBuf, BTreeSet<String>> {
    static RUNS: OnceLock<BTreeMap<PathBuf, BTreeSet<String>>> = OnceLock::new();
    RUNS.get_or_init(|| run_all(None))
}

fn unfired(runs: &BTreeMap<PathBuf, BTreeSet<String>>) -> Vec<String> {
    let fired: BTreeSet<&String> = runs.values().flatten().collect();
    kiro_rule_ids()
        .into_iter()
        .filter(|id| !fired.contains(id))
        .collect()
}

fn relative(path: &Path) -> String {
    path.strip_prefix(workspace_root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[test]
fn every_kiro_rule_fires_through_the_pipeline() {
    let missing = unfired(default_runs());
    assert!(
        missing.is_empty(),
        "These Kiro rules never fire on any Kiro fixture through the CLI pipeline: {}.\n\
         Add a minimal fixture at a real Kiro path under {REACH_ROOT}/<rule-id-lowercase>/ \
         (with an .agnix.toml when the rule needs a pinned Kiro version). If no real path \
         reaches the rule, fix the detection or validator routing, or retire the rule.",
        missing.join(", ")
    );
}

#[test]
fn every_kiro_rule_fires_when_the_project_selects_kiro() {
    // `tools = ["kiro"]` is how a Kiro project scopes agnix; tool filtering
    // must not hide any Kiro rule.
    let missing = unfired(&run_all(Some(r#"["kiro"]"#)));
    assert!(
        missing.is_empty(),
        "These Kiro rules never fire with tools = [\"kiro\"]: {}",
        missing.join(", ")
    );
}

#[test]
fn each_kiro_reach_fixture_produces_its_own_rule() {
    let known = kiro_rule_ids();
    let runs = default_runs();
    let mut problems = Vec::new();
    for fixture in reach_fixtures() {
        let name = fixture.file_name().unwrap().to_string_lossy().to_string();
        let rule = name.to_ascii_uppercase();
        if !known.contains(&rule) {
            problems.push(format!(
                "{}: directory name is not a Kiro rule ID in rules.json",
                relative(&fixture)
            ));
            continue;
        }
        let fired = &runs[&fixture];
        if !fired.contains(&rule) {
            problems.push(format!(
                "{}: expected {} but got [{}]",
                relative(&fixture),
                rule,
                fired.iter().cloned().collect::<Vec<_>>().join(", ")
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn kiro_reach_version_pins_are_kiro_pins() {
    // A per-fixture config only pins the Kiro version; anything else would
    // make the fixture depend on settings users do not have.
    for fixture in reach_fixtures() {
        let config = fixture.join(".agnix.toml");
        if !config.is_file() {
            continue;
        }
        let text = fs::read_to_string(&config).unwrap();
        let parsed: toml::Value = toml::from_str(&text)
            .unwrap_or_else(|e| panic!("{} does not parse: {}", relative(&config), e));
        let table = parsed.as_table().unwrap();
        let keys: Vec<&String> = table.keys().collect();
        assert_eq!(keys, ["tool_versions"], "{}", relative(&config));
        let versions = table["tool_versions"].as_table().unwrap();
        assert_eq!(
            versions.keys().collect::<Vec<_>>(),
            ["kiro"],
            "{}",
            relative(&config)
        );
    }
}
