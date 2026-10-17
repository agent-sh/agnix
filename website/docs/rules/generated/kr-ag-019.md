---
id: kr-ag-019
title: "KR-AG-019: Kiro Agent Missing Name for CLI 2.x - Kiro Agents"
sidebar_label: "KR-AG-019"
description: "agnix rule KR-AG-019 checks for kiro agent missing name for cli 2.x in kiro agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-AG-019", "kiro agent missing name for cli 2.x", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-019`
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
- https://kiro.dev/docs/cli/2x-reference/
- https://kiro.dev/docs/cli/v3/agent-config/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"description": "Reviews code", "allowedTools": ["read"]}
```

### Valid

```json
{"name": "review-agent", "allowedTools": ["read"]}
```
