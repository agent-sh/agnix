---
id: oc-dep-006
title: "OC-DEP-006: Deprecated MaxSteps Field - OpenCode"
sidebar_label: "OC-DEP-006"
description: "agnix rule OC-DEP-006 checks for deprecated maxsteps field in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-DEP-006", "deprecated maxsteps field", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-DEP-006`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe)`
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
  "agent": { "a": { "maxSteps": 50 } }
}
```

### Valid

```json
{
  "agent": { "a": { "maxTokens": 8000 } }
}
```
