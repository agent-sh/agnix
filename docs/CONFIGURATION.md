# Configuration Reference

Create `.agnix.toml` in your project root. All fields are optional with sensible defaults.

## Quick Examples

### Extend a Shared Config

```toml
extend = "../base.agnix.toml"
```

### Disable Specific Rules

```toml
[rules]
disabled_rules = ["CC-MEM-006", "PE-003", "XP-001"]
```

### Override a Rule Severity

```toml
[rules.severity]
MCP-008 = "Error"
```

### Target a Specific Tool

```toml
target = "ClaudeCode"  # Deprecated; prefer tools = ["claude-code"]
```

### Multi-Tool Project

```toml
tools = ["claude-code", "cursor", "github-copilot"]
```

### Include Custom Files

```toml
[files]
include_as_memory = ["docs/ai-rules/*.md"]
exclude = ["vendor/**"]
```

### Carve Out Rules for Specific Files

Disable a rule on specific files only, without disabling it globally. Useful when a file legitimately contains patterns that would otherwise trip a rule (e.g., a CLAUDE.md that documents quoted-example triggers):

```toml
[[overrides]]
paths = ["CLAUDE.md", "AGENTS.md"]
disabled_rules = ["CC-MEM-005"]
```

See the [Per-File Rule Overrides](#per-file-rule-overrides) section below for full semantics.

### Inline Suppression

```md
<!-- agnix-disable-next-line CC-MEM-005 -->
- make sure to use this exact example text

temporary exception <!-- agnix: noqa: PE-003 -->
```

## Full Reference

```toml
extend = []          # Optional base config paths, string or array in TOML files
severity = "Warning"  # Warning, Error, Info
target = "Generic"    # Deprecated: Generic, ClaudeCode, Cursor, Codex, Kiro

# Multi-tool support (overrides target)
tools = ["claude-code", "cursor", "github-copilot"]  # Valid: claude-code, cursor, codex, kiro, copilot, github-copilot, cline, opencode, gemini-cli, amp, roo-code, windsurf, generic

exclude = [
  "node_modules/**",
  ".git/**",
  "target/**",
]

[rules]
# Category toggles - all default to true
skills = true              # AS-*, CC-SK-* rules
hooks = true               # CC-HK-* rules
agents = true              # CC-AG-* rules
copilot = true             # COP-* rules
cursor = true              # CUR-* rules
memory = true              # CC-MEM-* rules
plugins = true             # CC-PL-* rules
mcp = true                 # MCP-* rules
prompt_engineering = true  # PE-* rules
xml = true                 # XML-* rules
imports = true             # REF-* rules
cross_platform = true      # XP-* rules
agents_md = true           # AGM-* rules

# Disable specific rules by ID
disabled_rules = ["CC-MEM-006", "PE-003"]

# Override reported diagnostic levels for specific rules
[rules.severity]
# MCP-008 = "Error"

# Version-aware validation (optional)
[tool_versions]
# claude_code = "1.0.0"
# cursor = "0.45.0"

[spec_revisions]
# mcp_protocol = "2025-11-25"

# File inclusion/exclusion for non-standard agent files
[files]
# Validate as CLAUDE.md-like memory/instruction files
# include_as_memory = ["docs/ai-rules/*.md", "custom/INSTRUCTIONS.md"]

# Validate as generic markdown (XML, imports, cross-platform rules)
# include_as_generic = ["internal/*.md"]

# Exclude from validation entirely (even built-in file types)
# exclude = ["vendor/**", "generated/**"]

# Per-file rule suppression (see "Per-file rule overrides" below).
# Each [[overrides]] block disables `disabled_rules` for files matching
# any pattern in `paths`. Multiple blocks stack (set union); ordering
# does not matter.
# [[overrides]]
# paths = ["CLAUDE.md", "AGENTS.md"]
# disabled_rules = ["CC-MEM-005"]
```

## Schema Validation

agnix automatically validates `.agnix.toml` files for:

- **Invalid rule IDs**: Warns if `disabled_rules` (in `[rules]` or `[[overrides]]`) contains IDs that don't match known rule ID prefixes from `knowledge-base/rules.json` (AS-, CC-*, CDX-*, OC-*, KR-*, MCP-, REF-, XP-, AGM-, COP-, CUR-, CLN-, PE-, VER-, imports::, and other supported tool prefixes)
- **Removed rule IDs**: Warns if `disabled_rules` or `[rules.severity]` references a removed rule ID such as `AS-007`, `AS-010`, or `AS-014`, and suggests the replacement rule family when one exists.
- **Invalid severity override IDs**: Warns if `[rules.severity]` keys don't match known rule ID prefixes.
- **Unknown tools**: Warns if `tools` array contains tool names that aren't recognized
- **Invalid file patterns**: Warns if `[files]` or `[[overrides]].paths` glob patterns have invalid syntax. The invalid pattern is dropped at config-load (it can't match anything) and the warning surfaces it so you know which one was ignored.
- **Unsafe override paths**: Warns if `[[overrides]].paths` entries are absolute (`/foo/...`) or contain `..` traversal. These patterns can never match a project-relative file path, so the override is a no-op even though it parses. (SDK consumers using `LintConfigBuilder::build()` get a hard error for these patterns instead.)
- **Deprecated fields**: Warns when using `mcp_protocol_version` (use `spec_revisions.mcp_protocol` instead)

These warnings appear before validation output and include suggestions for fixes.

## Config Inheritance

Use `extend` to share a base `.agnix.toml` across projects:

```toml
extend = "../base.agnix.toml"

[rules.severity]
MCP-008 = "Error"
```

`extend` accepts either a string or an array of strings. Paths are resolved relative to the config file that declares them. Parent configs load first, then child values merge on top. Tables merge recursively, while scalar values and arrays in the child replace the parent value.

## Per-Rule Severity Overrides

Use `[rules.severity]` to adjust the diagnostic level for one rule without disabling it:

```toml
[rules.severity]
MCP-008 = "Error"
VER-001 = "Info"
```

Allowed values are `Error`, `Warning`, and `Info`. Overrides apply to CLI, JSON, SARIF, LSP, MCP, and WASM validation paths because they are applied in the core pipeline.

## Inline Suppression

Inline suppressions are intended for small, local exceptions where a file legitimately contains example text or compatibility glue that would otherwise trip a rule.

```md
<!-- agnix-disable-next-line CC-MEM-005 -->
- make sure to keep this phrase in the example

This wording is intentional. <!-- agnix: noqa: PE-003 -->
```

Supported markers:

| Marker | Scope |
|--------|-------|
| `agnix: noqa` | Suppress all diagnostics on the same line |
| `agnix: noqa: RULE-ID` | Suppress one or more same-line rules |
| `agnix-disable-next-line` | Suppress all diagnostics on the following line |
| `agnix-disable-next-line RULE-ID` | Suppress one or more following-line rules |
| `agnix-disable RULE-ID` | Suppress one or more rules for the whole file |

### Generate Schema

Output JSON Schema for `.agnix.toml` validation:

```bash
# Output to stdout
agnix schema

# Save to file
agnix schema --output schemas/agnix.json
```

The VS Code extension automatically uses this schema for autocomplete and validation.

## Per-File Rule Overrides

`[[overrides]]` lets you disable specific rules on specific files, without disabling them everywhere. Use it when a file legitimately contains patterns that trip a rule (for example, a `CLAUDE.md` that documents quoted-example triggers, or a generated file that uses constructs the rule discourages).

### Schema

```toml
[[overrides]]
paths = ["CLAUDE.md", "docs/agents/**/*.md"]
disabled_rules = ["CC-MEM-005", "PE-003"]

[[overrides]]
paths = ["AGENTS.md"]
disabled_rules = ["CC-MEM-007"]
```

| Field | Type | Description |
|-------|------|-------------|
| `paths` | `[String]` | Glob patterns matched against project-relative paths. If any pattern matches a file, the block applies. |
| `disabled_rules` | `[String]` | Rule IDs to disable for matching files. Same ID format as `[rules].disabled_rules`. |

### Semantics

- **Union, never subtractive.** For each file, the effective disabled-rules set is `[rules].disabled_rules` ∪ (every `[[overrides]]` block whose `paths` matched). Overrides can only *add* to what is disabled; they cannot re-enable a rule that is globally disabled (via `[rules].disabled_rules`) or disabled by a category toggle (e.g., `[rules].skills = false`).
- **Multiple blocks stack.** If two `[[overrides]]` blocks both match a file, both contribute their `disabled_rules`. Ordering does not matter.
- **No effect on non-matching files.** A file not matched by any block sees only the global config.
- **`[files].exclude` wins.** If a file is excluded entirely via `[files].exclude`, it is skipped before any rule runs, so `[[overrides]]` on excluded paths is moot.
- **Empty `paths = []` matches nothing.** A block with no patterns has no effect; the `disabled_rules` it carries never apply to any file.

### Glob semantics

Patterns use the same matcher as `[files].exclude` (Rust [`glob`](https://docs.rs/glob/) with `require_literal_separator = true`):

- `*` matches within a single path component. `*.md` matches `README.md` but **not** `docs/README.md`.
- `**` matches across directories. `**/*.md` matches `README.md` and `docs/nested/page.md`.
- Exact paths match only themselves. `.claude/CLAUDE.md` matches that one file.

Paths are matched relative to the project root (where `.agnix.toml` lives). When invoked without a project root (e.g., single-file mode), patterns are matched against the file name only.

### Validation

- Invalid glob syntax in `paths` → warning (pattern is dropped at config-load; SDK `LintConfigBuilder::build()` rejects as error).
- Absolute paths (`/etc/...`) or `..` traversal in `paths` → warning (pattern is kept but can never match a project-relative path, so the override is a silent no-op; SDK `LintConfigBuilder::build()` and `build_lenient()` both reject as error).
- Unknown rule-ID prefixes in `disabled_rules` → warning.

### Example: CLAUDE.md carve-out

A project-level `CLAUDE.md` that documents Claude trigger phrases would normally trip CC-MEM-005 (generic-instruction detection) on its own example text. Carve it out without losing the rule everywhere else:

```toml
[[overrides]]
paths = ["CLAUDE.md", "AGENTS.md"]
disabled_rules = ["CC-MEM-005"]
```

CC-MEM-005 still fires on every other memory file in the repo.

## Rule Categories

| Category | Rules | Description |
|----------|-------|-------------|
| skills | AS-*, CC-SK-* | Agent skill validation |
| hooks | CC-HK-* | Hook configuration |
| agents | CC-AG-* | Subagent validation |
| copilot | COP-* | GitHub Copilot instructions |
| cursor | CUR-* | Cursor project rule validation |
| memory | CC-MEM-* | Memory/CLAUDE.md |
| plugins | CC-PL-* | Plugin validation |
| mcp | MCP-* | MCP tool validation |
| prompt_engineering | PE-* | Prompt best practices |
| xml | XML-* | XML tag balance |
| imports | REF-* | Import reference validation |
| cross_platform | XP-* | Cross-platform consistency |
| agents_md | AGM-* | AGENTS.md validation |

Version-awareness (`VER-*`) is always active and configured via `tool_versions` and `spec_revisions` (not a category toggle).

## Target Filtering

When `target` is set:
- **ClaudeCode** or **Generic**: All rules enabled
- **Cursor** or **Codex**: CC-* rules disabled

## Version-Aware Validation

When versions are not pinned, agnix uses defaults and adds assumption notes. Pin versions for precise validation:

```toml
[tool_versions]
claude_code = "1.0.0"
```

---

## Output Formats

### Text (default)

```bash
agnix .
```

Human-readable colored output with context.

### JSON

```bash
agnix --format json . > results.json
```

```json
{
  "version": "0.37.4",
  "files_checked": 5,
  "diagnostics": [
    {
      "level": "error",
      "rule": "CC-SK-001",
      "file": ".claude/skills/my-skill/SKILL.md",
      "line": 10,
      "column": 1,
      "message": "Invalid model 'claude-opus-5'",
      "suggestion": "Use one of the valid model values: claude-3-5-sonnet, claude-3-opus, claude-3-haiku",
      "category": "claude-code-skills",
      "rule_severity": "HIGH",
      "applies_to_tool": "claude-code"
    }
  ],
  "summary": {
    "errors": 1,
    "warnings": 0,
    "info": 0
  }
}
```

Note: category, rule_severity, and applies_to_tool are optional fields included when rule metadata is present.

### SARIF

```bash
agnix --format sarif . > results.sarif
```

Full SARIF 2.1.0 compliance for GitHub Code Scanning.

SARIF output includes rule metadata, diagnostic spans, safe fix suggestions when available, and CWE/OWASP taxonomies for rules with security metadata.

### GitHub Actions Annotations

```bash
agnix --format github .
```

Emits workflow-command annotations such as `::error file=...,line=...::message` for direct inline display in GitHub Actions logs.

### Explain a Rule

```bash
agnix explain MCP-018
agnix explain MCP-018 --format json
```

Prints a rule's severity, category, source URLs, evidence metadata, fix metadata, and examples from the same `rules.json` catalog used by validation.

---

## GitHub Action

### Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `path` | Path to validate | `.` |
| `strict` | Treat warnings as errors | `false` |
| `target` | Target tool | `generic` |
| `config` | Path to .agnix.toml | |
| `format` | Output format | `text` |
| `verbose` | Verbose output | `false` |
| `version` | agnix version | `latest` |
| `build-from-source` | Build from source | `false` |
| `fail-on-error` | Fail on errors | `true` |

### Outputs

| Output | Description |
|--------|-------------|
| `result` | success or failure |
| `errors` | Error count |
| `warnings` | Warning count |
| `sarif-file` | SARIF file path |

### Examples

**Basic:**

```yaml
- uses: agent-sh/agnix@v0
```

**Strict with target:**

```yaml
- uses: agent-sh/agnix@v0
  with:
    target: 'claude-code'
    strict: 'true'
```

**SARIF upload:**

```yaml
- uses: agent-sh/agnix@v0
  id: agnix
  with:
    format: 'sarif'

- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: ${{ steps.agnix.outputs.sarif-file }}
```

**Conditional failure:**

```yaml
- uses: agent-sh/agnix@v0
  id: validate
  with:
    fail-on-error: 'false'

- if: steps.validate.outputs.errors > 0
  run: |
    echo "Found ${{ steps.validate.outputs.errors }} errors"
    exit 1
```

---

## Pre-commit Hook

Integrate agnix into your pre-commit workflow.

### Installation

Add to `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/agent-sh/agnix
    rev: v0.37.4
    hooks:
      - id: agnix
```

### Available Hooks

| Hook ID | Description |
|---------|-------------|
| `agnix` | Validate configs (strict mode) |
| `agnix-fix` | Validate and auto-fix |

### With Auto-fix

```yaml
repos:
  - repo: https://github.com/agent-sh/agnix
    rev: v0.37.4
    hooks:
      - id: agnix-fix
```

### Requirements

The `agnix` binary must be installed and available in PATH:

```bash
cargo install agnix-cli
```
