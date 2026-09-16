---
id: cc-set-021
title: "CC-SET-021: Ineffective Project Remote Control Auto-start"
sidebar_label: "CC-SET-021"
description: "agnix rule CC-SET-021 checks for ineffective project remote control auto-start in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-021", "ineffective project remote control auto-start", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-021`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-08-05`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.222`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.222
- https://code.claude.com/docs/en/settings
- https://code.claude.com/docs/en/remote-control

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "remoteControlAtStartup": true
}
```

### Valid

```json
{
  "remoteControlAtStartup": false
}
```
