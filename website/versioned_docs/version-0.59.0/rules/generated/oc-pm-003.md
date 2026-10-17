---
id: oc-pm-003
title: "OC-PM-003: Permission Rule Shadowed by Later Catch-All"
sidebar_label: "OC-PM-003"
description: "agnix rule OC-PM-003 checks for permission rule shadowed by later catch-all in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-PM-003", "permission rule shadowed by later catch-all", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-PM-003`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/permissions/
- https://opencode.ai/v2/docs/permissions/
- https://opencode.ai/v2/docs/policies/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"permission": {"bash": {"git status *": "allow", "*": "ask"}}}
```

### Valid

```json
{"permission": {"bash": {"*": "ask", "git status *": "allow"}}}
```
