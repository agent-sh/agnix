//! FileType enum for validator dispatch.

use std::fmt;

/// Detected file type for validator dispatch.
///
/// Each variant maps to a class of agent configuration file that has
/// a dedicated set of validators registered in the
/// [`ValidatorRegistry`](crate::ValidatorRegistry).
///
/// The enum intentionally derives [`Hash`], [`Eq`], and [`Copy`] so that it
/// can be used as a key in [`HashMap`](std::collections::HashMap)-backed
/// registries without allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    /// Pi settings, models, keybindings, MCP, themes and package manifests.
    PiConfig,
    /// Pi system instructions and reusable prompt templates.
    PiPrompt,
    /// Antigravity CLI settings, MCP, hooks, plugins and rule registries.
    AntigravityConfig,
    /// Antigravity modular rules with activation frontmatter.
    AntigravityRule,
    /// Antigravity Markdown custom agents.
    AntigravityAgent,
    /// Grok TOML configuration and native JSON hooks/plugin metadata.
    GrokConfig,
    /// Grok project Markdown rules.
    GrokRule,
    /// Grok custom Markdown agent definitions.
    GrokAgent,
    /// SKILL.md files
    Skill,
    /// CLAUDE.md, AGENTS.md files
    ClaudeMd,
    /// .claude/agents/*.md or agents/*.md
    Agent,
    /// Amp code review checks (.agents/checks/*.md)
    AmpCheck,
    /// Claude settings files, including system managed-settings.d fragments
    Hooks,
    /// plugin.json (validator checks .claude-plugin/ location)
    Plugin,
    /// Claude Code plugin marketplace file (.claude-plugin/marketplace.json)
    ClaudeMarketplace,
    /// MCP configuration files (*.mcp.json, mcp.json, mcp-*.json)
    Mcp,
    /// GitHub Copilot global instructions (.github/copilot-instructions.md)
    Copilot,
    /// GitHub Copilot scoped instructions (.github/instructions/*.instructions.md)
    CopilotScoped,
    /// GitHub Copilot custom agents (.github/agents/*.agent.md)
    CopilotAgent,
    /// GitHub Copilot reusable prompts (.github/prompts/*.prompt.md)
    CopilotPrompt,
    /// GitHub Copilot hook files (.github/hooks/*.json, ~/.copilot/hooks/*.json),
    /// Copilot settings with inline hooks (.github/copilot/settings.json) and
    /// the setup workflow (.github/workflows/copilot-setup-steps.yml)
    CopilotHooks,
    /// GitHub Copilot CLI plugin manifest (.github/plugin/plugin.json,
    /// .plugin/plugin.json)
    CopilotPlugin,
    /// Claude Code rules (.claude/rules/*.md)
    ClaudeRule,
    /// Claude Code output style files (.claude/output-styles/*.md)
    ClaudeOutputStyle,
    /// Claude Code MEMORY.md indexes loaded at session start: subagent memory
    /// (.claude/agent-memory/<agent>/, .claude/agent-memory-local/<agent>/)
    /// and auto memory (.claude/projects/<project>/memory/)
    ClaudeMemoryIndex,
    /// .claudeignore, which Claude Code does not read
    ClaudeIgnore,
    /// Claude Code legacy custom command files (.claude/commands/**/*.md)
    ClaudeCommand,
    /// Cursor project rules (.cursor/rules/*.md, .cursor/rules/*.mdc, including nested dirs)
    CursorRule,
    /// Cursor hooks configuration (.cursor/hooks.json)
    CursorHooks,
    /// Cursor subagent definitions (.cursor/agents/**/*.md, including nested dirs)
    CursorAgent,
    /// Cursor cloud-agent environment configuration (.cursor/environment.json)
    CursorEnvironment,
    /// Legacy Cursor rules file (.cursorrules)
    CursorRulesLegacy,
    /// Cursor CLI configuration (.cursor/cli.json, ~/.cursor/cli-config.json)
    CursorCliConfig,
    /// Cursor allowlists (.cursor/permissions.json, ~/.cursor/permissions.json)
    CursorPermissions,
    /// Cursor plugin manifests (.cursor-plugin/plugin.json, .cursor-plugin/marketplace.json)
    CursorPlugin,
    /// Cursor ignore file (.cursorignore)
    CursorIgnore,
    /// Cline rules single file (.clinerules)
    ClineRules,
    /// Cline rules folder files (.clinerules/*.md, .clinerules/*.txt), and the
    /// workflows and hooks of the .clinerules/ and .cline/ layouts
    ClineRulesFolder,
    /// Cline ignore file (.clineignore)
    ClineIgnore,
    /// Cline MCP settings (cline_mcp_settings.json)
    ClineMcpSettings,
    /// OpenCode configuration (opencode.json, opencode.jsonc)
    OpenCodeConfig,
    /// OpenCode Markdown agents (.opencode/agents/**/*.md and the legacy
    /// agent/, modes/, mode/ directories; also ~/.config/opencode/agents/)
    OpenCodeAgent,
    /// OpenCode Markdown commands (.opencode/commands/**/*.md and the legacy
    /// command/ directory; also ~/.config/opencode/commands/)
    OpenCodeCommand,
    /// OpenCode custom tool modules (.opencode/tools/*.ts|js and the legacy
    /// tool/ directory; also ~/.config/opencode/tools/)
    OpenCodeTool,
    /// OpenCode plugin modules (.opencode/plugins/*.ts|js and the legacy
    /// plugin/ directory; also ~/.config/opencode/plugins/)
    OpenCodePlugin,
    /// Gemini CLI instruction files (GEMINI.md, GEMINI.local.md)
    GeminiMd,
    /// Gemini CLI settings (.gemini/settings.json)
    GeminiSettings,
    /// Amp settings (.amp/settings.json(c), ~/.config/amp/settings.json(c),
    /// ampcode/managed-settings.json)
    AmpSettings,
    /// Amp skill MCP servers (`mcp.json` in a skill directory below an Amp
    /// skills root, such as `.agents/skills/<skill>/mcp.json`)
    AmpSkillMcp,
    /// Gemini CLI extension manifest (gemini-extension.json)
    GeminiExtension,
    /// Gemini CLI ignore file (.geminiignore)
    GeminiIgnore,
    /// Gemini CLI extension hooks (`<extension>/hooks/hooks.json`: under
    /// `.gemini/extensions/<name>/`, or next to a `gemini-extension.json`)
    GeminiHooks,
    /// Gemini CLI custom command (`.gemini/commands/**/*.toml`, user and
    /// project, and an extension's `commands/**/*.toml`)
    GeminiCommand,
    /// Gemini CLI policy file (`.gemini/policies/*.toml`, an extension's
    /// `policies/*.toml`, and the admin policy directories)
    GeminiPolicy,
    /// Codex CLI configuration (.codex/config.toml, .codex/config.json, .codex/config.yaml/.yml)
    CodexConfig,
    /// Codex CLI plugin manifest (.codex-plugin/plugin.json)
    CodexPlugin,
    /// Codex CLI managed requirements (system `codex/requirements.toml` -
    /// `/etc/codex/` on Unix, `%ProgramData%\OpenAI\Codex\` on Windows)
    CodexRequirements,
    /// Codex CLI lifecycle hooks file (.codex/hooks.json, ~/.codex/hooks.json)
    CodexHooks,
    /// Codex CLI custom agent files (.codex/agents/*.toml, ~/.codex/agents/*.toml)
    CodexAgent,
    /// Codex CLI command rules (.codex/rules/*.rules, ~/.codex/rules/*.rules)
    CodexRules,
    /// Codex CLI custom prompts, deprecated in favor of skills
    /// (~/.codex/prompts/*.md)
    CodexPrompt,
    /// Codex skill interface metadata (<skill>/agents/openai.yaml)
    CodexSkillMetadata,
    /// Windsurf (Devin Desktop) workspace rule files (.devin/rules/**/*.md,
    /// legacy .windsurf/rules/**/*.md)
    WindsurfRule,
    /// Windsurf (Devin Desktop) workflow files (.devin/workflows/**/*.md,
    /// legacy .windsurf/workflows/**/*.md, ~/.codeium/windsurf/global_workflows/*.md)
    WindsurfWorkflow,
    /// Legacy Windsurf rules file (.windsurfrules)
    WindsurfRulesLegacy,
    /// Devin Desktop global rules (~/.codeium/windsurf/memories/global_rules.md)
    WindsurfGlobalRules,
    /// Cascade hooks (.devin/hooks.json, legacy .windsurf/hooks.json,
    /// ~/.codeium/windsurf/hooks.json, ~/.codeium/hooks.json, system hooks.json)
    WindsurfHooks,
    /// Kiro steering files (.kiro/steering/**/*.md)
    KiroSteering,
    /// Kiro legacy power manifest (POWER.md at a power root: installed powers
    /// under .kiro/powers and published powers anywhere)
    KiroPower,
    /// Kiro power Agent Plugins manifest (plugin.json at a power root:
    /// .kiro/powers/<name>/, .kiro/powers/installed/<name>/, power/,
    /// powers/<name>/)
    KiroPowerPlugin,
    /// Kiro power documents read raw by the agent (steering/*.md,
    /// dev.kiro/steering/*.md, dev.kiro/INSTRUCTIONS.md in a power)
    KiroPowerSteering,
    /// Kiro custom agent definitions (.kiro/agents/**/*.json or *.md, and the
    /// CLI 2.x fallback .amazonq/cli-agents/*.json)
    KiroAgent,
    /// Kiro IDE hook files (.kiro/hooks/*.kiro.hook)
    KiroHook,
    /// Kiro V3 standalone hook files (.kiro/hooks/*.json)
    KiroHookV3,
    /// Kiro MCP configuration (.kiro/settings/mcp.json, and the mcp.json of a
    /// power: legacy Kiro format or Agent Plugins format)
    KiroMcp,
    /// Kiro CLI settings (.kiro/settings/cli.json or ~/.kiro/settings/cli.json)
    KiroSettings,
    /// Kiro capability policy (settings/permissions.{yaml,json},
    /// workspace-roots/<hash>/permissions.{yaml,json}, managed-settings.json)
    KiroPermissions,
    /// Kiro workflow recipes (.kiro/workflows/*.workflow.json, *.workflow.yaml,
    /// *.workflow.yml; also ~/.kiro/workflows/)
    KiroWorkflow,
    /// Kiro ignore files (.kiroignore in the workspace root or a subdirectory,
    /// ~/.kiro/settings/kiroignore)
    KiroIgnore,
    /// File in a Kiro configuration location that no Kiro engine loads, such
    /// as .kiro/agents/reviewer.JSON or .kiro/settings/permissions.yml
    KiroNotLoaded,
    /// Gemini CLI agent definition (.gemini/agents/*.md)
    GeminiAgent,
    /// Other .md files (for XML/import checks)
    GenericMarkdown,
    /// Skip validation
    Unknown,
}

impl FileType {
    /// Returns `true` if this file type should be validated.
    ///
    /// This is the inverse of checking for [`FileType::Unknown`] and should
    /// be preferred over `file_type != FileType::Unknown` for clarity.
    #[must_use]
    pub fn is_validatable(self) -> bool {
        !matches!(self, FileType::Unknown)
    }

    /// Returns `true` for catch-all file types (e.g. `GenericMarkdown`) that
    /// are not specifically identified agent configuration files.
    ///
    /// The LSP uses this to skip validation for files that are only
    /// speculatively classified - arbitrary `.md` files that do not match any
    /// known agent file pattern. The CLI still validates these during explicit
    /// project scans.
    #[must_use]
    pub fn is_generic(self) -> bool {
        matches!(self, FileType::GenericMarkdown)
    }
}

impl fmt::Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            FileType::PiConfig => "PiConfig",
            FileType::PiPrompt => "PiPrompt",
            FileType::AntigravityConfig => "AntigravityConfig",
            FileType::AntigravityRule => "AntigravityRule",
            FileType::AntigravityAgent => "AntigravityAgent",
            FileType::GrokConfig => "GrokConfig",
            FileType::GrokRule => "GrokRule",
            FileType::GrokAgent => "GrokAgent",
            FileType::Skill => "Skill",
            FileType::ClaudeMd => "ClaudeMd",
            FileType::Agent => "Agent",
            FileType::AmpCheck => "AmpCheck",
            FileType::Hooks => "Hooks",
            FileType::Plugin => "Plugin",
            FileType::ClaudeMarketplace => "ClaudeMarketplace",
            FileType::Mcp => "Mcp",
            FileType::Copilot => "Copilot",
            FileType::CopilotScoped => "CopilotScoped",
            FileType::CopilotAgent => "CopilotAgent",
            FileType::CopilotPrompt => "CopilotPrompt",
            FileType::CopilotHooks => "CopilotHooks",
            FileType::CopilotPlugin => "CopilotPlugin",
            FileType::ClaudeRule => "ClaudeRule",
            FileType::ClaudeOutputStyle => "ClaudeOutputStyle",
            FileType::ClaudeMemoryIndex => "ClaudeMemoryIndex",
            FileType::ClaudeIgnore => "ClaudeIgnore",
            FileType::ClaudeCommand => "ClaudeCommand",
            FileType::CursorRule => "CursorRule",
            FileType::CursorHooks => "CursorHooks",
            FileType::CursorAgent => "CursorAgent",
            FileType::CursorEnvironment => "CursorEnvironment",
            FileType::CursorRulesLegacy => "CursorRulesLegacy",
            FileType::CursorCliConfig => "CursorCliConfig",
            FileType::CursorPermissions => "CursorPermissions",
            FileType::CursorPlugin => "CursorPlugin",
            FileType::CursorIgnore => "CursorIgnore",
            FileType::ClineRules => "ClineRules",
            FileType::ClineRulesFolder => "ClineRulesFolder",
            FileType::ClineIgnore => "ClineIgnore",
            FileType::ClineMcpSettings => "ClineMcpSettings",
            FileType::OpenCodeConfig => "OpenCodeConfig",
            FileType::OpenCodeAgent => "OpenCodeAgent",
            FileType::OpenCodeCommand => "OpenCodeCommand",
            FileType::OpenCodeTool => "OpenCodeTool",
            FileType::OpenCodePlugin => "OpenCodePlugin",
            FileType::GeminiMd => "GeminiMd",
            FileType::GeminiSettings => "GeminiSettings",
            FileType::AmpSettings => "AmpSettings",
            FileType::AmpSkillMcp => "AmpSkillMcp",
            FileType::GeminiExtension => "GeminiExtension",
            FileType::GeminiIgnore => "GeminiIgnore",
            FileType::GeminiHooks => "GeminiHooks",
            FileType::GeminiCommand => "GeminiCommand",
            FileType::GeminiPolicy => "GeminiPolicy",
            FileType::CodexConfig => "CodexConfig",
            FileType::CodexPlugin => "CodexPlugin",
            FileType::CodexRequirements => "CodexRequirements",
            FileType::CodexHooks => "CodexHooks",
            FileType::CodexAgent => "CodexAgent",
            FileType::CodexRules => "CodexRules",
            FileType::CodexPrompt => "CodexPrompt",
            FileType::CodexSkillMetadata => "CodexSkillMetadata",
            FileType::WindsurfRule => "WindsurfRule",
            FileType::WindsurfWorkflow => "WindsurfWorkflow",
            FileType::WindsurfRulesLegacy => "WindsurfRulesLegacy",
            FileType::WindsurfGlobalRules => "WindsurfGlobalRules",
            FileType::WindsurfHooks => "WindsurfHooks",
            FileType::KiroSteering => "KiroSteering",
            FileType::KiroPower => "KiroPower",
            FileType::KiroPowerPlugin => "KiroPowerPlugin",
            FileType::KiroPowerSteering => "KiroPowerSteering",
            FileType::KiroAgent => "KiroAgent",
            FileType::KiroHook => "KiroHook",
            FileType::KiroHookV3 => "KiroHookV3",
            FileType::KiroMcp => "KiroMcp",
            FileType::KiroSettings => "KiroSettings",
            FileType::KiroPermissions => "KiroPermissions",
            FileType::KiroWorkflow => "KiroWorkflow",
            FileType::KiroIgnore => "KiroIgnore",
            FileType::KiroNotLoaded => "KiroNotLoaded",
            FileType::GeminiAgent => "GeminiAgent",
            FileType::GenericMarkdown => "GenericMarkdown",
            FileType::Unknown => "Unknown",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All variants must round-trip through Display.
    #[test]
    fn display_all_variants() {
        let variants = [
            (FileType::Skill, "Skill"),
            (FileType::ClaudeMd, "ClaudeMd"),
            (FileType::Agent, "Agent"),
            (FileType::AmpCheck, "AmpCheck"),
            (FileType::Hooks, "Hooks"),
            (FileType::Plugin, "Plugin"),
            (FileType::ClaudeMarketplace, "ClaudeMarketplace"),
            (FileType::Mcp, "Mcp"),
            (FileType::Copilot, "Copilot"),
            (FileType::CopilotScoped, "CopilotScoped"),
            (FileType::CopilotAgent, "CopilotAgent"),
            (FileType::CopilotPrompt, "CopilotPrompt"),
            (FileType::CopilotHooks, "CopilotHooks"),
            (FileType::CopilotPlugin, "CopilotPlugin"),
            (FileType::ClaudeRule, "ClaudeRule"),
            (FileType::ClaudeOutputStyle, "ClaudeOutputStyle"),
            (FileType::ClaudeMemoryIndex, "ClaudeMemoryIndex"),
            (FileType::ClaudeIgnore, "ClaudeIgnore"),
            (FileType::ClaudeCommand, "ClaudeCommand"),
            (FileType::CursorRule, "CursorRule"),
            (FileType::CursorHooks, "CursorHooks"),
            (FileType::CursorAgent, "CursorAgent"),
            (FileType::CursorEnvironment, "CursorEnvironment"),
            (FileType::CursorRulesLegacy, "CursorRulesLegacy"),
            (FileType::CursorCliConfig, "CursorCliConfig"),
            (FileType::CursorPermissions, "CursorPermissions"),
            (FileType::CursorPlugin, "CursorPlugin"),
            (FileType::CursorIgnore, "CursorIgnore"),
            (FileType::ClineRules, "ClineRules"),
            (FileType::ClineRulesFolder, "ClineRulesFolder"),
            (FileType::ClineIgnore, "ClineIgnore"),
            (FileType::ClineMcpSettings, "ClineMcpSettings"),
            (FileType::OpenCodeConfig, "OpenCodeConfig"),
            (FileType::OpenCodeAgent, "OpenCodeAgent"),
            (FileType::OpenCodeCommand, "OpenCodeCommand"),
            (FileType::OpenCodeTool, "OpenCodeTool"),
            (FileType::OpenCodePlugin, "OpenCodePlugin"),
            (FileType::GeminiMd, "GeminiMd"),
            (FileType::GeminiSettings, "GeminiSettings"),
            (FileType::AmpSettings, "AmpSettings"),
            (FileType::AmpSkillMcp, "AmpSkillMcp"),
            (FileType::GeminiExtension, "GeminiExtension"),
            (FileType::GeminiIgnore, "GeminiIgnore"),
            (FileType::GeminiHooks, "GeminiHooks"),
            (FileType::GeminiCommand, "GeminiCommand"),
            (FileType::GeminiPolicy, "GeminiPolicy"),
            (FileType::CodexConfig, "CodexConfig"),
            (FileType::CodexPlugin, "CodexPlugin"),
            (FileType::CodexRequirements, "CodexRequirements"),
            (FileType::CodexHooks, "CodexHooks"),
            (FileType::CodexAgent, "CodexAgent"),
            (FileType::CodexRules, "CodexRules"),
            (FileType::CodexPrompt, "CodexPrompt"),
            (FileType::CodexSkillMetadata, "CodexSkillMetadata"),
            (FileType::WindsurfRule, "WindsurfRule"),
            (FileType::WindsurfWorkflow, "WindsurfWorkflow"),
            (FileType::WindsurfRulesLegacy, "WindsurfRulesLegacy"),
            (FileType::WindsurfGlobalRules, "WindsurfGlobalRules"),
            (FileType::WindsurfHooks, "WindsurfHooks"),
            (FileType::KiroSteering, "KiroSteering"),
            (FileType::KiroPower, "KiroPower"),
            (FileType::KiroPowerPlugin, "KiroPowerPlugin"),
            (FileType::KiroPowerSteering, "KiroPowerSteering"),
            (FileType::KiroAgent, "KiroAgent"),
            (FileType::KiroHook, "KiroHook"),
            (FileType::KiroHookV3, "KiroHookV3"),
            (FileType::KiroMcp, "KiroMcp"),
            (FileType::KiroSettings, "KiroSettings"),
            (FileType::KiroPermissions, "KiroPermissions"),
            (FileType::KiroWorkflow, "KiroWorkflow"),
            (FileType::KiroIgnore, "KiroIgnore"),
            (FileType::KiroNotLoaded, "KiroNotLoaded"),
            (FileType::GeminiAgent, "GeminiAgent"),
            (FileType::GenericMarkdown, "GenericMarkdown"),
            (FileType::Unknown, "Unknown"),
        ];

        for (variant, expected) in &variants {
            assert_eq!(variant.to_string(), *expected);
        }
    }

    /// `is_validatable` returns true for all variants except Unknown.
    #[test]
    fn is_validatable_all_variants() {
        let validatable = [
            FileType::Skill,
            FileType::ClaudeMd,
            FileType::Agent,
            FileType::AmpCheck,
            FileType::Hooks,
            FileType::Plugin,
            FileType::ClaudeMarketplace,
            FileType::Mcp,
            FileType::Copilot,
            FileType::CopilotScoped,
            FileType::CopilotAgent,
            FileType::CopilotPrompt,
            FileType::CopilotHooks,
            FileType::CopilotPlugin,
            FileType::ClaudeRule,
            FileType::ClaudeOutputStyle,
            FileType::ClaudeCommand,
            FileType::CursorRule,
            FileType::CursorHooks,
            FileType::CursorAgent,
            FileType::CursorEnvironment,
            FileType::CursorRulesLegacy,
            FileType::CursorCliConfig,
            FileType::CursorPermissions,
            FileType::CursorPlugin,
            FileType::CursorIgnore,
            FileType::ClineRules,
            FileType::ClineRulesFolder,
            FileType::ClineIgnore,
            FileType::ClineMcpSettings,
            FileType::OpenCodeConfig,
            FileType::OpenCodeAgent,
            FileType::OpenCodeCommand,
            FileType::OpenCodeTool,
            FileType::OpenCodePlugin,
            FileType::GeminiMd,
            FileType::GeminiSettings,
            FileType::AmpSettings,
            FileType::AmpSkillMcp,
            FileType::GeminiExtension,
            FileType::GeminiIgnore,
            FileType::GeminiHooks,
            FileType::GeminiCommand,
            FileType::GeminiPolicy,
            FileType::CodexConfig,
            FileType::CodexPlugin,
            FileType::CodexRequirements,
            FileType::CodexHooks,
            FileType::CodexAgent,
            FileType::CodexRules,
            FileType::CodexPrompt,
            FileType::CodexSkillMetadata,
            FileType::WindsurfRule,
            FileType::WindsurfWorkflow,
            FileType::WindsurfRulesLegacy,
            FileType::WindsurfGlobalRules,
            FileType::WindsurfHooks,
            FileType::KiroSteering,
            FileType::KiroPower,
            FileType::KiroPowerPlugin,
            FileType::KiroPowerSteering,
            FileType::KiroAgent,
            FileType::KiroHook,
            FileType::KiroHookV3,
            FileType::KiroMcp,
            FileType::KiroSettings,
            FileType::KiroPermissions,
            FileType::KiroWorkflow,
            FileType::KiroIgnore,
            FileType::KiroNotLoaded,
            FileType::GeminiAgent,
            FileType::GenericMarkdown,
        ];

        for variant in &validatable {
            assert!(
                variant.is_validatable(),
                "{} should be validatable",
                variant
            );
        }

        assert!(
            !FileType::Unknown.is_validatable(),
            "Unknown should not be validatable"
        );
    }

    /// `is_generic` returns true only for GenericMarkdown.
    #[test]
    fn is_generic_only_for_generic_markdown() {
        assert!(
            FileType::GenericMarkdown.is_generic(),
            "GenericMarkdown should be generic"
        );

        // Exhaustively verify all other variants are NOT generic
        let non_generic = [
            FileType::Skill,
            FileType::ClaudeMd,
            FileType::Agent,
            FileType::AmpCheck,
            FileType::Hooks,
            FileType::Plugin,
            FileType::ClaudeMarketplace,
            FileType::Mcp,
            FileType::Copilot,
            FileType::CopilotScoped,
            FileType::CopilotAgent,
            FileType::CopilotPrompt,
            FileType::CopilotHooks,
            FileType::CopilotPlugin,
            FileType::ClaudeRule,
            FileType::ClaudeOutputStyle,
            FileType::ClaudeCommand,
            FileType::CursorRule,
            FileType::CursorHooks,
            FileType::CursorAgent,
            FileType::CursorEnvironment,
            FileType::CursorRulesLegacy,
            FileType::CursorCliConfig,
            FileType::CursorPermissions,
            FileType::CursorPlugin,
            FileType::CursorIgnore,
            FileType::ClineRules,
            FileType::ClineRulesFolder,
            FileType::ClineIgnore,
            FileType::ClineMcpSettings,
            FileType::OpenCodeConfig,
            FileType::OpenCodeAgent,
            FileType::OpenCodeCommand,
            FileType::OpenCodeTool,
            FileType::OpenCodePlugin,
            FileType::GeminiMd,
            FileType::GeminiSettings,
            FileType::AmpSettings,
            FileType::AmpSkillMcp,
            FileType::GeminiExtension,
            FileType::GeminiIgnore,
            FileType::GeminiHooks,
            FileType::GeminiCommand,
            FileType::GeminiPolicy,
            FileType::CodexConfig,
            FileType::CodexPlugin,
            FileType::CodexRequirements,
            FileType::CodexHooks,
            FileType::CodexAgent,
            FileType::CodexRules,
            FileType::CodexPrompt,
            FileType::CodexSkillMetadata,
            FileType::WindsurfRule,
            FileType::WindsurfWorkflow,
            FileType::WindsurfRulesLegacy,
            FileType::WindsurfGlobalRules,
            FileType::WindsurfHooks,
            FileType::KiroSteering,
            FileType::KiroPower,
            FileType::KiroPowerPlugin,
            FileType::KiroPowerSteering,
            FileType::KiroAgent,
            FileType::KiroHook,
            FileType::KiroHookV3,
            FileType::KiroMcp,
            FileType::KiroSettings,
            FileType::KiroPermissions,
            FileType::KiroWorkflow,
            FileType::KiroIgnore,
            FileType::KiroNotLoaded,
            FileType::GeminiAgent,
            FileType::Unknown,
        ];

        for variant in &non_generic {
            assert!(!variant.is_generic(), "{} should not be generic", variant);
        }
    }

    /// FileType must be usable as a HashMap key (requires Hash + Eq).
    #[test]
    fn usable_as_hashmap_key() {
        use std::collections::HashMap;

        let mut map = HashMap::new();
        map.insert(FileType::Skill, "skill");
        map.insert(FileType::Unknown, "unknown");

        assert_eq!(map.get(&FileType::Skill), Some(&"skill"));
        assert_eq!(map.get(&FileType::Unknown), Some(&"unknown"));
    }

    /// FileType is Copy (no move semantics).
    #[test]
    fn file_type_is_copy() {
        let a = FileType::Skill;
        let b = a; // Copy
        assert_eq!(a, b); // `a` is still usable
    }
}
