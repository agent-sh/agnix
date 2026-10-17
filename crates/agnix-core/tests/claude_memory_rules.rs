//! Claude Code memory, prompt-wording and settings rules through the real
//! pipeline: path detection, registry and validators, on the fixture pack in
//! `tests/fixtures/claude-mem/`.

use agnix_core::{DiagnosticLevel, LintConfig, validate_project};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/claude-mem")
        .join(name)
}

/// (rule, file name, level) for every diagnostic of the given families.
fn findings(name: &str) -> Vec<(String, String, DiagnosticLevel)> {
    let result = validate_project(&fixture(name), &LintConfig::default()).unwrap();
    result
        .diagnostics
        .iter()
        .filter(|d| {
            ["CC-MEM-", "CC-SET-", "CC-OS-", "PE-", "XML-"]
                .iter()
                .any(|prefix| d.rule.starts_with(prefix))
        })
        .map(|d| {
            (
                d.rule.clone(),
                d.file.file_name().unwrap().to_string_lossy().into_owned(),
                d.level,
            )
        })
        .collect()
}

fn assert_fires(name: &str, rule: &str, file: &str) {
    let found = findings(name);
    assert!(
        found.iter().any(|(r, f, _)| r == rule && f == file),
        "{rule} on {file} in {name}: {found:?}"
    );
}

#[test]
fn emphasis_overuse_fires_on_claude_md() {
    assert_fires("emphasis", "CC-MEM-017", "CLAUDE.md");
}

#[test]
fn long_memory_file_rule_file_and_combined_size() {
    assert_fires("long-file", "CC-MEM-014", "CLAUDE.md");
    assert_fires("long-rule", "CC-MEM-014", "api.md");
    assert_fires("combined", "CC-MEM-015", "CLAUDE.md");
}

#[test]
fn memory_index_over_load_limit() {
    assert_fires("agent-memory", "CC-MEM-016", "MEMORY.md");
}

#[test]
fn agents_md_reach() {
    assert_fires("agents-prose", "CC-MEM-018", "CLAUDE.md");
    assert_fires("agents-local", "CC-MEM-018", "CLAUDE.local.md");
}

#[test]
fn local_memory_file_not_gitignored_inside_the_repository() {
    // The fixture sits in the agnix git repository, whose .gitignore does not
    // list CLAUDE.local.md.
    assert_fires("agents-local", "CC-MEM-020", "CLAUDE.local.md");
}

#[test]
fn import_paths_with_spaces() {
    let found = findings("import-space");
    let count = found.iter().filter(|(r, _, _)| r == "CC-MEM-019").count();
    assert_eq!(count, 2, "{found:?}");
}

#[test]
fn directory_tree_listing() {
    assert_fires("listing", "CC-MEM-021", "CLAUDE.md");
}

#[test]
fn claudeignore_has_no_effect() {
    assert_fires("claudeignore", "CC-SET-038", ".claudeignore");
}

#[test]
fn valid_memory_fixture_is_clean() {
    let found = findings("valid");
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn settings_key_ignored_at_this_scope() {
    let found = findings("settings-scope");
    let scope_hits = found.iter().filter(|(r, _, _)| r == "CC-SET-034").count();
    // autoMode, allowedProviders, remoteControlAtStartup: true,
    // defaultMode bypassPermissions, env CLAUDE_CONFIG_DIR (the telemetry
    // off value still applies).
    assert_eq!(scope_hits, 5, "{found:?}");
}

#[test]
fn permission_rule_checks() {
    assert_fires("settings-rules", "CC-SET-016", "settings.json");
    assert_fires("settings-rules", "CC-SET-035", "settings.json");
    assert_fires("settings-rules", "CC-SET-036", "settings.json");
    let found = findings("settings-rules");
    // Write(docs/**) in deny protects nothing: error level.
    assert!(
        found
            .iter()
            .any(|(r, _, l)| r == "CC-SET-016" && *l == DiagnosticLevel::Error),
        "{found:?}"
    );
}

#[test]
fn sandbox_weakening_and_secrets() {
    assert_fires("settings-sandbox", "CC-SET-037", "settings.json");
    assert_fires("settings-secrets", "CC-SET-039", "settings.json");
}

#[test]
fn memory_paths_and_output_style_case() {
    assert_fires("settings-paths", "CC-SET-040", "settings.json");
    assert_fires("settings-paths", "CC-OS-007", "settings.json");
}

#[test]
fn valid_settings_fixture_is_clean() {
    let found = findings("settings-valid");
    assert!(found.is_empty(), "{found:?}");
}
