---
id: kr-hk-001
title: "KR-HK-001: Invalid Kiro IDE Hook Event Type - Kiro Hooks"
sidebar_label: "KR-HK-001"
description: "agnix rule KR-HK-001 checks for invalid kiro ide hook event type in kiro hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-HK-001", "invalid kiro ide hook event type", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-001`
- **Severity**: `HIGH`
- **Category**: `Kiro Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `IDE 0.x .kiro.hook`

## Evidence Sources

- https://kiro.dev/docs/ide/0x-reference/
- https://kiro.dev/docs/ide/whats-new-v1/hooks/

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
    "type": "userPrompt"
  },
  "then": {
    "type": "askAgent",
    "prompt": "Review the change"
  }
}
```

### Valid

```json
{
  "when": {
    "type": "fileEdited",
    "patterns": [
      "src/**/*.ts"
    ]
  },
  "then": {
    "type": "askAgent",
    "prompt": "Review the change"
  }
}
```
