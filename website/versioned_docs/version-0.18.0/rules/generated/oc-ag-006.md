---
id: oc-ag-006
title: "OC-AG-006: Invalid Named Color - OpenCode"
sidebar_label: "OC-AG-006"
description: "agnix rule OC-AG-006 checks for invalid named color in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-AG-006", "invalid named color", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-AG-006`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-03-06`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/config

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "agent": { "a": { "color": "purple" } }
}
```

### Valid

```json
{
  "agent": { "a": { "color": "primary" } }
}
```
