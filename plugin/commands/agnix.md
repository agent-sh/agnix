---
description: Use when user asks to 'lint agent configs', 'validate skills', 'check CLAUDE.md', 'validate hooks', 'lint MCP', or mentions 'agent config issues', 'skill validation'.
codex-description: 'Use when user asks to "lint agent configs", "validate skills", "check CLAUDE.md", "validate hooks", "lint MCP". Validates agent configuration files against 457 rules across 10+ AI tools.'
argument-hint: "[path] [--fix] [--strict] [--target [target]]"
allowed-tools: Task, Read
---

# /agnix - Agent Config Linter

Lint agent configurations (Skills, Hooks, MCP, Memory, Plugins) for Claude Code, Codex CLI, Cursor, GitHub Copilot, Kiro and other tools, and show the findings.

## Arguments

From `$ARGUMENTS`:

- **path**: the first argument that is not a flag or the value of `--target`. Default `.`. Remove newlines from it before passing it on.
- **--fix**: apply auto-fixes.
- **--strict**: treat warnings as errors.
- **--target**: `generic` (default), `claude-code`, `cursor`, `codex` or `kiro`, as `--target=X` or `--target X`. Any other value falls back to `generic`.

## Run

Spawn `agnix:agnix-agent` with:

```
Validate agent configurations.
Path: {path}
Fix: {true|false}
Strict: {true|false}
Target: {target}

Return structured results between === AGNIX_RESULT === markers.
```

Without the Task tool, run the steps of the plugin's `skills/agnix/SKILL.md` in this session.

Parse the JSON between `=== AGNIX_RESULT ===` and `=== END_RESULT ===` as JSON. If the block is missing or does not parse, show the agent's raw output instead of reporting zero issues.

## Report

No issues:

```markdown
## Validation Passed

No issues found in agent configurations.

- Files validated: N
- Target: {target}
```

Issues found:

```markdown
## Agent Config Issues

| File | Line | Level | Rule | Message |
|------|------|-------|------|---------|
| SKILL.md | 3 | error | AS-004 | Invalid name |
| CLAUDE.md | 15 | warning | PE-003 | Generic instruction |

## Summary

- **Errors**: N
- **Warnings**: N
- **Fixable**: N

## Do Next

- [ ] Run `/agnix --fix` to auto-fix {fixable} issues
- [ ] Review remaining issues manually
```

After `--fix`:

```markdown
## Fixed Issues

| File | Line | Rule | Fix Applied |
|------|------|------|-------------|
| SKILL.md | 3 | AS-004 | Renamed to lowercase |

**Fixed**: N issues
**Remaining**: N issues (manual review needed)
```

## Errors

- agnix not installed: show the install command, `npm install -g agnix` or `cargo install agnix-cli`.
- Path not found: `Path not found: [path]`.
- agnix output that cannot be parsed: show the raw output.

## Links

- [agnix GitHub](https://github.com/agent-sh/agnix)
- [Rules Reference](https://github.com/agent-sh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md)
