---
id: cc-set-008
title: "CC-SET-008: Non-boolean sandbox.allowAppleEvents Setting"
sidebar_label: "CC-SET-008"
description: "agnix rule CC-SET-008 checks for non-boolean sandbox.allowappleevents setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-008", "non-boolean sandbox.allowappleevents setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-008`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-06-18`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.181`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.181
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
  "sandbox": {"allowAppleEvents": "true"}
}
```

### Valid

```json
{
  "sandbox": {"allowAppleEvents": true}
}
```
