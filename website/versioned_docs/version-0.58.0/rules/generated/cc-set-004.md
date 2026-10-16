---
id: cc-set-004
title: "CC-SET-004: Invalid Sandbox Path Setting - Claude Settings"
sidebar_label: "CC-SET-004"
description: "agnix rule CC-SET-004 checks for invalid sandbox path setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-004", "invalid sandbox path setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-004`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-05-08`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.133`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.133

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "sandbox": {"bwrapPath": "", "socatPath": 42}
}
```

### Valid

```json
{
  "sandbox": {"bwrapPath": "/usr/local/bin/bwrap", "socatPath": "/usr/bin/socat"}
}
```
