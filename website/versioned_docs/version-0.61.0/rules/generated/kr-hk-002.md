---
id: kr-hk-002
title: "KR-HK-002: Kiro File Hook Missing Patterns - Kiro Hooks"
sidebar_label: "KR-HK-002"
description: "agnix rule KR-HK-002 checks for kiro file hook missing patterns in kiro hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-HK-002", "kiro file hook missing patterns", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-002`
- **Severity**: `HIGH`
- **Category**: `Kiro Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `IDE 0.x .kiro.hook`

## Evidence Sources

- https://kiro.dev/docs/ide/0x-reference/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "when": {
    "type": "fileEdited"
  },
  "then": {
    "type": "runCommand",
    "command": "npm run lint"
  }
}
```

### Valid

```json
{
  "when": {
    "type": "fileEdited",
    "patterns": [
      "**/*.ts"
    ]
  },
  "then": {
    "type": "runCommand",
    "command": "npm run lint"
  }
}
```
