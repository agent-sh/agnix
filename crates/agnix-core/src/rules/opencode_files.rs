//! OpenCode agents, commands, tools, plugins, instruction files and skills.
//!
//! - Markdown agents (`.opencode/agents/**.md`, `~/.config/opencode/agents/`):
//!   the OpenCode agent field checks (OC-AG-*, OC-PM-*, OC-DEP-002) and
//!   OC-AG-010 (missing description).
//! - Markdown commands (`.opencode/commands/**.md`): OC-CMD-001, OC-CMD-003.
//! - Custom tools and plugins (`.opencode/tools/`, `.opencode/plugins/`):
//!   OC-TOOL-001.
//! - Files in the legacy singular directories (`agent/`, `command/`,
//!   `mode/`, `modes/`, `tool/`, `plugin/`, `skill/`): OC-DEP-008.
//! - AGENTS.md and CLAUDE.md in OpenCode projects: OC-AGM-002, OC-AGM-003.
//! - Skills: OC-SK-002 (the same skill ID in two discovery sources).

use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    parsers::frontmatter::split_frontmatter,
    rules::opencode_common::{
        AgentSource, Node, Out, agent_needs_description, check_agent, check_command_entry,
        find_opencode_project, from_yaml, opencode_global_dir, shell_blocks_with_arguments,
    },
    rules::opencode_version::{OpenCodeLines, lines_for},
    rules::{Validator, ValidatorMetadata},
};
use rust_i18n::t;
use std::path::{Component, Path, PathBuf};

// ============================================================================
// Shared helpers
// ============================================================================

/// The OpenCode subdirectory (`agents`, `command`, ...) of a file and the
/// file's path below it, without extension (the agent or command ID).
fn opencode_subdir_and_id(path: &Path) -> Option<(String, String)> {
    let parts: Vec<&str> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect();
    let root = parts.iter().enumerate().rev().find_map(|(index, part)| {
        let system_or_global = *part == "opencode"
            && index > 0
            && matches!(
                parts[index - 1],
                ".config" | "etc" | "Application Support" | "ProgramData"
            );
        (*part == ".opencode" || system_or_global).then_some(index)
    })?;
    let subdir = parts.get(root + 1)?.to_string();
    let rest = parts.get(root + 2..)?;
    if rest.is_empty() {
        return None;
    }
    let mut id = rest.join("/");
    if let Some(dot) = id.rfind('.') {
        id.truncate(dot);
    }
    Some((subdir, id))
}

/// OC-DEP-008: a file in a legacy singular directory.
fn singular_directory(out: &mut Out<'_, '_>, subdir: &str) {
    let replacement = match subdir {
        "agent" => "agents",
        "command" => "commands",
        "tool" => "tools",
        "plugin" => "plugins",
        "skill" => "skills",
        "mode" | "modes" => {
            out.info(
                "OC-DEP-008",
                (1, 1),
                t!("rules.oc_dep_008.modes", dir = subdir),
                t!("rules.oc_dep_008.modes_suggestion"),
            );
            return;
        }
        _ => return,
    };
    out.info(
        "OC-DEP-008",
        (1, 1),
        t!(
            "rules.oc_dep_008.message",
            dir = subdir,
            replacement = replacement
        ),
        t!("rules.oc_dep_008.suggestion", replacement = replacement),
    );
}

/// Parse frontmatter into a [`Node`] (an empty object when absent).
fn frontmatter_node(content: &str) -> Option<(Node, String, usize)> {
    let parts = split_frontmatter(content);
    let body_line = content[..parts.body_start.min(content.len())]
        .matches('\n')
        .count()
        + 1;
    if !parts.has_frontmatter || !parts.has_closing {
        let empty = Node {
            pos: (1, 1),
            span: None,
            val: crate::rules::opencode_common::Val::Obj(Vec::new()),
        };
        return Some((empty, parts.body, body_line));
    }
    let value: serde_yaml::Value = if parts.frontmatter.trim().is_empty() {
        serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
    } else {
        serde_yaml::from_str(&parts.frontmatter).ok()?
    };
    let first_line = content[..parts.frontmatter_start.min(content.len())]
        .matches('\n')
        .count()
        + 1;
    Some((
        from_yaml(&value, &parts.frontmatter, first_line),
        parts.body,
        body_line,
    ))
}

/// The lines a Markdown agent or command targets.
fn markdown_lines(
    frontmatter: &Node,
    config: &PerFileLintConfig<'_>,
    v2_keys: &[&str],
) -> OpenCodeLines {
    let v2 = v2_keys.iter().any(|key| frontmatter.has(key))
        || frontmatter
            .at("model")
            .and_then(Node::str)
            .is_some_and(|model| model.contains('#'));
    lines_for(config, v2)
}

// ============================================================================
// Markdown agents
// ============================================================================

const AGENT_RULE_IDS: &[&str] = &["OC-AG-010", "OC-DEP-008"];

pub struct OpenCodeAgentValidator;

impl Validator for OpenCodeAgentValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: AGENT_RULE_IDS,
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        let mut out = Out::new(path, config);
        let Some((subdir, id)) = opencode_subdir_and_id(path) else {
            return out.diags;
        };
        singular_directory(&mut out, &subdir);
        let Some((frontmatter, _, _)) = frontmatter_node(content) else {
            return out.diags;
        };
        let lines = markdown_lines(
            &frontmatter,
            config,
            &["permissions", "system", "disabled", "request"],
        );
        // Files in a mode directory are primary agents.
        let is_mode_file = subdir == "mode" || subdir == "modes";
        if !is_mode_file && agent_needs_description(&id, &frontmatter, AgentSource::Markdown, lines)
        {
            out.warn(
                "OC-AG-010",
                (1, 1),
                t!("rules.oc_ag_010.message", agent = id.as_str()),
                t!("rules.oc_ag_010.suggestion"),
            );
        }
        check_agent(&mut out, &id, &frontmatter, AgentSource::Markdown, lines);
        out.diags
    }
}

// ============================================================================
// Markdown commands
// ============================================================================

const COMMAND_RULE_IDS: &[&str] = &["OC-CMD-003"];

pub struct OpenCodeCommandValidator;

impl Validator for OpenCodeCommandValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: COMMAND_RULE_IDS,
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        let mut out = Out::new(path, config);
        let Some((subdir, id)) = opencode_subdir_and_id(path) else {
            return out.diags;
        };
        singular_directory(&mut out, &subdir);
        let Some((frontmatter, body, body_line)) = frontmatter_node(content) else {
            return out.diags;
        };
        if let Some(template) = frontmatter.get("template") {
            out.info(
                "OC-CMD-003",
                template.pos,
                t!("rules.oc_cmd_003.message", command = id.as_str()),
                t!("rules.oc_cmd_003.suggestion"),
            );
        }
        for (offset, placeholder) in shell_blocks_with_arguments(&body) {
            let line = body_line + body[..offset].matches('\n').count();
            out.warn(
                "OC-CMD-001",
                (line, 1),
                t!(
                    "rules.oc_cmd_001.message",
                    command = id.as_str(),
                    placeholder = placeholder.as_str()
                ),
                t!("rules.oc_cmd_001.suggestion"),
            );
        }
        check_command_entry(&mut out, &id, &frontmatter, true);
        out.diags
    }
}

// ============================================================================
// Custom tools and plugins
// ============================================================================

/// Built-in tool names of either line.
const BUILTIN_TOOLS: &[&str] = &[
    "bash",
    "shell",
    "read",
    "write",
    "edit",
    "patch",
    "apply_patch",
    "grep",
    "glob",
    "list",
    "skill",
    "task",
    "subagent",
    "webfetch",
    "websearch",
    "question",
    "lsp",
    "todowrite",
    "todoread",
    "execute",
];

const MODULE_RULE_IDS: &[&str] = &["OC-TOOL-001"];

pub struct OpenCodeModuleValidator;

impl Validator for OpenCodeModuleValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: MODULE_RULE_IDS,
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        let mut out = Out::new(path, config);
        let Some((subdir, id)) = opencode_subdir_and_id(path) else {
            return out.diags;
        };
        singular_directory(&mut out, &subdir);
        // "The filename becomes the tool name" for the default export; named
        // exports become `<filename>_<export>`.
        if (subdir == "tools" || subdir == "tool")
            && !id.contains('/')
            && BUILTIN_TOOLS.contains(&id.as_str())
            && content.contains("export default")
        {
            out.info(
                "OC-TOOL-001",
                (1, 1),
                t!("rules.oc_tool_001.message", tool = id.as_str()),
                t!("rules.oc_tool_001.suggestion"),
            );
        }
        out.diags
    }
}

// ============================================================================
// AGENTS.md and CLAUDE.md
// ============================================================================

const INSTRUCTION_RULE_IDS: &[&str] = &["OC-AGM-002", "OC-AGM-003"];

/// Credential prefixes for OC-AGM-002.
const SECRET_PREFIXES: &[&str] = &[
    "sk-ant-",
    "sk-proj-",
    "xoxb-",
    "xoxp-",
    "AKIA",
    "AIza",
    "ghp_",
    "gho_",
    "github_pat_",
];

pub struct OpenCodeInstructionValidator;

impl Validator for OpenCodeInstructionValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: INSTRUCTION_RULE_IDS,
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        let mut out = Out::new(path, config);
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if file_name != "AGENTS.md" && file_name != "CLAUDE.md" {
            return out.diags;
        }
        if !out.on("OC-AGM-002") && !out.on("OC-AGM-003") {
            return out.diags;
        }
        let dir = path.parent().unwrap_or(Path::new(""));
        let global = opencode_global_dir().is_some_and(|global| dir == global);
        let project = if global {
            Some((dir.to_path_buf(), lines_for(config, false)))
        } else {
            find_opencode_project(dir, config)
        };
        let Some((_, lines)) = project else {
            return out.diags;
        };
        if file_name == "AGENTS.md" {
            // CDX-AG-002 reports the first secret line of every AGENTS.md
            // (Codex reads it too); that line is not reported a second time.
            let codex_line = config
                .is_rule_enabled("CDX-AG-002")
                .then(|| crate::rules::codex::agents_md_secret_line(content))
                .flatten();
            for (index, line) in content.lines().enumerate() {
                if codex_line == Some(index + 1) {
                    continue;
                }
                if let Some(column) = SECRET_PREFIXES
                    .iter()
                    .filter_map(|prefix| line.find(prefix))
                    .min()
                {
                    out.warn(
                        "OC-AGM-002",
                        (index + 1, line[..column].chars().count() + 1),
                        t!("rules.oc_agm_002.message"),
                        t!("rules.oc_agm_002.suggestion"),
                    );
                }
            }
        } else if !config.fs().exists(&dir.join("AGENTS.md")) && !lines.v1_only() {
            if lines.v2_only() {
                out.warn(
                    "OC-AGM-003",
                    (1, 1),
                    t!("rules.oc_agm_003.message"),
                    t!("rules.oc_agm_003.suggestion"),
                );
            } else {
                out.info(
                    "OC-AGM-003",
                    (1, 1),
                    t!("rules.oc_agm_003.both_lines"),
                    t!("rules.oc_agm_003.suggestion"),
                );
            }
        }
        out.diags
    }
}

// ============================================================================
// Skills
// ============================================================================

const SKILL_RULE_IDS: &[&str] = &["OC-SK-002"];

/// Project skill sources, lowest precedence first (`.claude/skills`, then
/// `.agents/skills`, then `.opencode/skills`).
const SKILL_SOURCES: &[&str] = &[".claude", ".agents", ".opencode"];

pub struct OpenCodeSkillValidator;

impl Validator for OpenCodeSkillValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: SKILL_RULE_IDS,
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        _content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        let mut out = Out::new(path, config);
        if let Some((subdir, _)) = opencode_subdir_and_id(path)
            && subdir == "skill"
        {
            singular_directory(&mut out, &subdir);
        }
        if !out.on("OC-SK-002") {
            return out.diags;
        }
        // <root>/<source>/skills/<id>/SKILL.md
        let Some(skill_dir) = path.parent() else {
            return out.diags;
        };
        let Some(skills_dir) = skill_dir.parent() else {
            return out.diags;
        };
        let Some(source_dir) = skills_dir.parent() else {
            return out.diags;
        };
        let (Some(id), Some(source), Some("skills")) = (
            skill_dir.file_name().and_then(|name| name.to_str()),
            source_dir.file_name().and_then(|name| name.to_str()),
            skills_dir.file_name().and_then(|name| name.to_str()),
        ) else {
            return out.diags;
        };
        let Some(rank) = SKILL_SOURCES
            .iter()
            .position(|candidate| *candidate == source)
        else {
            return out.diags;
        };
        let Some(root) = source_dir.parent() else {
            return out.diags;
        };
        if find_opencode_project(root, config).is_none() {
            return out.diags;
        }
        let fs = config.fs();
        let winner: Option<PathBuf> = SKILL_SOURCES[rank + 1..]
            .iter()
            .rev()
            .map(|higher| root.join(higher).join("skills").join(id).join("SKILL.md"))
            .find(|candidate| fs.exists(candidate));
        if let Some(winner) = winner {
            let shown = winner
                .strip_prefix(root)
                .unwrap_or(&winner)
                .display()
                .to_string();
            out.info(
                "OC-SK-002",
                (1, 1),
                t!("rules.oc_sk_002.message", id = id, winner = shown.as_str()),
                t!("rules.oc_sk_002.suggestion"),
            );
        }
        out.diags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::fs::MockFileSystem;
    use std::sync::Arc;

    fn rules(diagnostics: &[Diagnostic]) -> Vec<&str> {
        diagnostics.iter().map(|d| d.rule.as_str()).collect()
    }

    #[test]
    fn ids_follow_the_path_below_the_opencode_directory() {
        assert_eq!(
            opencode_subdir_and_id(Path::new(".opencode/agents/team/reviewer.md")),
            Some(("agents".to_string(), "team/reviewer".to_string()))
        );
        assert_eq!(
            opencode_subdir_and_id(Path::new("/home/me/.config/opencode/command/ship.md")),
            Some(("command".to_string(), "ship".to_string()))
        );
        assert_eq!(opencode_subdir_and_id(Path::new("agents/x.md")), None);
    }

    fn agent(content: &str, path: &str) -> Vec<Diagnostic> {
        OpenCodeAgentValidator.validate(Path::new(path), content, &LintConfig::default())
    }

    #[test]
    fn documented_markdown_agents_are_clean() {
        let v1 = "---\ndescription: Reviews code for quality and best practices\nmode: subagent\nmodel: anthropic/claude-sonnet-4-20250514\ntemperature: 0.1\npermission:\n  edit: deny\n  bash: deny\n---\nYou are in code review mode.\n";
        assert!(
            agent(v1, ".opencode/agents/review.md").is_empty(),
            "{:?}",
            agent(v1, ".opencode/agents/review.md")
        );
        let v2 = "---\ndescription: Reviews changes for correctness and regressions\nmode: subagent\nmodel: anthropic/claude-sonnet-4-5#high\npermissions:\n  - action: edit\n    resource: \"*\"\n    effect: deny\n  - action: shell\n    resource: \"*\"\n    effect: deny\n---\nReview the current changes.\n";
        assert!(
            agent(v2, ".opencode/agents/reviewer.md").is_empty(),
            "{:?}",
            agent(v2, ".opencode/agents/reviewer.md")
        );
    }

    #[test]
    fn markdown_agent_without_description_and_legacy_fields() {
        let content = "---\nmode: subagent\ntools:\n  write: false\nmaxSteps: 5\npermission:\n  bash:\n    \"git status\": allow\n    \"*\": ask\n---\nBody\n";
        let diagnostics = agent(content, ".opencode/agent/helper.md");
        let found = rules(&diagnostics);
        for rule in [
            "OC-AG-010",
            "OC-DEP-008",
            "OC-DEP-002",
            "OC-DEP-006",
            "OC-PM-003",
            "OC-PM-007",
        ] {
            assert!(found.contains(&rule), "{rule}: {found:?}");
        }
        let pm003 = diagnostics.iter().find(|d| d.rule == "OC-PM-003").unwrap();
        assert_eq!(pm003.line, 9);
        // A primary agent in OpenCode 2 shape needs no description.
        let v2_primary = "---\nmode: primary\npermissions: []\n---\nBody\n";
        assert!(!rules(&agent(v2_primary, ".opencode/agents/writer.md")).contains(&"OC-AG-010"));
    }

    #[test]
    fn commands_report_frontmatter_template_and_shell_arguments() {
        let content = "---\ndescription: History\ntemplate: ignored\n---\nSummarize these commits:\n!`git log --oneline -$1`\n";
        let diagnostics = OpenCodeCommandValidator.validate(
            Path::new(".opencode/command/history.md"),
            content,
            &LintConfig::default(),
        );
        let found = rules(&diagnostics);
        assert_eq!(
            found,
            ["OC-DEP-008", "OC-CMD-003", "OC-CMD-001"],
            "{diagnostics:?}"
        );
        assert_eq!(diagnostics[1].line, 3);
        assert_eq!(diagnostics[2].line, 6);
        let clean = "---\ndescription: Review recent changes\n---\nRecent git commits:\n!`git log --oneline -10`\nReview $ARGUMENTS.\n";
        assert!(
            OpenCodeCommandValidator
                .validate(
                    Path::new(".opencode/commands/review.md"),
                    clean,
                    &LintConfig::default()
                )
                .is_empty()
        );
    }

    #[test]
    fn custom_tool_named_like_a_builtin() {
        let validate = |path: &str, content: &str| {
            OpenCodeModuleValidator.validate(Path::new(path), content, &LintConfig::default())
        };
        assert_eq!(
            rules(&validate(
                ".opencode/tools/bash.ts",
                "export default tool({})"
            )),
            ["OC-TOOL-001"]
        );
        assert!(validate(".opencode/tools/bash.ts", "export const run = tool({})").is_empty());
        assert!(validate(".opencode/tools/database.ts", "export default tool({})").is_empty());
        assert_eq!(
            rules(&validate(".opencode/plugin/notify.ts", "")),
            ["OC-DEP-008"]
        );
    }

    fn config_with(fs: MockFileSystem) -> LintConfig {
        let mut config = LintConfig::default();
        config.set_fs(Arc::new(fs));
        config
    }

    #[test]
    fn instruction_files_in_opencode_projects() {
        let fs = MockFileSystem::new();
        fs.add_dir("/p");
        fs.add_dir("/p/.opencode");
        fs.add_file("/p/CLAUDE.md", "# Rules");
        let config = config_with(fs);
        let claude =
            OpenCodeInstructionValidator.validate(Path::new("/p/CLAUDE.md"), "# Rules", &config);
        assert_eq!(rules(&claude), ["OC-AGM-003"]);
        assert_eq!(claude[0].level, crate::diagnostics::DiagnosticLevel::Info);
        // With only OpenCode selected, CDX-AG-002 does not run and OC-AGM-002
        // reports the secret. (Unscoped, CDX-AG-002 owns that line; see
        // `oc_agm_002_skips_the_line_cdx_ag_002_reports`.)
        let mut opencode = config.clone();
        opencode.set_tools(vec!["opencode".to_string()]);
        let agents = OpenCodeInstructionValidator.validate(
            Path::new("/p/AGENTS.md"),
            "token: ghp_abcdefghijklmnopqrstuvwxyz0123456789",
            &opencode,
        );
        assert_eq!(rules(&agents), ["OC-AGM-002"]);
        assert_eq!(agents[0].column, 8);

        // No OpenCode evidence: nothing is reported.
        let empty = config_with(MockFileSystem::new());
        assert!(
            OpenCodeInstructionValidator
                .validate(Path::new("/q/CLAUDE.md"), "# Rules", &empty)
                .is_empty()
        );
    }

    /// An AGENTS.md in an OpenCode project is read by Codex too, and both
    /// CDX-AG-002 and OC-AGM-002 reported the same secret line. CDX-AG-002
    /// reports the first secret line; OC-AGM-002 reports the other lines.
    #[test]
    fn oc_agm_002_skips_the_line_cdx_ag_002_reports() {
        let fs = MockFileSystem::new();
        fs.add_dir("/p");
        fs.add_dir("/p/.opencode");
        let config = config_with(fs);
        let content = "# P\n\nUse ghp_abcdefghijklmnopqrstuvwxyz0123456789 here.\nMaps key AIzaSyA1234567890abcdefghijklmnopqrstu\n";
        assert_eq!(crate::rules::codex::agents_md_secret_line(content), Some(3));
        let agents =
            OpenCodeInstructionValidator.validate(Path::new("/p/AGENTS.md"), content, &config);
        assert_eq!(rules(&agents), ["OC-AGM-002"]);
        assert_eq!(agents[0].line, 4);
        // With CDX-AG-002 disabled, OC-AGM-002 reports both lines.
        let mut no_codex = config.clone();
        no_codex.rules_mut().disabled_rules = vec!["CDX-AG-002".to_string()];
        let agents =
            OpenCodeInstructionValidator.validate(Path::new("/p/AGENTS.md"), content, &no_codex);
        assert_eq!(rules(&agents), ["OC-AGM-002", "OC-AGM-002"]);
    }

    #[test]
    fn claude_md_is_a_warning_when_the_project_targets_opencode_2() {
        let fs = MockFileSystem::new();
        fs.add_dir("/p");
        fs.add_file("/p/opencode.json", "{\"permissions\": []}");
        fs.add_file("/p/CLAUDE.md", "# Rules");
        let config = config_with(fs);
        let diagnostics =
            OpenCodeInstructionValidator.validate(Path::new("/p/CLAUDE.md"), "# Rules", &config);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].level,
            crate::diagnostics::DiagnosticLevel::Warning
        );
    }

    #[test]
    fn duplicate_skill_ids_across_sources() {
        let fs = MockFileSystem::new();
        fs.add_dir("/p");
        fs.add_dir("/p/.opencode");
        fs.add_file("/p/.claude/skills/release/SKILL.md", "");
        fs.add_file("/p/.opencode/skills/release/SKILL.md", "");
        let config = config_with(fs);
        let shadowed = OpenCodeSkillValidator.validate(
            Path::new("/p/.claude/skills/release/SKILL.md"),
            "",
            &config,
        );
        assert_eq!(rules(&shadowed), ["OC-SK-002"]);
        assert!(
            shadowed[0]
                .message
                .contains(".opencode/skills/release/SKILL.md")
        );
        let winner = OpenCodeSkillValidator.validate(
            Path::new("/p/.opencode/skills/release/SKILL.md"),
            "",
            &config,
        );
        assert!(winner.is_empty());
    }
}
