---
id: kr-ag-024
title: "KR-AG-024: Kiro Agent Tool Name Matches No Tool"
sidebar_label: "KR-AG-024"
description: "agnix rule KR-AG-024 checks for kiro agent tool name matches no tool in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-024", "kiro agent tool name matches no tool", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-024`
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
- https://kiro.dev/docs/reference/built-in-tools/
- https://kiro.dev/docs/tools/
- https://kiro.dev/docs/cli/v3/migration-guide/
- https://kiro.dev/docs/cli/v3/upgrade-agent/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent", "tools": ["read", "shell"], "allowedTools": ["Read", "mcp__jira__get_ticket"]}
```

### Valid

```json
{"name": "review-agent", "tools": ["read", "shell"], "allowedTools": ["read", "@jira/get_ticket"]}
```
