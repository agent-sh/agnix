---
id: cc-set-028
title: "CC-SET-028: Invalid feedbackDrafts Setting - Claude Settings"
sidebar_label: "CC-SET-028"
description: "agnix rule CC-SET-028 checks for invalid feedbackdrafts setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-028", "invalid feedbackdrafts setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-028`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-08-27`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.247`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.247
- https://code.claude.com/docs/en/settings-reference#feedbackdrafts

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "feedbackDrafts": false
}
```

### Valid

```json
{
  "feedbackDrafts": "quiet"
}
```
