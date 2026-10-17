//! Cross-platform validation rules
//!
//! Validates:
//! - XP-001: AGENTS.md content that not every reader uses (info)
//! - XP-002: AGENTS.md heading hierarchy (info)
//! - XP-003: Hard-coded user home paths in files agents load (warning)
//! - XP-007: AGENTS.md exceeds Codex CLI byte limit (warning)
//! - XP-008: CLAUDE.md @-imports that Cursor does not inline (warning)

use crate::{
    config::{LintConfig, PerFileLintConfig, TargetTool},
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
    schemas::cross_platform::{
        CODEX_BYTE_LIMIT, FeatureKind, check_byte_limit, check_markdown_structure,
        find_claude_specific_features, find_hard_coded_paths,
    },
};
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["XP-001", "XP-002", "XP-003", "XP-007", "XP-008"];

/// Tools that expand `@path` references in AGENTS.md: Amp ("To include other
/// files as context, @-mention them in agent files"), Copilot CLI ("use `@`
/// followed by a relative path to include another file") and Claude Code
/// (memory imports). Codex, Cursor and OpenCode do not.
const AT_REFERENCE_TOOLS: &[&str] = &["amp", "github-copilot", "copilot", "claude-code"];

/// Whether `tool` is in scope for `path`: selected with `tools` (or the
/// legacy `target`), or nothing is selected and one of `markers` (a
/// directory or file the tool keeps in a project, such as `.cursor`) sits
/// next to the file or above it, up to the lint root. Shared instruction files
/// are read by many tools, so an unscoped run applies one tool's facts only to
/// projects that show signs of using it.
pub(crate) fn tool_in_scope(
    path: &Path,
    config: &LintConfig,
    tool: &str,
    markers: &[&str],
) -> bool {
    let tools = config.tools();
    if !tools.is_empty() {
        return tools
            .iter()
            .any(|t| t.eq_ignore_ascii_case(tool) || LintConfig::is_tool_alias(t, tool));
    }
    if let Some(name) = config.target().tool_name() {
        return name == tool;
    }
    let fs = config.fs();
    let root = config.root_dir();
    for dir in path.ancestors().skip(1) {
        if markers.iter().any(|marker| fs.exists(&dir.join(marker))) {
            return true;
        }
        if root.is_some_and(|root| dir == root.as_path()) || dir.as_os_str().is_empty() {
            break;
        }
    }
    false
}

/// True when every selected tool expands `@path` references, so an
/// @-reference in AGENTS.md reaches all of them. False for an unscoped run.
fn every_selected_tool_expands_at_references(config: &LintConfig) -> bool {
    let tools = config.tools();
    if !tools.is_empty() {
        return tools
            .iter()
            .all(|t| AT_REFERENCE_TOOLS.iter().any(|r| r.eq_ignore_ascii_case(t)));
    }
    config.target() == TargetTool::ClaudeCode
}

pub struct CrossPlatformValidator;

impl Validator for CrossPlatformValidator {
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

        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let is_agents_md = matches!(
            filename,
            "AGENTS.md" | "AGENTS.local.md" | "AGENTS.override.md"
        );

        // XP-001: content in AGENTS.md that not every reader uses (INFO).
        // An agnix heuristic: @-references reach only the tools that expand
        // them, and skill or hook configuration does nothing in an instruction
        // file. @-references are not reported when every selected tool
        // expands them.
        if config.is_rule_enabled("XP-001") && is_agents_md {
            let skip_at_references = every_selected_tool_expands_at_references(config);
            for feature in find_claude_specific_features(content) {
                if feature.kind == FeatureKind::AtReference && skip_at_references {
                    continue;
                }
                let suggestion = match feature.kind {
                    FeatureKind::AtReference => t!("rules.xp_001.at_reference_suggestion"),
                    FeatureKind::InertConfig => t!("rules.xp_001.suggestion"),
                };
                diagnostics.push(
                    Diagnostic::info(
                        path.to_path_buf(),
                        feature.line,
                        feature.column,
                        "XP-001",
                        t!(
                            "rules.xp_001.message",
                            feature = feature.feature.as_str(),
                            filename = filename,
                            description = feature.description.as_str()
                        ),
                    )
                    .with_suggestion(suggestion),
                );
            }
        }

        // XP-002: AGENTS.md heading hierarchy (INFO). A style heuristic; a
        // file with no headings at all is AGM-002's finding.
        if config.is_rule_enabled("XP-002") && is_agents_md {
            let structure_issues = check_markdown_structure(content);
            for issue in structure_issues {
                diagnostics.push(
                    Diagnostic::info(
                        path.to_path_buf(),
                        issue.line,
                        issue.column,
                        "XP-002",
                        t!(
                            "rules.xp_002.message",
                            filename = filename,
                            issue = issue.issue.as_str()
                        ),
                    )
                    .with_suggestion(issue.suggestion),
                );
            }
        }

        // XP-003: Hard-coded user home paths (WARNING), in files agents load.
        // In generic Markdown such paths mostly document where a tool keeps
        // its data. Tool directories (`.claude/`, `.cursor/`) are not
        // reported: vendors document them and ask for them by name.
        if config.is_rule_enabled("XP-003")
            && crate::file_types::detect_file_type(path) != crate::FileType::GenericMarkdown
        {
            for path_issue in find_hard_coded_paths(content) {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        path_issue.line,
                        path_issue.column,
                        "XP-003",
                        t!(
                            "rules.xp_003.message",
                            platform = path_issue.platform.as_str(),
                            path = path_issue.path.as_str()
                        ),
                    )
                    .with_suggestion(t!("rules.xp_003.suggestion")),
                );
            }
        }

        // XP-007: AGENTS.md exceeds Codex byte limit (WARNING)
        //
        // `AGENTS.override.md` is included: Codex reads it *first* in each
        // directory ("AGENTS.override.md, then AGENTS.md, then
        // project_doc_fallback_filenames"), so it counts against the same
        // `project_doc_max_bytes` budget. Codex states the cap in bytes, so
        // this counts bytes.
        //
        // Note this remains a per-file check while the documented cap is
        // cumulative across the root-to-cwd chain; XP-009 covers the chain.
        if config.is_rule_enabled("XP-007")
            && matches!(filename, "AGENTS.md" | "AGENTS.override.md")
        {
            if let Some(exceeded) = check_byte_limit(content, CODEX_BYTE_LIMIT) {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        1,
                        "XP-007",
                        t!(
                            "rules.xp_007.message",
                            bytes = exceeded.byte_count,
                            limit = exceeded.limit
                        ),
                    )
                    .with_suggestion(t!("rules.xp_007.suggestion")),
                );
            }
        }

        // XP-008: CLAUDE.md @-imports when Cursor reads the file (WARNING).
        // Cursor "reads CLAUDE.md files the same way it reads AGENTS.md" and
        // applies them to every conversation, but an @-reference is only a
        // pointer there: "The file's contents are not inlined into the
        // prompt". The other XP-001 findings are inert for every tool, Claude
        // Code included, so they are not repeated here.
        if config.is_rule_enabled("XP-008")
            && filename == "CLAUDE.md"
            && tool_in_scope(path, config, "cursor", &[".cursor", ".cursorrules"])
        {
            for feature in find_claude_specific_features(content)
                .into_iter()
                .filter(|feature| feature.kind == FeatureKind::AtReference)
            {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        feature.line,
                        feature.column,
                        "XP-008",
                        t!("rules.xp_008.message", reference = feature.text.as_str()),
                    )
                    .with_suggestion(t!("rules.xp_008.suggestion")),
                );
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LintConfig, TargetTool};
    use crate::diagnostics::DiagnosticLevel;
    use crate::fs::MockFileSystem;
    use std::sync::Arc;

    fn run(path: &str, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        CrossPlatformValidator.validate(Path::new(path), content, config)
    }

    fn rules<'a>(diagnostics: &'a [Diagnostic], rule: &str) -> Vec<&'a Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == rule).collect()
    }

    fn with_tools(tools: &[&str]) -> LintConfig {
        let mut config = LintConfig::default();
        config.set_tools(tools.iter().map(|t| t.to_string()).collect());
        config
    }

    // ===== XP-001 =====

    #[test]
    fn test_xp_001_inert_config_in_agents_md_is_info() {
        for feature in [
            "context: fork",
            "agent: reviewer",
            "allowed-tools: Read Write",
            "- type: PreToolUse",
            "- event: Stop",
        ] {
            let content = format!("# Project\n\n{feature}\n");
            let diagnostics = run("AGENTS.md", &content, &LintConfig::default());
            let xp_001 = rules(&diagnostics, "XP-001");
            assert_eq!(xp_001.len(), 1, "{feature}");
            assert_eq!(xp_001[0].level, DiagnosticLevel::Info);
            // The old message called these "Claude-specific" for every reader;
            // no tool, Claude Code included, reads them from AGENTS.md.
            assert!(
                xp_001[0].message.contains("instruction file"),
                "{}",
                xp_001[0].message
            );
        }
    }

    #[test]
    fn test_xp_001_variants_and_claude_files() {
        let content = "# Config\n\ncontext: fork\n";
        for name in ["AGENTS.local.md", "AGENTS.override.md"] {
            assert_eq!(
                rules(&run(name, content, &LintConfig::default()), "XP-001").len(),
                1,
                "{name}"
            );
        }
        for name in ["CLAUDE.md", "CLAUDE.local.md"] {
            assert!(
                rules(&run(name, content, &LintConfig::default()), "XP-001").is_empty(),
                "{name}"
            );
        }
    }

    #[test]
    fn test_xp_001_at_reference_depends_on_selected_tools() {
        let content = "# Project\n\nSee @docs/coding.md for coding guidelines\n";
        let unscoped = run("AGENTS.md", content, &LintConfig::default());
        let xp_001 = rules(&unscoped, "XP-001");
        assert_eq!(xp_001.len(), 1);
        assert!(xp_001[0].message.contains("Codex, Cursor and OpenCode"));

        // Amp documents @-mentions in AGENTS.md; so do Copilot CLI and
        // Claude Code. With only those tools selected, nothing is reported.
        for tools in [&["amp"][..], &["github-copilot", "claude-code"][..]] {
            assert!(
                rules(&run("AGENTS.md", content, &with_tools(tools)), "XP-001").is_empty(),
                "{tools:?}"
            );
        }
        assert_eq!(
            rules(
                &run("AGENTS.md", content, &with_tools(&["amp", "codex"])),
                "XP-001"
            )
            .len(),
            1
        );
    }

    #[test]
    fn test_xp_001_guarded_sections() {
        let guarded = "# AGENTS.md\n\n## Claude Code Specific\n- type: Stop\n  command: cleanup\n\n## General\nagent: some-agent\n";
        let xp_001 = run("AGENTS.md", guarded, &LintConfig::default());
        let xp_001 = rules(&xp_001, "XP-001");
        assert_eq!(xp_001.len(), 1);
        assert!(xp_001[0].message.contains("agent"));
    }

    // ===== XP-002 =====

    #[test]
    fn test_xp_002_skipped_level_is_info_and_no_headers_is_not_reported() {
        let skipped = run(
            "AGENTS.md",
            "# Main Title\n\n#### Skipped to h4\n",
            &LintConfig::default(),
        );
        let xp_002 = rules(&skipped, "XP-002");
        assert_eq!(xp_002.len(), 1);
        assert_eq!(xp_002[0].level, DiagnosticLevel::Info);
        // AGM-002 reports a file without headings; XP-002 no longer repeats it.
        assert!(
            rules(
                &run("AGENTS.md", "Just plain text.", &LintConfig::default()),
                "XP-002"
            )
            .is_empty()
        );
        assert!(
            rules(
                &run("CLAUDE.md", "# A\n\n### B\n", &LintConfig::default()),
                "XP-002"
            )
            .is_empty()
        );
    }

    // ===== XP-003 =====

    #[test]
    fn test_xp_003_reports_user_home_paths() {
        let content = "# Project\n\n- Path: /Users/lunelson/Code/project/src\n- Windows: C:\\Users\\carol\\app\n";
        let diagnostics = run("AGENTS.md", content, &LintConfig::default());
        let xp_003 = rules(&diagnostics, "XP-003");
        assert_eq!(xp_003.len(), 2);
        assert_eq!((xp_003[0].line, xp_003[0].column), (3, 9));
        assert!(xp_003[0].message.contains("author's machine"));
        assert_eq!(
            rules(&run("CLAUDE.md", content, &LintConfig::default()), "XP-003").len(),
            2
        );
    }

    /// XP-003 used to report every mention of a tool directory (1,175 of its
    /// 1,183 findings on the real-config corpus). Vendors document those
    /// paths (Cursor hooks use ".cursor/hooks/format.sh", Copilot reads
    /// ".github/copilot/settings.json"), so naming them is not a defect.
    #[test]
    fn test_xp_003_tool_directories_and_user_relative_paths_are_fine() {
        let content = "# Project\n\nCheck .claude/settings.json, .cursor/rules, ~/.config/app/x, ~/Library/Foo and ./src/main.rs.\n";
        assert!(rules(&run("AGENTS.md", content, &LintConfig::default()), "XP-003").is_empty());
    }

    #[test]
    fn test_xp_003_skips_generic_markdown_and_placeholders() {
        let content = "Install into /Users/alice/tools.\n";
        assert!(
            rules(
                &run("notes/design.md", content, &LintConfig::default()),
                "XP-003"
            )
            .is_empty()
        );
        let placeholder = "# P\n\nUse /home/user/project or /Users/<name>/x.\n";
        assert!(
            rules(
                &run("AGENTS.md", placeholder, &LintConfig::default()),
                "XP-003"
            )
            .is_empty()
        );
    }

    // ===== XP-007 =====

    #[test]
    fn test_xp_007_suggestion_does_not_recommend_import() {
        let content = "x".repeat(CODEX_BYTE_LIMIT + 1);
        let diagnostics = run("AGENTS.md", &content, &LintConfig::default());
        let xp_007 = rules(&diagnostics, "XP-007");
        assert_eq!(xp_007.len(), 1);
        let suggestion = xp_007[0].suggestion.as_deref().unwrap_or_default();
        // Codex does not expand @import; content behind one is never loaded.
        assert!(!suggestion.contains("@import"), "{suggestion}");
        assert!(
            suggestion.contains("nested AGENTS.md") && suggestion.contains("project_doc_max_bytes")
        );
    }

    /// The coordinator's case: with `tools = ["kiro"]`, Codex's and Windsurf's
    /// AGENTS.md limits must not fire.
    #[test]
    fn test_xp_007_not_enabled_for_kiro_only() {
        let config = with_tools(&["kiro"]);
        assert!(!config.is_rule_enabled("XP-007"));
        assert!(!config.is_rule_enabled("XP-009"));
        assert!(!config.is_rule_enabled("AGM-003"));
        assert!(with_tools(&["codex"]).is_rule_enabled("XP-007"));
    }

    // ===== XP-008 =====

    #[test]
    fn test_xp_008_reports_imports_when_cursor_selected() {
        let content = "# Project\n\nSee @docs/style.md.\n\ncontext: fork\n";
        for config in [with_tools(&["cursor", "claude-code"]), {
            let mut c = LintConfig::default();
            c.set_target(TargetTool::Cursor);
            c
        }] {
            let diagnostics = run("CLAUDE.md", content, &config);
            let xp_008 = rules(&diagnostics, "XP-008");
            assert_eq!(xp_008.len(), 1, "{diagnostics:?}");
            assert_eq!(xp_008[0].level, DiagnosticLevel::Warning);
            assert!(
                xp_008[0].message.contains("@docs/style.md"),
                "{}",
                xp_008[0].message
            );
        }
    }

    #[test]
    fn test_xp_008_scope() {
        let content = "# Project\n\nSee @docs/style.md.\n";
        // Not for Claude-only runs, AGENTS.md, or unscoped runs without Cursor.
        assert!(
            rules(
                &run("CLAUDE.md", content, &with_tools(&["claude-code"])),
                "XP-008"
            )
            .is_empty()
        );
        assert!(
            rules(
                &run("AGENTS.md", content, &with_tools(&["cursor"])),
                "XP-008"
            )
            .is_empty()
        );
        assert!(rules(&run("CLAUDE.md", content, &LintConfig::default()), "XP-008").is_empty());

        // Unscoped, but the project has a .cursor directory.
        let fs = MockFileSystem::new();
        fs.add_dir("/repo");
        fs.add_dir("/repo/.cursor");
        let mut config = LintConfig::default();
        config.set_fs(Arc::new(fs));
        config.set_root_dir(std::path::PathBuf::from("/repo"));
        assert_eq!(
            rules(&run("/repo/CLAUDE.md", content, &config), "XP-008").len(),
            1
        );
    }

    #[test]
    fn test_xp_008_respects_claude_section_guards() {
        let content =
            "# Project\n\n## Claude Code\n\n@docs/claude-only.md\n\n## General\n\nKeep code clean.";
        assert!(
            rules(
                &run("CLAUDE.md", content, &with_tools(&["cursor"])),
                "XP-008"
            )
            .is_empty()
        );
    }

    // ===== Config integration =====

    #[test]
    fn test_config_disabled_cross_platform_category() {
        let mut config = LintConfig::default();
        config.rules_mut().cross_platform = false;
        let content = "context: fork\n/Users/alice/x\n";
        let diagnostics = run("AGENTS.md", content, &config);
        assert!(diagnostics.iter().all(|d| !d.rule.starts_with("XP-")));
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["XP-001".to_string()];
        let diagnostics = run("AGENTS.md", "# P\n\ncontext: fork\n", &config);
        assert!(rules(&diagnostics, "XP-001").is_empty());
        assert!(config.is_rule_enabled("XP-002"));
        assert!(config.is_rule_enabled("XP-003"));
    }

    #[test]
    fn test_tool_in_scope_uses_tools_target_and_markers() {
        assert!(tool_in_scope(
            Path::new("AGENTS.md"),
            &with_tools(&["Windsurf"]),
            "windsurf",
            &[]
        ));
        assert!(!tool_in_scope(
            Path::new("AGENTS.md"),
            &with_tools(&["cursor"]),
            "windsurf",
            &[".windsurf"]
        ));
        assert!(tool_in_scope(
            Path::new("AGENTS.md"),
            &with_tools(&["copilot"]),
            "github-copilot",
            &[]
        ));
        let mut codex = LintConfig::default();
        codex.set_target(TargetTool::Codex);
        assert!(!tool_in_scope(
            Path::new("AGENTS.md"),
            &codex,
            "windsurf",
            &[".windsurf"]
        ));
        assert!(!tool_in_scope(
            Path::new("AGENTS.md"),
            &LintConfig::default(),
            "windsurf",
            &[".windsurf"]
        ));
    }
}
