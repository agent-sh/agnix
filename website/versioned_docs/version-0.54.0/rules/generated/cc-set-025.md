---
id: cc-set-025
title: "CC-SET-025: Invalid modelPicker Setting - Claude Settings"
sidebar_label: "CC-SET-025"
description: "agnix rule CC-SET-025 checks for invalid modelpicker setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-025", "invalid modelpicker setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-025`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-08-25`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.242`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.243
- https://code.claude.com/docs/en/settings-reference#modelpicker

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "modelPicker": {
    "options": [{"label": "Missing model"}]
  }
}
```

### Valid

```json
{
  "modelPicker": {
    "options": [{"model": "opus", "label": "Opus"}],
    "replaceBuiltInOptions": false
  }
}
```
