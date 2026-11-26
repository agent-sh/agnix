---
id: cc-mem-019
title: "CC-MEM-019: Import Path Quoted or Split by a Space"
sidebar_label: "CC-MEM-019"
description: "agnix rule CC-MEM-019 checks for import path quoted or split by a space in claude memory files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-MEM-019", "import path quoted or split by a space", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-019`
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

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
- API conventions @"Design Docs/api-conventions.md"
```

### Valid

```markdown
- API conventions @Design\ Docs/api-conventions.md
```
