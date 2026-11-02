---
id: kr-ag-010
title: "KR-AG-010: Duplicate Tool Entries - Kiro Agents"
sidebar_label: "KR-AG-010"
description: "agnix rule KR-AG-010 checks for duplicate tool entries in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-010", "duplicate tool entries", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-010`
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

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent", "tools": ["read", "read"]}
```

### Valid

```json
{"name": "review-agent", "tools": ["read", "shell"]}
```
