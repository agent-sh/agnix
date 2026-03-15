---
id: oc-ag-005
title: "OC-AG-005: top_p Out of Range - OpenCode"
sidebar_label: "OC-AG-005"
description: "agnix rule OC-AG-005 checks for top_p out of range in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-AG-005", "top_p out of range", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-AG-005`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
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
  "agent": { "a": { "top_p": 1.5 } }
}
```

### Valid

```json
{
  "agent": { "a": { "top_p": 0.9 } }
}
```
