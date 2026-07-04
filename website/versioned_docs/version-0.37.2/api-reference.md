---
title: API Reference
description: "agnix CLI flags, output formats, MCP server tools, and LSP capabilities."
---

# API Reference

## CLI

```bash
agnix [OPTIONS] [PATH]
```

### Options

| Flag | Description |
|------|-------------|
| `[PATH]` | Directory or file to validate (default: `.`) |
| `--target <TOOL>` | Single tool focus (`generic`, `claude-code`, `cursor`, `codex`, `kiro`) |
| `--fix` | Apply HIGH and MEDIUM confidence fixes |
| `--dry-run` | Preview fixes without modifying files |
| `--fix-safe` | Apply only HIGH confidence fixes |
| `--fix-unsafe` | Apply all fixes, including LOW confidence fixes |
| `--show-fixes` | Show proposed fix diffs in text output |
| `--format <FMT>` | Output format: `text` (default), `json`, `sarif` |
| `--strict` | Treat warnings as errors (exit code 1) |
| `--config <PATH>` | Config file path (default: `.agnix.toml`) |
| `--watch`, `-w` | Watch mode - re-validate on file changes |
| `--locale <LOCALE>` | Set output locale, e.g. `en`, `es`, `zh-CN` |
| `--list-locales` | List supported locales and exit |
| `--max-files <N>` | Maximum number of files to validate |
| `--verbose`, `-v` | Verbose output |
| `--version` | Print version |
| `--help` | Print help |

### Subcommands

| Command | Description |
|---------|-------------|
| `agnix validate [PATH]` | Validate agent configs explicitly |
| `agnix init` | Initialize a config file |
| `agnix eval <FILE>` | Evaluate rule efficacy against labeled test cases |
| `agnix schema [--output FILE] [--fix]` | Output or regenerate JSON Schema for `.agnix.toml` |
| `agnix tools check` | Check configured tool versions |
| `agnix tools detect` | Detect installed tool versions |
| `agnix telemetry <status\|enable\|disable>` | Manage telemetry settings |

### Output formats

- **text** - Human-readable terminal output with colors
- **json** - Machine-readable JSON object with diagnostics and summary metadata (e.g. version, files_checked, diagnostics, summary, category, rule_severity, applies_to_tool)
- **sarif** - SARIF format for GitHub Code Scanning integration

## MCP server

```bash
cargo install agnix-mcp
agnix-mcp
```

The MCP server exposes these tools:

| Tool | Description |
|------|-------------|
| `validate_file` | Validate a single configuration file |
| `validate_project` | Validate all config files in a project |
| `get_rules` | List all available validation rules |
| `get_rule_docs` | Get documentation for a specific rule |

## LSP server

```bash
cargo install agnix-lsp
agnix-lsp
```

Supported LSP capabilities:

- `textDocument/publishDiagnostics` - real-time validation
- `textDocument/codeAction` - auto-fix suggestions
- `textDocument/hover` - rule documentation on hover
- `workspace/didChangeConfiguration` - runtime config updates
- `workspace/executeCommand` - project-level validation (`agnix.validateProjectRules` command)

## References

- [SPEC.md](https://github.com/agent-sh/agnix/blob/v0.37.2/SPEC.md) - full technical specification
- [MCP Protocol](https://modelcontextprotocol.io) - MCP specification
