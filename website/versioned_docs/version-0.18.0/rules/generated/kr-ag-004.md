---
id: kr-ag-004
title: "KR-AG-004: Invalid Kiro Agent Model Value - Kiro Agents"
sidebar_label: "KR-AG-004"
description: "agnix rule KR-AG-004 checks for invalid kiro agent model value in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-004", "invalid kiro agent model value", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-004`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Agents`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-02`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/cli/custom-agents/configuration-reference

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "model": "unsupported-model"
}
```

### Valid

```json
{
  "model": "claude-sonnet-4-5"
}
```
