//! Gemini CLI support through the real pipeline: extension hook and agent
//! routing (path detection plus the `gemini-extension.json` sibling lookup),
//! the hook rules (GM-004, GM-018, GM-019, GM-026) on settings.json and
//! extension hooks, the manifest rules (GM-005, GM-008, GM-028), GM-009 and
//! GM-010, and the fixture packs under tests/fixtures/gemini_hooks.
use agnix_core::{
    Diagnostic, DiagnosticLevel, FileType, LintConfig, MockFileSystem, ValidatorRegistry,
    detect_file_type, resolve_file_type, validate_content, validate_file, validate_project,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn fixtures(dir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/gemini_hooks")
        .join(dir)
}

/// Rule IDs per file (relative path), from `validate_project` on a fixture.
fn project_rules(dir: &str) -> BTreeMap<String, Vec<String>> {
    let root = fixtures(dir);
    let canonical = std::fs::canonicalize(&root).unwrap();
    let result = validate_project(&root, &LintConfig::default()).unwrap();
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
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
        out.entry(file).or_default().push(diagnostic.rule);
    }
    for rules in out.values_mut() {
        rules.sort();
        rules.dedup();
    }
    out
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
fn remember_shaped_extension_gets_gemini_rules_not_claude_hook_rules() {
    // Shaped like the `remember` plugin: `.gemini/` holds the manifest,
    // hooks/hooks.json (Gemini events, `${extensionPath}/../scripts`) and a
    // settings.json whose hooks use `${GEMINI_PROJECT_DIR}`.
    let root = fixtures("remember-layout");
    let hooks = root.join(".gemini/hooks/hooks.json");
    // Path-only detection leaves it alone; the manifest sibling refines it.
    assert_eq!(detect_file_type(&hooks), FileType::Unknown);
    assert_eq!(
        resolve_file_type(&hooks, &LintConfig::default()),
        FileType::GeminiHooks
    );
    assert_eq!(project_rules("remember-layout"), BTreeMap::new());

    // The same content at a Claude plugin path is linted as Claude hooks,
    // which reject the Gemini events: the routing is what keeps CC-HK away.
    let content = std::fs::read_to_string(&hooks).unwrap();
    let registry = ValidatorRegistry::with_defaults();
    let config = LintConfig::default();
    let as_claude = validate_content(
        Path::new("claude-plugin/hooks/hooks.json"),
        &content,
        &config,
        &registry,
    );
    assert!(
        as_claude.iter().any(|d| d.rule == "CC-HK-001"),
        "{as_claude:?}"
    );
    let as_gemini = match validate_file(&hooks, &config).unwrap() {
        agnix_core::ValidationOutcome::Success(diagnostics) => diagnostics,
        other => panic!("{other:?}"),
    };
    assert!(as_gemini.is_empty(), "{as_gemini:?}");
}

#[test]
fn extension_root_routes_hooks_and_agents() {
    let root = fixtures("extension-root");
    let config = LintConfig::default();
    assert_eq!(
        resolve_file_type(&root.join("hooks/hooks.json"), &config),
        FileType::GeminiHooks
    );
    assert_eq!(
        resolve_file_type(&root.join("agents/reviewer.md"), &config),
        FileType::GeminiAgent
    );
    let rules = project_rules("extension-root");
    assert!(
        rules
            .values()
            .flatten()
            .all(|rule| !rule.starts_with("GM-") && !rule.starts_with("CC-")),
        "{rules:?}"
    );
}

#[test]
fn installed_extension_is_detected_by_path() {
    let hooks = fixtures("installed/.gemini/extensions/audit/hooks/hooks.json");
    assert_eq!(detect_file_type(&hooks), FileType::GeminiHooks);
    // Catch-all matcher and a seconds-like timeout.
    assert_eq!(
        project_rules("installed"),
        expect(&[(
            ".gemini/extensions/audit/hooks/hooks.json",
            &["GM-018", "GM-026"]
        )])
    );
}

#[test]
fn invalid_fixtures_report_each_rule_through_the_pipeline() {
    assert_eq!(
        project_rules("invalid"),
        expect(&[
            (
                ".gemini/settings.json",
                &["GM-004", "GM-010", "GM-018", "GM-019", "GM-026"]
            ),
            ("ext/gemini-extension.json", &["GM-008", "GM-028"]),
            (
                "ext/hooks/hooks.json",
                &["GM-004", "GM-018", "GM-019", "GM-026"]
            ),
        ])
    );
}

#[test]
fn levels_match_the_rule_catalog() {
    let root = fixtures("invalid");
    let result = validate_project(&root, &LintConfig::default()).unwrap();
    for diagnostic in result.diagnostics.iter().filter(|d| d.rule != "VER-001") {
        let expected = match diagnostic.rule.as_str() {
            "GM-004" | "GM-010" | "GM-018" | "GM-019" => DiagnosticLevel::Warning,
            "GM-026" | "GM-028" => DiagnosticLevel::Info,
            "GM-008" => DiagnosticLevel::Error,
            other => panic!("unexpected {other}"),
        };
        assert_eq!(diagnostic.level, expected, "{diagnostic:?}");
    }
}

/// `validate_content` (the LSP path) refines through `config.fs()`.
fn mock_config(files: &[(&str, &str)]) -> LintConfig {
    let fs = MockFileSystem::new();
    for (path, content) in files {
        fs.add_file(path, *content);
    }
    let mut builder = LintConfig::builder();
    builder.fs(Arc::new(fs));
    builder.build().unwrap()
}

fn lsp(path: &str, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
    validate_content(
        Path::new(path),
        content,
        config,
        &ValidatorRegistry::with_defaults(),
    )
}

#[test]
fn validate_content_uses_the_configured_file_system() {
    let content = r#"{"hooks": {"BeforeTool": [{"matcher": "write_file", "hooks": [{"type": "command", "command": "echo ok"}]}]}}"#;
    let with_manifest = mock_config(&[("/repo/ext/gemini-extension.json", "{}")]);
    assert!(lsp("/repo/ext/hooks/hooks.json", content, &with_manifest).is_empty());
    let without = mock_config(&[]);
    let claude = lsp("/repo/ext/hooks/hooks.json", content, &without);
    assert!(claude.iter().any(|d| d.rule == "CC-HK-001"), "{claude:?}");
}

#[test]
fn files_overrides_win_over_sibling_refinement() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("hooks")).unwrap();
    std::fs::write(root.join("gemini-extension.json"), "{}").unwrap();
    std::fs::write(root.join("hooks/hooks.json"), "{}").unwrap();
    let mut config = LintConfig::default();
    config.set_root_dir(root.to_path_buf());
    config.files_mut().exclude = vec!["hooks/hooks.json".to_string()];
    assert_eq!(
        resolve_file_type(&root.join("hooks/hooks.json"), &config),
        FileType::Unknown
    );
    config.files_mut().exclude.clear();
    config.files_mut().include_as_generic = vec!["hooks/hooks.json".to_string()];
    assert_eq!(
        resolve_file_type(&root.join("hooks/hooks.json"), &config),
        FileType::GenericMarkdown
    );
    config.files_mut().include_as_generic.clear();
    config.files_mut().include_as_memory = vec!["other.md".to_string()];
    assert_eq!(
        resolve_file_type(&root.join("hooks/hooks.json"), &config),
        FileType::GeminiHooks
    );
}

#[test]
fn documented_settings_produce_no_gemini_findings() {
    // hooks + mcpServers + hooksConfig as documented: no GM-009 (agnix 0.58
    // warned on both keys) and no GM-004.
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/gemini_settings/valid/.gemini/settings.json");
    let diagnostics = match validate_file(&path, &LintConfig::default()).unwrap() {
        agnix_core::ValidationOutcome::Success(diagnostics) => diagnostics,
        other => panic!("{other:?}"),
    };
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn gemini_md_no_longer_gets_prompt_wording_rules() {
    // The vendor's own example lines: "Do not use class components" and
    // "Be concise" used to get PE-006 and PE-005.
    let content = "# Project\n\nThis project is a React app.\n\n## Rules\n\n- Do not use class components.\n- **Tone:** Be concise. Don't explain basic React concepts.\n";
    let diagnostics = lsp("GEMINI.md", content, &LintConfig::default());
    assert!(
        diagnostics.iter().all(|d| !d.rule.starts_with("PE-")),
        "{diagnostics:?}"
    );
    // CLAUDE.md keeps them.
    let claude = lsp("CLAUDE.md", content, &LintConfig::default());
    assert!(
        claude.iter().any(|d| d.rule.starts_with("PE-")),
        "{claude:?}"
    );
}
