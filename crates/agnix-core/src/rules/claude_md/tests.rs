use super::*;
use crate::config::LintConfig;
use crate::diagnostics::DiagnosticLevel;
use std::fs;
use tempfile::TempDir;

fn validate(path: &Path, content: &str) -> Vec<Diagnostic> {
    ClaudeMdValidator.validate(path, content, &LintConfig::default())
}

fn rules_at(path: &Path, content: &str) -> Vec<String> {
    validate(path, content)
        .into_iter()
        .map(|d| d.rule)
        .collect()
}

fn emphasis_content(lines: usize) -> String {
    (0..lines)
        .map(|i| format!("- IMPORTANT: rule {i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn write(dir: &Path, rel: &str, content: &str) -> PathBuf {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
    path
}

// ===== Scope =====

#[test]
fn test_skip_agents_files() {
    // CC-MEM rules follow Claude Code's CLAUDE.md docs; AGENTS.md is shared.
    let content = emphasis_content(10) + "\n" + &"x\n".repeat(300);
    for name in ["AGENTS.md", "AGENTS.local.md", "AGENTS.override.md"] {
        assert!(validate(Path::new(name), &content).is_empty(), "{name}");
    }
}

#[test]
fn test_cursor_rule_files_get_no_claude_memory_rules() {
    // ClaudeMdValidator is no longer registered for Cursor rules: CC-MEM-009,
    // CC-MEM-007, CC-MEM-004 and CC-MEM-010 used Claude sources and Claude
    // wording ("CLAUDE.md has N percent overlap") on Cursor files.
    let content = emphasis_content(10) + "\n" + &"x\n".repeat(300);
    for path in [
        ".cursor/rules/typescript.mdc",
        ".cursor/rules/typescript.md",
        ".cursorrules",
    ] {
        assert!(validate(Path::new(path), &content).is_empty(), "{path}");
    }
}

#[test]
fn test_claude_local_md_gets_rules() {
    let rules = rules_at(Path::new("CLAUDE.local.md"), &emphasis_content(6));
    assert!(rules.contains(&"CC-MEM-017".to_string()), "{rules:?}");
}

#[test]
fn test_config_disabled_memory_category() {
    let mut config = LintConfig::default();
    config.rules_mut().memory = false;
    let diagnostics =
        ClaudeMdValidator.validate(Path::new("CLAUDE.md"), &emphasis_content(6), &config);
    assert!(diagnostics.is_empty());
}

#[test]
fn test_config_cursor_target_disables_cc_mem_rules() {
    use crate::config::TargetTool;
    let mut config = LintConfig::default();
    config.set_target(TargetTool::Cursor);
    let diagnostics =
        ClaudeMdValidator.validate(Path::new("CLAUDE.md"), &emphasis_content(6), &config);
    assert!(diagnostics.is_empty());
}

#[test]
fn test_retired_rules_are_absent() {
    // CC-MEM-007 (pushed for strong wording) and CC-MEM-009 (unsourced token
    // budget) are retired; CC-MEM-005/006/008 were retired earlier.
    let content =
        "# Critical Rules\n\nYou should follow the coding style.\nBe helpful.\nNever use var.\n"
            .to_string()
            + &"x".repeat(7000);
    let rules = rules_at(Path::new("CLAUDE.md"), &content);
    for retired in [
        "CC-MEM-005",
        "CC-MEM-006",
        "CC-MEM-007",
        "CC-MEM-008",
        "CC-MEM-009",
    ] {
        assert!(!rules.contains(&retired.to_string()), "{retired}");
    }
}

#[test]
fn test_all_cc_mem_rules_can_be_disabled() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "CLAUDE.md", "");
    let long = emphasis_content(6) + "\n" + &"line\n".repeat(250);
    for rule in ["CC-MEM-014", "CC-MEM-017"] {
        assert!(rules_at(&path, &long).contains(&rule.to_string()), "{rule}");
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec![rule.to_string()];
        let diagnostics = ClaudeMdValidator.validate(&path, &long, &config);
        assert!(!diagnostics.iter().any(|d| d.rule == rule), "{rule}");
    }
}

// ===== CC-MEM-004 =====

#[test]
fn test_cc_mem_004_invalid_npm_script() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "package.json",
        r#"{"scripts": {"test": "jest", "build": "tsc"}}"#,
    );
    let path = write(
        dir.path(),
        "CLAUDE.md",
        "Run tests with npm run nonexistent\n",
    );
    let diagnostics = validate(&path, &fs::read_to_string(&path).unwrap());
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-MEM-004")
        .collect();
    assert_eq!(hits.len(), 1);
    assert!(hits[0].message.contains("nonexistent"));
    assert_eq!(hits[0].line, 1);
    assert_eq!(hits[0].column, 16);
}

#[test]
fn test_cc_mem_004_valid_npm_script() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "package.json",
        r#"{"scripts": {"test": "jest"}}"#,
    );
    let path = write(dir.path(), "CLAUDE.md", "Run tests with npm run test\n");
    assert!(!rules_at(&path, "Run tests with npm run test\n").contains(&"CC-MEM-004".into()));
}

// ===== CC-MEM-010 =====

#[test]
fn test_cc_mem_010_readme_duplication_names_the_file() {
    let dir = TempDir::new().unwrap();
    let shared = "This project validates agent configurations using Rust for performance.\n";
    write(dir.path(), "README.md", shared);
    // .claude/CLAUDE.md belongs to the project root and is compared with its
    // README.
    let path = write(dir.path(), ".claude/CLAUDE.md", shared);
    let diagnostics = validate(&path, shared);
    let hit = diagnostics
        .iter()
        .find(|d| d.rule == "CC-MEM-010")
        .expect("CC-MEM-010");
    assert!(
        hit.message.starts_with(".claude/CLAUDE.md has"),
        "{}",
        hit.message
    );
    assert!(hit.message.contains("agnix heuristic"));
}

#[test]
fn test_cc_mem_010_no_duplication() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "README.md",
        "Welcome to the project. Installation: npm install. Usage: npm start.\n",
    );
    let content = "Project-specific instructions for Claude. Focus on these guidelines.\n";
    let path = write(dir.path(), "CLAUDE.md", content);
    assert!(!rules_at(&path, content).contains(&"CC-MEM-010".into()));
}

// ===== CC-MEM-014 =====

#[test]
fn test_cc_mem_014_counts_all_lines() {
    // The memory page says "target under 200 lines per CLAUDE.md file"; blank
    // lines are lines. Previously only non-empty lines counted, so this file
    // (100 rules + 150 blank lines) passed.
    let mut lines: Vec<String> = (1..=100).map(|i| format!("Rule {i}")).collect();
    lines.extend(std::iter::repeat_n(String::new(), 150));
    let content = lines.join("\n") + "\n";
    let diagnostics = validate(Path::new("CLAUDE.md"), &content);
    let hit = diagnostics
        .iter()
        .find(|d| d.rule == "CC-MEM-014")
        .expect("CC-MEM-014");
    assert_eq!(hit.level, DiagnosticLevel::Warning);
    assert!(
        hit.message.contains("CLAUDE.md has 250 lines"),
        "{}",
        hit.message
    );
    let suggestion = hit.suggestion.as_deref().unwrap_or_default();
    assert!(suggestion.contains("path-scoped"), "{suggestion}");
    assert!(!suggestion.contains("(e.g. CLAUDE.local.md)"));
}

#[test]
fn test_cc_mem_014_boundary_and_stripped_comments() {
    let at_limit: String = (1..=200).map(|i| format!("Rule {i}\n")).collect();
    assert!(!rules_at(Path::new("CLAUDE.md"), &at_limit).contains(&"CC-MEM-014".into()));

    // Block-level HTML comments are stripped before Claude reads the file.
    let with_comment = format!("{at_limit}<!--\n{}-->\n", "maintainer note\n".repeat(50));
    assert!(!rules_at(Path::new("CLAUDE.md"), &with_comment).contains(&"CC-MEM-014".into()));

    let over = format!("{at_limit}one more\n");
    assert!(rules_at(Path::new("CLAUDE.md"), &over).contains(&"CC-MEM-014".into()));
}

#[test]
fn test_cc_mem_014_long_import() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "docs/guide.md", &"guide line\n".repeat(230));
    write(dir.path(), "docs/short.md", "short\n");
    let content = "# Project\nSee @docs/guide.md and @docs/short.md.\n";
    let path = write(dir.path(), "CLAUDE.md", content);
    let diagnostics = validate(&path, content);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-MEM-014")
        .collect();
    assert_eq!(hits.len(), 1, "{diagnostics:?}");
    assert_eq!(hits[0].line, 2);
    assert_eq!(hits[0].column, 5);
    assert!(hits[0].message.contains("'@docs/guide.md' has 230 lines"));
}

// ===== CC-MEM-015 =====

#[test]
fn test_cc_mem_015_combined_always_loaded_files() {
    let dir = TempDir::new().unwrap();
    let claude = "# Project\n@docs/conventions.md\n".to_string() + &"line\n".repeat(150);
    let path = write(dir.path(), "CLAUDE.md", &claude);
    write(
        dir.path(),
        "docs/conventions.md",
        &"convention\n".repeat(150),
    );
    write(
        dir.path(),
        ".claude/rules/always.md",
        &"always\n".repeat(120),
    );
    // Path-scoped rules load on demand and do not count.
    write(
        dir.path(),
        ".claude/rules/api.md",
        &("---\npaths:\n  - src/api/**\n---\n".to_string() + &"api\n".repeat(190)),
    );
    let diagnostics = validate(&path, &claude);
    let hit = diagnostics
        .iter()
        .find(|d| d.rule == "CC-MEM-015")
        .expect("CC-MEM-015");
    assert_eq!(hit.level, DiagnosticLevel::Info);
    assert!(
        hit.message.contains("total 422 lines across 3 files"),
        "{}",
        hit.message
    );
    assert!(hit.message.contains("@docs/conventions.md 150"));
    assert!(!hit.message.contains("api.md"));
}

#[test]
fn test_cc_mem_015_reported_once_and_under_limit() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "CLAUDE.md", &"line\n".repeat(150));
    let local = write(dir.path(), "CLAUDE.local.md", &"local\n".repeat(150));
    // 300 lines: under the heuristic.
    assert!(!rules_at(&path, &"line\n".repeat(150)).contains(&"CC-MEM-015".into()));
    write(dir.path(), ".claude/rules/a.md", &"a\n".repeat(150));
    assert!(rules_at(&path, &"line\n".repeat(150)).contains(&"CC-MEM-015".into()));
    // Only the directory's first memory file reports it.
    assert!(!rules_at(&local, &"local\n".repeat(150)).contains(&"CC-MEM-015".into()));
}

// ===== CC-MEM-016 =====

#[test]
fn test_cc_mem_016_memory_index_over_line_limit() {
    let path = Path::new(".claude/agent-memory/reviewer/MEMORY.md");
    let content: String = (1..=214).map(|i| format!("- entry {i}\n")).collect();
    let diagnostics = validate(path, &content);
    let hit = diagnostics
        .iter()
        .find(|d| d.rule == "CC-MEM-016")
        .expect("CC-MEM-016");
    assert_eq!(hit.line, 201);
    assert_eq!(hit.level, DiagnosticLevel::Warning);
    assert!(hit.message.contains("214 lines"));

    let ok: String = (1..=200).map(|i| format!("- entry {i}\n")).collect();
    assert!(validate(path, &ok).is_empty());
}

#[test]
fn test_cc_mem_016_memory_index_over_byte_limit() {
    let path = Path::new(".claude/projects/-home-me-repo/memory/MEMORY.md");
    let line = format!("- {}\n", "x".repeat(498));
    let content = line.repeat(60); // 30,000 bytes in 60 lines
    let hit = validate(path, &content)
        .into_iter()
        .find(|d| d.rule == "CC-MEM-016")
        .expect("CC-MEM-016");
    // 25,600 bytes run out on line 52.
    assert_eq!(hit.line, 52);
}

#[test]
fn test_memory_index_detection() {
    for (path, expected) in [
        (".claude/agent-memory/reviewer/MEMORY.md", true),
        (".claude/agent-memory-local/reviewer/MEMORY.md", true),
        ("home/.claude/projects/p/memory/MEMORY.md", true),
        ("docs/MEMORY.md", false),
        (".claude/agent-memory/reviewer/notes.md", false),
    ] {
        assert_eq!(
            crate::detect_file_type(Path::new(path)) == FileType::ClaudeMemoryIndex,
            expected,
            "{path}"
        );
    }
}

// ===== CC-MEM-017 =====

#[test]
fn test_cc_mem_017_emphasis_overuse() {
    let content = format!("# Rules\n{}\n", emphasis_content(6));
    let diagnostics = validate(Path::new("CLAUDE.md"), &content);
    let hit = diagnostics
        .iter()
        .find(|d| d.rule == "CC-MEM-017")
        .expect("CC-MEM-017");
    assert_eq!(hit.level, DiagnosticLevel::Info);
    assert_eq!((hit.line, hit.column), (2, 3));
    assert!(hit.message.starts_with("6 lines use emphasis"));
    assert!(!hit.has_fixes());
}

#[test]
fn test_cc_mem_017_threshold_and_rfc_documents() {
    assert!(!rules_at(Path::new("CLAUDE.md"), &emphasis_content(5)).contains(&"CC-MEM-017".into()));
    let rfc = format!(
        "The key words MUST and NEVER follow RFC 2119.\n{}",
        emphasis_content(8)
    );
    assert!(!rules_at(Path::new("CLAUDE.md"), &rfc).contains(&"CC-MEM-017".into()));
    // Lowercase "must" is ordinary wording, not emphasis.
    let plain: String = (0..10)
        .map(|i| format!("You must do step {i}.\n"))
        .collect();
    assert!(!rules_at(Path::new("CLAUDE.md"), &plain).contains(&"CC-MEM-017".into()));
}

// ===== CC-MEM-018 =====

#[test]
fn test_cc_mem_018_prose_deferral() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "AGENTS.md", "# Shared\n");
    let content = "# Claude\nRead AGENTS.md before making changes.\n";
    let path = write(dir.path(), "CLAUDE.md", content);
    let hit = validate(&path, content)
        .into_iter()
        .find(|d| d.rule == "CC-MEM-018")
        .expect("CC-MEM-018");
    assert_eq!((hit.line, hit.column), (2, 1));

    let imported = "@AGENTS.md\n\nRead AGENTS.md for shared rules.\n";
    assert!(!rules_at(&path, imported).contains(&"CC-MEM-018".into()));
}

#[test]
fn test_cc_mem_018_no_agents_md_no_finding() {
    let dir = TempDir::new().unwrap();
    let content = "Read AGENTS.md before making changes.\n";
    let path = write(dir.path(), "CLAUDE.md", content);
    assert!(!rules_at(&path, content).contains(&"CC-MEM-018".into()));
}

#[test]
fn test_cc_mem_018_local_file_hides_agents_md() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "AGENTS.md", "# Shared\n");
    let local = write(dir.path(), "CLAUDE.local.md", "My notes.\n");
    let hit = validate(&local, "My notes.\n")
        .into_iter()
        .find(|d| d.rule == "CC-MEM-018")
        .expect("CC-MEM-018");
    assert!(hit.message.contains("stops Claude Code from reading"));

    // With a CLAUDE.md in the same directory, Claude reads CLAUDE.md files
    // anyway, so CLAUDE.local.md changes nothing.
    write(dir.path(), "CLAUDE.md", "# Project\n");
    assert!(!rules_at(&local, "My notes.\n").contains(&"CC-MEM-018".into()));
}

// ===== CC-MEM-019 =====

#[test]
fn test_cc_mem_019_quoted_import() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "Design Docs/api.md", "api\n");
    let content = "- API conventions @\"Design Docs/api.md\"\n- ping @'alice.smith'\n";
    let path = write(dir.path(), "CLAUDE.md", content);
    let hits: Vec<_> = validate(&path, content)
        .into_iter()
        .filter(|d| d.rule == "CC-MEM-019")
        .collect();
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!((hits[0].line, hits[0].column), (1, 19));
    assert!(
        hits[0]
            .suggestion
            .as_deref()
            .unwrap_or_default()
            .contains("@Design\\ Docs/api.md")
    );
}

#[test]
fn test_cc_mem_019_unescaped_space() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "Design Docs/api.md", "api\n");
    let content = "- API conventions @Design Docs/api.md\n";
    let path = write(dir.path(), "CLAUDE.md", content);
    let hit = validate(&path, content)
        .into_iter()
        .find(|d| d.rule == "CC-MEM-019")
        .expect("CC-MEM-019");
    assert!(
        hit.message.contains("'Design Docs/api.md'"),
        "{}",
        hit.message
    );

    // The documented escaped form is fine.
    let escaped = "- API conventions @Design\\ Docs/api.md\n";
    assert!(!rules_at(&path, escaped).contains(&"CC-MEM-019".into()));
}

// ===== CC-MEM-020 =====

#[cfg(feature = "filesystem")]
#[test]
fn test_cc_mem_020_local_file_not_gitignored() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    let local = write(dir.path(), "CLAUDE.local.md", "notes\n");
    let hit = validate(&local, "notes\n")
        .into_iter()
        .find(|d| d.rule == "CC-MEM-020")
        .expect("CC-MEM-020");
    assert_eq!(hit.level, DiagnosticLevel::Info);

    write(dir.path(), ".gitignore", "CLAUDE.local.md\n");
    assert!(!rules_at(&local, "notes\n").contains(&"CC-MEM-020".into()));
}

#[test]
fn test_cc_mem_020_outside_git_repository() {
    // A host's temporary directory can itself be inside a Git repository.
    // Model the absence of all ancestor markers instead of assuming it.
    let filesystem = std::sync::Arc::new(crate::MockFileSystem::new());
    let local = Path::new("not-a-repository/CLAUDE.local.md");
    filesystem.add_file(local, "notes\n");
    let config = LintConfig::builder().fs(filesystem).build().unwrap();
    let diagnostics = ClaudeMdValidator.validate(local, "notes\n", &config);
    assert!(diagnostics.iter().all(|d| d.rule != "CC-MEM-020"));
}

// ===== CC-MEM-021 =====

#[test]
fn test_cc_mem_021_directory_tree() {
    let tree: String = (0..25).map(|i| format!("├── module{i}/\n")).collect();
    let content = format!("# Layout\n```\nrepo/\n{tree}```\n");
    let hit = validate(Path::new("CLAUDE.md"), &content)
        .into_iter()
        .find(|d| d.rule == "CC-MEM-021")
        .expect("CC-MEM-021");
    assert_eq!(hit.line, 4);
    assert!(hit.message.contains("25-line directory tree"));
    // CLAUDE.local.md is personal; the rule targets the checked-in file.
    assert!(!rules_at(Path::new("CLAUDE.local.md"), &content).contains(&"CC-MEM-021".into()));
}

#[test]
fn test_cc_mem_021_short_layout_is_fine() {
    let content = "# Layout\n- `src/api/` - handlers\n- `src/db/` - models\n";
    assert!(!rules_at(Path::new("CLAUDE.md"), content).contains(&"CC-MEM-021".into()));
}

// ===== CC-SET-038 =====

#[test]
fn test_cc_set_038_claudeignore() {
    let path = Path::new("project/.claudeignore");
    assert_eq!(crate::detect_file_type(path), FileType::ClaudeIgnore);
    let hit = validate(path, "# secrets\n.env\nsecrets/\n")
        .into_iter()
        .find(|d| d.rule == "CC-SET-038")
        .expect("CC-SET-038");
    assert_eq!(hit.level, DiagnosticLevel::Warning);
    assert!(hit.message.contains("2 entries"));
}
