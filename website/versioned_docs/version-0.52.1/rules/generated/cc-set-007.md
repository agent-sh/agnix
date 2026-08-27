---
id: cc-set-007
title: "CC-SET-007: Non-boolean enforceAvailableModels Setting"
sidebar_label: "CC-SET-007"
description: "agnix rule CC-SET-007 checks for non-boolean enforceavailablemodels setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-007", "non-boolean enforceavailablemodels setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-007`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-06-16`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.175`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.175
- https://code.claude.com/docs/en/settings

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "enforceAvailableModels": "true"
}
```

### Valid

```json
{
  "enforceAvailableModels": true
}
```
