//! AGENTS.md validation rules (AGM-001, AGM-002, AGM-004, AGM-007)
//!
//! Validates:
//! - AGM-001: Valid Markdown Structure (MEDIUM) - unclosed code fences, malformed links
//! - AGM-002: Missing Section Headers (LOW) - a long file with no headings
//! - AGM-004: Missing Project Context (LOW) - root file without a project description
//! - AGM-007: Root AGENTS.md lists no build or test command (LOW)
//!
//! AGM-003 (Windsurf's 12,000-character limit) was retired in 0.59.0: the
//! Windsurf family owns Windsurf's limits, and WS-011
//! ([`super::windsurf::WindsurfAgentsMdValidator`]) reported the same files.
//! AGM-005 was retired: it reported the same Claude Code features as XP-001.
//! AGM-006 (nested AGENTS.md) was retired earlier; AGM-008 and AGM-009 are
//! project-level checks in `project_level.rs`.

use crate::{
    config::{LintConfig, PerFileLintConfig},
    diagnostics::{Diagnostic, Fix},
    rules::{Validator, ValidatorMetadata},
    schemas::agents_md::{
        AGM_002_MIN_LINES, MarkdownIssueType, check_markdown_validity, check_project_context,
        check_section_headers, lists_build_or_test_commands, non_blank_line_count,
    },
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["AGM-001", "AGM-002", "AGM-004", "AGM-007"];

/// Whether `path` sits at a repository root: the lint root, or a directory
/// holding `.git` (a nested repository). AGM-004 and AGM-007 apply to these
/// files only; agents.md places the main file "at the root of the
/// repository" and nested files carry package-specific guidance. Without a
/// lint root (a single file validated on its own) the file counts as a root.
pub(crate) fn is_repository_root_file(path: &Path, config: &LintConfig) -> bool {
    let Some(dir) = path.parent() else {
        return true;
    };
    let Some(root) = config.root_dir() else {
        return true;
    };
    if dir.as_os_str().is_empty()
        || dir == root.as_path()
        || config
            .fs()
            .canonicalize(dir)
            .ok()
            .zip(config.fs().canonicalize(root).ok())
            .is_some_and(|(a, b)| a == b)
    {
        return true;
    }
    config.fs().exists(&dir.join(".git"))
}

/// Build manifests and task files that show a project has commands to run.
/// AGM-007 asks for build and test commands only where they exist: a notes or
/// knowledge-base repository has nothing to build.
const BUILD_MANIFESTS: &[&str] = &[
    "package.json",
    "deno.json",
    "deno.jsonc",
    "Cargo.toml",
    "go.mod",
    "pyproject.toml",
    "setup.py",
    "setup.cfg",
    "requirements.txt",
    "Pipfile",
    "tox.ini",
    "noxfile.py",
    "Makefile",
    "makefile",
    "GNUmakefile",
    "justfile",
    "Justfile",
    ".justfile",
    "Taskfile.yml",
    "Taskfile.yaml",
    "CMakeLists.txt",
    "meson.build",
    "build.zig",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "settings.gradle",
    "settings.gradle.kts",
    "build.sbt",
    "Gemfile",
    "Rakefile",
    "composer.json",
    "mix.exs",
    "Package.swift",
    "pubspec.yaml",
    "stack.yaml",
    "dune-project",
    "rebar.config",
    "MODULE.bazel",
    "WORKSPACE",
    "WORKSPACE.bazel",
    "BUILD.bazel",
    "flake.nix",
];

/// Project file extensions that show the same (.NET, Haskell, Ruby gems).
const BUILD_MANIFEST_EXTENSIONS: &[&str] =
    &["sln", "csproj", "fsproj", "vbproj", "cabal", "gemspec"];

/// Whether the directory holding `path` has a build manifest. A file with no
/// directory, or a directory that cannot be read, counts as buildable so the
/// rule still runs on a file validated on its own.
fn has_build_manifest(path: &Path, config: &LintConfig) -> bool {
    let Some(dir) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) else {
        return true;
    };
    let Ok(entries) = config.fs().read_dir(dir) else {
        return true;
    };
    entries.iter().any(|entry| {
        let name = entry
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        BUILD_MANIFESTS.contains(&name)
            || Path::new(name)
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| BUILD_MANIFEST_EXTENSIONS.contains(&ext))
    })
}

pub struct AgentsMdValidator;

impl Validator for AgentsMdValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Only validate AGENTS.md variants (not CLAUDE.md files)
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !matches!(
            filename,
            "AGENTS.md" | "AGENTS.local.md" | "AGENTS.override.md"
        ) {
            return diagnostics;
        }

        // AGM-001: Valid Markdown Structure (WARNING). AGENTS.md is "just
        // standard Markdown", and CommonMark defines an unclosed fence (it runs
        // to the end of the document), so this is advice about a likely
        // mistake, not a syntax error.
        if config.is_rule_enabled("AGM-001") {
            for issue in check_markdown_validity(content) {
                let diagnostic = match issue.issue_type {
                    MarkdownIssueType::UnclosedCodeBlock => {
                        // Closing the fence at the end of the file keeps
                        // every line after it as code, exactly as CommonMark
                        // reads it now; the edit is still left to the user
                        // because the real fix is usually earlier.
                        let insert_pos = content.len();
                        let prefix = if content.ends_with('\n') || content.is_empty() {
                            ""
                        } else {
                            "\n"
                        };
                        Diagnostic::warning(
                            path.to_path_buf(),
                            issue.line,
                            issue.column,
                            "AGM-001",
                            t!("rules.agm_001.unclosed_fence", line = issue.line),
                        )
                        .with_suggestion(t!("rules.agm_001.unclosed_fence_suggestion"))
                        .with_fix(Fix::insert(
                            insert_pos,
                            format!("{prefix}{}\n", issue.fence),
                            "Append a closing code fence",
                            false,
                        ))
                    }
                    MarkdownIssueType::MalformedLink => Diagnostic::warning(
                        path.to_path_buf(),
                        issue.line,
                        issue.column,
                        "AGM-001",
                        t!(
                            "rules.agm_001.message",
                            description = issue.description.as_str()
                        ),
                    )
                    .with_suggestion(t!("rules.agm_001.suggestion")),
                };
                diagnostics.push(diagnostic);
            }
        }

        // AGM-002: Missing Section Headers (INFO). Headings are optional in
        // AGENTS.md; only a long file without any is reported.
        if config.is_rule_enabled("AGM-002") {
            let lines = non_blank_line_count(content);
            if lines > AGM_002_MIN_LINES && check_section_headers(content).is_some() {
                diagnostics.push(
                    Diagnostic::info(
                        path.to_path_buf(),
                        1,
                        0,
                        "AGM-002",
                        t!("rules.agm_002.message", lines = lines),
                    )
                    .with_suggestion(t!("rules.agm_002.suggestion")),
                );
            }
        }

        let at_repository_root = filename == "AGENTS.md" && is_repository_root_file(path, config);

        // AGM-004: Missing Project Context (INFO). A project overview is a
        // "popular choice" in agents.md, and nested files describe a package,
        // so only the repository-root AGENTS.md is checked.
        if at_repository_root
            && config.is_rule_enabled("AGM-004")
            && let Some(issue) = check_project_context(content)
        {
            diagnostics.push(
                Diagnostic::info(
                    path.to_path_buf(),
                    issue.line,
                    issue.column,
                    "AGM-004",
                    t!("rules.agm_004.message"),
                )
                .with_suggestion(t!("rules.agm_004.suggestion")),
            );
        }

        // AGM-007: Root AGENTS.md lists no build or test command (INFO).
        if at_repository_root
            && config.is_rule_enabled("AGM-007")
            && !content.trim().is_empty()
            && !lists_build_or_test_commands(content)
            && has_build_manifest(path, config)
        {
            diagnostics.push(
                Diagnostic::info(
                    path.to_path_buf(),
                    1,
                    0,
                    "AGM-007",
                    t!("rules.agm_007.message"),
                )
                .with_suggestion(t!("rules.agm_007.suggestion")),
            );
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;
    use crate::fs::MockFileSystem;
    use std::sync::Arc;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = AgentsMdValidator;
        validator.validate(Path::new("AGENTS.md"), content, &LintConfig::default())
    }

    fn validate_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = AgentsMdValidator;
        validator.validate(Path::new("AGENTS.md"), content, config)
    }

    fn rules<'a>(diagnostics: &'a [Diagnostic], rule: &str) -> Vec<&'a Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == rule).collect()
    }

    fn windsurf_config() -> LintConfig {
        let mut config = LintConfig::default();
        config.set_tools(vec!["windsurf".to_string()]);
        config
    }

    // ===== Skip non-AGENTS.md files =====

    #[test]
    fn test_skip_non_agents_files() {
        let content = "```unclosed\nSome content";
        for name in ["CLAUDE.md", "README.md", "CLAUDE.local.md"] {
            let diagnostics =
                AgentsMdValidator.validate(Path::new(name), content, &LintConfig::default());
            assert!(diagnostics.is_empty(), "{name}: {diagnostics:?}");
        }
    }

    #[test]
    fn test_agents_variants_get_agm_001() {
        let content = "```unclosed\nSome content";
        for variant in ["AGENTS.local.md", "AGENTS.override.md"] {
            let diagnostics =
                AgentsMdValidator.validate(Path::new(variant), content, &LintConfig::default());
            assert_eq!(rules(&diagnostics, "AGM-001").len(), 1, "{variant}");
        }
    }

    // ===== AGM-001 =====

    /// AGM-001 used to be a HIGH error ("Invalid markdown"). agents.md says an
    /// AGENTS.md "is just standard Markdown", and CommonMark defines what an
    /// unclosed fence does, so it is now a warning that says so.
    #[test]
    fn test_agm_001_unclosed_fence_is_a_warning_with_line() {
        let content = "# Project\n\n```rust\nfn main() {}\n";
        let diagnostics = validate(content);
        let agm_001 = rules(&diagnostics, "AGM-001");
        assert_eq!(agm_001.len(), 1);
        assert_eq!(agm_001[0].level, DiagnosticLevel::Warning);
        assert_eq!(agm_001[0].line, 3);
        assert!(
            agm_001[0].message.contains("line 3") && agm_001[0].message.contains("treated as code"),
            "{}",
            agm_001[0].message
        );
        // The fix closes the fence with the same marker and stays unsafe.
        let fix = &agm_001[0].fixes[0];
        assert!(!fix.safe);
        assert_eq!(fix.replacement, "```\n");
    }

    #[test]
    fn test_agm_001_fix_matches_tilde_fence() {
        let content = "# Project\n\n~~~~\ncode";
        let diagnostics = validate(content);
        let agm_001 = rules(&diagnostics, "AGM-001");
        assert_eq!(agm_001[0].fixes[0].replacement, "\n~~~~\n");
    }

    #[test]
    fn test_agm_001_balanced_code_blocks() {
        let content =
            "# Project\n\n```python\nprint('x')\n```\n\nCheck [this](http://example.com).\n";
        assert!(rules(&validate(content), "AGM-001").is_empty());
    }

    #[test]
    fn test_agm_001_malformed_link_is_a_warning() {
        let content = "# P\n\nCheck [this link](http://example.com for more.\n";
        let diagnostics = validate(content);
        let agm_001 = rules(&diagnostics, "AGM-001");
        assert_eq!(agm_001.len(), 1);
        assert_eq!(agm_001[0].level, DiagnosticLevel::Warning);
    }

    // ===== AGM-002 =====

    /// AGM-002 used to warn on any AGENTS.md without headings. Headings are
    /// optional ("Use any headings you like"), so only a long file is
    /// reported, at info level.
    #[test]
    fn test_agm_002_short_file_without_headings_is_fine() {
        assert!(rules(&validate("Just plain text without any headers."), "AGM-002").is_empty());
    }

    #[test]
    fn test_agm_002_long_file_without_headings_is_info() {
        let content = (0..=AGM_002_MIN_LINES)
            .map(|i| format!("- instruction {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let diagnostics = validate(&content);
        let agm_002 = rules(&diagnostics, "AGM-002");
        assert_eq!(agm_002.len(), 1);
        assert_eq!(agm_002[0].level, DiagnosticLevel::Info);
        let with_heading = format!("# Rules\n{content}");
        assert!(rules(&validate(&with_heading), "AGM-002").is_empty());
    }

    // ===== AGM-003 retirement =====

    /// AGM-003 reported Windsurf's 12,000-character limit on the same
    /// AGENTS.md files as WS-011, which the Windsurf family owns.
    #[test]
    fn test_agm_003_is_retired() {
        assert!(!RULE_IDS.contains(&"AGM-003"));
        let content = "a".repeat(12_001);
        assert!(
            rules(
                &validate_with_config(&content, &windsurf_config()),
                "AGM-003"
            )
            .is_empty()
        );
    }

    // ===== AGM-004 =====

    #[test]
    fn test_agm_004_root_file_without_context_is_info() {
        let content = "# Guidelines\n\nFollow the style guide.\n";
        let diagnostics = validate(content);
        let agm_004 = rules(&diagnostics, "AGM-004");
        assert_eq!(agm_004.len(), 1);
        assert_eq!(agm_004[0].level, DiagnosticLevel::Info);
        assert!(
            rules(
                &validate("# Project\n\n## Tech Stack\n\n- Rust\n"),
                "AGM-004"
            )
            .is_empty()
        );
    }

    #[test]
    fn test_agm_004_and_007_skip_nested_files() {
        let fs = MockFileSystem::new();
        fs.add_dir("/repo");
        fs.add_file("/repo/package.json", "{}");
        fs.add_dir("/repo/pkg");
        fs.add_file("/repo/pkg/package.json", "{}");
        let mut config = LintConfig::default();
        config.set_fs(Arc::new(fs));
        config.set_root_dir(std::path::PathBuf::from("/repo"));
        let content = "# Guidelines\n\nUse tabs.\n";
        let nested = AgentsMdValidator.validate(Path::new("/repo/pkg/AGENTS.md"), content, &config);
        assert!(rules(&nested, "AGM-004").is_empty());
        assert!(rules(&nested, "AGM-007").is_empty());
        let root = AgentsMdValidator.validate(Path::new("/repo/AGENTS.md"), content, &config);
        assert_eq!(rules(&root, "AGM-004").len(), 1);
        assert_eq!(rules(&root, "AGM-007").len(), 1);
    }

    // ===== AGM-007 =====

    #[test]
    fn test_agm_007_root_file_without_commands() {
        let content = "# Project\n\nA date parser.\n\n## Style\n\nUse tabs.\n";
        let diagnostics = validate(content);
        let agm_007 = rules(&diagnostics, "AGM-007");
        assert_eq!(agm_007.len(), 1);
        assert_eq!(agm_007[0].level, DiagnosticLevel::Info);
        let with_commands = format!("{content}\n## Testing\n\nRun `cargo test`.\n");
        assert!(rules(&validate(&with_commands), "AGM-007").is_empty());
        // An empty file is OC-AGM-001's finding, not this one.
        assert!(rules(&validate(""), "AGM-007").is_empty());
    }

    #[test]
    fn test_agm_007_needs_a_build_manifest() {
        // A notes or knowledge-base repository has no commands to list.
        let fs = MockFileSystem::new();
        fs.add_dir("/notes");
        fs.add_file("/notes/AGENTS.md", "");
        fs.add_file("/notes/guide.md", "");
        fs.add_dir("/app");
        fs.add_file("/app/AGENTS.md", "");
        fs.add_file("/app/package.json", "{}");
        fs.add_dir("/dotnet");
        fs.add_file("/dotnet/AGENTS.md", "");
        fs.add_file("/dotnet/App.sln", "");
        let mut config = LintConfig::default();
        config.set_fs(Arc::new(fs));
        let content = "# Project\n\nA date parser.\n\n## Style\n\nUse tabs.\n";
        for (path, expected) in [
            ("/notes/AGENTS.md", 0),
            ("/app/AGENTS.md", 1),
            ("/dotnet/AGENTS.md", 1),
        ] {
            let diagnostics = AgentsMdValidator.validate(Path::new(path), content, &config);
            assert_eq!(rules(&diagnostics, "AGM-007").len(), expected, "{path}");
        }
    }

    // ===== AGM-005 retirement =====

    #[test]
    fn test_agm_005_is_retired() {
        assert!(!RULE_IDS.contains(&"AGM-005"));
        let content = "# Project\n\n- type: PreToolExecution\n  command: echo\n";
        assert!(rules(&validate(content), "AGM-005").is_empty());
    }

    // ===== Config Integration Tests =====

    #[test]
    fn test_config_disabled_agents_md_category() {
        let mut config = windsurf_config();
        config.rules_mut().agents_md = false;
        let content = format!("```unclosed\n{}", "x".repeat(13000));
        let diagnostics = validate_with_config(&content, &config);
        assert!(diagnostics.iter().all(|d| !d.rule.starts_with("AGM-")));
    }

    #[test]
    fn test_all_agm_rules_can_be_disabled() {
        let content = format!("```unclosed\n{}", "x\n".repeat(13000));
        for rule in ["AGM-001", "AGM-002", "AGM-004", "AGM-007"] {
            let mut config = windsurf_config();
            config.rules_mut().disabled_rules = vec![rule.to_string()];
            let diagnostics = validate_with_config(&content, &config);
            assert!(rules(&diagnostics, rule).is_empty(), "{rule}");
        }
    }

    #[test]
    fn test_valid_agents_md_no_errors() {
        let content = "# Project\n\nThis project validates agent configurations.\n\n## Build Commands\n\nRun npm install and npm run build.\n";
        let diagnostics = validate(content);
        assert!(
            diagnostics
                .iter()
                .all(|d| d.level != DiagnosticLevel::Error && !d.rule.starts_with("AGM-")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn opencode_rules_moved_to_the_opencode_instruction_validator() {
        // OC-AGM-001 was retired and OC-AGM-002 moved to
        // rules/opencode_files.rs, gated on an OpenCode project.
        let diagnostics = validate("Some content\nexport API_KEY=ghp_abc123\nOther stuff");
        assert!(diagnostics.iter().all(|d| !d.rule.starts_with("OC-AGM")));
    }
}
