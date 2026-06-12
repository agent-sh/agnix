---
title: Configuration
description: "Configure agnix with .agnix.toml - target tools, disable rules, set output format, and more."
---

# Configuration

agnix works with zero configuration. To customize, add `.agnix.toml` to your project root.

## Example

```toml
tools = ["claude-code"]

[rules]
disabled_rules = []
```

## Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `tools` | string[] | all | Multi-tool targeting: `claude-code`, `cursor`, `codex`, `copilot`, `github-copilot`, `generic` |
| `target` | string | `"Generic"` | Deprecated single-tool focus. Prefer `tools`. |
| `exclude` | string[] | built-in defaults | Project-relative glob patterns to skip |
| `[rules].disabled_rules` | string[] | `[]` | Rule IDs to skip globally (e.g. `["CC-MEM-005"]`) |
| `[files].include_as_memory` | string[] | `[]` | Extra Markdown files to validate as memory/instruction files |
| `[[overrides]].disabled_rules` | string[] | `[]` | Rule IDs to skip only for matching override paths |

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

## Full reference

For the complete configuration specification, see
[docs/CONFIGURATION.md](https://github.com/agent-sh/agnix/blob/v0.32.0/docs/CONFIGURATION.md)
in the repository.
