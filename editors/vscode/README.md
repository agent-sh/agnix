# agnix - Agent Config Linter

Real-time validation for AI agent configuration files in VS Code.
Install from the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=avifenesh.agnix).

**757 rules** | **Real-time diagnostics** | **Auto-fix** | **Completion** | **Multi-tool support**


## Features

- **Real-time validation** - Diagnostics as you type
- **Context-aware completions** - Frontmatter keys, values, and snippets
- **JSON Schema validation and autocomplete for `.agnix.toml` config files**
- **Validates 757 rules** - From official specs and best practices

- **Diagnostics panel** - Sidebar tree view of all issues by file
- **CodeLens** - Rule info shown inline above problematic lines
- **Quick-fix preview** - See diff before applying fixes
- **Safe fixes** - Apply only high-confidence fixes automatically
- **Ignore rules** - Disable rules directly from the editor
- **Multi-tool** - Claude Code, Codex CLI, OpenCode, Kiro, GitHub Copilot, Cline, Cursor, Amp, Gemini CLI, Windsurf (Devin Desktop)

![Validation in action](assets/vscode-validation.png)

## Supported File Types

The extension starts when the workspace contains one of these files and sends them to agnix-lsp in any language mode. User-level and system files (for example `~/.codex/hooks.json` or `/etc/gemini-cli/policies/*.toml`) are validated when opened.

| Tool | Files |
|------|-------|
| All | `SKILL.md`, `AGENTS.md` (`AGENTS.local.md`, `AGENTS.override.md`), `mcp.json`, `*.mcp.json`, `mcp-*.json` |
| Claude Code | `CLAUDE.md`, `CLAUDE.local.md`, `.claude/settings.json`, `.claude/settings.local.json`, managed settings, `hooks/hooks.json`, `plugin.json`, `.claude-plugin/marketplace.json`, `agents/*.md`, `.claude/rules/**/*.md`, `.claude/commands/**/*.md`, `.claude/output-styles/*.md`, `MEMORY.md` indexes, `.claudeignore` |
| Codex CLI | `.codex/config.toml` (`.json`, `.yaml`), `.codex/hooks.json`, `.codex/agents/*.toml`, `.codex/rules/*.rules`, `.codex/prompts/*.md`, `codex/requirements.toml`, `.codex-plugin/plugin.json`, `skills/<skill>/agents/openai.yaml` |
| OpenCode | `opencode.json`, `opencode.jsonc`, `.opencode/agents/**/*.md`, `.opencode/commands/**/*.md`, `.opencode/tools/*.ts` (`.js`), `.opencode/plugins/*.ts` (`.js`), including the singular legacy directories and `~/.config/opencode/` |
| Kiro | `.kiro/steering/**/*.md`, `.kiro/agents/**/*.json` (`.md`), `.kiro/hooks/*.json`, `.kiro/hooks/*.kiro.hook`, `.kiro/settings/mcp.json`, `.kiro/settings/cli.json`, `permissions.yaml`, `.kiro/workflows/*.workflow.json` (`.yaml`, `.yml`), `.kiroignore`, `POWER.md` and power files, `.amazonq/cli-agents/*.json` |
| GitHub Copilot | `.github/copilot-instructions.md`, `.github/instructions/**/*.instructions.md`, `.github/agents/**/*.md`, `*.agent.md`, `*.chatmode.md`, `.github/prompts/*.prompt.md`, `.github/hooks/*.json`, `.github/copilot/settings.json`, `.github/plugin/plugin.json`, `copilot-setup-steps.yml` |
| Cline | `.clinerules` (file or folder, with workflows and hooks), `.cline/rules/`, `.cline/workflows/`, `.cline/hooks/`, `.clineignore`, `cline_mcp_settings.json` |
| Cursor | `.cursor/rules/**/*.mdc` (`.md`), `.cursor/agents/**/*.md`, `.cursor/hooks.json`, `.cursor/mcp.json`, `.cursor/environment.json`, `.cursor/cli.json`, `~/.cursor/cli-config.json`, `.cursor/permissions.json`, `.cursor-plugin/plugin.json`, `.cursor-plugin/marketplace.json`, `.cursorignore`, `.cursorrules` |
| Amp | `.amp/settings.json` (`.jsonc`), `~/.config/amp/settings.json` (`.jsonc`), managed settings, `.agents/checks/**/*.md`, skill `mcp.json` |
| Gemini CLI | `GEMINI.md`, `.gemini/settings.json`, `.gemini/agents/*.md`, `.gemini/commands/**/*.toml`, `.gemini/policies/*.toml`, `gemini-extension.json`, `.geminiignore`, and an extension's `commands/`, `policies/`, `agents/` and `hooks/hooks.json` |
| Windsurf (Devin Desktop) | `.devin/rules/**/*.md`, `.devin/workflows/**/*.md`, `.devin/hooks.json` (and the legacy `.windsurf/` equivalents), `~/.codeium/windsurf/memories/global_rules.md`, global workflows, `.windsurfrules` |

## Commands

Access via Command Palette (`Ctrl+Shift+P` / `Cmd+Shift+P`):

| Command | Shortcut | Description |
|---------|----------|-------------|
| `agnix: Validate Current File` | `Ctrl+Shift+V` | Validate active file |
| `agnix: Validate Workspace` | - | Validate all agent configs including project-level rules (XP-004/005/006, VER-001) |
| `agnix: Fix All Issues in File` | `Ctrl+Shift+.` | Apply all available fixes |
| `agnix: Preview Fixes` | - | Browse fixes with diff preview |
| `agnix: Fix All Safe Issues` | `Ctrl+Alt+.` | Apply only safe fixes |
| `agnix: Show All Rules` | - | Browse 757 rules by category |

| `agnix: Show Rule Documentation` | - | Open docs for a rule (via CodeLens) |
| `agnix: Ignore Rule in Project` | - | Add rule to `.agnix.toml` disabled list |
| `agnix: Restart Language Server` | - | Restart the LSP server |
| `agnix: Show Output Channel` | - | View server logs |

## Context Menu

Right-click on agent config files to:
- Validate Current File
- Fix All Issues
- Preview Fixes (with diff)
- Fix All Safe Issues

## Requirements

The `agnix-lsp` binary is **automatically downloaded** on first use. No manual installation required.

If you prefer to install manually:

```bash
# From crates.io
cargo install agnix-lsp

# Or via Homebrew
brew tap agent-sh/agnix && brew install agnix
```

## Settings

All settings can be configured via VS Code's Settings UI or `settings.json`. Changes take effect immediately without restarting the LSP server.

### General Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `agnix.lspPath` | `agnix-lsp` | Path to LSP binary |
| `agnix.enable` | `true` | Enable/disable validation |
| `agnix.codeLens.enable` | `true` | Show CodeLens with rule info |
| `agnix.trace.server` | `off` | Server communication tracing |
| `agnix.severity` | `Warning` | Minimum severity level (Error, Warning, Info) |
| `agnix.target` | `Generic` | Target tool (deprecated, use `tools` instead) |
| `agnix.tools` | `[]` | Tools to validate for (e.g., `["claude-code", "cursor"]`) |

### Rule Categories

Enable or disable validation rule categories. All default to `true`.

| Setting | Rules | Description |
|---------|-------|-------------|
| `agnix.rules.skills` | AS-*, CC-SK-* | Skills validation |
| `agnix.rules.hooks` | CC-HK-* | Hooks configuration |
| `agnix.rules.agents` | CC-AG-* | Agent definitions |
| `agnix.rules.memory` | CC-MEM-* | Memory validation |
| `agnix.rules.plugins` | CC-PL-* | Plugin manifests |
| `agnix.rules.xml` | XML-* | XML balance checking |
| `agnix.rules.mcp` | MCP-* | MCP validation |
| `agnix.rules.imports` | REF-* | Import references |
| `agnix.rules.crossPlatform` | XP-* | Cross-platform compatibility |
| `agnix.rules.agentsMd` | AGM-* | AGENTS.md validation |
| `agnix.rules.copilot` | COP-* | GitHub Copilot instructions |
| `agnix.rules.cursor` | CUR-* | Cursor project rules |
| `agnix.rules.promptEngineering` | PE-* | Prompt engineering |
| `agnix.rules.disabledRules` | - | Specific rule IDs to disable |

### Version Pinning

Pin tool versions for version-aware validation. All default to `null` (use defaults).

| Setting | Example | Description |
|---------|---------|-------------|
| `agnix.versions.claudeCode` | `"1.0.0"` | Claude Code version |
| `agnix.versions.codex` | `"0.1.0"` | Codex CLI version |
| `agnix.versions.cursor` | `"0.45.0"` | Cursor version |
| `agnix.versions.copilot` | `"1.0.0"` | GitHub Copilot version |

### Spec Revisions

Pin specification revisions. All default to `null` (use latest).

| Setting | Example | Description |
|---------|---------|-------------|
| `agnix.specs.mcpProtocol` | `"2025-11-25"` | MCP protocol version |
| `agnix.specs.agentSkills` | `"1.0"` | Agent Skills spec revision |
| `agnix.specs.agentsMd` | `"1.0"` | AGENTS.md spec revision |

### Example settings.json

```json
{
  "agnix.severity": "Error",
  "agnix.tools": ["claude-code", "cursor"],
  "agnix.rules.promptEngineering": false,
  "agnix.rules.disabledRules": ["PE-004"],
  "agnix.versions.claudeCode": "1.0.0"
}
```

### Configuration Priority

VS Code settings take priority over `.agnix.toml`:

1. VS Code settings (highest priority)
2. `.agnix.toml` in workspace root
3. Default values (lowest priority)

## File-Based Configuration

Create `.agnix.toml` in your workspace for team-shared config:

```toml
target = "ClaudeCode"

[rules]
disabled_rules = ["PE-004"]
```

See [configuration docs](https://github.com/agent-sh/agnix/blob/main/docs/CONFIGURATION.md) for all options.

## Troubleshooting

### agnix-lsp not found

The extension automatically downloads agnix-lsp on first use. If automatic download fails:

```bash
# Manual install from crates.io
cargo install agnix-lsp

# Or specify full path in settings
"agnix.lspPath": "/path/to/agnix-lsp"
```

The auto-downloaded binary is stored in the extension's global storage directory.

### No diagnostics appearing

1. Check file type is supported (see table above)
2. Verify status bar shows "agnix" (not "agnix (error)")
3. Run `agnix: Show Output Channel` for error details

## Links

- [agnix on GitHub](https://github.com/agent-sh/agnix)
- [Validation Rules Reference](https://agent-sh.github.io/agnix/docs/rules)
- [Agent Skills Specification](https://agentskills.io)
- [Model Context Protocol](https://modelcontextprotocol.io)

## License

MIT OR Apache-2.0
