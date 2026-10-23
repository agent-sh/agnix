---
id: oc-ag-011
title: "OC-AG-011: hidden on a Non-Subagent - OpenCode"
sidebar_label: "OC-AG-011"
description: "agnix rule OC-AG-011 checks for hidden on a non-subagent in opencode files. Severity: LOW. See examples and fix guidance."
keywords: ["OC-AG-011", "hidden on a non-subagent", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-AG-011`
- **Severity**: `LOW`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `<2.0.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/agents/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
mode: primary
hidden: true
```

### Valid

```json
mode: subagent
hidden: true
```
