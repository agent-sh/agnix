---
name: agnix
description: "Use when user asks to 'lint agent configs', 'validate skills', 'check CLAUDE.md', 'validate hooks', 'lint MCP'. Validates agent configuration files against 457 rules."
allowed-tools: Bash(agnix:*), Bash(cargo:*), Read, Glob, Grep
---

# agnix

Lint agent configurations (Skills, Hooks, MCP, Memory, Plugins) before they break a workflow, across Claude Code, Codex CLI, OpenCode, Kiro, Cursor, GitHub Copilot and other tools.

## Run

1. `agnix --version`. If it is missing, install with `npm install -g agnix` or `cargo install agnix-cli`.
2. `agnix .` (or the path the user named).
3. If the user asked for fixes: `agnix --fix .`, then run step 2 again to confirm what remains. Otherwise change no files; `agnix --dry-run .` previews the fixes.

## CLI reference

| Command | Description |
|---------|-------------|
| `agnix .` | Validate current project |
| `agnix --fix .` | Apply safe auto-fixes |
| `agnix --dry-run .` | Show what would be fixed, change nothing |
| `agnix --strict .` | Treat warnings as errors |
| `agnix --target claude-code .` | Target one tool: `generic` (default), `claude-code`, `cursor`, `codex`, `kiro` |
| `agnix --watch .` | Watch mode |
| `agnix --format json .` | JSON output |

## Supported files

| File Type | Examples |
|-----------|----------|
| Skills | `SKILL.md` |
| Memory | `CLAUDE.md`, `AGENTS.md` |
| Hooks | `.claude/settings.json` |
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

## Common fixes

| Issue | Solution |
|-------|----------|
| Invalid skill name | Use lowercase with hyphens: `my-skill` |
| Generic instructions | Remove "be helpful", "be accurate" |
| Missing trigger phrase | Add "Use when..." to description |
| Directory/name mismatch | Rename directory to match `name:` field |

## Links

- [GitHub](https://github.com/agent-sh/agnix)
- [Rules Reference](https://agent-sh.github.io/agnix/docs/rules/)
