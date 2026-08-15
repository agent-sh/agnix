---
id: cc-set-022
title: "CC-SET-022: Invalid crossSessionInbound Setting"
sidebar_label: "CC-SET-022"
description: "agnix rule CC-SET-022 checks for invalid crosssessioninbound setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-022", "invalid crosssessioninbound setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-022`
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
- https://code.claude.com/docs/en/cross-session-messaging

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "crossSessionInbound": "prompt"
}
```

### Valid

```json
{
  "crossSessionInbound": "hold"
}
```
