---
id: kr-mcp-002
title: "KR-MCP-002: Hardcoded Secrets in Kiro MCP env - Kiro MCP"
sidebar_label: "KR-MCP-002"
description: "agnix rule KR-MCP-002 checks for hardcoded secrets in kiro mcp env in kiro mcp files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-MCP-002", "hardcoded secrets in kiro mcp env", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-002`
- **Severity**: `MEDIUM`
- **Category**: `Kiro MCP`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-02`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/mcp/configuration

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "mcpServers": {
    "server": {
      "command": "node",
      "env": {"API_KEY": "hardcoded-secret"}
    }
  }
}
```

### Valid

```json
{
  "mcpServers": {
    "server": {
      "command": "node",
      "env": {"API_KEY": "${API_KEY}"}
    }
  }
}
```
