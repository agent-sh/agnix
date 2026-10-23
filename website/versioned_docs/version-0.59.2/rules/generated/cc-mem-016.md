---
id: cc-mem-016
title: "CC-MEM-016: MEMORY.md Exceeds Load Limit - Claude Memory"
sidebar_label: "CC-MEM-016"
description: "agnix rule CC-MEM-016 checks for memory.md exceeds load limit in claude memory files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-MEM-016", "memory.md exceeds load limit", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-016`
- **Severity**: `MEDIUM`
- **Category**: `Claude Memory`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/memory
- https://code.claude.com/docs/en/sub-agents

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
.claude/agent-memory/reviewer/MEMORY.md with 214 lines
```

### Valid

```markdown
.claude/agent-memory/reviewer/MEMORY.md with one line per entry (under 200 lines and 25KB)
```
