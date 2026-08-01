---
id: cc-set-017
title: "CC-SET-017: Non-boolean sandbox.filesystem.disabled Setting"
sidebar_label: "CC-SET-017"
description: "agnix rule CC-SET-017 checks for non-boolean sandbox.filesystem.disabled setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-017", "non-boolean sandbox.filesystem.disabled setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-017`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-26`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.216`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.216
- https://code.claude.com/docs/en/sandboxing

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "sandbox": {"filesystem": {"disabled": "false"}}
}
```

### Valid

```json
{
  "sandbox": {"filesystem": {"disabled": false}}
}
```
