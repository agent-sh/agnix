---
id: kr-ag-005
title: "KR-AG-005: Kiro Agent Has No MCP Access - Kiro Agents"
sidebar_label: "KR-AG-005"
description: "agnix rule KR-AG-005 checks for kiro agent has no mcp access in kiro agents files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-AG-005", "kiro agent has no mcp access", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-005`
- **Severity**: `LOW`
- **Category**: `Kiro Agents`
- **Normative Level**: `BEST_PRACTICE`
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
  "includeMcpJson": false
}
```

### Valid

```json
{
  "includeMcpJson": true
}
```
