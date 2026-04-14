---
id: oc-dep-005
title: "OC-DEP-005: Deprecated TUI Keys - OpenCode"
sidebar_label: "OC-DEP-005"
description: "agnix rule OC-DEP-005 checks for deprecated tui keys in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-DEP-005", "deprecated tui keys", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-DEP-005`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
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
  "tui": { "colors": { "bg": "#000" } }
}
```

### Valid

```json
{
  "tui": { "theme": "dark" }
}
```
