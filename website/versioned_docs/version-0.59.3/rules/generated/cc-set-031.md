---
id: cc-set-031
title: "CC-SET-031: Invalid maxEffortLevel Setting - Claude Settings"
sidebar_label: "CC-SET-031"
description: "agnix rule CC-SET-031 checks for invalid maxeffortlevel setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-031", "invalid maxeffortlevel setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-031`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-09-14`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.267`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/settings-reference#maxeffortlevel
- https://github.com/anthropics/claude-code/releases/tag/v2.1.267

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "maxEffortLevel": "ultracode",
  "modelSettings": {
    "claude-opus-5": { "maxEffortLevel": 3 }
  }
}
```

### Valid

```json
{
  "maxEffortLevel": "medium",
  "modelSettings": {
    "claude-sonnet-4-6": { "maxEffortLevel": "max" }
  }
}
```
