---
id: oc-ag-008
title: "OC-AG-008: Invalid hidden Type - OpenCode"
sidebar_label: "OC-AG-008"
description: "agnix rule OC-AG-008 checks for invalid hidden type in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-AG-008", "invalid hidden type", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-AG-008`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
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
  "agent": { "a": { "hidden": "yes" } }
}
```

### Valid

```json
{
  "agent": { "a": { "hidden": true } }
}
```
