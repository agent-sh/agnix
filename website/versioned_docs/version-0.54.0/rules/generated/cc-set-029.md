---
id: cc-set-029
title: "CC-SET-029: Invalid spinnerTipsOverride Shape"
sidebar_label: "CC-SET-029"
description: "agnix rule CC-SET-029 checks for invalid spinnertipsoverride shape in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-029", "invalid spinnertipsoverride shape", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-029`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-08-27`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.247`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.247
- https://code.claude.com/docs/en/settings-reference#spinnertipsoverride

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "spinnerTipsOverride": {
    "tips": [
      { "text": "Missing an id", "cooldownSessions": 5000 }
    ]
  }
}
```

### Valid

```json
{
  "spinnerTipsOverride": {
    "label": "Acme tip",
    "tips": [
      "Run /review before opening a PR",
      { "id": "gateway-errors", "text": "Check the gateway status page first", "cooldownSessions": 5, "priority": 2 }
    ]
  }
}
```
