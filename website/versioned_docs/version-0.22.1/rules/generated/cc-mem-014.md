---
id: cc-mem-014
title: "CC-MEM-014: CLAUDE.md Exceeds Line Limit - Claude Memory"
sidebar_label: "CC-MEM-014"
description: "agnix rule CC-MEM-014 checks for claude.md exceeds line limit in claude memory files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-MEM-014", "claude.md exceeds line limit", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-014`
- **Severity**: `MEDIUM`
- **Category**: `Claude Memory`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-28`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/memory

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
# Project

(500+ lines of instructions that exceed recommended limits)
```

### Valid

```markdown
# Project

Keep CLAUDE.md concise and focused.
```
