---
id: kr-ag-001
title: "KR-AG-001: Unknown or Ignored Kiro Agent Field - Kiro Agents"
sidebar_label: "KR-AG-001"
description: "agnix rule KR-AG-001 checks for unknown or ignored kiro agent field in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-001", "unknown or ignored kiro agent field", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-001`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Agents`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/custom-agents/configuration-reference/
- https://kiro.dev/docs/cli/v3/agent-config/
- https://kiro.dev/docs/cli/v3/migration-guide/

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
  "useLegacyMcpJson": false
}
```
