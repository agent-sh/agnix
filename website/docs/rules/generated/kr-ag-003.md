---
id: kr-ag-003
title: "KR-AG-003: allowedTools Not Subset of tools - Kiro Agents"
sidebar_label: "KR-AG-003"
description: "agnix rule KR-AG-003 checks for allowedtools not subset of tools in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-003", "allowedtools not subset of tools", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-003`
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
  "tools": ["readFiles"],
  "allowedTools": ["runShellCommand"]
}
```

### Valid

```json
{
  "tools": ["readFiles", "listDirectory"],
  "allowedTools": ["readFiles"]
}
```
