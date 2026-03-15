---
id: oc-ag-007
title: "OC-AG-007: Redundant steps and maxSteps - OpenCode"
sidebar_label: "OC-AG-007"
description: "agnix rule OC-AG-007 checks for redundant steps and maxsteps in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-AG-007", "redundant steps and maxsteps", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-AG-007`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
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
  "agent": { "a": { "steps": 10, "maxSteps": 20 } }
}
```

### Valid

```json
{
  "agent": { "a": { "steps": 20 } }
}
```
