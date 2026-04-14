---
id: oc-pm-001
title: "OC-PM-001: Invalid Permission Action - OpenCode"
sidebar_label: "OC-PM-001"
description: "agnix rule OC-PM-001 checks for invalid permission action in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-PM-001", "invalid permission action", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-PM-001`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-03`

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
  "permission": { "read": "yes" }
}
```

### Valid

```json
{
  "permission": { "read": "allow", "bash": "ask" }
}
```
