---
id: oc-pm-002
title: "OC-PM-002: Unknown Permission Key - OpenCode"
sidebar_label: "OC-PM-002"
description: "agnix rule OC-PM-002 checks for unknown permission key in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-PM-002", "unknown permission key", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-PM-002`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
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
  "permission": { "unknown": "allow" }
}
```

### Valid

```json
{
  "permission": { "read": "allow" }
}
```
