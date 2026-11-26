---
id: kr-ag-003
title: "KR-AG-003: allowedTools Trusts a Tool Not in tools"
sidebar_label: "KR-AG-003"
description: "agnix rule KR-AG-003 checks for allowedtools trusts a tool not in tools in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-003", "allowedtools trusts a tool not in tools", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-003`
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

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent", "tools": ["read"], "allowedTools": ["read", "@git/git_status"]}
```

### Valid

```json
{"name": "review-agent", "tools": ["@builtin", "@git"], "allowedTools": ["read", "@git/git_status"]}
```
