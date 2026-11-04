---
id: oc-pm-006
title: "OC-PM-006: Shell Allow-All With Deny Patterns - OpenCode"
sidebar_label: "OC-PM-006"
description: "agnix rule OC-PM-006 checks for shell allow-all with deny patterns in opencode files. Severity: LOW. See examples and fix guidance."
keywords: ["OC-PM-006", "shell allow-all with deny patterns", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-PM-006`
- **Severity**: `LOW`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/v2/docs/permissions/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"permission": {"bash": {"*": "allow", "rm *": "deny"}}}
```

### Valid

```json
{"permission": {"bash": {"*": "ask", "git status *": "allow"}}}
```
