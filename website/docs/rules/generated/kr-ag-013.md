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
{"name": "review-agent", "prompt": "api_key = sk-live-0123456789abcdefghij"}
```

### Valid

```json
{"name": "review-agent", "prompt": "Use the API key from ${API_KEY}"}
```
