---
id: cc-set-010
title: "CC-SET-010: Invalid teammateMode Setting - Claude Settings"
sidebar_label: "CC-SET-010"
description: "agnix rule CC-SET-010 checks for invalid teammatemode setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-010", "invalid teammatemode setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-010`
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
- https://code.claude.com/docs/en/settings
- https://code.claude.com/docs/en/agent-teams

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "teammateMode": "screen"
}
```

### Valid

```json
{
  "teammateMode": "iterm2"
}
```
