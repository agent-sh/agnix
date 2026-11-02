---
id: kr-ag-004
title: "KR-AG-004: Kiro Agent Model Is Not a Kiro Model ID"
sidebar_label: "KR-AG-004"
description: "agnix rule KR-AG-004 checks for kiro agent model is not a kiro model id in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-004", "kiro agent model is not a kiro model id", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-004`
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
- https://kiro.dev/docs/models/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent", "model": "sonnet"}
```

### Valid

```json
{"name": "review-agent", "model": "claude-sonnet-4.6"}
```
