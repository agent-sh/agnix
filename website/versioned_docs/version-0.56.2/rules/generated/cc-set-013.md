---
id: cc-set-013
title: "CC-SET-013: Non-boolean autoMode.classifyAllShell Setting"
sidebar_label: "CC-SET-013"
description: "agnix rule CC-SET-013 checks for non-boolean automode.classifyallshell setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-013", "non-boolean automode.classifyallshell setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-013`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-06-27`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.193`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.193

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "autoMode": {"classifyAllShell": "true"}
}
```

### Valid

```json
{
  "autoMode": {"classifyAllShell": true}
}
```
