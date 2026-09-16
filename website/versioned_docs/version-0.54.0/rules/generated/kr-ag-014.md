---
id: kr-ag-014
title: "KR-AG-014: Invalid Universal Permissions Rule - Kiro Agents"
sidebar_label: "KR-AG-014"
description: "agnix rule KR-AG-014 checks for invalid universal permissions rule in kiro agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-AG-014", "invalid universal permissions rule", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-014`
- **Severity**: `HIGH`
- **Category**: `Kiro Agents`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-26`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.14.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/changelog/cli/2-14/
- https://kiro.dev/docs/cli/v3/agent-config/
- https://kiro.dev/docs/cli/v3/permissions/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"permissions": {"rules": [{"capability": "shell", "effect": "sometimes", "match": "npm *"}]}}
```

### Valid

```json
{"permissions": {"rules": [{"capability": "shell", "effect": "allow", "match": ["npm *"]}]}}
```
