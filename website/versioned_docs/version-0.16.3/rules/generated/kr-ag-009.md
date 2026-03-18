---
id: kr-ag-009
title: "KR-AG-009: Agent Missing Prompt - Kiro Agents"
sidebar_label: "KR-AG-009"
description: "agnix rule KR-AG-009 checks for agent missing prompt in kiro agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-AG-009", "agent missing prompt", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-009`
- **Severity**: `HIGH`
- **Category**: `Kiro Agents`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-06`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/agents
- https://kiro.dev/docs/configuration

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent"}
```

### Valid

```json
{"name": "review-agent", "prompt": "Review code"}
```
