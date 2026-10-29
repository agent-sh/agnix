//! File type detection based on path patterns.
//!
//! The detection logic is path-based only (no I/O) and is used by the
//! validation pipeline to dispatch files to the correct validators.

use std::path::Path;

use super::types::FileType;

// ============================================================================
// Named constants for hardcoded pattern sets
// ============================================================================

/// Directory names that indicate project documentation rather than agent
/// configuration. Markdown files under these directories are classified as
/// [`FileType::Unknown`] to avoid false positives from HTML tags, @mentions,
/// and cross-platform references.
///
/// Matching is case-insensitive.
pub const DOCUMENTATION_DIRECTORIES: &[&str] = &[
    "docs",
    "doc",
    "documentation",
    "wiki",
    "licenses",
    "examples",
    "api-docs",
    "api_docs",
];

/// Filenames (lowercase) of common project files that are not agent
/// configurations. Files matching these names are classified as
/// [`FileType::Unknown`] to avoid false positives.
pub const EXCLUDED_FILENAMES: &[&str] = &[
    "changelog.md",
    "history.md",
    "releases.md",
    "readme.md",
    "contributing.md",
    "license.md",
    "code_of_conduct.md",
    "security.md",
    "pull_request_template.md",
    "issue_template.md",
    "bug_report.md",
    "feature_request.md",
    "developer.md",
    "developers.md",
    "development.md",
    "hacking.md",
    "maintainers.md",
    "governance.md",
    "support.md",
    "authors.md",
    "credits.md",
    "thanks.md",
    "migration.md",
    "upgrading.md",
];

/// Parent directory names (case-insensitive) that cause a `.md` file to be
/// classified as [`FileType::Unknown`] rather than [`FileType::GenericMarkdown`].
pub const EXCLUDED_PARENT_DIRECTORIES: &[&str] =
    &[".github", "issue_template", "pull_request_template"];

// ============================================================================
// Detection helpers
// ============================================================================

/// Returns true for a file in a pi subagent directory: user agents in
/// `~/.pi/agent/agents/` or project agents in `.pi/agents/`.
fn is_pi_agents_dir(path: &Path) -> bool {
    path_contains_consecutive_components(path, ".pi", "agents")
        || (path_contains_consecutive_components(path, ".pi", "agent")
            && path_contains_consecutive_components(path, "agent", "agents"))
}

/// Returns true if the path contains two consecutive normal path components.
pub(crate) fn path_contains_consecutive_components(path: &Path, first: &str, second: &str) -> bool {
    let mut previous: Option<&str> = None;

    for component in path.components() {
        let current = match component {
            std::path::Component::Normal(name) => match name.to_str() {
                Some(name_str) => name_str,
                None => {
                    previous = None;
                    continue;
                }
            },
            _ => {
                previous = None;
                continue;
            }
        };

        if let Some(prev) = previous
            && prev.eq_ignore_ascii_case(first)
            && current.eq_ignore_ascii_case(second)
        {
            return true;
        }

        previous = Some(current);
    }

    false
}

fn path_ends_with_components_ignore_ascii_case(path: &Path, expected: &[&str]) -> bool {
    let components: Vec<_> = path
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect();
    components.len() >= expected.len()
        && components[components.len() - expected.len()..]
            .iter()
            .zip(expected)
            .all(|(actual, expected)| actual.eq_ignore_ascii_case(expected))
}

/// Returns true for Claude Code's documented macOS, Linux/WSL, and Windows
/// system policy files. These sources are merged with `managed-settings.d`
/// fragments before Claude Code applies the effective policy.
pub(crate) fn is_claude_system_managed_settings_path(path: &Path) -> bool {
    let Some(filename) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    if filename.eq_ignore_ascii_case("managed-settings.json")
        && [
            &[
                "Library",
                "Application Support",
                "ClaudeCode",
                "managed-settings.json",
            ][..],
            &["etc", "claude-code", "managed-settings.json"][..],
            &["Program Files", "ClaudeCode", "managed-settings.json"][..],
        ]
        .iter()
        .any(|suffix| path_ends_with_components_ignore_ascii_case(path, suffix))
    {
        return true;
    }

    if filename.starts_with('.') || !ends_with_ignore_ascii_case(filename, ".json") {
        return false;
    }

    let Some(parent) = path.parent() else {
        return false;
    };
    [
        &[
            "Library",
            "Application Support",
            "ClaudeCode",
            "managed-settings.d",
        ][..],
        &["etc", "claude-code", "managed-settings.d"][..],
        &["Program Files", "ClaudeCode", "managed-settings.d"][..],
    ]
    .iter()
    .any(|suffix| path_ends_with_components_ignore_ascii_case(parent, suffix))
}

/// Returns true for Claude Code's project-compatible managed settings path and
/// the documented macOS, Linux/WSL, and Windows system policy files.
pub(crate) fn is_claude_managed_settings_path(path: &Path) -> bool {
    path_ends_with_components_ignore_ascii_case(path, &[".claude", "managed-settings.json"])
        || is_claude_system_managed_settings_path(path)
}

/// Case-insensitive suffix check that avoids allocating temporary strings.
fn ends_with_ignore_ascii_case(value: &str, suffix: &str) -> bool {
    if value.len() < suffix.len() {
        return false;
    }

    value
        .get(value.len() - suffix.len()..)
        .is_some_and(|tail| tail.eq_ignore_ascii_case(suffix))
}

fn parent_eq_ignore_ascii_case(parent: Option<&str>, expected: &str) -> bool {
    parent.is_some_and(|p| p.eq_ignore_ascii_case(expected))
}

fn is_agents_instruction_filename(name: &str) -> bool {
    matches!(name, "AGENTS.md" | "AGENTS.local.md" | "AGENTS.override.md")
}

/// Returns true if the file is inside a documentation directory that
/// is unlikely to contain agent configuration files. This prevents
/// false positives from XML tags, broken links, and cross-platform
/// references in project documentation.
fn is_documentation_directory(path: &Path) -> bool {
    for component in path.components() {
        if let std::path::Component::Normal(name) = component {
            if let Some(name_str) = name.to_str() {
                if DOCUMENTATION_DIRECTORIES
                    .iter()
                    .any(|d| d.eq_ignore_ascii_case(name_str))
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Returns true if the path contains `.github/instructions` as consecutive
/// components anywhere in the path. This allows scoped Copilot instruction
/// files to live in subdirectories under `.github/instructions/`.
fn is_under_github_instructions(path: &Path) -> bool {
    path_contains_consecutive_components(path, ".github", "instructions")
}

/// Returns true for a Copilot custom agent file: any `.md` file under
/// `.github/agents/` or `~/.copilot/agents/` (at any depth; the CLI joins
/// subdirectories into the agent ID with `--`), and `*.agent.md` or legacy
/// `*.chatmode.md` files elsewhere, which only Copilot uses. Claude Code
/// agent directories and documentation folders keep their own handling, and
/// project files such as README.md are not agents unless named `*.agent.md`.
fn is_copilot_agent_path(path: &Path, filename: &str) -> bool {
    if !ends_with_ignore_ascii_case(filename, ".md") {
        return false;
    }
    let copilot_suffix = ends_with_ignore_ascii_case(filename, ".agent.md")
        || ends_with_ignore_ascii_case(filename, ".chatmode.md");
    if path_contains_consecutive_components(path, ".github", "agents")
        || path_contains_consecutive_components(path, ".copilot", "agents")
    {
        return copilot_suffix || !is_excluded_filename(filename);
    }
    copilot_suffix
        && !path_contains_consecutive_components(path, ".claude", "agents")
        && !is_documentation_directory(path)
}

/// Returns true for a Copilot hook file: any `*.json` directly in
/// `.github/hooks/` ("stored in your repository at `.github/hooks/*.json`") or
/// in the user-level `~/.copilot/hooks/`.
fn is_copilot_hook_file(filename: &str, parent: Option<&str>, grandparent: Option<&str>) -> bool {
    ends_with_ignore_ascii_case(filename, ".json")
        && parent_eq_ignore_ascii_case(parent, "hooks")
        && (parent_eq_ignore_ascii_case(grandparent, ".github")
            || parent_eq_ignore_ascii_case(grandparent, ".copilot"))
}

/// Returns true for Copilot settings files that carry an inline `hooks` block:
/// `.github/copilot/settings.json`, `.github/copilot/settings.local.json` and
/// the user-level `~/.copilot/settings.json`. They are not Claude settings.
fn is_copilot_settings_file(
    filename: &str,
    parent: Option<&str>,
    grandparent: Option<&str>,
) -> bool {
    matches!(filename, "settings.json" | "settings.local.json")
        && (parent_eq_ignore_ascii_case(parent, ".copilot")
            || (parent_eq_ignore_ascii_case(parent, "copilot")
                && parent_eq_ignore_ascii_case(grandparent, ".github")))
}

/// Returns true for a Copilot CLI plugin manifest in a location only Copilot
/// reads: `.github/plugin/plugin.json` or `.plugin/plugin.json`.
fn is_copilot_plugin_manifest(parent: Option<&str>, grandparent: Option<&str>) -> bool {
    parent_eq_ignore_ascii_case(parent, ".plugin")
        || (parent_eq_ignore_ascii_case(parent, "plugin")
            && parent_eq_ignore_ascii_case(grandparent, ".github"))
}

/// Returns true if the path contains `.cursor/rules` as consecutive
/// components anywhere in the path. This allows Cursor rules to live in
/// nested subdirectories under `.cursor/rules/`.
fn is_under_cursor_rules(path: &Path) -> bool {
    path_contains_consecutive_components(path, ".cursor", "rules")
}

/// Returns true if the path contains `.claude/rules` as consecutive
/// components anywhere in the path. Rules are discovered recursively, so
/// `.claude/rules/frontend/style.md` counts as well as `.claude/rules/style.md`.
fn is_under_claude_rules(path: &Path) -> bool {
    path_contains_consecutive_components(path, ".claude", "rules")
}

/// Returns true for a MEMORY.md index Claude Code loads at session start:
/// subagent memory in `.claude/agent-memory/<agent>/` or
/// `.claude/agent-memory-local/<agent>/` (sub-agents "Enable persistent
/// memory"), and auto memory in `.claude/projects/<project>/memory/` (memory
/// "Storage location").
fn is_claude_memory_index(path: &Path) -> bool {
    let comps = normal_components(path);
    let Some((filename, dirs)) = comps.split_last() else {
        return false;
    };
    if *filename != "MEMORY.md" {
        return false;
    }
    matches!(
        dirs,
        [.., ".claude", "agent-memory" | "agent-memory-local", _]
            | [.., ".claude", "projects", _, "memory"]
    )
}

/// Returns true if the path contains `.cursor/agents` as consecutive
/// components anywhere in the path.
fn is_under_cursor_agents(path: &Path) -> bool {
    path_contains_consecutive_components(path, ".cursor", "agents")
}

/// Returns true for Amp code review check locations: `.agents/checks` as
/// consecutive components anywhere in the path, and the global check
/// directories `$XDG_CONFIG_HOME/agents/checks/` and
/// `$XDG_CONFIG_HOME/amp/checks/` ("When `XDG_CONFIG_HOME` is not set, it
/// uses `~/.config`", ampcode.com/docs/review).
fn is_under_agents_checks(path: &Path) -> bool {
    path_contains_consecutive_components(path, ".agents", "checks")
        || path.parent().is_some_and(|dir| {
            dir.ancestors().any(|ancestor| {
                path_ends_with_components_ignore_ascii_case(
                    ancestor,
                    &[".config", "agents", "checks"],
                ) || path_ends_with_components_ignore_ascii_case(
                    ancestor,
                    &[".config", "amp", "checks"],
                )
            })
        })
}

/// Amp's local skill roots, as trailing directory names
/// (ampcode.com/docs/customize/skills, "Skill Sources and Precedence"):
/// `.agents/skills/` (project, and `~/.agents/skills/`), `.claude/skills/`
/// (project, and `~/.claude/skills/`), `~/.config/agents/skills/` and
/// `~/.config/amp/skills/`.
const AMP_SKILL_ROOTS: &[&[&str]] = &[
    &[".agents", "skills"],
    &[".claude", "skills"],
    &[".config", "agents", "skills"],
    &[".config", "amp", "skills"],
];

/// A file inside a skill directory below one of Amp's local skill roots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AmpSkillLocation<'p> {
    /// The nearest skills root directory, such as `<project>/.agents/skills`.
    pub root: &'p Path,
    /// Directories from the root to the file's directory: 1 for
    /// `<root>/<skill>/SKILL.md`, 2 for `<root>/something/deep/SKILL.md`.
    pub depth: usize,
}

impl AmpSkillLocation<'_> {
    /// The root is a Claude Code location (`.claude/skills/`), which Amp
    /// skips when `amp.skills.disableClaudeCodeSkills` is set.
    pub fn is_claude_root(&self) -> bool {
        path_ends_with_components_ignore_ascii_case(self.root, &[".claude", "skills"])
    }
}

/// Where `path` sits below the nearest Amp skills root, or `None` outside
/// them (a file directly in a root is not in a skill directory).
pub(crate) fn amp_skill_location(path: &Path) -> Option<AmpSkillLocation<'_>> {
    let dir = path.parent()?;
    dir.ancestors().enumerate().find_map(|(depth, ancestor)| {
        AMP_SKILL_ROOTS
            .iter()
            .any(|root| path_ends_with_components_ignore_ascii_case(ancestor, root))
            .then_some(AmpSkillLocation {
                root: ancestor,
                depth,
            })
            .filter(|location| location.depth > 0)
    })
}

/// A skill's sibling `mcp.json` in Amp's layout, `<skills root>/<skill>/mcp.json`
/// ("A skill can define MCP servers in a sibling `mcp.json` file"). It uses
/// Amp's flat shape (server names at the top level, no `mcpServers` wrapper).
fn is_amp_skill_mcp_path(path: &Path) -> bool {
    amp_skill_location(path).is_some_and(|location| location.depth == 1)
}

// ============================================================================
// Windsurf (Devin Desktop)
// ============================================================================
//
// Devin Desktop (formerly Windsurf) reads workspace rules from
// "`.devin/rules/*.md` (preferred) or `.windsurf/rules/*.md` (fallback)",
// workflows from `.devin/workflows/*.md` or the legacy `.windsurf/workflows/`,
// global workflows from `~/.codeium/windsurf/global_workflows/*.md`, global
// rules from `~/.codeium/windsurf/memories/global_rules.md`, and Cascade hooks
// from `.devin/hooks.json` (legacy `.windsurf/hooks.json`), the user files
// `~/.codeium/windsurf/hooks.json` (IDE) and `~/.codeium/hooks.json`
// (JetBrains plugin), and the system `hooks.json` files
// (docs.devin.ai/desktop/cascade/memories, workflows, hooks). Rule and
// workflow directories are matched at any depth below `rules/` or
// `workflows/`, as `.windsurf/rules` always was. `.devin/` is also the Devin
// CLI configuration directory (`.devin/config.json`, `.devin/hooks.v1.json`),
// so only these paths are matched there.

/// Returns true if the path contains `.devin/rules` or `.windsurf/rules` as
/// consecutive components anywhere in the path.
fn is_under_windsurf_rules(path: &Path) -> bool {
    path_contains_consecutive_components(path, ".devin", "rules")
        || path_contains_consecutive_components(path, ".windsurf", "rules")
}

/// Returns true if the path contains `.devin/workflows` or
/// `.windsurf/workflows` as consecutive components anywhere in the path, or
/// is a global workflow (`~/.codeium/windsurf/global_workflows/*.md`).
fn is_under_windsurf_workflows(path: &Path) -> bool {
    path_contains_consecutive_components(path, ".devin", "workflows")
        || path_contains_consecutive_components(path, ".windsurf", "workflows")
        || path.parent().is_some_and(|dir| {
            path_ends_with_components_ignore_ascii_case(
                dir,
                &[".codeium", "windsurf", "global_workflows"],
            )
        })
}

/// Devin Desktop's global rules file, `~/.codeium/windsurf/memories/global_rules.md`.
/// Devin CLI's project-level `.devin/global_rules.md` and
/// `.windsurf/global_rules.md` are not matched: Devin CLI states no size limit
/// for them, so they stay generic Markdown.
fn is_windsurf_global_rules(path: &Path) -> bool {
    path_ends_with_components_ignore_ascii_case(
        path,
        &[".codeium", "windsurf", "memories", "global_rules.md"],
    )
}

/// Cascade hook files: workspace `.devin/hooks.json` and legacy
/// `.windsurf/hooks.json`, user `~/.codeium/windsurf/hooks.json` (Devin
/// Desktop IDE) and `~/.codeium/hooks.json` (JetBrains plugin), and the
/// system files of macOS, Linux/WSL and Windows with their legacy `Windsurf`
/// equivalents.
fn is_windsurf_hooks_path(path: &Path, parent: Option<&str>) -> bool {
    parent_eq_ignore_ascii_case(parent, ".devin")
        || parent_eq_ignore_ascii_case(parent, ".windsurf")
        || [
            &[".codeium", "windsurf", "hooks.json"][..],
            &[".codeium", "hooks.json"][..],
            &["Library", "Application Support", "Devin", "hooks.json"][..],
            &["Library", "Application Support", "Windsurf", "hooks.json"][..],
            &["etc", "devin", "hooks.json"][..],
            &["etc", "windsurf", "hooks.json"][..],
            &["ProgramData", "Devin", "hooks.json"][..],
            &["ProgramData", "Windsurf", "hooks.json"][..],
        ]
        .iter()
        .any(|suffix| path_ends_with_components_ignore_ascii_case(path, suffix))
}

/// Returns true if any path component is `.clinerules`, meaning the file
/// lives somewhere under a `.clinerules/` directory. This covers direct
/// children (`.clinerules/*.md`), workflows, hooks, and skills subdirectories.
fn has_clinerules_ancestor(path: &Path) -> bool {
    path.components().any(
        |c| matches!(c, std::path::Component::Normal(name) if name.to_str() == Some(".clinerules")),
    )
}

/// Returns true for a file in a Cline hooks directory: `.clinerules/hooks/`
/// (VS Code extension) or `.cline/hooks/` (CLI).
fn is_cline_hooks_dir(path: &Path) -> bool {
    path_contains_consecutive_components(path, ".clinerules", "hooks")
        || path_contains_consecutive_components(path, ".cline", "hooks")
}

// ============================================================================
// Codex CLI and OpenCode
// ============================================================================

/// The subdirectory directly below the nearest OpenCode configuration
/// directory in `path`'s parent chain: the project `.opencode/`, the global
/// `~/.config/opencode/`, or a system directory (`/etc/opencode`,
/// `/Library/Application Support/opencode`, `%ProgramData%\opencode`), which
/// "loads agents/, commands/, skills/, and plugins/" like the global one.
fn opencode_subdir(path: &Path) -> Option<&str> {
    let dirs = normal_components(path.parent()?);
    let root = dirs.iter().enumerate().rev().find_map(|(index, dir)| {
        let system_or_global = *dir == "opencode"
            && index > 0
            && matches!(
                dirs[index - 1],
                ".config" | "etc" | "Application Support" | "ProgramData"
            );
        (*dir == ".opencode" || system_or_global).then_some(index)
    })?;
    dirs.get(root + 1).copied()
}

/// OpenCode directory-based files: Markdown agents (`agents/`, and the legacy
/// `agent/`, `modes/` and `mode/`), Markdown commands (`commands/`, legacy
/// `command/`), and TypeScript or JavaScript custom tools and plugins
/// (`tools/`, `plugins/`, legacy `tool/`, `plugin/`). Nested paths are part of
/// the agent or command ID, so any depth counts.
fn detect_opencode_dir_file(path: &Path, filename: &str) -> Option<FileType> {
    let subdir = opencode_subdir(path)?;
    let is_markdown = filename.ends_with(".md");
    let is_module = [".ts", ".js", ".mts", ".mjs", ".cts", ".cjs"]
        .iter()
        .any(|ext| filename.ends_with(ext));
    match subdir {
        "agents" | "agent" | "modes" | "mode" if is_markdown => Some(FileType::OpenCodeAgent),
        "commands" | "command" if is_markdown => Some(FileType::OpenCodeCommand),
        "tools" | "tool" if is_module => Some(FileType::OpenCodeTool),
        "plugins" | "plugin" if is_module => Some(FileType::OpenCodePlugin),
        _ => None,
    }
}

/// Codex skill interface metadata: `agents/openai.yaml` inside a skill
/// directory, which lives under a `skills/` directory (`.agents/skills/<name>/`,
/// `~/.codex/skills/<name>/`, or a plugin's `skills/<name>/`).
fn is_codex_skill_metadata(path: &Path, filename: &str, parent: Option<&str>) -> bool {
    filename == "openai.yaml"
        && parent == Some("agents")
        && path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .is_some_and(|dir| normal_components(dir).contains(&"skills"))
}

// ============================================================================
// Kiro CLI
// ============================================================================
//
// Kiro detection follows where each engine loads each file (Kiro CLI 2.x "V2"
// and the V3 harness):
//
// - Agents: V2 reads the top-level `*.json` of `<cwd>/.kiro/agents/` (or of
//   `<cwd>/.amazonq/cli-agents/` when `.kiro/agents/` is absent) and
//   `~/.kiro/agents/`. V3 reads `*.json` and `*.md` recursively under
//   `.kiro/agents/` and never reads `.amazonq`. V3 compares the extension
//   case-sensitively (as does the CLI 2.x lineage code), and both try every
//   matching file, including README.md, prompt files and
//   `*.attribution.json`, so all of them are agents.
// - Hooks: V3 reads the top-level `*.json` of `.kiro/hooks/` (no recursion);
//   `.kiro.hook` files are Kiro IDE 0.x hooks.
// - Steering: `.kiro/steering/**/*.md` (V3 scans recursively, case-sensitive).
// - Settings: `.kiro/settings/mcp.json`, `.kiro/settings/cli.json`; there is
//   no `.kiro/settings.json` (only the 2.x session tool's description still
//   names it).
// - Permissions (V3): `permissions.yaml`, else `permissions.json`, in
//   `<config>/settings/` (user) and `<config>/workspace-roots/<hash>/`
//   (workspace), plus the administrator's `managed-settings.json`.
// - Workflows (V3): the top-level `*.workflow.json`, `*.workflow.yaml` and
//   `*.workflow.yml` files of `.kiro/workflows/` and `~/.kiro/workflows/`.
// - Ignore files (V3, IDE): `.kiroignore` in a workspace root or any
//   subdirectory, and the global `~/.kiro/settings/kiroignore`.
// - Powers (V3): an installed power lives in `~/.kiro/powers/installed/<name>/`
//   and has `plugin.json` (Agent Plugins format) or `POWER.md` (legacy) at
//   its root, `mcp.json`, steering in `steering/*.md` (legacy) or
//   `dev.kiro/steering/*.md` (plugin) read raw on demand, and
//   `dev.kiro/INSTRUCTIONS.md`. Published powers live anywhere, so outside
//   `.kiro/powers` only the Kiro-specific `POWER.md` name and power-named
//   directories (`power/`, `powers/<name>/`, `dev.kiro/`) are trusted.
//
// Names that resolve through a path lookup (`mcp.json`, `cli.json`,
// `permissions.yaml`, `POWER.md` in an installed power) match
// case-insensitively, because case-insensitive file systems find them. Files
// found by listing a directory (agents, hooks, steering) match exactly.
// Near misses that no engine loads are `FileType::KiroNotLoaded` (KIRO-026).

/// Why a file in a Kiro configuration location is never loaded by Kiro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KiroNotLoadedReason {
    /// The extension differs from the one Kiro lists only by case
    /// (`.kiro/agents/reviewer.JSON`).
    ExtensionCase {
        expected: &'static str,
        dir: &'static str,
    },
    /// A JSON variant Kiro does not list (`.kiro/agents/reviewer.jsonc`,
    /// `.kiro/hooks/lint.yaml`).
    UnsupportedExtension {
        expected: &'static str,
        dir: &'static str,
    },
    /// A JSON file in a subdirectory of `.kiro/hooks/`. The validator reports
    /// it only when the content is a hook document.
    NestedHookFile,
    /// A settings file whose name is not the one Kiro opens
    /// (`permissions.yml`, ` permissions.yaml`, `mcp.jsonc`).
    Misnamed { expected: &'static str },
    /// A settings file placed directly in `.kiro/` (`.kiro/mcp.json`).
    Misplaced { expected: &'static str },
    /// `.kiro/settings.json`. The validator reports it only when it holds
    /// Kiro CLI setting keys.
    LegacySettings,
    /// A file in `.kiro/workflows/` whose name lacks an exact
    /// `.workflow.json`, `.workflow.yaml` or `.workflow.yml` suffix
    /// (`plan.json`, `plan.Workflow.json`). For a bare `.json`/`.yaml`/`.yml`
    /// the validator reports it only when the content is a recipe.
    WorkflowSuffix { case_only: bool },
    /// A recipe in a subdirectory of `.kiro/workflows/` (recipes are not
    /// discovered recursively).
    NestedWorkflow,
}

/// Recipe suffixes Kiro lists in `.kiro/workflows/`.
pub(crate) const KIRO_WORKFLOW_SUFFIXES: &[&str] =
    &[".workflow.json", ".workflow.yaml", ".workflow.yml"];

/// Which Kiro engines load a file that is detected as `FileType::KiroAgent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KiroAgentLoaders {
    pub v2: bool,
    pub v3: bool,
}

enum KiroClass {
    Type(FileType),
    NotLoaded(KiroNotLoadedReason),
}

fn normal_components(path: &Path) -> Vec<&str> {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect()
}

fn eq_any_ignore_ascii_case(value: &str, candidates: &[&str]) -> bool {
    candidates.iter().any(|c| value.eq_ignore_ascii_case(c))
}

/// `dirs` is the directory chain of a file; returns true when it ends with
/// `expected`, compared case-insensitively.
fn dirs_end_with(dirs: &[&str], expected: &[&str]) -> bool {
    dirs.len() >= expected.len()
        && dirs[dirs.len() - expected.len()..]
            .iter()
            .zip(expected)
            .all(|(actual, expected)| actual.eq_ignore_ascii_case(expected))
}

/// Index of the `.kiro` directory a path belongs to (the innermost one).
fn kiro_dir_index(dirs: &[&str]) -> Option<usize> {
    dirs.iter().rposition(|c| c.eq_ignore_ascii_case(".kiro"))
}

/// Returns true if any directory of the path is a `.kiro` directory.
pub(crate) fn is_inside_kiro_dir(path: &Path) -> bool {
    let comps = normal_components(path);
    comps
        .split_last()
        .is_some_and(|(_, dirs)| kiro_dir_index(dirs).is_some())
}

/// Returns true for the directory of an installed or in-repo power under
/// `.kiro/powers`: `powers/<name>` or `powers/installed/<name>`, relative to
/// `.kiro`.
fn is_kiro_power_root(rel_dirs: &[&str]) -> bool {
    match rel_dirs {
        [powers, _name] => powers.eq_ignore_ascii_case("powers"),
        [powers, installed, _name] => {
            powers.eq_ignore_ascii_case("powers") && installed.eq_ignore_ascii_case("installed")
        }
        _ => false,
    }
}

/// Returns true for a published power directory outside `.kiro`, recognised
/// by name: `.../power/` or `.../powers/<name>/`.
fn is_named_power_root(dirs: &[&str]) -> bool {
    dirs_end_with(dirs, &["power"])
        || (dirs.len() >= 2 && dirs[dirs.len() - 2].eq_ignore_ascii_case("powers"))
}

/// Returns true for a skill bundled in a power, `<power>/skills/<dir>/SKILL.md`,
/// where the power directory is recognised by path (installed or in-repo
/// under `.kiro/powers`, or a published `power/` / `powers/<name>/`).
pub(crate) fn is_kiro_power_skill(path: &Path) -> bool {
    let comps = normal_components(path);
    let n = comps.len();
    if n < 4 || comps[n - 1] != "SKILL.md" || !comps[n - 3].eq_ignore_ascii_case("skills") {
        return false;
    }
    let power_root = &comps[..n - 3];
    if let Some(idx) = kiro_dir_index(power_root)
        && is_kiro_power_root(&power_root[idx + 1..])
    {
        return true;
    }
    kiro_dir_index(power_root).is_none() && is_named_power_root(power_root)
}

/// Settings file names Kiro opens by path, for near-miss matching.
const KIRO_SETTINGS_FILENAMES: &[&str] = &[
    "mcp.json",
    "cli.json",
    "permissions.yaml",
    "permissions.json",
];

/// The Kiro settings file a near-miss name was meant to be: surrounding
/// whitespace (` permissions.yaml`), `.yml` for `.yaml`, or `.jsonc`.
fn kiro_settings_near_miss(filename: &str) -> Option<&'static str> {
    let trimmed = filename.trim();
    if trimmed.len() != filename.len()
        && let Some(expected) = KIRO_SETTINGS_FILENAMES
            .iter()
            .find(|name| trimmed.eq_ignore_ascii_case(name))
    {
        return Some(expected);
    }
    if trimmed.eq_ignore_ascii_case("permissions.yml") {
        return Some("permissions.yaml");
    }
    if trimmed.eq_ignore_ascii_case("mcp.jsonc") {
        return Some("mcp.json");
    }
    if trimmed.eq_ignore_ascii_case("cli.jsonc") {
        return Some("cli.json");
    }
    None
}

/// Returns true for the documents a power hands to the agent raw, given the
/// directories between the power root and the file: `steering/*.md`
/// (legacy), `dev.kiro/steering/*.md` and `dev.kiro/INSTRUCTIONS.md`
/// (Agent Plugins). Steering is listed top-level only.
fn is_kiro_power_steering(sub_dirs: &[&str], filename: &str) -> bool {
    let steering_doc = filename.ends_with(".md")
        && match sub_dirs {
            [steering] => steering.eq_ignore_ascii_case("steering"),
            [dev_kiro, steering] => {
                dev_kiro.eq_ignore_ascii_case("dev.kiro")
                    && steering.eq_ignore_ascii_case("steering")
            }
            _ => false,
        };
    let instructions = filename == "INSTRUCTIONS.md"
        && matches!(sub_dirs, [dev_kiro] if dev_kiro.eq_ignore_ascii_case("dev.kiro"));
    steering_doc || instructions
}

/// Classify a file below a `.kiro` directory. `rel_dirs` are the directories
/// between `.kiro` and the file.
fn classify_in_kiro_dir(rel_dirs: &[&str], filename: &str) -> Option<KiroClass> {
    use KiroClass::{NotLoaded, Type};
    use KiroNotLoadedReason as Reason;

    let first = rel_dirs.first().copied().unwrap_or("");
    let is = |name: &str| first.eq_ignore_ascii_case(name);

    if rel_dirs.is_empty() {
        let lower = filename.to_ascii_lowercase();
        return match lower.as_str() {
            "settings.json" => Some(NotLoaded(Reason::LegacySettings)),
            "mcp.json" => Some(NotLoaded(Reason::Misplaced {
                expected: ".kiro/settings/mcp.json",
            })),
            "cli.json" => Some(NotLoaded(Reason::Misplaced {
                expected: ".kiro/settings/cli.json",
            })),
            "permissions.yaml" | "permissions.yml" => Some(NotLoaded(Reason::Misplaced {
                expected: "~/.kiro/settings/permissions.yaml",
            })),
            "permissions.json" => Some(NotLoaded(Reason::Misplaced {
                expected: "~/.kiro/settings/permissions.json",
            })),
            "settings.local.json" | "plugin.json" => Some(Type(FileType::Unknown)),
            _ => None,
        };
    }

    if is("steering") {
        if filename.ends_with(".md") {
            return Some(Type(FileType::KiroSteering));
        }
        if ends_with_ignore_ascii_case(filename, ".md") {
            return Some(NotLoaded(Reason::ExtensionCase {
                expected: ".md",
                dir: ".kiro/steering",
            }));
        }
        return None;
    }

    if is("agents") {
        if filename.ends_with(".json") || filename.ends_with(".md") {
            return Some(Type(FileType::KiroAgent));
        }
        for expected in [".json", ".md"] {
            if ends_with_ignore_ascii_case(filename, expected) {
                return Some(NotLoaded(Reason::ExtensionCase {
                    expected,
                    dir: ".kiro/agents",
                }));
            }
        }
        if ends_with_ignore_ascii_case(filename, ".jsonc")
            || ends_with_ignore_ascii_case(filename, ".json5")
        {
            return Some(NotLoaded(Reason::UnsupportedExtension {
                expected: ".json",
                dir: ".kiro/agents",
            }));
        }
        return None;
    }

    if is("hooks") {
        if rel_dirs.len() > 1 {
            return ends_with_ignore_ascii_case(filename, ".json")
                .then_some(NotLoaded(Reason::NestedHookFile));
        }
        if filename.ends_with(".json") {
            return Some(Type(FileType::KiroHookV3));
        }
        if ends_with_ignore_ascii_case(filename, ".kiro.hook") {
            return Some(Type(FileType::KiroHook));
        }
        if ends_with_ignore_ascii_case(filename, ".json") {
            return Some(NotLoaded(Reason::ExtensionCase {
                expected: ".json",
                dir: ".kiro/hooks",
            }));
        }
        if [".jsonc", ".json5", ".yaml", ".yml"]
            .iter()
            .any(|ext| ends_with_ignore_ascii_case(filename, ext))
        {
            return Some(NotLoaded(Reason::UnsupportedExtension {
                expected: ".json",
                dir: ".kiro/hooks",
            }));
        }
        return None;
    }

    if is("settings") {
        if ends_with_ignore_ascii_case(filename, ".md") {
            return None;
        }
        if rel_dirs.len() == 1 {
            if filename.eq_ignore_ascii_case("mcp.json") {
                return Some(Type(FileType::KiroMcp));
            }
            // The global ignore file (`~/.kiro/settings/kiroignore`).
            if filename.eq_ignore_ascii_case("kiroignore") {
                return Some(Type(FileType::KiroIgnore));
            }
            if filename.eq_ignore_ascii_case("cli.json") {
                return Some(Type(FileType::KiroSettings));
            }
            if eq_any_ignore_ascii_case(filename, &["permissions.yaml", "permissions.json"]) {
                return Some(Type(FileType::KiroPermissions));
            }
            if let Some(expected) = kiro_settings_near_miss(filename) {
                return Some(NotLoaded(Reason::Misnamed { expected }));
            }
        }
        // lsp.json, sandbox.json, IDE-style settings.json and anything else
        // here are not Claude Code settings or generic MCP files.
        return Some(Type(FileType::Unknown));
    }

    if is("workspace-roots") && rel_dirs.len() == 2 {
        if eq_any_ignore_ascii_case(filename, &["permissions.yaml", "permissions.json"]) {
            return Some(Type(FileType::KiroPermissions));
        }
        if let Some(expected) = kiro_settings_near_miss(filename)
            && expected.starts_with("permissions.")
        {
            return Some(NotLoaded(Reason::Misnamed { expected }));
        }
        return None;
    }

    if is("workflows") {
        let exact = KIRO_WORKFLOW_SUFFIXES
            .iter()
            .any(|suffix| filename.ends_with(suffix));
        let any_case = KIRO_WORKFLOW_SUFFIXES
            .iter()
            .any(|suffix| ends_with_ignore_ascii_case(filename, suffix));
        if rel_dirs.len() > 1 {
            return any_case.then_some(NotLoaded(Reason::NestedWorkflow));
        }
        if exact {
            return Some(Type(FileType::KiroWorkflow));
        }
        if any_case {
            return Some(NotLoaded(Reason::WorkflowSuffix { case_only: true }));
        }
        if [".json", ".yaml", ".yml"]
            .iter()
            .any(|ext| ends_with_ignore_ascii_case(filename, ext))
        {
            return Some(NotLoaded(Reason::WorkflowSuffix { case_only: false }));
        }
        return None;
    }

    if is("powers") {
        for root_len in [2, 3] {
            if rel_dirs.len() < root_len || !is_kiro_power_root(&rel_dirs[..root_len]) {
                continue;
            }
            let sub_dirs = &rel_dirs[root_len..];
            if sub_dirs.is_empty() {
                if filename.eq_ignore_ascii_case("POWER.md") {
                    return Some(Type(FileType::KiroPower));
                }
                if filename.eq_ignore_ascii_case("plugin.json") {
                    return Some(Type(FileType::KiroPowerPlugin));
                }
                if filename.eq_ignore_ascii_case("mcp.json") {
                    return Some(Type(FileType::KiroMcp));
                }
            } else if is_kiro_power_steering(sub_dirs, filename) {
                return Some(Type(FileType::KiroPowerSteering));
            }
        }
        return None;
    }

    None
}

/// Classify Kiro files that are recognised without a `.kiro` directory:
/// published powers and the V2 `.amazonq/cli-agents` fallback.
fn classify_outside_kiro_dir(dirs: &[&str], filename: &str) -> Option<KiroClass> {
    use KiroClass::Type;

    // POWER.md is Kiro's legacy power manifest, and installing a power from a
    // directory requires it (or plugin.json) at the power root. Published
    // powers live at a repository root or in any folder, so the exact name is
    // the signal. Documentation folders are skipped as for other Markdown;
    // `examples/` is not, since a POWER.md there is an example power.
    if filename == "POWER.md"
        && !dirs.iter().any(|dir| {
            !dir.eq_ignore_ascii_case("examples")
                && DOCUMENTATION_DIRECTORIES
                    .iter()
                    .any(|doc| dir.eq_ignore_ascii_case(doc))
        })
    {
        return Some(Type(FileType::KiroPower));
    }

    // `dev.kiro/` is Kiro's vendor-extension directory inside an Agent
    // Plugins power, so `dev.kiro/steering/*.md` and `dev.kiro/INSTRUCTIONS.md`
    // are power documents anywhere. A legacy `steering/` directory counts only
    // in a power-named directory.
    for depth in [1, 2] {
        let Some(split) = dirs.len().checked_sub(depth) else {
            continue;
        };
        let (power_root, sub_dirs) = dirs.split_at(split);
        if is_kiro_power_steering(sub_dirs, filename)
            && (sub_dirs[0].eq_ignore_ascii_case("dev.kiro") || is_named_power_root(power_root))
        {
            return Some(Type(FileType::KiroPowerSteering));
        }
    }

    if is_named_power_root(dirs) {
        match filename {
            "plugin.json" => return Some(Type(FileType::KiroPowerPlugin)),
            "mcp.json" => return Some(Type(FileType::KiroMcp)),
            _ => {}
        }
    }

    // V2 falls back to `<cwd>/.amazonq/cli-agents/` (top-level `*.json`) when
    // `.kiro/agents/` is absent. `~/.aws/amazonq/cli-agents` is not read.
    if filename.ends_with(".json") && dirs_end_with(dirs, &[".amazonq", "cli-agents"]) {
        return Some(Type(FileType::KiroAgent));
    }

    None
}

fn classify_kiro(path: &Path) -> Option<KiroClass> {
    let comps = normal_components(path);
    let (filename, dirs) = comps.split_last()?;
    let kiro_idx = kiro_dir_index(dirs);

    // `.kiroignore` applies in the workspace root and in any subdirectory.
    if *filename == ".kiroignore" {
        return Some(KiroClass::Type(FileType::KiroIgnore));
    }

    if let Some(idx) = kiro_idx
        && let Some(class) = classify_in_kiro_dir(&dirs[idx + 1..], filename)
    {
        return Some(class);
    }
    if let Some(class) = classify_outside_kiro_dir(dirs, filename) {
        return Some(class);
    }

    // Kiro never reads Claude Code or Claude plugin files from its own tree,
    // so keep the generic `settings.json` / `plugin.json` / `hooks/hooks.json`
    // fallbacks away from it.
    if kiro_idx.is_some()
        && (eq_any_ignore_ascii_case(
            filename,
            &["settings.json", "settings.local.json", "plugin.json"],
        ) || filename.eq_ignore_ascii_case("hooks.json") && dirs_end_with(dirs, &["hooks"]))
    {
        return Some(KiroClass::Type(FileType::Unknown));
    }

    is_kiro_managed_settings_path(path).then_some(KiroClass::Type(FileType::KiroPermissions))
}

/// Kiro's administrator policy file (`{"rules": [...]}`, deny/ask only), read
/// by the IDE and the CLI on macOS, Linux and Windows.
fn is_kiro_managed_settings_path(path: &Path) -> bool {
    [
        &[
            "Library",
            "Application Support",
            "Kiro",
            "managed-settings.json",
        ][..],
        &["etc", "kiro", "managed-settings.json"][..],
        &["ProgramData", "Kiro", "managed-settings.json"][..],
    ]
    .iter()
    .any(|suffix| path_ends_with_components_ignore_ascii_case(path, suffix))
}

/// Detect a Kiro file type, or `None` when the path is not a Kiro location.
fn detect_kiro_file_type(path: &Path) -> Option<FileType> {
    match classify_kiro(path)? {
        KiroClass::Type(file_type) => Some(file_type),
        KiroClass::NotLoaded(_) => Some(FileType::KiroNotLoaded),
    }
}

/// The reason a `FileType::KiroNotLoaded` file is never loaded.
pub(crate) fn kiro_not_loaded_reason(path: &Path) -> Option<KiroNotLoadedReason> {
    match classify_kiro(path)? {
        KiroClass::NotLoaded(reason) => Some(reason),
        KiroClass::Type(_) => None,
    }
}

/// The engines that load a Kiro agent file, from its path alone: V2 reads
/// only top-level `.json` files (of `.kiro/agents/` or the
/// `.amazonq/cli-agents/` fallback); V3 reads `.json` and `.md` at any depth
/// under `.kiro/agents/` and never reads `.amazonq`. Combine it with
/// `rules::kiro_engine::engines_for` so a check never blames an engine that
/// does not read the file. Returns `None` for paths that are not agent files.
pub(crate) fn kiro_agent_loaders(path: &Path) -> Option<KiroAgentLoaders> {
    if detect_kiro_file_type(path) != Some(FileType::KiroAgent) {
        return None;
    }
    let comps = normal_components(path);
    let (filename, dirs) = comps.split_last()?;
    let Some(idx) = kiro_dir_index(dirs).filter(|&idx| {
        dirs.get(idx + 1)
            .is_some_and(|dir| dir.eq_ignore_ascii_case("agents"))
    }) else {
        // `.amazonq/cli-agents/*.json`: the V2 fallback only.
        return Some(KiroAgentLoaders {
            v2: true,
            v3: false,
        });
    };
    let top_level = dirs.len() == idx + 2;
    Some(KiroAgentLoaders {
        v2: top_level && filename.ends_with(".json"),
        v3: true,
    })
}

fn is_excluded_filename(name: &str) -> bool {
    EXCLUDED_FILENAMES
        .iter()
        .any(|&excl| excl.eq_ignore_ascii_case(name))
}

fn is_excluded_parent(parent: Option<&str>) -> bool {
    parent.is_some_and(|p| {
        EXCLUDED_PARENT_DIRECTORIES
            .iter()
            .any(|&excl| p.eq_ignore_ascii_case(excl))
    })
}

// ============================================================================
// Gemini CLI extensions
// ============================================================================

/// The candidate extension roots and file type of a file that Gemini CLI
/// reads from an extension directory, judged by the path below the root
/// only. Whether a root really is an extension is decided by the callers:
/// installed extensions by their location, other roots by a
/// `gemini-extension.json`.
///
/// geminicli.com/docs/extensions/reference: "Define hooks in a
/// `hooks/hooks.json` file within your extension directory", "Add agent
/// definition files (`.md`) to an `agents/` directory in your extension
/// root", "Provide custom commands by placing TOML files in a `commands/`
/// subdirectory" (FileCommandLoader.ts globs `**/*.toml` under
/// `<extension>/commands`, so every `commands` ancestor is a candidate) and
/// "create a `policies/` directory in your extension's root and place your
/// `.toml` policy files inside it".
fn gemini_extension_members(path: &Path) -> Vec<(&Path, FileType)> {
    let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
        return Vec::new();
    };
    let Some(dir) = path.parent() else {
        return Vec::new();
    };
    let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let single = |file_type| dir.parent().map(|root| vec![(root, file_type)]);
    let members = if filename == "hooks.json" && dir_name.eq_ignore_ascii_case("hooks") {
        single(FileType::GeminiHooks)
    } else if ends_with_ignore_ascii_case(filename, ".md")
        && dir_name.eq_ignore_ascii_case("agents")
    {
        single(FileType::GeminiAgent)
    } else if ends_with_ignore_ascii_case(filename, ".toml") {
        let mut members: Vec<(&Path, FileType)> = dir
            .ancestors()
            .filter(|ancestor| {
                ancestor
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case("commands"))
            })
            .filter_map(|commands| commands.parent())
            .map(|root| (root, FileType::GeminiCommand))
            .collect();
        if dir_name.eq_ignore_ascii_case("policies")
            && let Some(root) = dir.parent()
        {
            members.push((root, FileType::GeminiPolicy));
        }
        Some(members)
    } else {
        None
    };
    members.unwrap_or_default()
}

/// A `.toml` file below a `.gemini/commands/` directory (`~/.gemini/commands/`
/// or `<project>/.gemini/commands/`), at any depth: "Subdirectories are used
/// to create namespaced commands" (geminicli.com/docs/cli/custom-commands).
fn is_gemini_command_path(path: &Path) -> bool {
    let comps = normal_components(path);
    let Some((filename, dirs)) = comps.split_last() else {
        return false;
    };
    ends_with_ignore_ascii_case(filename, ".toml")
        && dirs.windows(2).any(|pair| {
            pair[0].eq_ignore_ascii_case(".gemini") && pair[1].eq_ignore_ascii_case("commands")
        })
}

/// A `.toml` file directly in a Gemini CLI policy directory: the user and
/// workspace tiers (`~/.gemini/policies/*.toml`,
/// `$WORKSPACE_ROOT/.gemini/policies/*.toml`) and the admin directories of
/// geminicli.com/docs/reference/policy-engine ("Standard Locations"):
/// `/etc/gemini-cli/policies`, `/Library/Application Support/GeminiCli/policies`
/// and `C:\ProgramData\gemini-cli\policies`.
fn is_gemini_policy_path(path: &Path) -> bool {
    let comps = normal_components(path);
    let Some((filename, dirs)) = comps.split_last() else {
        return false;
    };
    ends_with_ignore_ascii_case(filename, ".toml")
        && (dirs_end_with(dirs, &[".gemini", "policies"])
            || dirs_end_with(dirs, &["etc", "gemini-cli", "policies"])
            || dirs_end_with(
                dirs,
                &["Library", "Application Support", "GeminiCli", "policies"],
            )
            || dirs_end_with(dirs, &["ProgramData", "gemini-cli", "policies"]))
}

/// `root` is `.gemini/extensions/<name>`, where Gemini CLI installs (and
/// links) extensions: "Gemini CLI loads extensions from
/// `<home>/.gemini/extensions`".
fn is_installed_gemini_extension_root(root: &Path) -> bool {
    fn name_of(p: Option<&Path>) -> Option<&str> {
        p.and_then(Path::file_name).and_then(|n| n.to_str())
    }
    let parent = root.parent();
    root.file_name().is_some()
        && parent_eq_ignore_ascii_case(name_of(parent), "extensions")
        && parent_eq_ignore_ascii_case(name_of(parent.and_then(|p| p.parent())), ".gemini")
}

/// Refine a path-detected [`FileType`] with sibling-file lookups.
///
/// [`detect_file_type`] is path-only by design. Some layouts can only be
/// told apart by a sibling file, so the pipeline calls this for real files
/// once `[files]` overrides are applied (overridden types never get here).
///
/// Only the generic fallbacks that path detection gives such files are
/// refined (Claude `Hooks` for any `hooks/hooks.json`, Claude `Agent` for
/// any `agents/*.md`, `GenericMarkdown`, `Unknown`), so a tool-specific
/// detection is never replaced, and nothing inside `.kiro/` is touched.
///
/// Each tool adds one branch that returns `Some` for its own layouts:
/// - Gemini CLI: a `hooks/hooks.json`, `agents/*.md`, `commands/**/*.toml`
///   or `policies/*.toml` whose extension root (a repository that is an
///   extension, or a `.gemini/` folder that ships one) holds
///   `gemini-extension.json`.
pub(crate) fn refine_file_type(
    path: &Path,
    detected: FileType,
    fs: &dyn crate::fs::FileSystem,
) -> FileType {
    if !matches!(
        detected,
        FileType::Hooks | FileType::Agent | FileType::GenericMarkdown | FileType::Unknown
    ) || is_inside_kiro_dir(path)
    {
        return detected;
    }
    if let Some(file_type) = refine_gemini_extension_member(path, fs) {
        return file_type;
    }
    detected
}

fn refine_gemini_extension_member(path: &Path, fs: &dyn crate::fs::FileSystem) -> Option<FileType> {
    gemini_extension_members(path)
        .into_iter()
        .find(|(root, _)| fs.is_file(&root.join("gemini-extension.json")))
        .map(|(_, file_type)| file_type)
}

/// The extension directory of a Gemini CLI extension policy file
/// (`<extension>/policies/*.toml`): an installed extension under
/// `.gemini/extensions/<name>/`, or a directory that holds a
/// `gemini-extension.json`. `None` for user, workspace and admin policies.
pub(crate) fn gemini_policy_extension_root<'p>(
    path: &'p Path,
    fs: &dyn crate::fs::FileSystem,
) -> Option<&'p Path> {
    gemini_extension_members(path)
        .into_iter()
        .filter(|(_, file_type)| *file_type == FileType::GeminiPolicy)
        .map(|(root, _)| root)
        .find(|root| {
            is_installed_gemini_extension_root(root)
                || fs.is_file(&root.join("gemini-extension.json"))
        })
}

// ============================================================================
// Primary detection function
// ============================================================================

/// Detect file type based on path patterns.
///
/// Classification is purely path-based (no file I/O). The returned
/// [`FileType`] determines which validators the pipeline dispatches for
/// the file.
pub fn detect_file_type(path: &Path) -> FileType {
    if let Some(file_type) = super::native::detect(path) {
        return file_type;
    }
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let parent = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str());
    let grandparent = path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str());

    // Cursor subagent definitions should always be classified as CursorAgent,
    // including AGENTS.md / CLAUDE.md filenames under .cursor/agents.
    if ends_with_ignore_ascii_case(filename, ".md") && is_under_cursor_agents(path) {
        return FileType::CursorAgent;
    }

    // Kiro files take precedence over filename-based matches
    // (e.g., .kiro/steering/AGENTS.md is KiroSteering, not ClaudeMd, and
    // .kiro/agents/mcp-manager.md is a Kiro agent, not a Claude agent).
    if let Some(file_type) = detect_kiro_file_type(path) {
        return file_type;
    }

    // OpenCode agents, commands, custom tools and plugins live in fixed
    // directories under `.opencode/` or `~/.config/opencode/`; the directory
    // decides the type even for AGENTS.md or README.md file names, which
    // OpenCode loads as agents or commands like any other Markdown file.
    if let Some(file_type) = detect_opencode_dir_file(path, filename) {
        return file_type;
    }

    // Codex custom prompts (`~/.codex/prompts/*.md`), deprecated in favor of
    // skills.
    if filename.ends_with(".md") && path_contains_consecutive_components(path, ".codex", "prompts")
    {
        return FileType::CodexPrompt;
    }

    // Gemini CLI local agent markdown files live under `.gemini/agents/`.
    // They have YAML frontmatter with `kind: local`, a `mcp_servers` map,
    // and a system_prompt. Schema documented via the gemini-cli source at
    // packages/core/src/agents/agentLoader.ts.
    if ends_with_ignore_ascii_case(filename, ".md")
        && parent_eq_ignore_ascii_case(parent, "agents")
        && parent_eq_ignore_ascii_case(grandparent, ".gemini")
    {
        return FileType::GeminiAgent;
    }

    // Gemini CLI extensions installed under `.gemini/extensions/<name>/`:
    // `hooks/hooks.json` holds Gemini hooks (not Claude hooks) and
    // `agents/*.md` Gemini agents (not Claude subagents). Extension roots
    // elsewhere need the `gemini-extension.json` sibling, see
    // `refine_file_type`.
    if let Some((_, file_type)) = gemini_extension_members(path)
        .into_iter()
        .find(|(root, _)| is_installed_gemini_extension_root(root))
    {
        return file_type;
    }

    // Gemini CLI custom commands (`.gemini/commands/**/*.toml`) and policy
    // files (`.gemini/policies/*.toml`, admin policy directories).
    if is_gemini_command_path(path) {
        return FileType::GeminiCommand;
    }
    if is_gemini_policy_path(path) {
        return FileType::GeminiPolicy;
    }

    match filename {
        // Amp code review checks (.agents/checks/**/*.md), excluding AGENTS
        // variants so AGENTS.md keeps ClaudeMd validator coverage.
        name if name.ends_with(".md")
            && is_under_agents_checks(path)
            && !is_agents_instruction_filename(name) =>
        {
            FileType::AmpCheck
        }
        "SKILL.md" => FileType::Skill,
        "CLAUDE.md" | "CLAUDE.local.md" | "AGENTS.md" | "AGENTS.local.md"
        | "AGENTS.override.md" => FileType::ClaudeMd,
        // Amp settings: workspace `.amp/settings.json(c)`, user
        // `~/.config/amp/settings.json(c)` and the managed-settings.json of
        // `/Library/Application Support/ampcode/`, `/etc/ampcode/` and
        // `%ProgramData%\ampcode\` (ampcode.com/docs/cli/settings).
        "settings.json" | "settings.jsonc"
            if parent_eq_ignore_ascii_case(parent, ".amp")
                || (parent_eq_ignore_ascii_case(parent, "amp")
                    && parent_eq_ignore_ascii_case(grandparent, ".config")) =>
        {
            FileType::AmpSettings
        }
        "managed-settings.json" if parent_eq_ignore_ascii_case(parent, "ampcode") => {
            FileType::AmpSettings
        }
        // No Amp page documents `.amp/settings.local.json`; it is not Claude
        // Code's `settings.local.json` either.
        "settings.local.json" if parent_eq_ignore_ascii_case(parent, ".amp") => FileType::Unknown,
        "settings.json" | "settings.local.json"
            if parent_eq_ignore_ascii_case(parent, ".gemini") =>
        {
            FileType::GeminiSettings
        }
        // GitHub Copilot settings with an inline `hooks` block
        // (.github/copilot/settings.json, ~/.copilot/settings.json)
        name if is_copilot_settings_file(name, parent, grandparent) => FileType::CopilotHooks,
        _ if is_claude_managed_settings_path(path) => FileType::Hooks,
        "settings.json" | "settings.local.json" => FileType::Hooks,
        // Codex CLI plugin manifest (.codex-plugin/plugin.json)
        "plugin.json" if parent_eq_ignore_ascii_case(parent, ".codex-plugin") => {
            FileType::CodexPlugin
        }
        // GitHub Copilot CLI plugin manifest (.github/plugin/plugin.json,
        // .plugin/plugin.json)
        "plugin.json" if is_copilot_plugin_manifest(parent, grandparent) => FileType::CopilotPlugin,
        // Cursor plugin manifests (.cursor-plugin/plugin.json and the
        // multi-plugin .cursor-plugin/marketplace.json)
        "plugin.json" | "marketplace.json"
            if parent_eq_ignore_ascii_case(parent, ".cursor-plugin") =>
        {
            FileType::CursorPlugin
        }
        // Classify any other plugin.json as Plugin - validator checks .claude-plugin/ constraint (CC-PL-001)
        "plugin.json" => FileType::Plugin,
        // Claude Code plugin marketplace: "Save the marketplace file at
        // `.claude-plugin/marketplace.json` in your marketplace's directory."
        "marketplace.json" if parent_eq_ignore_ascii_case(parent, ".claude-plugin") => {
            FileType::ClaudeMarketplace
        }
        // Amp skill MCP servers (`<skills root>/<skill>/mcp.json`, flat shape)
        "mcp.json" if is_amp_skill_mcp_path(path) => FileType::AmpSkillMcp,
        // MCP configuration files
        "mcp.json" => FileType::Mcp,
        name if name.ends_with(".mcp.json") => FileType::Mcp,
        name if name.starts_with("mcp-") && name.ends_with(".json") => FileType::Mcp,
        // GitHub Copilot global instructions (.github/copilot-instructions.md)
        "copilot-instructions.md" if parent_eq_ignore_ascii_case(parent, ".github") => {
            FileType::Copilot
        }
        // GitHub Copilot scoped instructions (.github/instructions/**/*.instructions.md)
        name if name.ends_with(".instructions.md") && is_under_github_instructions(path) => {
            FileType::CopilotScoped
        }
        // GitHub Copilot custom agents (.github/agents/**/*.md,
        // ~/.copilot/agents/**/*.md, *.agent.md and legacy *.chatmode.md)
        name if is_copilot_agent_path(path, name) => FileType::CopilotAgent,
        // README.md and other project files in those folders are not agents
        // (and not Claude Code subagents either).
        name if ends_with_ignore_ascii_case(name, ".md")
            && (path_contains_consecutive_components(path, ".github", "agents")
                || path_contains_consecutive_components(path, ".copilot", "agents")) =>
        {
            FileType::Unknown
        }
        // GitHub Copilot reusable prompts (.github/prompts/*.prompt.md)
        name if name.ends_with(".prompt.md")
            && parent_eq_ignore_ascii_case(parent, "prompts")
            && parent_eq_ignore_ascii_case(grandparent, ".github") =>
        {
            FileType::CopilotPrompt
        }
        // GitHub Copilot hook files (.github/hooks/*.json, ~/.copilot/hooks/*.json)
        name if is_copilot_hook_file(name, parent, grandparent) => FileType::CopilotHooks,
        // GitHub Copilot setup workflow (.github/workflows/copilot-setup-steps.yml/.yaml)
        "copilot-setup-steps.yml" | "copilot-setup-steps.yaml"
            if parent_eq_ignore_ascii_case(parent, "workflows")
                && parent_eq_ignore_ascii_case(grandparent, ".github") =>
        {
            FileType::CopilotHooks
        }
        // Claude Code rules (.claude/rules/**/*.md)
        //
        // Matched at any depth, not just directly under `rules/`: the memory
        // reference says "All `.md` files are discovered recursively, so you
        // can organize rules into subdirectories like `frontend/` or
        // `backend/`". The previous parent/grandparent check saw only depth 1,
        // so nested rule files were never validated at all. Mirrors the
        // existing `.cursor/rules` handling.
        name if name.ends_with(".md") && is_under_claude_rules(path) => FileType::ClaudeRule,
        // Claude Code legacy custom commands (.claude/commands/**/*.md): "a
        // Markdown file in `.claude/commands/` is the older format and still
        // works", including subdirectories (`frontend/component.md` ->
        // `/frontend:component`).
        name if ends_with_ignore_ascii_case(name, ".md")
            && path_contains_consecutive_components(path, ".claude", "commands") =>
        {
            FileType::ClaudeCommand
        }
        // Claude Code output styles (.claude/output-styles/*.md)
        name if name.ends_with(".md")
            && parent == Some("output-styles")
            && grandparent == Some(".claude") =>
        {
            FileType::ClaudeOutputStyle
        }
        // Claude Code MEMORY.md indexes (subagent memory and auto memory)
        "MEMORY.md" if is_claude_memory_index(path) => FileType::ClaudeMemoryIndex,
        // .claudeignore: documented as having no effect in Claude Code
        ".claudeignore" => FileType::ClaudeIgnore,
        // Cursor project rules (.cursor/rules/**/*.md and .mdc)
        name if (ends_with_ignore_ascii_case(name, ".md")
            || ends_with_ignore_ascii_case(name, ".mdc"))
            && is_under_cursor_rules(path) =>
        {
            FileType::CursorRule
        }
        // Cursor hooks configuration (.cursor/hooks.json)
        name if name.eq_ignore_ascii_case("hooks.json")
            && parent.is_some_and(|p| p.eq_ignore_ascii_case(".cursor")) =>
        {
            FileType::CursorHooks
        }
        // Claude Code plugin hooks configuration (<plugin-root>/hooks/hooks.json).
        // Path-only heuristic: any hooks/hooks.json that is not the Copilot
        // (.github/hooks/), Cursor (.cursor/) or Gemini CLI extension
        // (.gemini/hooks/) layout is treated as Claude hooks.
        // Accepted trade-off (same reasoning as CodexRequirements below): an
        // unrelated project keeping a hooks/hooks.json would be linted as Claude
        // hooks; the layout is uncommon enough to accept.
        "hooks.json"
            if parent_eq_ignore_ascii_case(parent, "hooks")
                && !parent_eq_ignore_ascii_case(grandparent, ".github")
                && !parent_eq_ignore_ascii_case(grandparent, ".cursor")
                && !parent_eq_ignore_ascii_case(grandparent, ".gemini") =>
        {
            FileType::Hooks
        }
        // Cursor cloud-agent environment configuration (.cursor/environment.json)
        name if name.eq_ignore_ascii_case("environment.json")
            && parent.is_some_and(|p| p.eq_ignore_ascii_case(".cursor")) =>
        {
            FileType::CursorEnvironment
        }
        // Cursor CLI configuration (.cursor/cli.json in a project,
        // ~/.cursor/cli-config.json for the user)
        name if (name.eq_ignore_ascii_case("cli.json")
            || name.eq_ignore_ascii_case("cli-config.json"))
            && parent_eq_ignore_ascii_case(parent, ".cursor") =>
        {
            FileType::CursorCliConfig
        }
        // Cursor allowlists (.cursor/permissions.json, ~/.cursor/permissions.json)
        name if name.eq_ignore_ascii_case("permissions.json")
            && parent_eq_ignore_ascii_case(parent, ".cursor") =>
        {
            FileType::CursorPermissions
        }
        // Cursor ignore file (.cursorignore)
        ".cursorignore" => FileType::CursorIgnore,
        // Legacy Cursor rules file (.cursorrules or .cursorrules.md)
        ".cursorrules" | ".cursorrules.md" => FileType::CursorRulesLegacy,
        // Cline rules single file (.clinerules without extension)
        ".clinerules" => FileType::ClineRules,
        // Cline ignore file (.clineignore), being removed by Cline
        ".clineignore" => FileType::ClineIgnore,
        // Cline MCP settings (~/.cline/data/settings/cline_mcp_settings.json and
        // the VS Code extension's global storage copy)
        "cline_mcp_settings.json" => FileType::ClineMcpSettings,
        // Legacy Windsurf rules file (.windsurfrules)
        ".windsurfrules" => FileType::WindsurfRulesLegacy,
        // Cascade hooks (.devin/hooks.json, .windsurf/hooks.json, user and
        // system hooks.json)
        name if name.eq_ignore_ascii_case("hooks.json") && is_windsurf_hooks_path(path, parent) => {
            FileType::WindsurfHooks
        }
        // Devin Desktop global rules (~/.codeium/windsurf/memories/global_rules.md)
        _ if is_windsurf_global_rules(path) => FileType::WindsurfGlobalRules,
        // Gemini CLI ignore file (.geminiignore)
        ".geminiignore" => FileType::GeminiIgnore,
        // Notes kept next to Cline hook scripts are not hooks (a hook file is
        // named after its event, with no extension in the VS Code extension).
        name if (name.ends_with(".md") || name.ends_with(".txt")) && is_cline_hooks_dir(path) => {
            FileType::Unknown
        }
        // Cline rules folder - .md/.txt under .clinerules/, or Markdown
        // rules and workflows under the documented .cline/ layout. Skills in
        // .clinerules/skills/ are skills: their resource files are not rules.
        name if (name.ends_with(".md") || name.ends_with(".txt"))
            && ((has_clinerules_ancestor(path)
                && !path_contains_consecutive_components(path, ".clinerules", "skills"))
                || (name.ends_with(".md")
                    && (path_contains_consecutive_components(path, ".cline", "rules")
                        || path_contains_consecutive_components(path, ".cline", "workflows")))) =>
        {
            FileType::ClineRulesFolder
        }
        // Windsurf rules (.devin/rules/**/*.md, .windsurf/rules/**/*.md)
        name if name.ends_with(".md") && is_under_windsurf_rules(path) => FileType::WindsurfRule,
        // Windsurf workflows (.devin/workflows/**/*.md, .windsurf/workflows/**/*.md,
        // ~/.codeium/windsurf/global_workflows/*.md)
        name if name.ends_with(".md") && is_under_windsurf_workflows(path) => {
            FileType::WindsurfWorkflow
        }
        // OpenCode configuration (opencode.json / opencode.jsonc)
        "opencode.json" | "opencode.jsonc" => FileType::OpenCodeConfig,
        // Gemini CLI extension manifest (gemini-extension.json)
        "gemini-extension.json" => FileType::GeminiExtension,
        // Gemini CLI instruction files (GEMINI.md, GEMINI.local.md)
        "GEMINI.md" | "GEMINI.local.md" => FileType::GeminiMd,
        // Codex CLI configuration (.codex/config.toml / config.json / config.yaml / config.yml)
        // Path safety: symlink rejection and size limits are enforced upstream
        // by file_utils::safe_read_file before content reaches any validator.
        "config.toml" | "config.json" | "config.yaml" | "config.yml"
            if parent == Some(".codex") =>
        {
            FileType::CodexConfig
        }
        // Codex CLI managed requirements (admin-written, system location):
        // `/etc/codex/requirements.toml` (Unix) or
        // `%ProgramData%\OpenAI\Codex\requirements.toml` (Windows). The parent
        // dir is `codex`/`Codex` (case-insensitive); the project `.codex/` dir
        // is deliberately NOT matched - Codex does not read requirements.toml
        // from there.
        //
        // Accepted trade-off: a project that happens to keep an unrelated
        // `requirements.toml` under a literal `codex/` directory would be
        // classified here and linted as Codex requirements. Detection is
        // path-only (no content peek) by design, and this combination is rare,
        // so we accept it rather than gate on an absolute system path (which
        // would defeat linting an explicitly-passed file).
        "requirements.toml" if parent_eq_ignore_ascii_case(parent, "codex") => {
            FileType::CodexRequirements
        }
        // Codex CLI lifecycle hooks next to the project or user config layer
        // (`.codex/hooks.json`, `~/.codex/hooks.json`).
        "hooks.json" if parent == Some(".codex") => FileType::CodexHooks,
        // Codex CLI custom agents (`.codex/agents/*.toml`, `~/.codex/agents/*.toml`).
        name if name.ends_with(".toml")
            && parent == Some("agents")
            && grandparent == Some(".codex") =>
        {
            FileType::CodexAgent
        }
        // Codex CLI command rules in a config layer's `rules/` folder
        // (`.codex/rules/*.rules`, `~/.codex/rules/*.rules`, `/etc/codex/rules/`).
        name if name.ends_with(".rules")
            && parent == Some("rules")
            && (grandparent == Some(".codex")
                || parent_eq_ignore_ascii_case(grandparent, "codex")) =>
        {
            FileType::CodexRules
        }
        // Codex skill interface metadata (`<skill>/agents/openai.yaml`).
        name if is_codex_skill_metadata(path, name, parent) => FileType::CodexSkillMetadata,
        name if name.ends_with(".md") => {
            // Agent directories take precedence over filename exclusions.
            // Files like agents/README.md should be validated as agent configs.
            if is_pi_agents_dir(path) {
                // pi subagent definitions (`~/.pi/agent/agents/`, project
                // `.pi/agents/`) use pi's own fields and tool names.
                FileType::Unknown
            } else if parent == Some("agents") || grandparent == Some("agents") {
                FileType::Agent
            } else {
                // Exclude common project files that are not agent configurations.
                // These files commonly contain HTML, @mentions, and cross-platform
                // references that would produce false positives if validated.
                if is_excluded_filename(name) {
                    FileType::Unknown
                } else if is_documentation_directory(path) {
                    // Markdown files in documentation directories are not agent configs
                    FileType::Unknown
                } else if is_excluded_parent(parent) {
                    FileType::Unknown
                } else {
                    FileType::GenericMarkdown
                }
            }
        }
        // Cline hook scripts under .clinerules/hooks/ (VS Code extension) or
        // .cline/hooks/ (CLI), any extension or none
        // (e.g., .clinerules/hooks/TaskStart, .cline/hooks/PreToolUse.sh)
        _ if is_cline_hooks_dir(path) => FileType::ClineRulesFolder,
        _ => FileType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Named constant completeness ----

    /// DOCUMENTATION_DIRECTORIES has the expected number of entries.
    #[test]
    fn documentation_directories_count() {
        assert_eq!(
            DOCUMENTATION_DIRECTORIES.len(),
            8,
            "Expected 8 documentation directory entries"
        );
    }

    /// EXCLUDED_FILENAMES has the expected number of entries.
    #[test]
    fn excluded_filenames_count() {
        assert_eq!(
            EXCLUDED_FILENAMES.len(),
            24,
            "Expected 24 excluded filename entries"
        );
    }

    /// EXCLUDED_PARENT_DIRECTORIES has the expected number of entries.
    #[test]
    fn excluded_parent_directories_count() {
        assert_eq!(
            EXCLUDED_PARENT_DIRECTORIES.len(),
            3,
            "Expected 3 excluded parent directory entries"
        );
    }

    /// All EXCLUDED_FILENAMES must be lowercase.
    #[test]
    fn excluded_filenames_are_lowercase() {
        for name in EXCLUDED_FILENAMES {
            assert_eq!(
                *name,
                name.to_ascii_lowercase(),
                "EXCLUDED_FILENAMES entry '{}' must be lowercase",
                name
            );
        }
    }

    /// No duplicates in DOCUMENTATION_DIRECTORIES.
    #[test]
    fn no_duplicate_documentation_directories() {
        let mut seen = std::collections::HashSet::new();
        for dir in DOCUMENTATION_DIRECTORIES {
            assert!(
                seen.insert(dir.to_ascii_lowercase()),
                "Duplicate entry in DOCUMENTATION_DIRECTORIES: {}",
                dir
            );
        }
    }

    /// No duplicates in EXCLUDED_FILENAMES.
    #[test]
    fn no_duplicate_excluded_filenames() {
        let mut seen = std::collections::HashSet::new();
        for name in EXCLUDED_FILENAMES {
            assert!(
                seen.insert(*name),
                "Duplicate entry in EXCLUDED_FILENAMES: {}",
                name
            );
        }
    }

    /// No duplicates in EXCLUDED_PARENT_DIRECTORIES.
    #[test]
    fn no_duplicate_excluded_parent_directories() {
        let mut seen = std::collections::HashSet::new();
        for dir in EXCLUDED_PARENT_DIRECTORIES {
            assert!(
                seen.insert(dir.to_ascii_lowercase()),
                "Duplicate entry in EXCLUDED_PARENT_DIRECTORIES: {}",
                dir
            );
        }
    }

    // ---- Detection function tests ----

    #[test]
    fn detect_skill_md() {
        assert_eq!(
            detect_file_type(Path::new("project/SKILL.md")),
            FileType::Skill
        );
    }

    #[test]
    fn detect_claude_md_variants() {
        for name in &[
            "CLAUDE.md",
            "CLAUDE.local.md",
            "AGENTS.md",
            "AGENTS.local.md",
            "AGENTS.override.md",
        ] {
            assert_eq!(
                detect_file_type(Path::new(name)),
                FileType::ClaudeMd,
                "Expected ClaudeMd for {}",
                name
            );
        }
    }

    #[test]
    fn detect_mcp_variants() {
        assert_eq!(detect_file_type(Path::new("mcp.json")), FileType::Mcp);
        assert_eq!(
            detect_file_type(Path::new("server.mcp.json")),
            FileType::Mcp
        );
        assert_eq!(
            detect_file_type(Path::new("mcp-server.json")),
            FileType::Mcp
        );
    }

    #[test]
    fn detect_copilot_global() {
        assert_eq!(
            detect_file_type(Path::new(".github/copilot-instructions.md")),
            FileType::Copilot
        );
        assert_eq!(
            detect_file_type(Path::new(".GITHUB/copilot-instructions.md")),
            FileType::Copilot
        );
    }

    #[test]
    fn detect_copilot_scoped() {
        assert_eq!(
            detect_file_type(Path::new(".github/instructions/rust.instructions.md")),
            FileType::CopilotScoped
        );
    }

    #[test]
    fn detect_copilot_agent() {
        assert_eq!(
            detect_file_type(Path::new(".github/agents/reviewer.agent.md")),
            FileType::CopilotAgent
        );
        assert_eq!(
            detect_file_type(Path::new(".GITHUB/AGENTS/reviewer.agent.md")),
            FileType::CopilotAgent
        );
    }

    #[test]
    fn detect_copilot_prompt() {
        assert_eq!(
            detect_file_type(Path::new(".github/prompts/refactor.prompt.md")),
            FileType::CopilotPrompt
        );
        assert_eq!(
            detect_file_type(Path::new(".GITHUB/PROMPTS/refactor.prompt.md")),
            FileType::CopilotPrompt
        );
    }

    #[test]
    fn detect_copilot_hooks_json() {
        assert_eq!(
            detect_file_type(Path::new(".github/hooks/hooks.json")),
            FileType::CopilotHooks
        );
        assert_eq!(
            detect_file_type(Path::new(".GITHUB/HOOKS/hooks.json")),
            FileType::CopilotHooks
        );
    }

    #[test]
    fn detect_copilot_setup_steps_workflow() {
        assert_eq!(
            detect_file_type(Path::new(".github/workflows/copilot-setup-steps.yml")),
            FileType::CopilotHooks
        );
        assert_eq!(
            detect_file_type(Path::new(".github/workflows/copilot-setup-steps.yaml")),
            FileType::CopilotHooks
        );
        assert_eq!(
            detect_file_type(Path::new(".GITHUB/WORKFLOWS/copilot-setup-steps.yml")),
            FileType::CopilotHooks
        );
    }

    #[test]
    fn detect_excluded_filenames() {
        for name in EXCLUDED_FILENAMES {
            let lowercase_path = format!("project/{}", name);
            let path = Path::new(&lowercase_path);
            assert_eq!(
                detect_file_type(path),
                FileType::Unknown,
                "Expected Unknown for excluded filename: {}",
                name
            );
        }
    }

    #[test]
    fn detect_documentation_directories() {
        for dir in DOCUMENTATION_DIRECTORIES {
            let path = Path::new(dir).join("guide.md");
            assert_eq!(
                detect_file_type(&path),
                FileType::Unknown,
                "Expected Unknown for file in documentation directory: {}",
                dir
            );
        }
    }

    #[test]
    fn detect_excluded_parent_directories() {
        for dir in EXCLUDED_PARENT_DIRECTORIES {
            let path = Path::new(dir).join("template.md");
            assert_eq!(
                detect_file_type(&path),
                FileType::Unknown,
                "Expected Unknown for file in excluded parent: {}",
                dir
            );
        }
    }

    #[test]
    fn detect_pi_agents_not_claude_agents() {
        for path in [
            "/home/u/.pi/agent/agents/scout.md",
            "repo/.pi/agents/reviewer.md",
        ] {
            assert_eq!(
                detect_file_type(Path::new(path)),
                FileType::Unknown,
                "{path}"
            );
        }
        // A plain plugin `agents/` directory is still a Claude agent.
        assert_eq!(
            detect_file_type(Path::new("my-plugin/agents/reviewer.md")),
            FileType::Agent
        );
    }

    #[test]
    fn detect_agents_directory_takes_precedence() {
        // Even README.md in agents/ should be Agent, not excluded
        assert_eq!(
            detect_file_type(Path::new("agents/README.md")),
            FileType::Agent
        );
        assert_eq!(
            detect_file_type(Path::new("agents/sub/file.md")),
            FileType::Agent
        );
    }

    #[test]
    fn detect_generic_markdown() {
        assert_eq!(
            detect_file_type(Path::new("project/custom.md")),
            FileType::GenericMarkdown
        );
    }

    #[test]
    fn detect_hooks() {
        assert_eq!(
            detect_file_type(Path::new("settings.json")),
            FileType::Hooks
        );
        assert_eq!(
            detect_file_type(Path::new("settings.local.json")),
            FileType::Hooks
        );
    }

    #[test]
    fn detect_claude_managed_settings() {
        for path in [
            ".claude/managed-settings.json",
            "/Library/Application Support/ClaudeCode/managed-settings.json",
            "/etc/claude-code/managed-settings.json",
            "C:/Program Files/ClaudeCode/managed-settings.json",
            "/Library/Application Support/ClaudeCode/managed-settings.d/10-security.json",
            "/etc/claude-code/managed-settings.d/sandbox.json",
            "/etc/claude-code/managed-settings.d/managed-settings.json",
            "C:/Program Files/ClaudeCode/managed-settings.d/90-credentials.json",
        ] {
            assert_eq!(
                detect_file_type(Path::new(path)),
                FileType::Hooks,
                "expected Claude managed settings dispatch for {path}"
            );
        }
        assert_eq!(
            detect_file_type(Path::new("other/managed-settings.json")),
            FileType::Unknown
        );
        for path in [
            "/etc/claude-code/managed-settings.d/.hidden.json",
            "/etc/claude-code/managed-settings.d/readme.txt",
            "other/managed-settings.d/sandbox.json",
        ] {
            assert_eq!(
                detect_file_type(Path::new(path)),
                FileType::Unknown,
                "expected non-active managed settings path to stay unknown: {path}"
            );
        }
    }

    #[test]
    fn classify_claude_system_managed_settings_merge_sources() {
        for path in [
            "/Library/Application Support/ClaudeCode/managed-settings.json",
            "/etc/claude-code/managed-settings.json",
            "C:/Program Files/ClaudeCode/managed-settings.json",
            "/Library/Application Support/ClaudeCode/managed-settings.d/10-security.json",
            "/etc/claude-code/managed-settings.d/sandbox.json",
            "C:/Program Files/ClaudeCode/managed-settings.d/90-credentials.json",
        ] {
            assert!(
                is_claude_system_managed_settings_path(Path::new(path)),
                "expected merged system settings source: {path}"
            );
        }

        for path in [
            ".claude/managed-settings.json",
            "/etc/claude-code/managed-settings.d/.hidden.json",
            "other/managed-settings.d/sandbox.json",
        ] {
            assert!(
                !is_claude_system_managed_settings_path(Path::new(path)),
                "unexpected merged system settings source: {path}"
            );
        }
    }

    #[test]
    fn detect_plugin() {
        assert_eq!(detect_file_type(Path::new("plugin.json")), FileType::Plugin);
        assert_eq!(
            detect_file_type(Path::new(".claude-plugin/plugin.json")),
            FileType::Plugin
        );
    }

    #[test]
    fn detect_codex_plugin() {
        assert_eq!(
            detect_file_type(Path::new(".codex-plugin/plugin.json")),
            FileType::CodexPlugin
        );
        assert_eq!(
            detect_file_type(Path::new("project/.codex-plugin/plugin.json")),
            FileType::CodexPlugin
        );
        // Case-insensitive parent
        assert_eq!(
            detect_file_type(Path::new(".CODEX-PLUGIN/plugin.json")),
            FileType::CodexPlugin
        );
    }

    #[test]
    fn detect_claude_rule() {
        assert_eq!(
            detect_file_type(Path::new(".claude/rules/custom.md")),
            FileType::ClaudeRule
        );
    }

    #[test]
    fn detect_claude_output_style() {
        assert_eq!(
            detect_file_type(Path::new(".claude/output-styles/concise.md")),
            FileType::ClaudeOutputStyle
        );
    }

    #[test]
    fn detect_claude_output_style_not_rules_dir() {
        // .claude/rules/*.md must remain ClaudeRule, NOT ClaudeOutputStyle.
        assert_eq!(
            detect_file_type(Path::new(".claude/rules/custom.md")),
            FileType::ClaudeRule
        );
    }

    #[test]
    fn detect_amp_check() {
        assert_eq!(
            detect_file_type(Path::new(".agents/checks/security.md")),
            FileType::AmpCheck
        );
        assert_eq!(
            detect_file_type(Path::new("apps/web/.agents/checks/api/auth.md")),
            FileType::AmpCheck
        );
        assert_eq!(
            detect_file_type(Path::new(".agents/checks/SKILL.md")),
            FileType::AmpCheck
        );
    }

    #[test]
    fn detect_agents_filename_under_checks_is_claude_md() {
        assert_eq!(
            detect_file_type(Path::new(".agents/checks/AGENTS.md")),
            FileType::ClaudeMd
        );
    }

    #[test]
    fn detect_amp_check_not_under_checks() {
        assert_ne!(
            detect_file_type(Path::new(".agents/skills/security.md")),
            FileType::AmpCheck
        );
        assert_ne!(
            detect_file_type(Path::new("agents/checks/security.md")),
            FileType::AmpCheck
        );
    }

    #[test]
    fn detect_cursor_rule() {
        assert_eq!(
            detect_file_type(Path::new(".cursor/rules/custom.mdc")),
            FileType::CursorRule
        );
        assert_eq!(
            detect_file_type(Path::new(".cursor/rules/custom.md")),
            FileType::CursorRule
        );
        assert_eq!(
            detect_file_type(Path::new(".cursor/rules/frontend/components.mdc")),
            FileType::CursorRule
        );
        assert_eq!(
            detect_file_type(Path::new(".cursor/rules/frontend/components.md")),
            FileType::CursorRule
        );
        assert_eq!(
            detect_file_type(Path::new(".cursor/rules/custom.MD")),
            FileType::CursorRule
        );
        assert_eq!(
            detect_file_type(Path::new(".cursor/rules/custom.MDC")),
            FileType::CursorRule
        );
    }

    #[test]
    fn detect_cursor_rule_does_not_match_other_cursor_markdown() {
        assert_ne!(
            detect_file_type(Path::new(".cursor/README.md")),
            FileType::CursorRule
        );
    }

    #[test]
    fn detect_cursor_hooks() {
        assert_eq!(
            detect_file_type(Path::new(".cursor/hooks.json")),
            FileType::CursorHooks
        );
        assert_eq!(
            detect_file_type(Path::new("project/.cursor/hooks.json")),
            FileType::CursorHooks
        );
        assert_eq!(
            detect_file_type(Path::new("project/.CURSOR/HOOKS.JSON")),
            FileType::CursorHooks
        );
        assert_ne!(
            detect_file_type(Path::new("project/hooks.json")),
            FileType::CursorHooks
        );
        assert_ne!(
            detect_file_type(Path::new("project/.cursor/subdir/hooks.json")),
            FileType::CursorHooks
        );
    }

    #[test]
    fn detect_cursor_environment() {
        assert_eq!(
            detect_file_type(Path::new(".cursor/environment.json")),
            FileType::CursorEnvironment
        );
        assert_eq!(
            detect_file_type(Path::new("project/.cursor/environment.json")),
            FileType::CursorEnvironment
        );
        assert_eq!(
            detect_file_type(Path::new("project/.CURSOR/ENVIRONMENT.JSON")),
            FileType::CursorEnvironment
        );
        assert_ne!(
            detect_file_type(Path::new("project/environment.json")),
            FileType::CursorEnvironment
        );
    }

    #[test]
    fn detect_cursor_agent() {
        assert_eq!(
            detect_file_type(Path::new(".cursor/agents/reviewer.md")),
            FileType::CursorAgent
        );
        assert_eq!(
            detect_file_type(Path::new("project/.cursor/agents/reviewer.md")),
            FileType::CursorAgent
        );
        assert_eq!(
            detect_file_type(Path::new("project/.cursor/agents/frontend/react.md")),
            FileType::CursorAgent
        );
        assert_eq!(
            detect_file_type(Path::new("project/.cursor/agents/AGENTS.md")),
            FileType::CursorAgent
        );
        assert_eq!(
            detect_file_type(Path::new("project/.cursor/agents/CLAUDE.md")),
            FileType::CursorAgent
        );
        assert_eq!(
            detect_file_type(Path::new("project/.cursor/agents/reviewer.MD")),
            FileType::CursorAgent
        );
    }

    #[test]
    fn detect_cursor_agent_does_not_match_non_cursor_agents() {
        assert_ne!(
            detect_file_type(Path::new("agents/reviewer.md")),
            FileType::CursorAgent
        );
        assert_ne!(
            detect_file_type(Path::new(".claude/agents/reviewer.md")),
            FileType::CursorAgent
        );
    }

    #[test]
    fn detect_cursor_rules_legacy() {
        assert_eq!(
            detect_file_type(Path::new(".cursorrules")),
            FileType::CursorRulesLegacy
        );
        assert_eq!(
            detect_file_type(Path::new(".cursorrules.md")),
            FileType::CursorRulesLegacy
        );
    }

    #[test]
    fn detect_cline_rules() {
        assert_eq!(
            detect_file_type(Path::new(".clinerules")),
            FileType::ClineRules
        );
    }

    #[test]
    fn detect_cline_rules_folder() {
        assert_eq!(
            detect_file_type(Path::new(".clinerules/custom.md")),
            FileType::ClineRulesFolder
        );
    }

    #[test]
    fn detect_cline_rules_folder_txt() {
        assert_eq!(
            detect_file_type(Path::new(".clinerules/custom.txt")),
            FileType::ClineRulesFolder
        );
        assert_eq!(
            detect_file_type(Path::new(".clinerules/01-coding.txt")),
            FileType::ClineRulesFolder
        );
    }

    #[test]
    fn detect_cline_rules_alternate_folder() {
        assert_eq!(
            detect_file_type(Path::new(".cline/rules/coding.md")),
            FileType::ClineRulesFolder
        );
        assert_eq!(
            detect_file_type(Path::new("project/.cline/rules/nested/testing.md")),
            FileType::ClineRulesFolder
        );
        assert_eq!(
            detect_file_type(Path::new(".cline/rules/coding.txt")),
            FileType::Unknown
        );
        assert_eq!(
            detect_file_type(Path::new(".cline/skills/coding.md")),
            FileType::GenericMarkdown
        );
    }

    #[test]
    fn detect_cline_rules_folder_non_md_txt_rejected() {
        assert_eq!(
            detect_file_type(Path::new(".clinerules/config.json")),
            FileType::Unknown,
            "Non-.md/.txt files in .clinerules/ should be Unknown"
        );
        assert_eq!(
            detect_file_type(Path::new(".clinerules/config.yaml")),
            FileType::Unknown,
            "Non-.md/.txt files in .clinerules/ should be Unknown"
        );
        assert_eq!(
            detect_file_type(Path::new(".clinerules/config.toml")),
            FileType::Unknown,
            "Non-.md/.txt files in .clinerules/ should be Unknown"
        );
    }

    #[test]
    fn detect_opencode_config() {
        assert_eq!(
            detect_file_type(Path::new("opencode.json")),
            FileType::OpenCodeConfig
        );
        assert_eq!(
            detect_file_type(Path::new("opencode.jsonc")),
            FileType::OpenCodeConfig
        );
    }

    #[test]
    fn detect_gemini_md() {
        assert_eq!(detect_file_type(Path::new("GEMINI.md")), FileType::GeminiMd);
        assert_eq!(
            detect_file_type(Path::new("GEMINI.local.md")),
            FileType::GeminiMd
        );
    }

    #[test]
    fn detect_codex_config() {
        assert_eq!(
            detect_file_type(Path::new(".codex/config.toml")),
            FileType::CodexConfig
        );
        assert_eq!(
            detect_file_type(Path::new(".codex/config.json")),
            FileType::CodexConfig
        );
        assert_eq!(
            detect_file_type(Path::new(".codex/config.yaml")),
            FileType::CodexConfig
        );
        assert_eq!(
            detect_file_type(Path::new(".codex/config.yml")),
            FileType::CodexConfig
        );
    }

    #[test]
    fn detect_codex_requirements() {
        // System (Unix) location: /etc/codex/requirements.toml
        assert_eq!(
            detect_file_type(Path::new("/etc/codex/requirements.toml")),
            FileType::CodexRequirements
        );
        // Windows ProgramData location uses a `Codex` parent (case-insensitive)
        assert_eq!(
            detect_file_type(Path::new("OpenAI/Codex/requirements.toml")),
            FileType::CodexRequirements
        );
        // Project `.codex/` must NOT match - Codex never reads requirements.toml there
        assert_ne!(
            detect_file_type(Path::new(".codex/requirements.toml")),
            FileType::CodexRequirements
        );
        // A bare requirements.toml with no codex parent is not ours
        assert_ne!(
            detect_file_type(Path::new("requirements.toml")),
            FileType::CodexRequirements
        );
    }

    #[test]
    fn detect_codex_hooks_agents_rules_prompts_and_skill_metadata() {
        for (path, expected) in [
            (".codex/hooks.json", FileType::CodexHooks),
            ("/home/me/.codex/hooks.json", FileType::CodexHooks),
            (".codex/agents/reviewer.toml", FileType::CodexAgent),
            (
                "/home/me/.codex/agents/pr-explorer.toml",
                FileType::CodexAgent,
            ),
            (".codex/rules/default.rules", FileType::CodexRules),
            ("/etc/codex/rules/team.rules", FileType::CodexRules),
            ("/home/me/.codex/prompts/draftpr.md", FileType::CodexPrompt),
            (
                ".agents/skills/sora/agents/openai.yaml",
                FileType::CodexSkillMetadata,
            ),
            (
                "/home/me/.codex/skills/.system/imagegen/agents/openai.yaml",
                FileType::CodexSkillMetadata,
            ),
            (
                "plugin/skills/a/agents/openai.yaml",
                FileType::CodexSkillMetadata,
            ),
        ] {
            assert_eq!(detect_file_type(Path::new(path)), expected, "{path}");
        }
        // Not Codex files: other tools' hooks.json, TOML outside .codex/agents,
        // and an openai.yaml that is not inside a skill.
        assert_eq!(
            detect_file_type(Path::new("plugin/hooks/hooks.json")),
            FileType::Hooks
        );
        assert_eq!(
            detect_file_type(Path::new(".cursor/hooks.json")),
            FileType::CursorHooks
        );
        for path in [
            "agents/reviewer.toml",
            ".codex/reviewer.toml",
            "rules/default.rules",
            "agents/openai.yaml",
            "project/agents/openai.yaml",
        ] {
            assert_eq!(
                detect_file_type(Path::new(path)),
                FileType::Unknown,
                "{path}"
            );
        }
    }

    #[test]
    fn detect_opencode_agents_commands_tools_and_plugins() {
        for (path, expected) in [
            (".opencode/agents/review.md", FileType::OpenCodeAgent),
            (".opencode/agents/team/reviewer.md", FileType::OpenCodeAgent),
            (".opencode/agent/review.md", FileType::OpenCodeAgent),
            (".opencode/modes/focus.md", FileType::OpenCodeAgent),
            (".opencode/agents/AGENTS.md", FileType::OpenCodeAgent),
            (
                "/home/me/.config/opencode/agents/review.md",
                FileType::OpenCodeAgent,
            ),
            ("/etc/opencode/agents/review.md", FileType::OpenCodeAgent),
            (".opencode/commands/review.md", FileType::OpenCodeCommand),
            (
                ".opencode/command/team/review.md",
                FileType::OpenCodeCommand,
            ),
            (
                "/home/me/.config/opencode/commands/audit.md",
                FileType::OpenCodeCommand,
            ),
            (".opencode/tools/database.ts", FileType::OpenCodeTool),
            (".opencode/tool/math.js", FileType::OpenCodeTool),
            (".opencode/plugins/notify.ts", FileType::OpenCodePlugin),
            (
                "/home/me/.config/opencode/plugin/agentsys.ts",
                FileType::OpenCodePlugin,
            ),
        ] {
            assert_eq!(detect_file_type(Path::new(path)), expected, "{path}");
        }
        // Skills, configs and unrelated files keep their types.
        assert_eq!(
            detect_file_type(Path::new(".opencode/skills/git-release/SKILL.md")),
            FileType::Skill
        );
        assert_eq!(
            detect_file_type(Path::new(".opencode/opencode.jsonc")),
            FileType::OpenCodeConfig
        );
        assert_eq!(
            detect_file_type(Path::new(
                "/home/me/.config/opencode/node_modules/x/README.md"
            )),
            FileType::Unknown
        );
        // Claude subagents are unaffected.
        assert_eq!(
            detect_file_type(Path::new(".claude/agents/review.md")),
            FileType::Agent
        );
        // A tools/ directory outside OpenCode is not a custom tool.
        assert_eq!(
            detect_file_type(Path::new("src/tools/database.ts")),
            FileType::Unknown
        );
    }

    #[test]
    fn detect_excluded_filename_case_insensitive() {
        assert_eq!(
            detect_file_type(Path::new("project/README.md")),
            FileType::Unknown
        );
        assert_eq!(
            detect_file_type(Path::new("project/Readme.md")),
            FileType::Unknown
        );
    }

    #[test]
    fn detect_unknown_for_non_config_files() {
        assert_eq!(
            detect_file_type(Path::new("src/main.rs")),
            FileType::Unknown
        );
        assert_eq!(
            detect_file_type(Path::new("package.json")),
            FileType::Unknown
        );
    }

    #[test]
    fn is_documentation_directory_case_insensitive() {
        assert!(is_documentation_directory(Path::new("DOCS/guide.md")));
        assert!(is_documentation_directory(Path::new("Docs/guide.md")));
        assert!(is_documentation_directory(Path::new("docs/guide.md")));
    }

    #[test]
    fn is_documentation_directory_negative() {
        assert!(!is_documentation_directory(Path::new("src/lib.rs")));
        assert!(!is_documentation_directory(Path::new("agents/task.md")));
    }

    // ---- CopilotScoped subdirectory detection ----

    #[test]
    fn detect_copilot_scoped_subdirectory() {
        assert_eq!(
            detect_file_type(Path::new(
                ".github/instructions/frontend/react.instructions.md"
            )),
            FileType::CopilotScoped
        );
    }

    #[test]
    fn detect_copilot_scoped_deep_nesting() {
        assert_eq!(
            detect_file_type(Path::new(
                ".github/instructions/frontend/components/dialog.instructions.md"
            )),
            FileType::CopilotScoped
        );
    }

    #[test]
    fn detect_copilot_scoped_not_under_github() {
        // .instructions.md under a different parent should NOT be CopilotScoped
        assert_ne!(
            detect_file_type(Path::new("other/instructions/react.instructions.md")),
            FileType::CopilotScoped
        );
    }

    #[test]
    fn detect_copilot_scoped_wrong_order() {
        // instructions/.github is the wrong order - should NOT be CopilotScoped
        assert_ne!(
            detect_file_type(Path::new("instructions/.github/foo.instructions.md")),
            FileType::CopilotScoped
        );
    }

    // ---- Gemini Settings / Extension / Ignore detection ----

    #[test]
    fn detect_gemini_settings() {
        assert_eq!(
            detect_file_type(Path::new(".gemini/settings.json")),
            FileType::GeminiSettings
        );
        assert_eq!(
            detect_file_type(Path::new(".gemini/settings.local.json")),
            FileType::GeminiSettings
        );
        assert_eq!(
            detect_file_type(Path::new(".GEMINI/settings.json")),
            FileType::GeminiSettings
        );
        assert_eq!(
            detect_file_type(Path::new(".GeMiNi/settings.local.json")),
            FileType::GeminiSettings
        );
    }

    #[test]
    fn detect_amp_settings() {
        for path in [
            ".amp/settings.json",
            ".amp/settings.jsonc",
            ".AMP/settings.json",
            "/home/u/.config/amp/settings.json",
            "/home/u/.config/amp/settings.jsonc",
            "C:/Users/u/.config/amp/settings.json",
            "/Library/Application Support/ampcode/managed-settings.json",
            "/etc/ampcode/managed-settings.json",
            "C:/ProgramData/ampcode/managed-settings.json",
        ] {
            assert_eq!(
                detect_file_type(Path::new(path)),
                FileType::AmpSettings,
                "{path}"
            );
        }
        // `.amp/settings.local.json` was AmpSettings; no Amp page documents
        // it, and it must not fall through to the Claude settings arm.
        assert_eq!(
            detect_file_type(Path::new(".amp/settings.local.json")),
            FileType::Unknown
        );
        // An `amp/` directory outside `~/.config` is not Amp's user config.
        assert_eq!(
            detect_file_type(Path::new("tools/amp/settings.json")),
            FileType::Hooks
        );
        // Claude Code and Kiro managed settings keep their own types.
        assert_eq!(
            detect_file_type(Path::new("/etc/claude-code/managed-settings.json")),
            FileType::Hooks
        );
        assert_eq!(
            detect_file_type(Path::new("/etc/kiro/managed-settings.json")),
            FileType::KiroPermissions
        );
    }

    #[test]
    fn detect_amp_global_checks() {
        for path in [
            "/home/u/.config/agents/checks/no-emojis.md",
            "/home/u/.config/amp/checks/api/auth.md",
        ] {
            assert_eq!(
                detect_file_type(Path::new(path)),
                FileType::AmpCheck,
                "{path}"
            );
        }
        assert_ne!(
            detect_file_type(Path::new("/home/u/agents/checks/no-emojis.md")),
            FileType::AmpCheck
        );
    }

    #[test]
    fn detect_amp_skill_mcp() {
        for path in [
            ".agents/skills/browser/mcp.json",
            ".claude/skills/linear/mcp.json",
            "/home/u/.agents/skills/linear/mcp.json",
            "/home/u/.config/agents/skills/linear/mcp.json",
            "/home/u/.config/amp/skills/linear/mcp.json",
        ] {
            assert_eq!(
                detect_file_type(Path::new(path)),
                FileType::AmpSkillMcp,
                "{path}"
            );
        }
        for (path, expected) in [
            // Directly in the root, or deeper than a skill directory.
            (".agents/skills/mcp.json", FileType::Mcp),
            (".agents/skills/browser/config/mcp.json", FileType::Mcp),
            ("skills/browser/mcp.json", FileType::Mcp),
            ("mcp.json", FileType::Mcp),
            // Kiro detection runs first and keeps power files.
            (".kiro/powers/linear/mcp.json", FileType::KiroMcp),
        ] {
            assert_eq!(detect_file_type(Path::new(path)), expected, "{path}");
        }
    }

    #[test]
    fn amp_skill_location_depth() {
        let location = |path: &str| amp_skill_location(Path::new(path)).map(|l| l.depth);
        assert_eq!(location(".agents/skills/a/SKILL.md"), Some(1));
        // The skills page example: `~/.agents/skills/something/deep/SKILL.md`.
        assert_eq!(
            location("/home/u/.agents/skills/something/deep/SKILL.md"),
            Some(2)
        );
        assert_eq!(location(".claude/skills/a/b/c/d/e/SKILL.md"), Some(5));
        assert_eq!(location(".claude/skills/a/b/c/d/e/f/SKILL.md"), Some(6));
        assert_eq!(location("/home/u/.config/amp/skills/a/SKILL.md"), Some(1));
        assert_eq!(location(".agents/skills/SKILL.md"), None);
        assert_eq!(location("skills/a/SKILL.md"), None);
        // The nearest root counts.
        assert_eq!(
            location(".agents/skills/a/.claude/skills/b/SKILL.md"),
            Some(1)
        );
        let claude = amp_skill_location(Path::new(".claude/skills/a/SKILL.md")).unwrap();
        assert!(claude.is_claude_root());
        assert!(
            !amp_skill_location(Path::new(".agents/skills/a/SKILL.md"))
                .unwrap()
                .is_claude_root()
        );
        assert_eq!(claude.root, Path::new(".claude/skills"));
    }

    #[test]
    fn detect_amp_check_case_insensitive_path() {
        assert_eq!(
            detect_file_type(Path::new(".AGENTS/CHECKS/security.md")),
            FileType::AmpCheck
        );
    }

    #[test]
    fn detect_settings_json_without_gemini_parent_is_hooks() {
        // settings.json without .gemini parent should remain Hooks
        assert_eq!(
            detect_file_type(Path::new("settings.json")),
            FileType::Hooks
        );
        assert_eq!(
            detect_file_type(Path::new(".claude/settings.json")),
            FileType::Hooks
        );
        assert_eq!(
            detect_file_type(Path::new("settings.local.json")),
            FileType::Hooks
        );
    }

    #[test]
    fn detect_gemini_extension() {
        assert_eq!(
            detect_file_type(Path::new("gemini-extension.json")),
            FileType::GeminiExtension
        );
        assert_eq!(
            detect_file_type(Path::new("project/gemini-extension.json")),
            FileType::GeminiExtension
        );
    }

    #[test]
    fn detect_gemini_ignore() {
        assert_eq!(
            detect_file_type(Path::new(".geminiignore")),
            FileType::GeminiIgnore
        );
        assert_eq!(
            detect_file_type(Path::new("project/.geminiignore")),
            FileType::GeminiIgnore
        );
    }

    // ---- Former Roo Code paths ----

    #[test]
    fn former_roo_code_paths_use_generic_detection() {
        // Roo Code was discontinued on 2026-05-15 and its file types were
        // removed. Its former paths now take the generic arms: Markdown
        // becomes GenericMarkdown, `mcp.json` the generic MCP type, `SKILL.md`
        // an Agent Skill, and the extensionless dotfiles are not linted.
        use FileType::*;
        assert_types(&[
            (".roorules", Unknown),
            ("project/.roorules", Unknown),
            (".roomodes", Unknown),
            (".rooignore", Unknown),
            (".roo/rules/general.md", GenericMarkdown),
            ("project/.roo/rules/coding.md", GenericMarkdown),
            (".roo/rules-architect/general.md", GenericMarkdown),
            (".roo/rules/SKILL.md", Skill),
            (".roo/rules-architect/SKILL.md", Skill),
            (".roo/mcp.json", Mcp),
            ("project/.ROO/mcp.json", Mcp),
            (".roo/skills/review/SKILL.md", Skill),
        ]);
    }

    #[test]
    fn detect_mcp_json_is_mcp() {
        assert_eq!(detect_file_type(Path::new("mcp.json")), FileType::Mcp);
        assert_eq!(
            detect_file_type(Path::new(".claude/mcp.json")),
            FileType::Mcp
        );
    }

    // ---- Windsurf detection ----

    #[test]
    fn detect_windsurf_rule() {
        assert_eq!(
            detect_file_type(Path::new(".windsurf/rules/custom.md")),
            FileType::WindsurfRule
        );
    }

    #[test]
    fn detect_windsurf_rule_nested() {
        assert_eq!(
            detect_file_type(Path::new(".windsurf/rules/frontend/style.md")),
            FileType::WindsurfRule
        );
    }

    #[test]
    fn detect_windsurf_workflow() {
        assert_eq!(
            detect_file_type(Path::new(".windsurf/workflows/deploy.md")),
            FileType::WindsurfWorkflow
        );
    }

    #[test]
    fn detect_windsurf_rules_legacy() {
        assert_eq!(
            detect_file_type(Path::new(".windsurfrules")),
            FileType::WindsurfRulesLegacy
        );
    }

    #[test]
    fn detect_windsurf_other_not_rule() {
        assert_ne!(
            detect_file_type(Path::new(".windsurf/README.md")),
            FileType::WindsurfRule
        );
    }

    #[test]
    fn detect_devin_desktop_locations() {
        use FileType::*;
        assert_types(&[
            // Preferred `.devin/` directories, same types as `.windsurf/`.
            (".devin/rules/style.md", WindsurfRule),
            ("packages/web/.devin/rules/frontend/style.md", WindsurfRule),
            ("/home/u/.devin/rules/style.md", WindsurfRule),
            (".DEVIN/Rules/style.md", WindsurfRule),
            (".devin/workflows/deploy.md", WindsurfWorkflow),
            (".devin/workflows/release/deploy.md", WindsurfWorkflow),
            (
                "/home/u/.codeium/windsurf/global_workflows/deploy.md",
                WindsurfWorkflow,
            ),
            (
                "/home/u/.codeium/windsurf/memories/global_rules.md",
                WindsurfGlobalRules,
            ),
            // Devin CLI's project global rules state no limit.
            (".devin/global_rules.md", GenericMarkdown),
            (".windsurf/global_rules.md", GenericMarkdown),
            // Skills keep the Agent Skills type.
            (".devin/skills/deploy/SKILL.md", Skill),
            (".devin/rules/SKILL.md", Skill),
            (".devin/rules/AGENTS.md", ClaudeMd),
            // Not rule or workflow files.
            (".devin/rules/notes.txt", Unknown),
            (".devin/README.md", Unknown),
            ("rules/style.md", GenericMarkdown),
            ("devin/rules/style.md", GenericMarkdown),
        ]);
    }

    #[test]
    fn detect_windsurf_hooks_json() {
        use FileType::*;
        assert_types(&[
            (".devin/hooks.json", WindsurfHooks),
            (".windsurf/hooks.json", WindsurfHooks),
            ("project/.Devin/Hooks.json", WindsurfHooks),
            ("/home/u/.codeium/windsurf/hooks.json", WindsurfHooks),
            ("/home/u/.codeium/hooks.json", WindsurfHooks),
            (
                "/Library/Application Support/Devin/hooks.json",
                WindsurfHooks,
            ),
            (
                "/Library/Application Support/Windsurf/hooks.json",
                WindsurfHooks,
            ),
            ("/etc/devin/hooks.json", WindsurfHooks),
            ("/etc/windsurf/hooks.json", WindsurfHooks),
            ("C:/ProgramData/Devin/hooks.json", WindsurfHooks),
            ("C:/ProgramData/Windsurf/hooks.json", WindsurfHooks),
            // Devin CLI lifecycle hooks use another format and file name.
            (".devin/hooks.v1.json", Unknown),
            (".devin/config.json", Unknown),
            // Other tools' hooks.json files keep their types.
            (".cursor/hooks.json", CursorHooks),
            (".github/hooks/hooks.json", CopilotHooks),
            ("my-plugin/hooks/hooks.json", Hooks),
            (".devin/hooks/hooks.json", Hooks),
            (".kiro/hooks/hooks.json", KiroHookV3),
            ("hooks.json", Unknown),
            ("devin/hooks.json", Unknown),
            ("/home/u/.codeium/windsurf/other/hooks.json", Unknown),
        ]);
    }

    // ---- Kiro Steering detection ----

    #[test]
    fn detect_kiro_steering() {
        assert_eq!(
            detect_file_type(Path::new(".kiro/steering/typescript.md")),
            FileType::KiroSteering
        );
    }

    #[test]
    fn detect_kiro_steering_nested() {
        assert_eq!(
            detect_file_type(Path::new("project/.kiro/steering/guidelines.md")),
            FileType::KiroSteering
        );
    }

    #[test]
    fn detect_kiro_steering_not_outside_kiro() {
        // A .md file in steering/ but not under .kiro/ should not be KiroSteering
        assert_ne!(
            detect_file_type(Path::new("steering/guidelines.md")),
            FileType::KiroSteering
        );
    }

    #[test]
    fn detect_kiro_steering_not_other_kiro_file() {
        // A .md file directly under .kiro/ (not in steering/) should not be KiroSteering
        assert_ne!(
            detect_file_type(Path::new(".kiro/README.md")),
            FileType::KiroSteering
        );
    }

    #[test]
    fn detect_kiro_steering_overrides_filename_matches() {
        // AGENTS.md under .kiro/steering/ should be KiroSteering, not ClaudeMd
        assert_eq!(
            detect_file_type(Path::new(".kiro/steering/AGENTS.md")),
            FileType::KiroSteering
        );
        assert_eq!(
            detect_file_type(Path::new(".kiro/steering/SKILL.md")),
            FileType::KiroSteering
        );
    }

    #[test]
    fn detect_kiro_power_from_fixture_path() {
        assert_eq!(
            detect_file_type(Path::new(
                "tests/fixtures/kiro-powers/.kiro/powers/valid-power/POWER.md"
            )),
            FileType::KiroPower
        );
    }

    #[test]
    fn detect_kiro_power_from_dot_kiro_powers_path() {
        assert_eq!(
            detect_file_type(Path::new(".kiro/powers/deploy/POWER.md")),
            FileType::KiroPower
        );
        assert_eq!(
            detect_file_type(Path::new(".KIRO/POWERS/deploy/POWER.md")),
            FileType::KiroPower
        );
    }

    #[test]
    fn detect_kiro_power_not_any_power_md() {
        assert_ne!(
            detect_file_type(Path::new("docs/POWER.md")),
            FileType::KiroPower
        );
    }

    #[test]
    fn detect_kiro_power_avoids_documentation_dirs() {
        // Published powers live at a repository root or in any folder
        // (kiro.dev/docs/powers/create), so `tmp/kiro-powers/POWER.md` is now a
        // power: POWER.md is Kiro's power manifest name. This test used to
        // assert the opposite. Documentation folders are still skipped.
        assert_eq!(
            detect_file_type(Path::new("tmp/kiro-powers/POWER.md")),
            FileType::KiroPower
        );
        assert_ne!(
            detect_file_type(Path::new("docs/fixtures/kiro-powers/POWER.md")),
            FileType::KiroPower
        );
    }

    #[test]
    fn detect_kiro_agent_json() {
        assert_eq!(
            detect_file_type(Path::new(".kiro/agents/reviewer.json")),
            FileType::KiroAgent
        );
        // Both engines compare the extension case-sensitively, so
        // `reviewer.JSON` is never loaded (it used to be a KiroAgent); it is
        // KiroNotLoaded and gets KIRO-026. The directories still match
        // case-insensitively.
        assert_eq!(
            detect_file_type(Path::new("home/.kiro/agents/reviewer.JSON")),
            FileType::KiroNotLoaded
        );
        assert_eq!(
            detect_file_type(Path::new("home/.KIRO/AGENTS/reviewer.json")),
            FileType::KiroAgent
        );
        assert_eq!(
            detect_file_type(Path::new(".kiro/agents/team/reviewer.json")),
            FileType::KiroAgent
        );
    }

    #[test]
    fn detect_kiro_agent_markdown() {
        assert_eq!(
            detect_file_type(Path::new(".kiro/agents/reviewer.md")),
            FileType::KiroAgent
        );
        // Exact `.md` only (see detect_kiro_agent_json).
        assert_eq!(
            detect_file_type(Path::new("home/.KIRO/AGENTS/team/reviewer.md")),
            FileType::KiroAgent
        );
        assert_eq!(
            detect_file_type(Path::new("home/.KIRO/AGENTS/team/reviewer.MD")),
            FileType::KiroNotLoaded
        );
    }

    #[test]
    fn detect_kiro_agent_not_other_kiro_json() {
        assert_ne!(
            detect_file_type(Path::new(".kiro/settings/agent.json")),
            FileType::KiroAgent
        );
    }

    #[test]
    fn detect_kiro_agent_dir_reserved_names_are_agents() {
        // Both engines load every top-level `*.json` of `.kiro/agents/` as an
        // agent (V3 also every `*.md`), whatever the name, so these are Kiro
        // agents. They used to be excluded and reached the Claude plugin,
        // Claude settings and generic MCP validators instead.
        for name in [
            ".kiro/agents/plugin.json",
            ".kiro/agents/settings.json",
            ".kiro/agents/mcp.json",
            ".kiro/agents/mcp-prod.json",
            ".kiro/agents/server.mcp.json",
            ".kiro/agents/settings.local.json",
            ".kiro/agents/SETTINGS.LOCAL.json",
            ".kiro/agents/mcp-manager.md",
        ] {
            assert_eq!(
                detect_file_type(Path::new(name)),
                FileType::KiroAgent,
                "{name}"
            );
        }
    }

    #[test]
    fn detect_kiro_hook_file() {
        assert_eq!(
            detect_file_type(Path::new(".kiro/hooks/on-save.kiro.hook")),
            FileType::KiroHook
        );
        assert_eq!(
            detect_file_type(Path::new(".KIRO/HOOKS/on-save.kiro.hook")),
            FileType::KiroHook
        );
    }

    #[test]
    fn detect_kiro_v3_hook_file() {
        assert_eq!(
            detect_file_type(Path::new(".kiro/hooks/lint-on-save.json")),
            FileType::KiroHookV3
        );
        assert_eq!(
            detect_file_type(Path::new(".KIRO/HOOKS/global-audit.json")),
            FileType::KiroHookV3
        );
        // V3 lists `.kiro/hooks/` and keeps names ending in `.json` exactly.
        assert_eq!(
            detect_file_type(Path::new(".KIRO/HOOKS/global-audit.JSON")),
            FileType::KiroNotLoaded
        );
    }

    #[test]
    fn detect_kiro_v3_hook_wrong_directory_is_not_v3_hook() {
        assert_ne!(
            detect_file_type(Path::new(".kiro/agents/lint-on-save.json")),
            FileType::KiroHookV3
        );
        assert_ne!(
            detect_file_type(Path::new(".kiro/hooks/nested/deep.json")),
            FileType::KiroHookV3
        );
    }

    #[test]
    fn detect_kiro_hook_not_wrong_directory() {
        assert_ne!(
            detect_file_type(Path::new(".kiro/agents/on-save.kiro.hook")),
            FileType::KiroHook
        );
    }

    #[test]
    fn detect_kiro_mcp_settings_file() {
        assert_eq!(
            detect_file_type(Path::new(".kiro/settings/mcp.json")),
            FileType::KiroMcp
        );
        assert_eq!(
            detect_file_type(Path::new(".KIRO/SETTINGS/MCP.JSON")),
            FileType::KiroMcp
        );
    }

    #[test]
    fn detect_kiro_mcp_takes_precedence_over_generic_mcp() {
        assert_eq!(
            detect_file_type(Path::new("workspace/.kiro/settings/mcp.json")),
            FileType::KiroMcp
        );
    }

    #[test]
    fn detect_kiro_mcp_not_other_kiro_json() {
        assert_ne!(
            detect_file_type(Path::new(".kiro/settings/not-mcp.json")),
            FileType::KiroMcp
        );
    }

    #[test]
    fn detect_gemini_extension_hooks_json_not_claude_hooks() {
        // Gemini CLI extension hooks use Gemini events (BeforeAgent, AfterTool).
        assert_ne!(
            detect_file_type(Path::new("remember/.gemini/hooks/hooks.json")),
            FileType::Hooks
        );
    }

    #[test]
    fn detect_installed_gemini_extension_members() {
        use FileType::*;
        assert_types(&[
            (
                "/home/u/.gemini/extensions/remember/hooks/hooks.json",
                GeminiHooks,
            ),
            (".gemini/extensions/remember/hooks/hooks.json", GeminiHooks),
            (".GEMINI/Extensions/remember/Hooks/hooks.json", GeminiHooks),
            (
                "/home/u/.gemini/extensions/remember/agents/reviewer.md",
                GeminiAgent,
            ),
            // Not members: other files, other folders, other roots.
            (".gemini/extensions/remember/hooks/other.json", Unknown),
            (".gemini/extensions/remember/agents/sub/deep.md", Agent),
            ("extensions/remember/hooks/hooks.json", Hooks),
            // Path-only detection cannot see gemini-extension.json; these
            // are refined by `refine_file_type`.
            ("my-ext/hooks/hooks.json", Hooks),
            ("my-ext/agents/reviewer.md", Agent),
            ("remember/.gemini/hooks/hooks.json", Unknown),
        ]);
    }

    #[test]
    fn detect_gemini_commands_and_policies() {
        use FileType::*;
        assert_types(&[
            // Custom commands: user and project, at any depth.
            ("/home/u/.gemini/commands/test.toml", GeminiCommand),
            (".gemini/commands/git/commit.toml", GeminiCommand),
            (".gemini/commands/a/b/c.toml", GeminiCommand),
            (".GEMINI/Commands/review.TOML", GeminiCommand),
            // Installed extensions.
            (
                "/home/u/.gemini/extensions/gcp/commands/gcs/sync.toml",
                GeminiCommand,
            ),
            (".gemini/extensions/gcp/policies/rules.toml", GeminiPolicy),
            // Policies: user, workspace and admin directories, top level only.
            ("/home/u/.gemini/policies/my-rules.toml", GeminiPolicy),
            (".gemini/policies/rules.toml", GeminiPolicy),
            ("/etc/gemini-cli/policies/org.toml", GeminiPolicy),
            (
                "/Library/Application Support/GeminiCli/policies/org.toml",
                GeminiPolicy,
            ),
            ("C:/ProgramData/gemini-cli/policies/org.toml", GeminiPolicy),
            (".gemini/policies/sub/rules.toml", Unknown),
            // Not commands or policies.
            (".gemini/commands/notes.md", GenericMarkdown),
            (".gemini/commands.toml", Unknown),
            ("commands/deploy.toml", Unknown),
            ("policies/rules.toml", Unknown),
            (".codex/config.toml", CodexConfig),
        ]);
    }

    #[test]
    fn refine_gemini_extension_commands_and_policies() {
        use FileType::*;
        let fs = crate::fs::MockFileSystem::new();
        fs.add_file("ext/gemini-extension.json", "{}");
        fs.add_file("remember/.gemini/gemini-extension.json", "{}");
        for (path, expected) in [
            ("ext/commands/deploy.toml", GeminiCommand),
            ("ext/commands/gcs/sync.toml", GeminiCommand),
            ("ext/policies/policies.toml", GeminiPolicy),
            ("remember/.gemini/policies/p.toml", GeminiPolicy),
            // No manifest above the folder, or not a member location.
            ("other/commands/deploy.toml", Unknown),
            ("other/policies/p.toml", Unknown),
            ("ext/policies/sub/p.toml", Unknown),
            ("ext/config.toml", Unknown),
        ] {
            assert_eq!(refine(path, &fs), expected, "{path}");
        }
        // The extension root of a policy file, for the extension-tier rules.
        assert_eq!(
            gemini_policy_extension_root(Path::new("ext/policies/policies.toml"), &fs),
            Some(Path::new("ext"))
        );
        assert_eq!(
            gemini_policy_extension_root(
                Path::new("/home/u/.gemini/extensions/gcp/policies/p.toml"),
                &fs
            ),
            Some(Path::new("/home/u/.gemini/extensions/gcp"))
        );
        assert_eq!(
            gemini_policy_extension_root(Path::new("proj/.gemini/policies/p.toml"), &fs),
            None
        );
    }

    fn refine(path: &str, fs: &crate::fs::MockFileSystem) -> FileType {
        refine_file_type(Path::new(path), detect_file_type(Path::new(path)), fs)
    }

    #[test]
    fn refine_gemini_extension_roots_by_manifest() {
        use FileType::*;
        let fs = crate::fs::MockFileSystem::new();
        // A repository that is an extension, and remember's `.gemini/` layout.
        fs.add_file("ext/gemini-extension.json", "{}");
        fs.add_file("remember/.gemini/gemini-extension.json", "{}");
        // A directory named like the manifest is not a manifest.
        fs.add_dir("odd/gemini-extension.json");
        for (path, expected) in [
            ("ext/hooks/hooks.json", GeminiHooks),
            ("ext/agents/reviewer.md", GeminiAgent),
            ("remember/.gemini/hooks/hooks.json", GeminiHooks),
            ("remember/.gemini/agents/a.md", GeminiAgent),
            // No manifest next to the folder: unchanged.
            ("other/hooks/hooks.json", Hooks),
            ("other/agents/reviewer.md", Agent),
            ("ext/sub/hooks/hooks.json", Hooks),
            ("odd/hooks/hooks.json", Hooks),
            ("remember/hooks/hooks.json", Hooks),
            // Other members of an extension root keep their type.
            ("ext/gemini-extension.json", GeminiExtension),
            ("ext/GEMINI.md", GeminiMd),
        ] {
            assert_eq!(refine(path, &fs), expected, "{path}");
        }
    }

    #[test]
    fn refine_never_replaces_specific_or_kiro_types() {
        use FileType::*;
        let fs = crate::fs::MockFileSystem::new();
        for root in [".github", ".cursor", ".kiro/specs/x", ".claude"] {
            fs.add_file(format!("{root}/gemini-extension.json"), "{}");
        }
        for (path, expected) in [
            (".github/hooks/hooks.json", CopilotHooks),
            (".cursor/agents/reviewer.md", CursorAgent),
            (".kiro/specs/x/hooks/hooks.json", Unknown),
            // A Claude `.claude/agents/*.md` is the generic Agent type, so a
            // (contrived) manifest next to it would refine it.
            (".claude/agents/reviewer.md", GeminiAgent),
        ] {
            assert_eq!(refine(path, &fs), expected, "{path}");
        }
    }

    #[test]
    fn detect_plugin_hooks_json_as_hooks() {
        // Claude Code plugin hooks/hooks.json → FileType::Hooks
        assert_eq!(
            detect_file_type(Path::new("my-plugin/hooks/hooks.json")),
            FileType::Hooks
        );
        assert_eq!(
            detect_file_type(Path::new("project/plugins/my-plugin/hooks/hooks.json")),
            FileType::Hooks
        );
    }

    #[test]
    fn detect_github_hooks_still_copilot_hooks() {
        // .github/hooks/hooks.json must remain CopilotHooks
        assert_eq!(
            detect_file_type(Path::new(".github/hooks/hooks.json")),
            FileType::CopilotHooks
        );
    }

    #[test]
    fn detect_cursor_hooks_still_cursor_hooks() {
        // .cursor/hooks.json must remain CursorHooks (cursor guard)
        assert_eq!(
            detect_file_type(Path::new(".cursor/hooks.json")),
            FileType::CursorHooks
        );
    }

    // ---- Kiro detection matrix (spec 1.1, 2.1, 3.1, 4.1, 5.1, 6.1-6.3) ----

    fn assert_types(cases: &[(&str, FileType)]) {
        for (path, expected) in cases {
            assert_eq!(detect_file_type(Path::new(path)), *expected, "{path}");
        }
    }

    #[test]
    fn kiro_agents_match_where_each_engine_loads_them() {
        use FileType::*;
        assert_types(&[
            (".kiro/agents/reviewer.json", KiroAgent),
            ("/home/u/.kiro/agents/reviewer.json", KiroAgent),
            // V3 recurses and reads `.md`; it also tries non-agent Markdown.
            (".kiro/agents/team/planner.md", KiroAgent),
            (".kiro/agents/team/deep/planner.json", KiroAgent),
            (".kiro/agents/README.md", KiroAgent),
            (".kiro/agents/AGENTS.md", KiroAgent),
            (".kiro/agents/resources/LANE_SECURITY.md", KiroAgent),
            (".kiro/agents/prompts/coder.md", KiroAgent),
            (".kiro/agents/ada.json.attribution.json", KiroAgent),
            // CLI 2.x falls back to `.amazonq/cli-agents/` (top level, JSON).
            (".amazonq/cli-agents/qlocal.json", KiroAgent),
            ("/home/u/.aws/amazonq/cli-agents/qglobal.json", Unknown),
            (".amazonq/cli-agents/sub/nested.json", Unknown),
            // Not loaded: other extensions and case variants.
            (".kiro/agents/reviewer.yaml", Unknown),
            (".kiro/agents/run.sh", Unknown),
            (".kiro/agents/reviewer.jsonc", KiroNotLoaded),
            (".kiro/agents/reviewer.Json", KiroNotLoaded),
            // Other tools' agent directories are untouched.
            (".claude/agents/reviewer.md", Agent),
            ("agents/reviewer.md", Agent),
            (".cursor/agents/reviewer.md", CursorAgent),
            (".github/agents/reviewer.agent.md", CopilotAgent),
            (".gemini/agents/reviewer.md", GeminiAgent),
            ("kiro/agents/reviewer.json", Unknown),
            (".kiro-old/agents/reviewer.json", Unknown),
        ]);
    }

    #[test]
    fn kiro_agent_loaders_follow_the_engines() {
        let loaders = |path: &str| kiro_agent_loaders(Path::new(path));
        let both = KiroAgentLoaders { v2: true, v3: true };
        let v3 = KiroAgentLoaders {
            v2: false,
            v3: true,
        };
        let v2 = KiroAgentLoaders {
            v2: true,
            v3: false,
        };
        assert_eq!(loaders(".kiro/agents/a.json"), Some(both));
        assert_eq!(loaders("/home/u/.kiro/agents/a.json"), Some(both));
        assert_eq!(loaders(".kiro/agents/a.md"), Some(v3));
        assert_eq!(loaders(".kiro/agents/team/a.json"), Some(v3));
        assert_eq!(loaders(".amazonq/cli-agents/a.json"), Some(v2));
        assert_eq!(loaders(".kiro/agents/a.JSON"), None);
        assert_eq!(loaders(".kiro/hooks/a.json"), None);
    }

    #[test]
    fn kiro_hooks_are_top_level_json_files() {
        use FileType::*;
        assert_types(&[
            (".kiro/hooks/lint.json", KiroHookV3),
            ("/home/u/.kiro/hooks/audit.json", KiroHookV3),
            // V3 reads every top-level `.json`, whatever the name.
            (".kiro/hooks/mcp.json", KiroHookV3),
            (".kiro/hooks/hooks.json", KiroHookV3),
            (".kiro/hooks/save.kiro.hook.json", KiroHookV3),
            (".kiro/hooks/save.kiro.hook", KiroHook),
            // No recursion.
            (".kiro/hooks/team/lint.json", KiroNotLoaded),
            (".kiro/hooks/team/save.kiro.hook", Unknown),
            (".kiro/hooks/lint.yaml", KiroNotLoaded),
            (".kiro/hooks/scripts/check.sh", Unknown),
            (".kiro/hooks/README.md", Unknown),
            // Other tools' hook files.
            (".github/hooks/hooks.json", CopilotHooks),
            ("my-plugin/hooks/hooks.json", Hooks),
            (".cursor/hooks.json", CursorHooks),
            ("hooks/lint.json", Unknown),
        ]);
    }

    #[test]
    fn kiro_mcp_is_settings_mcp_json_only() {
        use FileType::*;
        assert_types(&[
            (".kiro/settings/mcp.json", KiroMcp),
            ("/home/u/.kiro/settings/mcp.json", KiroMcp),
            // Opened by path, so case-insensitive file systems find it.
            (".KIRO/SETTINGS/MCP.JSON", KiroMcp),
            (".kiro/mcp.json", KiroNotLoaded),
            (".kiro/settings/mcp.jsonc", KiroNotLoaded),
            (".kiro/settings/mcp.json ", KiroNotLoaded),
            // Not Kiro files, and not generic MCP files either.
            (".kiro/settings/aws-mcp.json", Unknown),
            (".kiro/settings/mcp-servers.json", Unknown),
            // Other tools' MCP files.
            ("mcp.json", Mcp),
            (".cursor/mcp.json", Mcp),
            (".amazonq/mcp.json", Mcp),
            // Was RooMcp; Roo Code support was removed, so it is generic MCP.
            (".roo/mcp.json", Mcp),
        ]);
    }

    #[test]
    fn kiro_steering_is_recursive_markdown() {
        use FileType::*;
        assert_types(&[
            (".kiro/steering/style.md", KiroSteering),
            ("/home/u/.kiro/steering/team/style.md", KiroSteering),
            (".kiro/steering/AGENTS.md", KiroSteering),
            (".kiro/steering/banner/SKILL.md", KiroSteering),
            (".kiro/steering/Style.MD", KiroNotLoaded),
            (".kiro/steering/notes.txt", Unknown),
            ("steering/style.md", GenericMarkdown),
            (".windsurf/rules/style.md", WindsurfRule),
        ]);
    }

    #[test]
    fn kiro_skills_stay_agent_skills() {
        use FileType::*;
        assert_types(&[
            (".kiro/skills/review/SKILL.md", Skill),
            ("/home/u/.kiro/skills/review/SKILL.md", Skill),
            (".KIRO/SKILLS/review/SKILL.md", Skill),
            (".kiro/skills/review/references/guide.md", GenericMarkdown),
            (".kiro/powers/installed/aws/skills/setup/SKILL.md", Skill),
        ]);
    }

    #[test]
    fn kiro_installed_powers_have_typed_parts() {
        use FileType::*;
        assert_types(&[
            (".kiro/powers/aws/POWER.md", KiroPower),
            ("/home/u/.kiro/powers/installed/aws/POWER.md", KiroPower),
            (".kiro/powers/aws/power.md", KiroPower),
            (".kiro/powers/installed/aws/plugin.json", KiroPowerPlugin),
            (".kiro/powers/aws/plugin.json", KiroPowerPlugin),
            (".kiro/powers/installed/aws/mcp.json", KiroMcp),
            (".kiro/powers/aws/mcp.json", KiroMcp),
            (".kiro/powers/aws/steering/patterns.md", KiroPowerSteering),
            (
                ".kiro/powers/installed/aws/steering/patterns.md",
                KiroPowerSteering,
            ),
            (
                ".kiro/powers/installed/aws/dev.kiro/steering/patterns.md",
                KiroPowerSteering,
            ),
            (
                ".kiro/powers/installed/aws/dev.kiro/INSTRUCTIONS.md",
                KiroPowerSteering,
            ),
            // Steering is listed top-level only.
            (".kiro/powers/aws/steering/sub/patterns.md", GenericMarkdown),
            (".kiro/powers/installed.json", Unknown),
            (".kiro/powers/aws/nested/plugin.json", Unknown),
        ]);
    }

    #[test]
    fn kiro_published_powers_are_recognised_by_kiro_signals_only() {
        use FileType::*;
        assert_types(&[
            ("POWER.md", KiroPower),
            ("arm-soc-migration/POWER.md", KiroPower),
            ("power/POWER.md", KiroPower),
            ("powers/kanon/POWER.md", KiroPower),
            ("examples/security-audit/POWER.md", KiroPower),
            ("docs/POWER.md", Unknown),
            ("power.md", GenericMarkdown),
            ("power/plugin.json", KiroPowerPlugin),
            ("powers/supabase/plugin.json", KiroPowerPlugin),
            ("power/mcp.json", KiroMcp),
            ("powers/database-power/mcp.json", KiroMcp),
            ("power/steering/patterns.md", KiroPowerSteering),
            (
                "powers/database-power/steering/patterns.md",
                KiroPowerSteering,
            ),
            ("my-power/dev.kiro/steering/patterns.md", KiroPowerSteering),
            ("my-power/dev.kiro/INSTRUCTIONS.md", KiroPowerSteering),
            // Without a Kiro signal, plugin and MCP files keep their types.
            ("plugin.json", Plugin),
            ("my-power/plugin.json", Plugin),
            (".claude-plugin/plugin.json", Plugin),
            (
                "/home/u/.claude/plugins/marketplaces/m/.claude-plugin/plugin.json",
                Plugin,
            ),
            (".codex-plugin/plugin.json", CodexPlugin),
            ("my-power/mcp.json", Mcp),
            ("src/steering/patterns.md", GenericMarkdown),
        ]);
    }

    #[test]
    fn kiro_settings_files() {
        use FileType::*;
        assert_types(&[
            (".kiro/settings/cli.json", KiroSettings),
            ("/home/u/.kiro/settings/cli.json", KiroSettings),
            // Not read by Kiro CLI (KIRO-026 when it holds CLI keys), and never
            // Claude Code settings.
            (".kiro/settings.json", KiroNotLoaded),
            ("/home/u/.kiro/settings.json", KiroNotLoaded),
            (".kiro/settings.local.json", Unknown),
            (".kiro/settings/settings.json", Unknown),
            ("packages/cli/.kiro/settings/settings.json", Unknown),
            (".kiro/settings/lsp.json", Unknown),
            (".kiro/settings/sandbox.json", Unknown),
            (".kiro/cli.json", KiroNotLoaded),
            (".kiro/plugin.json", Unknown),
            (".kiro/specs/feature/settings.json", Unknown),
            (".kiro/specs/feature/hooks/hooks.json", Unknown),
            // Other tools' settings files.
            (".claude/settings.json", Hooks),
            ("settings.json", Hooks),
            (".gemini/settings.json", GeminiSettings),
            (".amp/settings.json", AmpSettings),
        ]);
    }

    #[test]
    fn kiro_permissions_files() {
        use FileType::*;
        for path in [
            ".kiro/settings/permissions.yaml",
            "/home/u/.kiro/settings/permissions.json",
            "/home/u/.kiro/workspace-roots/0123456789abcdef/permissions.yaml",
            "/home/u/.kiro/workspace-roots/0123456789abcdef/permissions.json",
            "/Library/Application Support/Kiro/managed-settings.json",
            "/etc/kiro/managed-settings.json",
            "C:/ProgramData/Kiro/managed-settings.json",
        ] {
            assert_eq!(detect_file_type(Path::new(path)), KiroPermissions, "{path}");
        }
        assert_types(&[
            // Only `.yaml` and `.json` are opened.
            (".kiro/settings/permissions.yml", KiroNotLoaded),
            (".kiro/settings/ permissions.yaml", KiroNotLoaded),
            (
                "/home/u/.kiro/workspace-roots/abc/permissions.yml",
                KiroNotLoaded,
            ),
            (".kiro/permissions.yaml", KiroNotLoaded),
            ("/home/u/.kiro/workspace-roots/permissions.yaml", Unknown),
            (".kiro/workspace-roots/a/b/permissions.yaml", Unknown),
            // Claude Code's managed settings are unaffected.
            (
                "/Library/Application Support/ClaudeCode/managed-settings.json",
                Hooks,
            ),
            ("other/managed-settings.json", Unknown),
        ]);
    }

    #[test]
    fn kiro_not_loaded_reasons() {
        use KiroNotLoadedReason::*;
        let reason = |path: &str| kiro_not_loaded_reason(Path::new(path));
        assert_eq!(
            reason(".kiro/agents/a.JSON"),
            Some(ExtensionCase {
                expected: ".json",
                dir: ".kiro/agents"
            })
        );
        assert_eq!(
            reason(".kiro/steering/a.Md"),
            Some(ExtensionCase {
                expected: ".md",
                dir: ".kiro/steering"
            })
        );
        assert_eq!(
            reason(".kiro/hooks/a.yml"),
            Some(UnsupportedExtension {
                expected: ".json",
                dir: ".kiro/hooks"
            })
        );
        assert_eq!(reason(".kiro/hooks/sub/a.json"), Some(NestedHookFile));
        assert_eq!(
            reason(".kiro/settings/permissions.yml"),
            Some(Misnamed {
                expected: "permissions.yaml"
            })
        );
        assert_eq!(
            reason(".kiro/settings/ cli.json"),
            Some(Misnamed {
                expected: "cli.json"
            })
        );
        assert_eq!(
            reason(".kiro/mcp.json"),
            Some(Misplaced {
                expected: ".kiro/settings/mcp.json"
            })
        );
        assert_eq!(reason(".kiro/settings.json"), Some(LegacySettings));
        assert_eq!(reason(".kiro/agents/a.json"), None);
        assert_eq!(reason("settings.json"), None);
    }

    #[test]
    fn kiro_workflows_are_top_level_recipe_files() {
        use FileType::*;
        assert_types(&[
            (".kiro/workflows/plan.workflow.json", KiroWorkflow),
            (".kiro/workflows/plan.workflow.yaml", KiroWorkflow),
            (".kiro/workflows/plan.workflow.yml", KiroWorkflow),
            ("/home/u/.kiro/workflows/review.workflow.json", KiroWorkflow),
            // Near misses Kiro never loads (KIRO-026).
            (".kiro/workflows/plan.json", KiroNotLoaded),
            (".kiro/workflows/plan.yaml", KiroNotLoaded),
            (".kiro/workflows/plan.Workflow.json", KiroNotLoaded),
            (".kiro/workflows/team/plan.workflow.json", KiroNotLoaded),
            (".kiro/workflows/README.md", Unknown),
            (".kiro/workflows/watch-review.mjs", Unknown),
            // Run output and other tools' workflow files.
            (".kiro/workflow-output/status.json", Unknown),
            (".github/workflows/ci.yml", Unknown),
            (".windsurf/workflows/deploy.md", WindsurfWorkflow),
            ("workflows/plan.workflow.json", Unknown),
        ]);
    }

    #[test]
    fn kiro_ignore_files() {
        use FileType::*;
        assert_types(&[
            (".kiroignore", KiroIgnore),
            ("packages/api/.kiroignore", KiroIgnore),
            ("/home/u/.kiro/settings/kiroignore", KiroIgnore),
            (".kiro/settings/kiroignore", KiroIgnore),
            // Other tools' ignore files.
            (".gitignore", Unknown),
            (".geminiignore", GeminiIgnore),
            // Was RooIgnore; Roo Code support was removed.
            (".rooignore", Unknown),
            // Cursor's ignore file has its own type (CUR-034).
            (".cursorignore", CursorIgnore),
            ("kiroignore", Unknown),
        ]);
    }

    #[test]
    fn kiro_power_skills() {
        for path in [
            ".kiro/powers/installed/aws/skills/setup/SKILL.md",
            "/home/u/.kiro/powers/aws/skills/setup/SKILL.md",
            "power/skills/setup/SKILL.md",
            "powers/supabase/skills/setup/SKILL.md",
        ] {
            assert!(is_kiro_power_skill(Path::new(path)), "{path}");
            assert_eq!(detect_file_type(Path::new(path)), FileType::Skill, "{path}");
        }
        for path in [
            ".kiro/skills/setup/SKILL.md",
            ".kiro/powers/aws/skills/SKILL.md",
            ".kiro/powers/aws/skills/a/b/SKILL.md",
            "plugins/x/skills/setup/SKILL.md",
            ".claude/skills/setup/SKILL.md",
            "my-power/skills/setup/SKILL.md",
        ] {
            assert!(!is_kiro_power_skill(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn inside_kiro_dir() {
        assert!(is_inside_kiro_dir(Path::new(
            ".kiro/settings/settings.json"
        )));
        assert!(is_inside_kiro_dir(Path::new("/home/u/.KIRO/x.json")));
        assert!(!is_inside_kiro_dir(Path::new(".kiro")));
        assert!(!is_inside_kiro_dir(Path::new("kiro/settings.json")));
        assert!(!is_inside_kiro_dir(Path::new(".claude/settings.json")));
    }
}
