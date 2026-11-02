---
id: cc-mem-015
title: "CC-MEM-015: Always-Loaded Instructions Exceed Combined Size"
sidebar_label: "CC-MEM-015"
description: "agnix rule CC-MEM-015 checks for always-loaded instructions exceed combined size in claude memory files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-MEM-015", "always-loaded instructions exceed combined size", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-015`
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

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
CLAUDE.md (150 lines) + @docs/conventions.md (150) + unconditional .claude/rules/style.md (150)
```

### Valid

```markdown
CLAUDE.md (120 lines) plus .claude/rules/api.md with paths: [src/api/**]
```
