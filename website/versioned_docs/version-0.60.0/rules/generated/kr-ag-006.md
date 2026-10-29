---
id: kr-ag-006
title: "KR-AG-006: Kiro Agent Sub-agent Access Cannot Work"
sidebar_label: "KR-AG-006"
description: "agnix rule KR-AG-006 checks for kiro agent sub-agent access cannot work in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-006", "kiro agent sub-agent access cannot work", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-006`
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

- https://kiro.dev/docs/custom-agents/subagents/
- https://kiro.dev/docs/custom-agents/configuration-reference/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "lead", "tools": ["read"], "toolsSettings": {"subagent": {"availableAgents": ["reviewer"]}}}
```

### Valid

```json
{"name": "lead", "tools": ["read", "subagent"], "toolsSettings": {"subagent": {"availableAgents": ["reviewer"]}}}
```
