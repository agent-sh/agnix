---
id: oc-ag-004
title: "OC-AG-004: Steps Not a Positive Integer - OpenCode"
sidebar_label: "OC-AG-004"
description: "agnix rule OC-AG-004 checks for steps not a positive integer in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-AG-004", "steps not a positive integer", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-AG-004`
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
  "agent": { "custom": { "steps": -5 } }
}
```

### Valid

```json
{
  "agent": { "custom": { "steps": 50 } }
}
```
