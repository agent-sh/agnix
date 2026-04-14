---
id: kr-hk-006
title: "KR-HK-006: Kiro CLI Hook Missing Command - Kiro Hooks"
sidebar_label: "KR-HK-006"
description: "agnix rule KR-HK-006 checks for kiro cli hook missing command in kiro hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-HK-006", "kiro cli hook missing command", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-006`
- **Severity**: `HIGH`
- **Category**: `Kiro Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-02`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/cli/hooks

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "hooks": {
    "preToolUse": [{"toolTypes": ["readFiles"]}]
  }
}
```

### Valid

```json
{
  "hooks": {
    "preToolUse": [{"command": "echo pre", "toolTypes": ["readFiles"]}]
  }
}
```
