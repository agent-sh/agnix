---
id: kr-ag-011
title: "KR-AG-011: Empty Tools Array - Kiro Agents"
sidebar_label: "KR-AG-011"
description: "agnix rule KR-AG-011 checks for empty tools array in kiro agents files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-AG-011", "empty tools array", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-011`
- **Severity**: `LOW`
- **Category**: `Kiro Agents`
- **Normative Level**: `BEST_PRACTICE`
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
{"name": "review-agent", "tools": []}
```

### Valid

```json
{"name": "review-agent", "tools": ["read"]}
```
