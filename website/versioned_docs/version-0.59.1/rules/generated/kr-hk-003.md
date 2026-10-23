---
id: kr-hk-003
title: "KR-HK-003: Kiro IDE Hook Missing Action - Kiro Hooks"
sidebar_label: "KR-HK-003"
description: "agnix rule KR-HK-003 checks for kiro ide hook missing action in kiro hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-HK-003", "kiro ide hook missing action", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-003`
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
    "type": "promptSubmit"
  },
  "then": {
    "type": "askAgent"
  }
}
```

### Valid

```json
{
  "when": {
    "type": "promptSubmit"
  },
  "then": {
    "type": "askAgent",
    "prompt": "Check the runbooks first"
  }
}
```
