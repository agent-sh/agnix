---
id: kr-hk-004
title: "KR-HK-004: Kiro Tool Hook Missing toolTypes Filter"
sidebar_label: "KR-HK-004"
description: "agnix rule KR-HK-004 checks for kiro tool hook missing tooltypes filter in kiro hooks files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-HK-004", "kiro tool hook missing tooltypes filter", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-004`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Hooks`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `IDE 0.x .kiro.hook`

## Evidence Sources

- https://kiro.dev/docs/ide/0x-reference/
- https://kiro.dev/docs/hooks/best-practices/

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
    "type": "preToolUse"
  },
  "then": {
    "type": "askAgent",
    "prompt": "Check every tool"
  }
}
```

### Valid

```json
{
  "when": {
    "type": "preToolUse",
    "patterns": [
      "fs_write"
    ]
  },
  "then": {
    "type": "askAgent",
    "prompt": "Check the write"
  }
}
```
