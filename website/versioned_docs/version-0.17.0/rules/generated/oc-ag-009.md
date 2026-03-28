---
id: oc-ag-009
title: "OC-AG-009: Invalid Agent Disable Type - OpenCode"
sidebar_label: "OC-AG-009"
description: "agnix rule OC-AG-009 checks for invalid agent disable type in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-AG-009", "invalid agent disable type", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-AG-009`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-03-28`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "agent": { "a": { "disabled": "yes" } }
}
```

### Valid

```json
{
  "agent": { "a": { "disabled": true } }
}
```
