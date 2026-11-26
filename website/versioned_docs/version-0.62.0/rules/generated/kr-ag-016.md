---
id: kr-ag-016
title: "KR-AG-016: Invalid Kiro Agent JSON - Kiro Agents"
sidebar_label: "KR-AG-016"
description: "agnix rule KR-AG-016 checks for invalid kiro agent json in kiro agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-AG-016", "invalid kiro agent json", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-016`
- **Severity**: `HIGH`
- **Category**: `Kiro Agents`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/custom-agents/configuration-reference/
- https://kiro.dev/docs/cli/v3/agent-config/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "name": "review-agent",
  "tools": ["read"],
}
```

### Valid

```json
{
  "name": "review-agent",
  "tools": ["read"]
}
```
