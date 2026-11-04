---
id: kr-ag-012
title: "KR-AG-012: toolAliases Renames a Tool Not in tools"
sidebar_label: "KR-AG-012"
description: "agnix rule KR-AG-012 checks for toolaliases renames a tool not in tools in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-012", "toolaliases renames a tool not in tools", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-012`
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

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent", "tools": ["read"], "toolAliases": {"@github-mcp/get_issues": "github_issues"}}
```

### Valid

```json
{"name": "review-agent", "tools": ["@github-mcp"], "toolAliases": {"@github-mcp/get_issues": "github_issues"}}
```
