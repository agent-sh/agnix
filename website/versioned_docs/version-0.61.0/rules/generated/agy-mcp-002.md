---
id: agy-mcp-002
title: "AGY-MCP-002: Unsupported Antigravity MCP Transport"
sidebar_label: "AGY-MCP-002"
description: "agnix rule AGY-MCP-002 checks for unsupported antigravity mcp transport in antigravity files. Severity: HIGH. See examples and fix guidance."
keywords: ["AGY-MCP-002", "unsupported antigravity mcp transport", "antigravity", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `AGY-MCP-002`
- **Severity**: `HIGH`
- **Category**: `antigravity`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-29`

## Applicability

- **Tool**: `antigravity`
- **Version Range**: `>=1.4.9`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://antigravity.google/docs/mcp

## Test Coverage Metadata

- Unit tests: `false`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
Configuration omitted required fields for this rule.
```

### Valid

```text
Configuration includes required fields and follows the rule.
```
