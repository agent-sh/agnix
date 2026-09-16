---
id: cc-set-014
title: "CC-SET-014: autoMode Setting Ignored in settings.local.json"
sidebar_label: "CC-SET-014"
description: "agnix rule CC-SET-014 checks for automode setting ignored in settings.local.json in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-014", "automode setting ignored in settings.local.json", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-014`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-15`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.207`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.207

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "autoMode": {"classifyAllShell": true}
}
```

### Valid

```json
{
  "model": "claude-sonnet-4"
}
```
