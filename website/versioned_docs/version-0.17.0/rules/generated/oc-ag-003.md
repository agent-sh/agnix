---
id: oc-ag-003
title: "OC-AG-003: Temperature Out of Range - OpenCode"
sidebar_label: "OC-AG-003"
description: "agnix rule OC-AG-003 checks for temperature out of range in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-AG-003", "temperature out of range", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-AG-003`
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
  "agent": { "custom": { "temperature": 3.5 } }
}
```

### Valid

```json
{
  "agent": { "custom": { "temperature": 0.7 } }
}
```
