---
id: cc-set-009
title: "CC-SET-009: Non-boolean attribution.sessionUrl Setting"
sidebar_label: "CC-SET-009"
description: "agnix rule CC-SET-009 checks for non-boolean attribution.sessionurl setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-009", "non-boolean attribution.sessionurl setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-009`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-06-21`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.183`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.183
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
  "attribution": {"sessionUrl": "false"}
}
```

### Valid

```json
{
  "attribution": {"sessionUrl": false}
}
```
