---
id: oc-dep-002
title: "OC-DEP-002: Deprecated tools Field - OpenCode"
sidebar_label: "OC-DEP-002"
description: "agnix rule OC-DEP-002 checks for deprecated tools field in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-DEP-002", "deprecated tools field", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-DEP-002`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe)`
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
  "tools": {}
}
```

### Valid

```json
{
  "permission": { "read": "allow" }
}
```
