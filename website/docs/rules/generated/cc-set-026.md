---
id: cc-set-026
title: "CC-SET-026: Invalid promptCacheTtl Setting - Claude Settings"
sidebar_label: "CC-SET-026"
description: "agnix rule CC-SET-026 checks for invalid promptcachettl setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-026", "invalid promptcachettl setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-026`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-08-25`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.242`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.243
- https://code.claude.com/docs/en/settings-reference#promptcachettl

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "promptCacheTtl": "30m"
}
```

### Valid

```json
{
  "promptCacheTtl": "1h"
}
```
