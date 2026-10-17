---
id: kr-ag-009
title: "KR-AG-009: Agent Prompt Empty or Invalid - Kiro Agents"
sidebar_label: "KR-AG-009"
description: "agnix rule KR-AG-009 checks for agent prompt empty or invalid in kiro agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-AG-009", "agent prompt empty or invalid", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-009`
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
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent", "prompt": ["Review code"]}
```

### Valid

```json
{"name": "review-agent", "prompt": "Review code"}
```
