---
id: oc-pm-005
title: "OC-PM-005: OpenCode 1 Action Name in a permissions Array"
sidebar_label: "OC-PM-005"
description: "agnix rule OC-PM-005 checks for opencode 1 action name in a permissions array in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-PM-005", "opencode 1 action name in a permissions array", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-PM-005`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `>=2.0.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/v2/docs/migrate-v1/
- https://opencode.ai/v2/docs/permissions/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"action": "bash", "resource": "git push *", "effect": "ask"}
```

### Valid

```json
{"action": "shell", "resource": "git push *", "effect": "ask"}
```
