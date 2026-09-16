---
id: cc-set-011
title: "CC-SET-011: Non-boolean respondToBashCommands Setting"
sidebar_label: "CC-SET-011"
description: "agnix rule CC-SET-011 checks for non-boolean respondtobashcommands setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-011", "non-boolean respondtobashcommands setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-011`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-06-24`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.186`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.186

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "respondToBashCommands": "false"
}
```

### Valid

```json
{
  "respondToBashCommands": false
}
```
