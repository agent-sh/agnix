---
id: cc-mem-003
title: "CC-MEM-003: Import Depth Exceeds 4 - Claude Memory"
sidebar_label: "CC-MEM-003"
description: "agnix rule CC-MEM-003 checks for import depth exceeds 4 in claude memory files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-MEM-003", "import depth exceeds 4", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-003`
- **Severity**: `HIGH`
- **Category**: `Claude Memory`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-31`

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
# Project Memory

@import ./level1.md

This starts a chain: level1 -> level2 -> level3 -> level4 -> level5 -> level6 (exceeds depth 5).
```

### Valid

```markdown
# Project Memory

@import ./docs/guidelines.md

Keep import chains shallow.
```
