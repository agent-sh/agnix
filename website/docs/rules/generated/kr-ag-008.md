---
id: kr-ag-008
title: "KR-AG-008: Invalid Explicit Agent Name - Kiro Agents"
sidebar_label: "KR-AG-008"
description: "agnix rule KR-AG-008 checks for invalid explicit agent name in kiro agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-AG-008", "invalid explicit agent name", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-008`
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
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": ""}
```

### Valid

```json
{"name": "review-agent"}
```
