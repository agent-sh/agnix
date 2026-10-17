---
id: kr-ag-018
title: "KR-AG-018: Kiro Agent Field Has the Wrong Type - Kiro Agents"
sidebar_label: "KR-AG-018"
description: "agnix rule KR-AG-018 checks for kiro agent field has the wrong type in kiro agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-AG-018", "kiro agent field has the wrong type", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-018`
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
{"name": "review-agent", "tools": "read", "includeMcpJson": "yes"}
```

### Valid

```json
{"name": "review-agent", "tools": ["read"], "includeMcpJson": true}
```
