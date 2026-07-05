---
title: Configuration
description: "Configure agnix with .agnix.toml - target tools, disable rules, set output format, and more."
---

# Configuration

agnix works with zero configuration. To customize, add `.agnix.toml` to your project root.

## Example

```toml
target = "ClaudeCode"
tools = ["claude-code"]
max_files_to_validate = 10000
locale = "en"

[rules]
disabled_rules = []
```

## Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `target` | string | `Generic` | Legacy single tool focus: `Generic`, `ClaudeCode`, `Cursor`, `Codex`, `Kiro` |
| `tools` | string[] | `[]` | Multi-tool targeting. Overrides `target`. Use values like `claude-code`, `cursor`, `codex`, `kiro`, `github-copilot`, `cline`, `opencode`, `gemini-cli`, `amp`, `roo-code`, `windsurf`, `generic`. |
| `severity` | string | `Warning` | Minimum severity level: `Warning`, `Error`, or `Info` |
| `max_files_to_validate` | int | `10000` | Maximum files to scan |
| `locale` | string | `"en"` | Output locale |
| `[rules].disabled_rules` | string[] | `[]` | Rule IDs to skip (e.g. `["CC-MEM-005"]`) |
| `[rules].disabled_validators` | string[] | `[]` | Validator names to skip |
| `[files]` | table | default excludes | Include or exclude non-standard files |
| `[[overrides]]` | table array | `[]` | Per-file disabled rule overrides |

## CLI flags

CLI flags override `.agnix.toml` values:

```bash
# Target a specific tool
agnix --target cursor .

# Apply fixes
agnix --fix .

# JSON output for CI
agnix --format json .

# SARIF output for GitHub Code Scanning
agnix --format sarif .

# Strict mode
agnix --strict .
```

`--strict`, `--fix`, `--fix-safe`, `--fix-unsafe`, `--dry-run`, `--show-fixes`, and `--format` are CLI flags, not `.agnix.toml` keys.

## Full reference

For the complete configuration specification, see
[docs/CONFIGURATION.md](https://github.com/agent-sh/agnix/blob/v0.37.5/docs/CONFIGURATION.md)
in the repository.
