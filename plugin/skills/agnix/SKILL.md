---
name: agnix
description: "Use when user asks to 'lint agent configs', 'validate skills', 'check CLAUDE.md', 'validate hooks', 'lint MCP'. Validates agent configuration files against 455 rules across 10+ AI tools."
argument-hint: "[path] [--fix] [--strict] [--target=generic|claude-code|cursor|codex|kiro]"
allowed-tools: Bash(agnix:*), Bash(cargo:*), Read, Glob, Grep
---

# agnix

Lint agent configurations (Skills, Hooks, MCP, Memory, Plugins) before they break a workflow, across Claude Code, Codex CLI, OpenCode, Kiro, Cursor, GitHub Copilot, Cline, Gemini CLI, Windsurf, Amp and others.

Arguments, from `$ARGUMENTS`: a path (default `.`), `--fix`, `--strict`, and `--target` as `--target=X` or `--target X`. Without `--target` the CLI uses `generic`.

## Run

1. `agnix --version`. If it is missing, install with `npm install -g agnix` or `cargo install agnix-cli` (also on Homebrew and PyPI).
2. `agnix [--strict] [--target <target>] <path>`.
3. With `--fix`: `agnix --fix [--target <target>] <path>`, then run step 2 again to confirm what remains. Without `--fix`, change no files; `agnix --dry-run <path>` previews the fixes.

## CLI reference

| Command | Description |
|---------|-------------|
| `agnix .` | Validate current project |
| `agnix --fix .` | Apply safe auto-fixes |
| `agnix --dry-run .` | Show what would be fixed, change nothing |
| `agnix --strict .` | Treat warnings as errors |
| `agnix --target claude-code .` | Target one tool: `generic`, `claude-code`, `cursor`, `codex`, `kiro` |
| `agnix --watch .` | Watch mode - re-validate on changes |
| `agnix --format json .` | JSON output |
| `agnix --format sarif .` | SARIF for GitHub Code Scanning |

## Supported files

| File Type | Examples |
|-----------|----------|
| Skills | `SKILL.md` |
| Memory | `CLAUDE.md`, `AGENTS.md` |
| Hooks | `${STATE_DIR}/settings.json` |
| MCP | `*.mcp.json` |
| Cursor | `.cursor/rules/*.mdc` |
| Copilot | `.github/copilot-instructions.md` |

## Output

```
CLAUDE.md:15:1 warning: Generic instruction 'Be helpful' [fixable]
  help: Remove generic instructions. Claude already knows this.

skills/review/SKILL.md:3:1 error: Invalid name [fixable]
  help: Use lowercase letters and hyphens only

Found 1 error, 1 warning (2 fixable)
```

Exit codes: `0` no errors (warnings allowed), `1` errors found, `2` invalid arguments.

## Rule categories

| Prefix | Category | Examples |
|--------|----------|----------|
| AS-* | Agent Skills | Name format, triggers, description |
| CC-* | Claude Code | Hooks, memory, plugins |
| MCP-* | MCP Protocol | Server config, tool definitions |
| PE-* | Prompt Engineering | Generic instructions, redundancy |
| XP-* | Cross-Platform | Compatibility across tools |
| AGM-* | AGENTS.md | Structure, sections |
| COP-* | GitHub Copilot | Instructions format |
| CUR-* | Cursor | MDC format, rules |

## Common fixes

| Issue | Solution |
|-------|----------|
| Invalid skill name | Use lowercase with hyphens: `my-skill` |
| Directory/name mismatch | Rename directory to match `name:` field |
| Generic instructions | Remove "be helpful", "be accurate" |
| Missing trigger phrase | Add "Use when..." to description |

For CI, use the [GitHub Action](https://github.com/agent-sh/agnix#github-action).

## Links

- [GitHub](https://github.com/agent-sh/agnix)
- [Rules Reference](https://github.com/agent-sh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md)
- [Configuration](https://github.com/agent-sh/agnix/blob/main/docs/CONFIGURATION.md)
