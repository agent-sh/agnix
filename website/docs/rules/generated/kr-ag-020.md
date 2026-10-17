---
id: kr-ag-020
title: "KR-AG-020: Non-Agent File in Kiro Agents Directory"
sidebar_label: "KR-AG-020"
description: "agnix rule KR-AG-020 checks for non-agent file in kiro agents directory in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-020", "non-agent file in kiro agents directory", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-020`
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
- https://kiro.dev/docs/custom-agents/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"artifact": ".kiro/agents/review-agent.json", "spans": []}
```

### Valid

```json
{"name": "review-agent", "description": "Reviews code"}
```
