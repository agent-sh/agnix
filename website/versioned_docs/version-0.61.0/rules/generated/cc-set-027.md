---
id: cc-set-027
title: "CC-SET-027: Invalid subagentPromptCacheTtl Setting"
sidebar_label: "CC-SET-027"
description: "agnix rule CC-SET-027 checks for invalid subagentpromptcachettl setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-027", "invalid subagentpromptcachettl setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-027`
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
- https://code.claude.com/docs/en/settings-reference#subagentpromptcachettl

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "subagentPromptCacheTtl": 300
}
```

### Valid

```json
{
  "subagentPromptCacheTtl": "5m"
}
```
