---
id: kr-ag-013
title: "KR-AG-013: Secrets in Agent Prompt - Kiro Agents"
sidebar_label: "KR-AG-013"
description: "agnix rule KR-AG-013 checks for secrets in agent prompt in kiro agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-AG-013", "secrets in agent prompt", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-013`
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
{"prompt": "API_KEY=sk-live-secret123"}
```

### Valid

```json
{"prompt": "Use ${API_KEY} from env"}
```
