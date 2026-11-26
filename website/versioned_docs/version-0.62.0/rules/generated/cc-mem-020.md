---
id: cc-mem-020
title: "CC-MEM-020: Personal Claude File Not Gitignored"
sidebar_label: "CC-MEM-020"
description: "agnix rule CC-MEM-020 checks for personal claude file not gitignored in claude memory files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-MEM-020", "personal claude file not gitignored", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-020`
- **Severity**: `LOW`
- **Category**: `Claude Memory`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/memory
- https://code.claude.com/docs/en/settings
- https://code.claude.com/docs/en/sub-agents

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
CLAUDE.local.md in a git repository whose .gitignore does not list it
```

### Valid

```markdown
.gitignore:
CLAUDE.local.md
.claude/agent-memory-local/
```
