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
- **Verified On**: `2026-03-02`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/hooks/types

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "event": "fileEdited",
  "runCommand": "echo missing patterns"
}
```

### Valid

```json
{
  "event": "fileEdited",
  "patterns": ["**/*.ts"],
  "runCommand": "echo ok"
}
```
