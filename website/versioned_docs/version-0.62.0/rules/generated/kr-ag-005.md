---
id: kr-ag-005
title: "KR-AG-005: Referenced MCP Server Not Loaded by Kiro CLI 2.x"
sidebar_label: "KR-AG-005"
description: "agnix rule KR-AG-005 checks for referenced mcp server not loaded by kiro cli 2.x in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-005", "referenced mcp server not loaded by kiro cli 2.x", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-005`
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
- https://kiro.dev/docs/mcp/configuration/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent", "includeMcpJson": false, "tools": ["read", "@git"]}
```

### Valid

```json
{"name": "review-agent", "includeMcpJson": true, "tools": ["read", "@git"]}
```
