//! Project-level cross-file validation rules (AGM-006, XP-004/005/006, VER-001).
// All items in this module require the filesystem feature. The file-level inner
// attribute avoids repeating #[cfg(feature = "filesystem")] on each import and
// function - unlike pipeline.rs, this file has no non-filesystem items.
#![cfg(feature = "filesystem")]

use std::path::{Path, PathBuf};

use rust_i18n::t;

use crate::config::LintConfig;
use crate::diagnostics::Diagnostic;
use crate::file_utils;
use crate::parsers::frontmatter::normalize_line_endings;
use crate::schemas;

/// Join an iterator of paths into a comma-separated string.
///
/// Uses `to_string_lossy()` to handle non-UTF-8 paths without panicking;
/// the `Cow::Borrowed` case for valid UTF-8 avoids an intermediate `String`
/// allocation per path.
fn join_paths<'a>(paths: impl Iterator<Item = &'a Path>) -> String {
    paths.enumerate().fold(String::new(), |mut acc, (i, p)| {
        if i > 0 {
            acc.push_str(", ");
        }
        acc.push_str(&p.to_string_lossy());
        acc
    })
}

/// Run project-level checks that require cross-file analysis.
///
/// These checks analyze relationships between multiple files in the project:
/// - AGM-006: Multiple AGENTS.md files
/// - XP-004: Conflicting build/test commands across instruction files
/// - XP-005: Conflicting tool constraints across instruction files
/// - XP-006: Multiple instruction layers without documented precedence
/// - VER-001: No tool/spec versions pinned
///
/// Both `agents_md_paths` and `instruction_file_paths` must be pre-sorted
/// for deterministic output ordering.
pub(crate) fn run_project_level_checks(
    agents_md_paths: &[PathBuf],
    instruction_file_paths: &[PathBuf],
    config: &LintConfig,
    root_dir: &Path,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // AGM-006: Check for multiple AGENTS.md files in the directory tree
    if config.is_rule_enabled("AGM-006") && agents_md_paths.len() > 1 {
        for agents_file in agents_md_paths.iter() {
            let parent_files =
                schemas::agents_md::check_agents_md_hierarchy(agents_file, agents_md_paths);
            let description = if !parent_files.is_empty() {
                let parent_paths = join_paths(parent_files.iter().map(|p| p.as_path()));
                format!(
                    "Nested AGENTS.md detected - parent AGENTS.md files exist at: {parent_paths}",
                )
            } else {
                let other_paths = join_paths(
                    agents_md_paths
                        .iter()
                        .filter(|p| p.as_path() != agents_file.as_path())
                        .map(|p| p.as_path()),
                );
                format!(
                    "Multiple AGENTS.md files detected - other AGENTS.md files exist at: {other_paths}",
                )
            };

            diagnostics.push(
                Diagnostic::warning(
                    agents_file.clone(),
                    1,
                    0,
                    "AGM-006",
                    description,
                )
                .with_suggestion(
                    "Some tools load AGENTS.md hierarchically. Document inheritance behavior or consolidate files.".to_string(),
                ),
            );
        }
    }

    // XP-004, XP-005, XP-006: Cross-layer contradiction detection.
    // Note: when XP-004 is disabled, file-read failures are silently dropped
    // from analysis (no read-error diagnostic is emitted). XP-005 and XP-006
    // then operate only on the files that were successfully read. This is
    // intentional - XP-004 owns the read-error diagnostic.
    let xp004_enabled = config.is_rule_enabled("XP-004");
    let xp005_enabled = config.is_rule_enabled("XP-005");
    let xp006_enabled = config.is_rule_enabled("XP-006");

    if (xp004_enabled || xp005_enabled || xp006_enabled) && instruction_file_paths.len() > 1 {
        // Read content of all instruction files
        let mut file_contents: Vec<(PathBuf, String)> = Vec::new();
        for file_path in instruction_file_paths.iter() {
            match file_utils::safe_read_file(file_path) {
                Ok(raw) => {
                    // Match on the Cow to avoid a second scan: Borrowed means LF-only
                    // (reuse the already-owned String), Owned means normalization was needed.
                    let content = match normalize_line_endings(&raw) {
                        std::borrow::Cow::Borrowed(_) => raw,
                        std::borrow::Cow::Owned(normalized) => normalized,
                    };
                    file_contents.push((file_path.clone(), content));
                }
                Err(e) => {
                    if xp004_enabled {
                        diagnostics.push(
                            Diagnostic::error(
                                file_path.clone(),
                                0,
                                0,
                                "XP-004",
                                t!("rules.xp_004_read_error", error = e.to_string()),
                            )
                            .with_suggestion(t!("rules.xp_004_read_error_suggestion")),
                        );
                    }
                    // When XP-004 is disabled, the unreadable file is silently
                    // excluded from XP-005/006 analysis. See comment above.
                }
            }
        }

        // XP-004: Detect conflicting build/test commands
        if xp004_enabled {
            let file_commands: Vec<_> = file_contents
                .iter()
                .filter_map(|(path, content)| {
                    let cmds = schemas::cross_platform::extract_build_commands(content);
                    if cmds.is_empty() {
                        None
                    } else {
                        Some((path.clone(), cmds))
                    }
                })
                .collect();

            let build_conflicts = schemas::cross_platform::detect_build_conflicts(&file_commands);
            for conflict in build_conflicts {
                diagnostics.push(
                    Diagnostic::warning(
                        conflict.file1.clone(),
                        conflict.file1_line,
                        0,
                        "XP-004",
                        t!(
                            "rules.xp_004.message",
                            file1 = conflict.file1.display().to_string(),
                            mgr1 = conflict.file1_manager.as_str(),
                            file2 = conflict.file2.display().to_string(),
                            mgr2 = conflict.file2_manager.as_str(),
                            cmd_type = match conflict.command_type {
                                schemas::cross_platform::CommandType::Install => "install",
                                schemas::cross_platform::CommandType::Build => "build",
                                schemas::cross_platform::CommandType::Test => "test",
                                schemas::cross_platform::CommandType::Run => "run",
                                schemas::cross_platform::CommandType::Other => "other",
                            }
                        ),
                    )
                    .with_suggestion(t!("rules.xp_004.suggestion")),
                );
            }
        }

        // XP-005: Detect conflicting tool constraints
        if xp005_enabled {
            let file_constraints: Vec<_> = file_contents
                .iter()
                .filter_map(|(path, content)| {
                    let constraints = schemas::cross_platform::extract_tool_constraints(content);
                    if constraints.is_empty() {
                        None
                    } else {
                        Some((path.clone(), constraints))
                    }
                })
                .collect();

            let tool_conflicts = schemas::cross_platform::detect_tool_conflicts(&file_constraints);
            for conflict in tool_conflicts {
                diagnostics.push(
                    Diagnostic::error(
                        conflict.allow_file.clone(),
                        conflict.allow_line,
                        0,
                        "XP-005",
                        t!(
                            "rules.xp_005.message",
                            tool = conflict.tool_name.as_str(),
                            allow_file = conflict.allow_file.display().to_string(),
                            disallow_file = conflict.disallow_file.display().to_string()
                        ),
                    )
                    .with_suggestion(t!("rules.xp_005.suggestion")),
                );
            }
        }

        // XP-006: Detect multiple layers without documented precedence
        if xp006_enabled {
            let layers: Vec<_> = file_contents
                .iter()
                .map(|(path, content)| schemas::cross_platform::categorize_layer(path, content))
                .collect();

            if let Some(issue) = schemas::cross_platform::detect_precedence_issues(&layers) {
                // Report on the first layer file
                if let Some(first_layer) = issue.layers.first() {
                    diagnostics.push(
                        Diagnostic::warning(
                            first_layer.path.clone(),
                            1,
                            0,
                            "XP-006",
                            issue.description,
                        )
                        .with_suggestion(t!("rules.xp_006.suggestion")),
                    );
                }
            }
        }
    }

    // VER-001: Warn when no tool/spec versions are explicitly pinned
    if config.is_rule_enabled("VER-001") {
        let has_any_version_pinned = config.is_claude_code_version_pinned()
            || config.tool_versions().codex.is_some()
            || config.tool_versions().cursor.is_some()
            || config.tool_versions().copilot.is_some()
            || config.is_mcp_revision_pinned()
            || config.spec_revisions().agent_skills_spec.is_some()
            || config.spec_revisions().agents_md_spec.is_some();

        if !has_any_version_pinned {
            // Use .agnix.toml path or project root as the file reference
            let config_file = root_dir.join(".agnix.toml");
            let report_path = if config_file.exists() {
                config_file
            } else {
                root_dir.to_path_buf()
            };

            diagnostics.push(
                Diagnostic::info(report_path, 1, 0, "VER-001", t!("rules.ver_001.message"))
                    .with_suggestion(t!("rules.ver_001.suggestion")),
            );
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;

    #[test]
    fn test_join_paths_empty() {
        assert_eq!(join_paths(std::iter::empty()), "");
    }

    #[test]
    fn test_join_paths_single() {
        let p = std::path::Path::new("/foo/bar.md");
        // Build expected from the path itself so the assertion is platform-correct
        // (path separators may render differently on Windows).
        assert_eq!(join_paths(std::iter::once(p)), p.to_string_lossy().as_ref());
    }

    #[test]
    fn test_join_paths_multiple() {
        let a = std::path::Path::new("/a.md");
        let b = std::path::Path::new("/b.md");
        let c = std::path::Path::new("/c.md");
        // Build expected from the paths themselves to stay platform-correct.
        let expected = format!(
            "{}, {}, {}",
            a.to_string_lossy(),
            b.to_string_lossy(),
            c.to_string_lossy()
        );
        assert_eq!(join_paths([a, b, c].iter().copied()), expected);
    }

    #[test]
    fn test_xp004_read_error_for_missing_instruction_file() {
        use crate::DiagnosticLevel;

        let temp = tempfile::TempDir::new().unwrap();

        // Write a real CLAUDE.md so one file is readable
        let claude_md = temp.path().join("CLAUDE.md");
        std::fs::write(&claude_md, "# Project\n\nRun cargo test to run tests.\n").unwrap();

        // AGENTS.md deliberately does NOT exist on disk
        let agents_md = temp.path().join("AGENTS.md");

        let instruction_file_paths = vec![claude_md, agents_md.clone()];

        let diagnostics = run_project_level_checks(
            &[],
            &instruction_file_paths,
            &LintConfig::default(),
            temp.path(),
        );

        let xp004_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "XP-004" && d.level == DiagnosticLevel::Error)
            .collect();

        assert_eq!(
            xp004_errors.len(),
            1,
            "Expected exactly one XP-004 error for the unreadable AGENTS.md, got: {xp004_errors:?}"
        );
        assert_eq!(
            xp004_errors[0].file, agents_md,
            "XP-004 error should reference the missing AGENTS.md path"
        );
        assert_eq!(
            xp004_errors[0].line, 0,
            "Read-error diagnostic should have line 0"
        );
        assert_eq!(
            xp004_errors[0].column, 0,
            "Read-error diagnostic should have column 0"
        );
        assert!(
            xp004_errors[0]
                .message
                .contains("Failed to read instruction file"),
            "XP-004 message should describe the read failure, got: {}",
            xp004_errors[0].message
        );
        assert!(
            xp004_errors[0].suggestion.is_some(),
            "XP-004 read-error diagnostic should include a suggestion"
        );
    }

    #[test]
    fn test_agm006_disabled_skips_diagnostics() {
        let temp = tempfile::TempDir::new().unwrap();

        let root_agents = temp.path().join("AGENTS.md");
        std::fs::write(&root_agents, "# Root agents\n").unwrap();
        let sub_dir = temp.path().join("subdir");
        std::fs::create_dir(&sub_dir).unwrap();
        let nested_agents = sub_dir.join("AGENTS.md");
        std::fs::write(&nested_agents, "# Nested agents\n").unwrap();

        let agents_md_paths = vec![root_agents, nested_agents];

        let config = LintConfig::builder()
            .disable_rule("AGM-006")
            .build()
            .unwrap();
        let diagnostics = run_project_level_checks(&agents_md_paths, &[], &config, temp.path());
        let agm006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-006").collect();
        assert!(
            agm006.is_empty(),
            "Disabling AGM-006 should suppress all AGM-006 diagnostics, got: {agm006:?}"
        );

        // Sanity check: with default config, AGM-006 diagnostics DO appear
        let diagnostics =
            run_project_level_checks(&agents_md_paths, &[], &LintConfig::default(), temp.path());
        assert!(
            diagnostics.iter().any(|d| d.rule == "AGM-006"),
            "Default config should produce AGM-006 diagnostics for multiple AGENTS.md files"
        );
    }

    #[test]
    fn test_agm006_message_variants() {
        let temp = tempfile::TempDir::new().unwrap();

        // Nested hierarchy: root AGENTS.md and subdir/AGENTS.md
        let root_agents = temp.path().join("AGENTS.md");
        std::fs::write(&root_agents, "# Root\n").unwrap();
        let sub_dir = temp.path().join("subdir");
        std::fs::create_dir(&sub_dir).unwrap();
        let nested_agents = sub_dir.join("AGENTS.md");
        std::fs::write(&nested_agents, "# Nested\n").unwrap();

        let agents_md_paths = vec![root_agents.clone(), nested_agents.clone()];
        let diagnostics =
            run_project_level_checks(&agents_md_paths, &[], &LintConfig::default(), temp.path());

        let agm006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-006").collect();
        assert_eq!(
            agm006.len(),
            2,
            "Expected one diagnostic per AGENTS.md file"
        );

        let nested_diag = agm006
            .iter()
            .find(|d| d.file == nested_agents)
            .expect("Expected a diagnostic for the nested AGENTS.md");
        assert!(
            nested_diag.message.contains("Nested AGENTS.md detected"),
            "Nested file should get 'Nested AGENTS.md detected' message, got: {}",
            nested_diag.message
        );

        let root_diag = agm006
            .iter()
            .find(|d| d.file == root_agents)
            .expect("Expected a diagnostic for the root AGENTS.md");
        assert!(
            root_diag
                .message
                .contains("Multiple AGENTS.md files detected"),
            "Root file should get 'Multiple AGENTS.md files detected' message, got: {}",
            root_diag.message
        );

        // Verify listed paths: each message should name the other file
        assert!(
            nested_diag
                .message
                .contains(&root_agents.to_string_lossy().as_ref()),
            "Nested file's message should list the root AGENTS.md path, got: {}",
            nested_diag.message
        );
        assert!(
            root_diag
                .message
                .contains(&nested_agents.to_string_lossy().as_ref()),
            "Root file's message should list the nested AGENTS.md path, got: {}",
            root_diag.message
        );
    }

    #[test]
    fn test_xp004_disabled_no_spurious_read_error() {
        let temp = tempfile::TempDir::new().unwrap();

        let claude_md = temp.path().join("CLAUDE.md");
        std::fs::write(&claude_md, "# Project\n\nRun cargo test to run tests.\n").unwrap();

        // AGENTS.md deliberately does NOT exist on disk
        let agents_md = temp.path().join("AGENTS.md");

        let instruction_file_paths = vec![claude_md, agents_md];

        let config = LintConfig::builder()
            .disable_rule("XP-004")
            .build()
            .unwrap();
        let diagnostics =
            run_project_level_checks(&[], &instruction_file_paths, &config, temp.path());

        assert!(
            diagnostics.iter().all(|d| d.rule != "XP-004"),
            "Disabling XP-004 should suppress read-error diagnostics, got: {diagnostics:?}"
        );
    }

    #[test]
    fn test_xp004_disabled_xp005_enabled_silent_skip() {
        // When XP-004 is disabled and an instruction file cannot be read,
        // no XP-004 diagnostic is emitted. XP-005 (and XP-006) analyze only
        // the files that were successfully read. With only one readable file
        // remaining, XP-005/006 produce no diagnostics (need > 1 file).
        // This is the documented behavior: XP-004 owns the read-error diagnostic.
        let temp = tempfile::TempDir::new().unwrap();

        let claude_md = temp.path().join("CLAUDE.md");
        std::fs::write(&claude_md, "# Project\n\nRun cargo test to run tests.\n").unwrap();
        // AGENTS.md does not exist - will fail to read
        let agents_md = temp.path().join("AGENTS.md");

        let instruction_file_paths = vec![claude_md, agents_md];

        let config = LintConfig::builder()
            .disable_rule("XP-004")
            .build()
            .unwrap();
        let diagnostics =
            run_project_level_checks(&[], &instruction_file_paths, &config, temp.path());

        assert!(
            diagnostics.iter().all(|d| d.rule != "XP-004"),
            "No XP-004 diagnostics expected when rule is disabled"
        );
        assert!(
            diagnostics
                .iter()
                .all(|d| d.rule != "XP-005" && d.rule != "XP-006"),
            "No XP-005/006 diagnostics expected when only one file is readable"
        );
    }

    #[test]
    fn test_all_xp_rules_disabled_skips_diagnostics() {
        let temp = tempfile::TempDir::new().unwrap();

        let claude_md = temp.path().join("CLAUDE.md");
        std::fs::write(&claude_md, "# Project\n\nRun cargo test to run tests.\n").unwrap();
        let agents_md = temp.path().join("AGENTS.md");

        let instruction_file_paths = vec![claude_md, agents_md];

        let config = LintConfig::builder()
            .disable_rule("XP-004")
            .disable_rule("XP-005")
            .disable_rule("XP-006")
            .build()
            .unwrap();
        let diagnostics =
            run_project_level_checks(&[], &instruction_file_paths, &config, temp.path());

        assert!(
            diagnostics.iter().all(|d| !d.rule.starts_with("XP-")),
            "Disabling all XP rules should produce zero XP diagnostics, got: {diagnostics:?}"
        );

        // Sanity check: with default config, XP-004 read-error diagnostic appears
        let diagnostics = run_project_level_checks(
            &[],
            &instruction_file_paths,
            &LintConfig::default(),
            temp.path(),
        );
        assert!(
            diagnostics.iter().any(|d| d.rule.starts_with("XP-")),
            "Default config should produce XP diagnostics for unreadable file"
        );
    }

    #[test]
    fn test_ver001_disabled_skips_diagnostics() {
        let temp = tempfile::TempDir::new().unwrap();

        let config = LintConfig::builder()
            .disable_rule("VER-001")
            .build()
            .unwrap();
        let diagnostics = run_project_level_checks(&[], &[], &config, temp.path());

        assert!(
            diagnostics.iter().all(|d| d.rule != "VER-001"),
            "Disabling VER-001 should suppress VER-001 diagnostics, got: {diagnostics:?}"
        );

        // Sanity check: default config with no versions pinned should produce VER-001
        let diagnostics = run_project_level_checks(&[], &[], &LintConfig::default(), temp.path());
        let ver001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "VER-001").collect();
        assert!(
            !ver001.is_empty(),
            "Default config should produce VER-001 when no versions are pinned"
        );
        // When .agnix.toml is absent, the diagnostic should reference the project root
        assert_eq!(
            ver001[0].file,
            temp.path(),
            "VER-001 should reference root_dir when .agnix.toml is absent, got: {}",
            ver001[0].file.display()
        );
    }

    #[test]
    fn test_ver001_suppressed_when_version_pinned() {
        use crate::config::ToolVersions;
        let temp = tempfile::TempDir::new().unwrap();

        let config = LintConfig::builder()
            .tool_versions(ToolVersions {
                codex: Some("0.1.0".to_string()),
                ..Default::default()
            })
            .build()
            .unwrap();
        let diagnostics = run_project_level_checks(&[], &[], &config, temp.path());

        assert!(
            diagnostics.iter().all(|d| d.rule != "VER-001"),
            "VER-001 should not fire when at least one tool version is pinned, got: {diagnostics:?}"
        );
    }

    #[test]
    fn test_xp_single_instruction_file_no_diagnostics() {
        let temp = tempfile::TempDir::new().unwrap();

        let claude_md = temp.path().join("CLAUDE.md");
        std::fs::write(&claude_md, "# Project\n\nnpm install\n").unwrap();

        // Exactly one file: the XP block requires len() > 1, so no XP diagnostics
        let diagnostics =
            run_project_level_checks(&[], &[claude_md], &LintConfig::default(), temp.path());

        assert!(
            diagnostics.iter().all(|d| !d.rule.starts_with("XP-")),
            "No XP diagnostics should be produced for a single instruction file, got: {diagnostics:?}"
        );
    }

    #[test]
    fn test_ver001_uses_agnix_toml_path_when_present() {
        let temp = tempfile::TempDir::new().unwrap();

        let agnix_toml = temp.path().join(".agnix.toml");
        std::fs::write(&agnix_toml, "# no versions pinned\n").unwrap();

        let diagnostics = run_project_level_checks(&[], &[], &LintConfig::default(), temp.path());

        let ver001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "VER-001").collect();
        assert_eq!(ver001.len(), 1, "Expected one VER-001 diagnostic");
        assert_eq!(
            ver001[0].file,
            agnix_toml,
            "VER-001 diagnostic should reference .agnix.toml when it exists, got: {}",
            ver001[0].file.display()
        );
    }
}
