---
id: cc-set-023
title: "CC-SET-023: Invalid dialogExpiry Setting - Claude Settings"
sidebar_label: "CC-SET-023"
description: "agnix rule CC-SET-023 checks for invalid dialogexpiry setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-023", "invalid dialogexpiry setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-023`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-08-08`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.224`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.224
- https://code.claude.com/docs/en/settings

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "dialogExpiry": "30s"
}
```

### Valid

```json
{
  "dialogExpiry": "10m"
}
```
