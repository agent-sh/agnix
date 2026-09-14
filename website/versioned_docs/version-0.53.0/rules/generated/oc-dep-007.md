---
id: oc-dep-007
title: "OC-DEP-007: Deprecated Reference Field - OpenCode"
sidebar_label: "OC-DEP-007"
description: "agnix rule OC-DEP-007 checks for deprecated reference field in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-DEP-007", "deprecated reference field", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-DEP-007`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe)`
- **Verified On**: `2026-07-27`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/config.json
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
  "reference": { "lib": "github:org/repo" }
}
```

### Valid

```json
{
  "references": { "lib": "github:org/repo" }
}
```
