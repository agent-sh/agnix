//! Project-level cross-file validation rules (AGM-008, AGM-009, AS-025,
//! XP-004/005/006, VER-001).
// All items in this module require the filesystem feature. The file-level inner
// attribute avoids repeating #[cfg(feature = "filesystem")] on each import and
// function - unlike pipeline.rs, this file has no non-filesystem items.
#![cfg(feature = "filesystem")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rust_i18n::t;

use crate::config::LintConfig;
use crate::diagnostics::Diagnostic;
use crate::file_utils;
use crate::parsers::frontmatter::normalize_line_endings;
use crate::schemas;

/// Run project-level checks that require cross-file analysis.
///
/// These checks analyze relationships between multiple files in the project:
/// - XP-004: Conflicting build/test commands across instruction files
/// - XP-005: Conflicting tool constraints across instruction files
/// - XP-006: Multiple instruction layers without documented precedence
/// - VER-001: No tool/spec versions pinned
/// - AGM-008: Legacy AGENT.md without AGENTS.md
/// - AGM-009: Nested AGENTS.md without a root AGENTS.md
/// - AS-025: The same skill name in two skill roots one client reads
///
/// `agents_md_paths` holds every AGENTS.md, AGENT.md and SKILL.md the walk
/// found. Both it and `instruction_file_paths` must be pre-sorted for
/// deterministic output ordering.
pub(crate) fn run_project_level_checks(
    agents_md_paths: &[PathBuf],
    instruction_file_paths: &[PathBuf],
    config: &LintConfig,
    root_dir: &Path,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

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
                    if xp004_enabled && config.for_path(file_path).is_rule_enabled("XP-004") {
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

        // Drop any `AGENTS.md` shadowed by an `AGENTS.override.md` in the same
        // directory. Codex "includes at most one file per directory", checking
        // the override first, so the shadowed file is never loaded - the doc's
        // own tree labels it "Ignored because an override exists". Comparing the
        // two as peers reported a conflict between a file and the thing whose
        // whole purpose is to differ from it.
        let shadowed: std::collections::BTreeSet<PathBuf> = file_contents
            .iter()
            .filter(|(path, _)| {
                // Case-insensitive, matching `is_instruction_file()`. The three
                // places that name this file have to agree: that helper collects
                // it case-insensitively, so an exact-match filter here left a
                // lowercase `agents.override.md` shadowing nothing while still
                // being treated as an instruction file.
                path.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case("AGENTS.override.md"))
            })
            .filter_map(|(path, _)| path.parent().map(|dir| dir.join("AGENTS.md")))
            .collect();
        if !shadowed.is_empty() {
            file_contents.retain(|(path, _)| !shadowed.contains(path));
        }

        // XP-004/005/006: each detector runs on a per-rule filtered subset
        // of `file_contents`. A file whose `[[overrides]]` disable the rule
        // is removed from that rule's candidate set up front, so the
        // detector never sees it. This makes suppression deterministic and
        // independent of which file the detector picks as the diagnostic
        // report path (cross-file detectors group by HashMap, so the report
        // path is otherwise non-deterministic when only one side of a
        // conflict carries the override).

        // XP-004: Detect conflicting build/test commands
        if xp004_enabled {
            let file_commands: Vec<_> = file_contents
                .iter()
                .filter(|(path, _)| config.for_path(path).is_rule_enabled("XP-004"))
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
                .filter(|(path, _)| config.for_path(path).is_rule_enabled("XP-005"))
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
                // A warning: the vendors ask authors to avoid conflicting
                // instructions (SHOULD); no tool rejects the files.
                diagnostics.push(
                    Diagnostic::warning(
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
            // Filter participating files first, then dedupe identical-content
            // instruction files (e.g., CLAUDE.md and AGENTS.md that are
            // intentional byte-for-byte copies). Identical files are not
            // conflicting layers and should not trigger a precedence warning.
            let xp006_contents: Vec<&(PathBuf, String)> = file_contents
                .iter()
                .filter(|(path, _)| config.for_path(path).is_rule_enabled("XP-006"))
                .collect();

            let mut deduped_contents: Vec<&(PathBuf, String)> = Vec::new();
            for entry in &xp006_contents {
                let dominated = deduped_contents.iter().any(|(_, c)| c == &entry.1);
                if !dominated {
                    deduped_contents.push(entry);
                }
            }

            let layers: Vec<_> = deduped_contents
                .iter()
                .map(|(path, content)| schemas::cross_platform::categorize_layer(path, content))
                .collect();

            if let Some(issue) = schemas::cross_platform::detect_precedence_issues(&layers) {
                // Report on the first layer file
                if let Some(first_layer) = issue.layers.first() {
                    // Info: an agnix heuristic. Each tool documents its own
                    // precedence, so this only asks authors to keep the
                    // files consistent.
                    diagnostics.push(
                        Diagnostic::info(
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

    check_agents_md_layout(agents_md_paths, config, root_dir, &mut diagnostics);
    check_duplicate_skill_names(agents_md_paths, config, &mut diagnostics);

    // VER-001: Warn when no tool/spec versions are explicitly pinned
    if config.is_rule_enabled("VER-001") {
        let has_any_version_pinned = config.is_claude_code_version_pinned()
            || config.tool_versions().codex.is_some()
            || config.tool_versions().cursor.is_some()
            || config.tool_versions().copilot.is_some()
            || config.tool_versions().kiro.is_some()
            || config.tool_versions().opencode.is_some()
            || config.is_mcp_revision_pinned()
            || config.spec_revisions().agent_skills_spec.is_some()
            || config.spec_revisions().agents_md_spec.is_some();

        if !has_any_version_pinned {
            let report_path = config
                .config_path()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| root_dir.to_path_buf());

            // Honor `[[overrides]]` carve-outs targeting `.agnix.toml` (or the
            // project root). Per-file gating runs after the global guard above.
            if config.for_path(&report_path).is_rule_enabled("VER-001") {
                diagnostics.push(
                    Diagnostic::info(report_path, 1, 0, "VER-001", t!("rules.ver_001.message"))
                        .with_suggestion(t!("rules.ver_001.suggestion")),
                );
            }
        }
    }

    diagnostics
}

/// AGM-008 and AGM-009.
///
/// - AGM-008: an `AGENT.md` in a directory without `AGENTS.md`. agents.md
///   asks to "Rename existing files to AGENTS.md and create symbolic links
///   for backward compatibility"; Amp reads AGENT.md only as a
///   fallback, and the other AGENTS.md readers do not read it.
/// - AGM-009: nested AGENTS.md files in a repository whose root has none
///   ("Create an AGENTS.md file at the root of the repository"). Checked only
///   when the lint root is a Git repository root.
fn check_agents_md_layout(
    agents_md_paths: &[PathBuf],
    config: &LintConfig,
    root_dir: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let fs = config.fs();
    let named = |path: &PathBuf, name: &str| {
        path.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == name)
    };

    if config.is_rule_enabled("AGM-008") {
        for legacy in agents_md_paths.iter().filter(|p| named(p, "AGENT.md")) {
            let Some(dir) = legacy.parent() else { continue };
            if fs.exists(&dir.join("AGENTS.md")) {
                continue;
            }
            if config.for_path(legacy).is_rule_enabled("AGM-008") {
                diagnostics.push(
                    Diagnostic::info(legacy.clone(), 1, 0, "AGM-008", t!("rules.agm_008.message"))
                        .with_suggestion(t!("rules.agm_008.suggestion")),
                );
            }
        }
    }

    if config.is_rule_enabled("AGM-009") && fs.exists(&root_dir.join(".git")) {
        let root_has_file = ["AGENTS.md", "AGENTS.override.md"]
            .iter()
            .any(|name| fs.exists(&root_dir.join(name)));
        let nested = agents_md_paths
            .iter()
            .filter(|p| named(p, "AGENTS.md"))
            .filter(|p| p.parent().is_some_and(|dir| dir != root_dir))
            .min_by_key(|p| (p.components().count(), (*p).clone()));
        if !root_has_file
            && let Some(nested) = nested
            && config.for_path(nested).is_rule_enabled("AGM-009")
        {
            diagnostics.push(
                Diagnostic::info(nested.clone(), 1, 0, "AGM-009", t!("rules.agm_009.message"))
                    .with_suggestion(t!("rules.agm_009.suggestion")),
            );
        }
    }
}

/// Project skill directories and the clients that read each one.
const SKILL_ROOTS: &[(&str, &[&str])] = &[
    (
        ".claude/skills",
        &[
            "claude-code",
            "cursor",
            "github-copilot",
            "opencode",
            "cline",
            "windsurf",
        ],
    ),
    (
        ".agents/skills",
        &[
            "codex",
            "amp",
            "cline",
            "windsurf",
            "github-copilot",
            "opencode",
            "cursor",
            "gemini-cli",
        ],
    ),
    (".codex/skills", &["codex", "cursor"]),
    (".cursor/skills", &["cursor"]),
    (".github/skills", &["github-copilot"]),
    (".opencode/skills", &["opencode"]),
    (".cline/skills", &["cline"]),
    (".clinerules/skills", &["cline"]),
    (".windsurf/skills", &["windsurf"]),
    (".devin/skills", &["windsurf"]),
    (".kiro/skills", &["kiro"]),
    (".gemini/skills", &["gemini-cli"]),
];

/// AS-025: one skill name in two skill roots that a single client reads.
/// Clients resolve such a collision in their own order (the Agent Skills
/// client guide: "either first-found or last-found is acceptable"), so one
/// copy is silently shadowed. Skills are grouped by project: the directory
/// that holds the skill roots (`<project>/.claude/skills/<name>/SKILL.md`).
/// Byte-identical copies (mirrored skills) are not reported.
fn check_duplicate_skill_names(
    walked_paths: &[PathBuf],
    config: &LintConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !config.is_rule_enabled("AS-025") {
        return;
    }
    let fs = config.fs();
    let selected = config.tools();
    let in_scope = |client: &str| {
        selected.is_empty()
            || selected
                .iter()
                .any(|t| t.eq_ignore_ascii_case(client) || LintConfig::is_tool_alias(t, client))
    };

    struct SkillEntry {
        project: PathBuf,
        root: &'static str,
        readers: &'static [&'static str],
        name: String,
        path: PathBuf,
        /// `<root>/<skill directory>`, such as `.agents/skills/review`.
        location: String,
        content: String,
    }
    let mut skills: Vec<SkillEntry> = Vec::new();
    for skill_md in walked_paths
        .iter()
        .filter(|p| p.file_name().and_then(|n| n.to_str()) == Some("SKILL.md"))
    {
        // <project>/<root>/<skill>/SKILL.md, where <root> is two components.
        let Some(root_path) = skill_md.parent().and_then(Path::parent) else {
            continue;
        };
        let Some((root, readers)) = SKILL_ROOTS
            .iter()
            .find(|(root, _)| root_path.ends_with(root))
        else {
            continue;
        };
        let Some(project) = root_path.parent().and_then(Path::parent) else {
            continue;
        };
        let Ok(content) = fs.read_to_string(skill_md) else {
            continue;
        };
        let skill_dir = skill_md
            .parent()
            .and_then(Path::file_name)
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if let Some(name) = crate::rules::skill::skill_name(skill_md, &content) {
            skills.push(SkillEntry {
                project: project.to_path_buf(),
                root,
                readers,
                name,
                path: skill_md.clone(),
                location: format!("{root}/{skill_dir}"),
                content,
            });
        }
    }

    let mut reported: std::collections::BTreeSet<PathBuf> = std::collections::BTreeSet::new();
    for (i, a) in skills.iter().enumerate() {
        for b in skills.iter().skip(i + 1) {
            if a.project != b.project
                || a.root == b.root
                || a.name != b.name
                || a.content == b.content
                // A stub that sends the agent to the other copy
                // ("Read `.agents/skills/review/SKILL.md`") is one skill.
                || a.content.contains(&b.location)
                || b.content.contains(&a.location)
            {
                continue;
            }
            let shared: Vec<&str> = a
                .readers
                .iter()
                .copied()
                .filter(|client| b.readers.contains(client) && in_scope(client))
                .collect();
            if shared.is_empty() || !reported.insert(b.path.clone()) {
                continue;
            }
            if config.for_path(&b.path).is_rule_enabled("AS-025") {
                diagnostics.push(
                    Diagnostic::info(
                        b.path.clone(),
                        1,
                        0,
                        "AS-025",
                        t!(
                            "rules.as_025.message",
                            name = a.name.as_str(),
                            other = a.path.display().to_string(),
                            clients = shared.join(", ")
                        ),
                    )
                    .with_suggestion(t!("rules.as_025.suggestion")),
                );
            }
        }
    }
}

/// Build every Codex instruction chain, one per leaf directory. XP-009 used
/// this before it was retired; the Codex rule that replaces it (CDX-AG-010)
/// calls it.
///
/// A chain is a single root-to-directory path: "Starting at the project root
/// (typically the Git root), Codex walks down to your current working
/// directory", taking at most one file per directory in the order
/// `AGENTS.override.md`, then `AGENTS.md`. Sibling subtrees are separate
/// chains and are never concatenated, so each is summed on its own. A leaf
/// inside a nested Git repository starts its chain at that repository.
///
/// Only files the project walk already collected are considered, so this adds no
/// filesystem traversal. Each chain is returned root-first, the order Codex
/// concatenates in.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn codex_instruction_chains(
    instruction_file_paths: &[PathBuf],
    root_dir: &Path,
    fs: &dyn crate::fs::FileSystem,
) -> Vec<Vec<PathBuf>> {
    // `AGENTS.override.md` first, then `AGENTS.md`. Codex also honors any
    // `project_doc_fallback_filenames`, but those live in the user's Codex
    // `config.toml` rather than anything agnix reads, so the chain is built from
    // the two default names. A project relying on fallbacks would have a longer
    // real chain, which makes this conservative: it can under-report, never
    // over-report.
    let precedence: [&str; 2] = ["AGENTS.override.md", "AGENTS.md"];

    // One entry per directory, keeping only the highest-precedence name there.
    let mut by_dir: BTreeMap<PathBuf, PathBuf> = BTreeMap::new();
    for path in instruction_file_paths {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        // Case-insensitive for the same reason as the shadow filter above.
        let Some(rank) = precedence.iter().position(|p| p.eq_ignore_ascii_case(name)) else {
            continue;
        };
        let Some(dir) = path.parent() else { continue };
        if !dir.starts_with(root_dir) {
            continue;
        }
        by_dir
            .entry(dir.to_path_buf())
            .and_modify(|existing| {
                let existing_rank = existing
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|n| precedence.iter().position(|p| p.eq_ignore_ascii_case(n)))
                    .unwrap_or(usize::MAX);
                if rank < existing_rank {
                    *existing = path.clone();
                }
            })
            .or_insert_with(|| path.clone());
    }

    // A leaf is a directory with no deeper instruction-bearing descendant. Every
    // leaf yields one chain: itself plus each ancestor that holds a file.
    let dirs: Vec<&PathBuf> = by_dir.keys().collect();
    dirs.iter()
        .filter(|dir| {
            !dirs
                .iter()
                .any(|other| other != *dir && other.starts_with(dir.as_path()))
        })
        .map(|leaf| {
            // Codex starts at the Git root, so a nested repository (its own
            // `.git` directory or file) begins a new chain: files above it are
            // never loaded for this leaf.
            let start = leaf
                .ancestors()
                .take_while(|dir| dir.starts_with(root_dir))
                .find(|dir| fs.exists(&dir.join(".git")))
                .unwrap_or(root_dir);
            let mut chain: Vec<PathBuf> = by_dir
                .iter()
                .filter(|(dir, _)| leaf.starts_with(dir.as_path()) && dir.starts_with(start))
                .map(|(_, file)| file.clone())
                .collect();
            // BTreeMap iteration is shallow-to-deep for nested paths, matching
            // root-down concatenation; sort defensively on depth anyway.
            chain.sort_by_key(|p| p.components().count());
            chain
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;

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
    fn test_agm006_retired_config_and_diagnostics() {
        let temp = tempfile::TempDir::new().unwrap();

        let root_agents = temp.path().join("AGENTS.md");
        std::fs::write(&root_agents, "# Root agents\n").unwrap();
        let sub_dir = temp.path().join("subdir");
        std::fs::create_dir(&sub_dir).unwrap();
        let nested_agents = sub_dir.join("AGENTS.md");
        std::fs::write(&nested_agents, "# Nested agents\n").unwrap();

        let agents_md_paths = vec![root_agents, nested_agents];

        // Retired rule IDs are rejected by the validated builder.
        assert!(
            LintConfig::builder()
                .disable_rule("AGM-006")
                .build()
                .is_err()
        );

        // Sanity check: with default config, AGM-006 remains retired
        let diagnostics =
            run_project_level_checks(&agents_md_paths, &[], &LintConfig::default(), temp.path());
        assert!(
            !diagnostics.iter().any(|d| d.rule == "AGM-006"),
            "Default config must not produce AGM-006 diagnostics for multiple AGENTS.md files"
        );
    }

    #[test]
    fn test_agm006_retired_for_nested_and_root_files() {
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
        assert!(
            agm006.is_empty(),
            "Supported hierarchy must not be diagnosed"
        );
    }

    #[test]
    fn test_agm_008_legacy_agent_md() {
        let temp = tempfile::TempDir::new().unwrap();
        let legacy = temp.path().join("AGENT.md");
        std::fs::write(&legacy, "# Old\n").unwrap();
        let pkg = temp.path().join("pkg");
        std::fs::create_dir(&pkg).unwrap();
        std::fs::write(pkg.join("AGENT.md"), "# Pkg\n").unwrap();
        std::fs::write(pkg.join("AGENTS.md"), "# Pkg\n").unwrap();
        let paths = vec![legacy.clone(), pkg.join("AGENT.md"), pkg.join("AGENTS.md")];
        let diagnostics =
            run_project_level_checks(&paths, &[], &LintConfig::default(), temp.path());
        let agm_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-008").collect();
        // pkg/AGENT.md sits next to an AGENTS.md (a compatibility copy or link).
        assert_eq!(agm_008.len(), 1, "{diagnostics:?}");
        assert_eq!(agm_008[0].file, legacy);
        assert_eq!(agm_008[0].level, crate::DiagnosticLevel::Info);
    }

    #[test]
    fn test_agm_009_nested_without_root() {
        let temp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".git")).unwrap();
        let pkg = temp.path().join("pkg");
        std::fs::create_dir(&pkg).unwrap();
        let nested = pkg.join("AGENTS.md");
        std::fs::write(&nested, "# Pkg\n").unwrap();
        let diagnostics = run_project_level_checks(
            std::slice::from_ref(&nested),
            &[],
            &LintConfig::default(),
            temp.path(),
        );
        let agm_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AGM-009").collect();
        assert_eq!(agm_009.len(), 1, "{diagnostics:?}");
        assert_eq!(agm_009[0].file, nested);

        // With a root file, or outside a repository root, nothing is reported.
        let root = temp.path().join("AGENTS.md");
        std::fs::write(&root, "# Root\n").unwrap();
        let diagnostics = run_project_level_checks(
            &[root.clone(), nested.clone()],
            &[],
            &LintConfig::default(),
            temp.path(),
        );
        assert!(diagnostics.iter().all(|d| d.rule != "AGM-009"));
        std::fs::remove_file(&root).unwrap();
        std::fs::remove_dir(temp.path().join(".git")).unwrap();
        let diagnostics = run_project_level_checks(
            std::slice::from_ref(&nested),
            &[],
            &LintConfig::default(),
            temp.path(),
        );
        assert!(diagnostics.iter().all(|d| d.rule != "AGM-009"));
    }

    /// The Codex chain builder (used by CDX-AG-010 after XP-009's retirement):
    /// one chain per leaf directory, at most one file per directory with
    /// `AGENTS.override.md` (any case) ahead of `AGENTS.md`, sibling subtrees
    /// apart, and a nested Git repository starting its own chain.
    #[test]
    fn test_codex_instruction_chains() {
        let fs = crate::fs::MockFileSystem::new();
        let root = Path::new("/repo");
        let files = [
            "/repo/AGENTS.md",
            "/repo/api/AGENTS.md",
            "/repo/api/agents.override.md",
            "/repo/web/AGENTS.md",
            "/repo/sub/AGENTS.md",
            "/repo/sub/x/AGENTS.md",
        ];
        for file in files {
            fs.add_file(file, "x");
        }
        fs.add_dir("/repo/sub/.git");
        let paths: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
        let mut chains = codex_instruction_chains(&paths, root, &fs);
        chains.sort();
        let expected: Vec<Vec<PathBuf>> = vec![
            vec![
                "/repo/AGENTS.md".into(),
                "/repo/api/agents.override.md".into(),
            ],
            vec!["/repo/AGENTS.md".into(), "/repo/web/AGENTS.md".into()],
            vec!["/repo/sub/AGENTS.md".into(), "/repo/sub/x/AGENTS.md".into()],
        ];
        assert_eq!(chains, expected);
    }

    #[test]
    fn test_as_025_duplicate_skill_names_across_roots() {
        let temp = tempfile::TempDir::new().unwrap();
        let write = |root: &str, dir: &str, body: &str| {
            let path = temp.path().join(root).join(dir);
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(
                path.join("SKILL.md"),
                format!("---\nname: review\ndescription: Use when reviewing\n---\n{body}\n"),
            )
            .unwrap();
            path.join("SKILL.md")
        };
        write(".claude/skills", "review", "Claude copy");
        let agents = write(".agents/skills", "review", "Generic copy");
        // Kiro reads neither of the others.
        write(".kiro/skills", "review", "Kiro copy");
        let skills = |temp: &tempfile::TempDir| -> Vec<PathBuf> {
            let mut paths: Vec<PathBuf> = [".claude/skills", ".agents/skills", ".kiro/skills"]
                .iter()
                .map(|root| temp.path().join(root).join("review/SKILL.md"))
                .collect();
            paths.sort();
            paths
        };

        let diagnostics =
            run_project_level_checks(&skills(&temp), &[], &LintConfig::default(), temp.path());
        let as_025: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-025").collect();
        assert_eq!(as_025.len(), 1, "{diagnostics:?}");
        // Reported on the later path in walk order, naming the other copy.
        assert!(as_025[0].file.ends_with(".claude/skills/review/SKILL.md"));
        assert!(as_025[0].message.contains(&agents.display().to_string()));
        assert!(
            as_025[0].message.contains("cursor"),
            "{}",
            as_025[0].message
        );

        // Scoped to tools that never read both roots: nothing to report.
        let config = LintConfig::builder()
            .tools(vec!["claude-code".to_string(), "codex".to_string()])
            .build()
            .unwrap();
        let diagnostics = run_project_level_checks(&skills(&temp), &[], &config, temp.path());
        assert!(
            diagnostics.iter().all(|d| d.rule != "AS-025"),
            "{diagnostics:?}"
        );

        // A stub that sends the agent to the other copy is one skill.
        write(
            ".claude/skills",
            "review",
            "Read `.agents/skills/review/SKILL.md` in full and follow it.",
        );
        let diagnostics =
            run_project_level_checks(&skills(&temp), &[], &LintConfig::default(), temp.path());
        assert!(
            diagnostics.iter().all(|d| d.rule != "AS-025"),
            "{diagnostics:?}"
        );

        // Mirrored, identical copies are not a collision.
        write(".claude/skills", "review", "Claude copy");
        write(".agents/skills", "review", "Claude copy");
        let diagnostics =
            run_project_level_checks(&skills(&temp), &[], &LintConfig::default(), temp.path());
        assert!(
            diagnostics.iter().all(|d| d.rule != "AS-025"),
            "{diagnostics:?}"
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
    fn test_xp006_identical_claude_agents_no_warning() {
        // Issue #660: CLAUDE.md and AGENTS.md with identical content should not
        // trigger "multiple instruction layers" warning.
        let temp = tempfile::TempDir::new().unwrap();

        let content = "# Project\n\nRun cargo test to run tests.\n";
        let claude_md = temp.path().join("CLAUDE.md");
        std::fs::write(&claude_md, content).unwrap();
        let agents_md = temp.path().join("AGENTS.md");
        std::fs::write(&agents_md, content).unwrap();

        let instruction_file_paths = vec![claude_md, agents_md];

        let diagnostics = run_project_level_checks(
            &[],
            &instruction_file_paths,
            &LintConfig::default(),
            temp.path(),
        );

        assert!(
            diagnostics.iter().all(|d| d.rule != "XP-006"),
            "Identical CLAUDE.md and AGENTS.md should not trigger XP-006, got: {diagnostics:?}"
        );
    }

    #[test]
    fn test_xp006_different_claude_agents_still_warns() {
        // Different content should still trigger the warning
        let temp = tempfile::TempDir::new().unwrap();

        let claude_md = temp.path().join("CLAUDE.md");
        std::fs::write(&claude_md, "# Claude instructions\n").unwrap();
        let agents_md = temp.path().join("AGENTS.md");
        std::fs::write(&agents_md, "# Different agent instructions\n").unwrap();

        let instruction_file_paths = vec![claude_md, agents_md];

        let diagnostics = run_project_level_checks(
            &[],
            &instruction_file_paths,
            &LintConfig::default(),
            temp.path(),
        );

        assert!(
            diagnostics.iter().any(|d| d.rule == "XP-006"),
            "Different CLAUDE.md and AGENTS.md should trigger XP-006"
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

        let config = LintConfig::load(&agnix_toml).unwrap();
        let diagnostics = run_project_level_checks(&[], &[], &config, temp.path());

        let ver001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "VER-001").collect();
        assert_eq!(ver001.len(), 1, "Expected one VER-001 diagnostic");
        assert_eq!(
            ver001[0].file,
            agnix_toml.canonicalize().unwrap(),
            "VER-001 diagnostic should reference .agnix.toml when it exists, got: {}",
            ver001[0].file.display()
        );
    }
}
