/**
 * Glob patterns for the agent configuration files agnix lints.
 *
 * The patterns follow the path detection in
 * `crates/agnix-core/src/file_types/detection.rs` (`detect_file_type` and
 * `refine_file_type`): every file type other than `GenericMarkdown` and
 * `Unknown` is matched by at least one pattern. Where detection treats a name
 * as an agent file anywhere (`plugin.json`, `mcp.json`, `hooks/hooks.json`,
 * `agents/*.md`), one pattern covers every tool that uses that name, so the
 * tool-specific locations below it (`.codex-plugin/plugin.json`,
 * `.cursor/mcp.json`, `.gemini/agents/*.md`, ...) are not listed again.
 *
 * VS Code matches document filter patterns against the absolute path, so the
 * user and system locations (`~/.codeium/...`, `/etc/...`) are written with a
 * leading `**` like the workspace ones. Patterns use only the glob syntax that
 * VS Code document filters, file system watchers and file search all accept:
 * `**` as a whole path segment, `*`, and `{a,b}` groups without nesting.
 *
 * This module does not import `vscode`, so the unit tests can load it.
 */

/** Directories below which OpenCode loads agents, commands, tools and plugins. */
const OPENCODE_ROOTS =
  '{.opencode,.config/opencode,etc/opencode,Application Support/opencode,ProgramData/opencode}';

/** Module extensions OpenCode loads as custom tools and plugins. */
const JS_MODULE_EXTENSIONS = '{ts,js,mts,mjs,cts,cjs}';

/**
 * Files agnix recognises from their path alone. These drive the
 * `workspaceContains` activation events in package.json (a unit test keeps
 * both lists identical), the file system watchers and workspace validation.
 */
export const AGNIX_FILE_PATTERNS: readonly string[] = [
  // Agent Skills (any tool) and Claude Code / AGENTS.md memory files
  '**/SKILL.md',
  '**/CLAUDE.md',
  '**/CLAUDE.local.md',
  '**/AGENTS.md',
  '**/AGENTS.local.md',
  '**/AGENTS.override.md',

  // Claude Code
  '**/.claude/settings.json',
  '**/.claude/settings.local.json',
  '**/.claude/managed-settings.json',
  '**/{Library/Application Support/ClaudeCode,etc/claude-code,Program Files/ClaudeCode}/managed-settings.json',
  '**/{Library/Application Support/ClaudeCode,etc/claude-code,Program Files/ClaudeCode}/managed-settings.d/*.json',
  // Plugin hooks; also Gemini CLI extension hooks
  '**/hooks/hooks.json',
  // Claude Code, Codex, Copilot, Cursor and Kiro power plugin manifests
  '**/plugin.json',
  '**/.claude-plugin/marketplace.json',
  // Claude Code subagents; also Gemini CLI agents (.gemini/agents/,
  // .gemini/extensions/<name>/agents/)
  '**/agents/*.md',
  '**/agents/*/*.md',
  '**/.claude/rules/**/*.md',
  '**/.claude/commands/**/*.md',
  '**/.claude/output-styles/*.md',
  '**/.claude/{agent-memory,agent-memory-local}/*/MEMORY.md',
  '**/.claude/projects/*/memory/MEMORY.md',
  '**/.claudeignore',

  // MCP: any mcp.json (.cursor/mcp.json, .kiro/settings/mcp.json, Amp skill
  // mcp.json), *.mcp.json (.mcp.json) and mcp-*.json
  '**/mcp.json',
  '**/*.mcp.json',
  '**/mcp-*.json',

  // GitHub Copilot
  '**/.github/copilot-instructions.md',
  '**/.github/instructions/**/*.instructions.md',
  '**/{.github,.copilot}/agents/**/*.md',
  '**/*.agent.md',
  '**/*.chatmode.md',
  '**/.github/prompts/*.prompt.md',
  '**/{.github,.copilot}/hooks/*.json',
  '**/{.github/copilot,.copilot}/{settings.json,settings.local.json}',
  '**/.github/workflows/copilot-setup-steps.{yml,yaml}',

  // Cursor
  '**/.cursor/rules/**/*.{md,mdc}',
  '**/.cursor/agents/**/*.md',
  '**/.cursor/{hooks.json,environment.json,cli.json,cli-config.json,permissions.json}',
  '**/.cursor-plugin/marketplace.json',
  '**/.cursorignore',
  '**/{.cursorrules,.cursorrules.md}',

  // Cline
  '**/.clinerules',
  '**/.clinerules/**/*.{md,txt}',
  '**/.cline/{rules,workflows}/**/*.md',
  '**/{.clinerules,.cline}/hooks/**',
  '**/.clineignore',
  '**/cline_mcp_settings.json',

  // OpenCode
  '**/opencode.{json,jsonc}',
  `**/${OPENCODE_ROOTS}/{agents,agent,modes,mode}/**/*.md`,
  `**/${OPENCODE_ROOTS}/{commands,command}/**/*.md`,
  `**/${OPENCODE_ROOTS}/{tools,tool,plugins,plugin}/**/*.${JS_MODULE_EXTENSIONS}`,

  // Gemini CLI
  '**/{GEMINI.md,GEMINI.local.md}',
  '**/.gemini/{settings.json,settings.local.json}',
  '**/gemini-extension.json',
  '**/.geminiignore',
  '**/.gemini/commands/**/*.toml',
  '**/.gemini/extensions/*/commands/**/*.toml',
  '**/.gemini/policies/*.toml',
  '**/.gemini/extensions/*/policies/*.toml',
  '**/{etc/gemini-cli,Library/Application Support/GeminiCli,ProgramData/gemini-cli}/policies/*.toml',

  // Amp (skill mcp.json is covered by **/mcp.json)
  '**/.agents/checks/**/*.md',
  '**/.config/{agents,amp}/checks/**/*.md',
  '**/{.amp,.config/amp}/settings.{json,jsonc}',
  '**/ampcode/managed-settings.json',

  // Codex CLI (.codex-plugin/plugin.json is covered by **/plugin.json)
  '**/.codex/config.{toml,json,yaml,yml}',
  '**/.codex/hooks.json',
  '**/.codex/agents/*.toml',
  '**/{.codex,codex,Codex}/rules/*.rules',
  '**/{codex,Codex}/requirements.toml',
  '**/.codex/prompts/**/*.md',
  '**/skills/**/*/agents/openai.yaml',

  // Windsurf (Devin Desktop)
  '**/{.devin,.windsurf}/rules/**/*.md',
  '**/{.devin,.windsurf}/workflows/**/*.md',
  '**/.codeium/windsurf/global_workflows/*.md',
  '**/.codeium/windsurf/memories/global_rules.md',
  '**/.windsurfrules',
  '**/{.devin,.windsurf}/hooks.json',
  '**/.codeium/{hooks.json,windsurf/hooks.json}',
  '**/{Library/Application Support/Devin,Library/Application Support/Windsurf,etc/devin,etc/windsurf,ProgramData/Devin,ProgramData/Windsurf}/hooks.json',

  // Kiro, including the near misses Kiro never loads (KIRO-026)
  '**/.kiro/steering/**/*.md',
  '**/.kiro/agents/**/*.{json,md,jsonc,json5}',
  '**/.amazonq/cli-agents/*.json',
  '**/.kiro/hooks/**/*.{json,kiro.hook,jsonc,json5,yaml,yml}',
  '**/.kiro/settings/{mcp.json,cli.json,permissions.yaml,permissions.json,kiroignore,permissions.yml,mcp.jsonc,cli.jsonc}',
  '**/.kiro/workspace-roots/*/{permissions.yaml,permissions.json,permissions.yml}',
  '**/.kiro/{settings.json,cli.json,permissions.yaml,permissions.yml,permissions.json}',
  '**/{Library/Application Support/Kiro,etc/kiro,ProgramData/Kiro}/managed-settings.json',
  '**/.kiro/workflows/**/*.{json,yaml,yml}',
  '**/.kiroignore',
  '**/POWER.md',
  '**/{power,powers/*,powers/installed/*}/steering/*.md',
  '**/dev.kiro/steering/*.md',
  '**/dev.kiro/INSTRUCTIONS.md',
];

/**
 * Files agnix recognises only with a sibling-file lookup: the commands and
 * policies of a Gemini CLI extension outside `.gemini/extensions/`, which
 * count when the extension root holds a `gemini-extension.json`. They are too
 * broad to activate the extension on their own; the `gemini-extension.json`
 * activation event covers them.
 */
export const AGNIX_SIBLING_DETECTED_PATTERNS: readonly string[] = [
  '**/commands/**/*.toml',
  '**/policies/*.toml',
];

/** Patterns for file system watchers and workspace validation. */
export const AGNIX_WATCH_PATTERNS: readonly string[] = [
  ...AGNIX_FILE_PATTERNS,
  ...AGNIX_SIBLING_DETECTED_PATTERNS,
];

/** A document filter that is structurally a `vscode.DocumentFilter`. */
export interface AgnixDocumentFilter {
  scheme: string;
  language?: string;
  pattern?: string;
}

/**
 * Documents sent to agnix-lsp. Markdown and JSON are selected by language, as
 * before, so `files.includeAsMemory` and `files.includeAsGeneric` globs work;
 * the LSP skips Markdown that is not an agent file. Every watch pattern is
 * selected by path as well, which covers TOML, YAML, JSONC, TypeScript,
 * JavaScript, `.rules`, ignore files and extensionless hook scripts whatever
 * language mode VS Code gives them.
 */
export const AGNIX_DOCUMENT_SELECTOR: AgnixDocumentFilter[] = [
  { scheme: 'file', language: 'markdown' },
  { scheme: 'file', language: 'skill-markdown' },
  { scheme: 'file', language: 'json' },
  { scheme: 'file', pattern: '**/*.mdc' },
  ...AGNIX_WATCH_PATTERNS.map((pattern) => ({ scheme: 'file', pattern })),
];
