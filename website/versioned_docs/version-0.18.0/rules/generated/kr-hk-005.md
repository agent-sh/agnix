---
id: kr-hk-005
title: "KR-HK-005: Invalid Kiro CLI Hook Event Key - Kiro Hooks"
sidebar_label: "KR-HK-005"
description: "agnix rule KR-HK-005 checks for invalid kiro cli hook event key in kiro hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-HK-005", "invalid kiro cli hook event key", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-005`
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
    "beforePrompt": [{"command": "echo bad"}]
  }
}
```

### Valid

```json
{
  "hooks": {
    "preToolUse": [{"command": "echo pre"}]
  }
}
```
