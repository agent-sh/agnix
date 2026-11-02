---
id: cc-set-020
title: "CC-SET-020: Invalid workflowSizeGuideline Setting"
sidebar_label: "CC-SET-020"
description: "agnix rule CC-SET-020 checks for invalid workflowsizeguideline setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-020", "invalid workflowsizeguideline setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-020`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-26`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.219`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.219
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
  "workflowSizeGuideline": "tiny"
}
```

### Valid

```json
{
  "workflowSizeGuideline": "medium"
}
```
