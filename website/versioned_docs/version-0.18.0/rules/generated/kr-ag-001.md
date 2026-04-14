---
id: kr-ag-001
title: "KR-AG-001: Unknown Field in Kiro Agent JSON - Kiro Agents"
sidebar_label: "KR-AG-001"
description: "agnix rule KR-AG-001 checks for unknown field in kiro agent json in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-001", "unknown field in kiro agent json", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-001`
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
  "name": "review-agent",
  "madeUpField": true
}
```

### Valid

```json
{
  "name": "review-agent",
  "prompt": "Review the diff",
  "model": "claude-sonnet-4"
}
```
