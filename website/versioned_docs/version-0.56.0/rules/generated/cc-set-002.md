---
id: cc-set-002
title: "CC-SET-002: Non-boolean channelsEnabled Setting"
sidebar_label: "CC-SET-002"
description: "agnix rule CC-SET-002 checks for non-boolean channelsenabled setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-002", "non-boolean channelsenabled setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-002`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-05-06`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.128`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.128

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "channelsEnabled": "true"
}
```

### Valid

```json
{
  "channelsEnabled": true
}
```
