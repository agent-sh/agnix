---
id: oc-ag-002
title: "OC-AG-002: Invalid Color Format - OpenCode"
sidebar_label: "OC-AG-002"
description: "agnix rule OC-AG-002 checks for invalid color format in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-AG-002", "invalid color format", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-AG-002`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-02`

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
  "agent": { "custom": { "color": "red" } }
}
```

### Valid

```json
{
  "agent": { "custom": { "color": "#ff0000" } }
}
```
