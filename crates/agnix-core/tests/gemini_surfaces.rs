//! Gemini CLI custom commands, policies, settings and manifest MCP checks,
//! and agent frontmatter, through the real pipeline (path detection, the
//! `gemini-extension.json` sibling refinement, the registry and the
//! validators) on the fixture pack under tests/fixtures/gemini_surfaces.
use agnix_core::{DiagnosticLevel, FileType, LintConfig, resolve_file_type, validate_project};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixtures(dir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/gemini_surfaces")
        .join(dir)
}

/// (rule, level) pairs per file (relative path), from `validate_project`.
fn project_findings(dir: &str) -> BTreeMap<String, Vec<(String, DiagnosticLevel)>> {
    let root = fixtures(dir);
    let canonical = std::fs::canonicalize(&root).unwrap();
    let result = validate_project(&root, &LintConfig::default()).unwrap();
    let mut out: BTreeMap<String, Vec<(String, DiagnosticLevel)>> = BTreeMap::new();
    // VER-001 (no versions pinned) is a project-level note, not about a file.
    for diagnostic in result
        .diagnostics
        .into_iter()
        .filter(|d| d.rule != "VER-001")
    {
        let file = diagnostic
            .file
            .strip_prefix(&canonical)
            .or_else(|_| diagnostic.file.strip_prefix(&root))
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| diagnostic.file.display().to_string());
        out.entry(file)
            .or_default()
            .push((diagnostic.rule, diagnostic.level));
    }
    for findings in out.values_mut() {
        findings.sort();
        findings.dedup();
    }
    out
}

/// Rule IDs per file, sorted and deduplicated.
fn project_rules(dir: &str) -> BTreeMap<String, Vec<String>> {
    project_findings(dir)
        .into_iter()
        .map(|(file, findings)| {
            let mut rules: Vec<String> = findings.into_iter().map(|(rule, _)| rule).collect();
            rules.dedup();
            (file, rules)
        })
        .collect()
}

fn expect(pairs: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
    pairs
        .iter()
        .map(|(file, rules)| {
            (
                (*file).to_string(),
                rules.iter().map(|r| (*r).to_string()).collect(),
            )
        })
        .collect()
}

#[test]
fn commands_and_policies_are_detected() {
    let config = LintConfig::default();
    for (path, expected) in [
        (
            "valid/.gemini/commands/git/commit.toml",
            FileType::GeminiCommand,
        ),
        (
            "invalid/.gemini/policies/rules.toml",
            FileType::GeminiPolicy,
        ),
        // Members of an extension root, found through gemini-extension.json.
        ("extension/commands/deploy.toml", FileType::GeminiCommand),
        ("extension/commands/gcs/sync.toml", FileType::GeminiCommand),
        ("extension/policies/policies.toml", FileType::GeminiPolicy),
        ("extension/agents/reviewer.md", FileType::GeminiAgent),
    ] {
        assert_eq!(
            resolve_file_type(&fixtures(path), &config),
            expected,
            "{path}"
        );
    }
}

#[test]
fn documented_configuration_gets_no_gemini_findings() {
    // Commands from the custom-commands page, documented settings, a local
    // agent from the subagents page, a list of remote agents and a skipped
    // `_draft.md`.
    let rules = project_rules("valid");
    assert!(
        rules
            .values()
            .flatten()
            .all(|rule| !rule.starts_with("GM-")),
        "{rules:?}"
    );
}

#[test]
fn invalid_project_reports_each_rule() {
    assert_eq!(
        project_rules("invalid"),
        expect(&[
            (".gemini/agents/bad-name.md", &["GM-AG-002"]),
            (".gemini/agents/no-frontmatter.md", &["GM-AG-002"]),
            (".gemini/agents/remote-secret.md", &["GM-AG-005"]),
            // No GM-AG-006: the agent does not load (GM-AG-003, GM-AG-004).
            (
                ".gemini/agents/strict-keys.md",
                &["GM-011", "GM-AG-003", "GM-AG-004"]
            ),
            (".gemini/commands/broken.toml", &["GM-CMD-001"]),
            (".gemini/commands/no-description.toml", &["GM-CMD-002"]),
            (".gemini/commands/no-prompt.toml", &["GM-CMD-001"]),
            (".gemini/commands/unbalanced.toml", &["GM-CMD-003"]),
            (
                ".gemini/policies/rules.toml",
                &["GM-011", "GM-020", "GM-027"]
            ),
            (
                ".gemini/settings.json",
                &[
                    "GM-011", "GM-013", "GM-014", "GM-015", "GM-016", "GM-017", "GM-024", "GM-025",
                ]
            ),
        ])
    );
}

#[test]
fn extension_root_reports_extension_rules() {
    assert_eq!(
        project_rules("extension"),
        expect(&[
            ("agents/reviewer.md", &["GM-AG-006"]),
            ("commands/deploy.toml", &["GM-CMD-002"]),
            (
                "gemini-extension.json",
                &["GM-011", "GM-014", "GM-015", "GM-022", "GM-023"]
            ),
            ("policies/policies.toml", &["GM-021"]),
        ])
    );
}

#[test]
fn levels_match_the_rule_catalog() {
    for dir in ["invalid", "extension"] {
        for (file, findings) in project_findings(dir) {
            for (rule, level) in findings {
                let expected: &[DiagnosticLevel] = match rule.as_str() {
                    "GM-CMD-001" | "GM-CMD-003" | "GM-AG-002" | "GM-AG-003" => {
                        &[DiagnosticLevel::Error]
                    }
                    // Integer fields are load errors; temperature is advice.
                    "GM-AG-004" => &[DiagnosticLevel::Error, DiagnosticLevel::Warning],
                    // A missing transport is an error, several are info.
                    "GM-014" => &[DiagnosticLevel::Error, DiagnosticLevel::Info],
                    // Project settings warn; an extension's trust is ignored.
                    "GM-015" => &[DiagnosticLevel::Warning, DiagnosticLevel::Info],
                    // Parse errors are errors, schema problems warnings.
                    "GM-027" => &[DiagnosticLevel::Warning, DiagnosticLevel::Error],
                    "GM-011" | "GM-016" | "GM-017" | "GM-021" | "GM-022" | "GM-024"
                    | "GM-AG-005" => &[DiagnosticLevel::Warning],
                    "GM-013" | "GM-020" | "GM-023" | "GM-025" | "GM-CMD-002" | "GM-AG-006" => {
                        &[DiagnosticLevel::Info]
                    }
                    other => panic!("unexpected {other} in {dir}/{file}"),
                };
                assert!(
                    expected.contains(&level),
                    "{dir}/{file}: {rule} at {level:?}"
                );
            }
        }
    }
    // The extension's trust is ignored there (info); the project's warns.
    let extension = project_findings("extension");
    assert!(
        extension["gemini-extension.json"].contains(&("GM-015".to_string(), DiagnosticLevel::Info))
    );
    let invalid = project_findings("invalid");
    assert!(
        invalid[".gemini/settings.json"]
            .contains(&("GM-015".to_string(), DiagnosticLevel::Warning))
    );
}
