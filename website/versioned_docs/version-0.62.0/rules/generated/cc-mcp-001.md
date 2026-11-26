---
id: cc-mcp-001
title: "CC-MCP-001: Credential Variable Reads as Empty in Remote MCP Server"
sidebar_label: "CC-MCP-001"
description: "agnix rule CC-MCP-001 checks for credential variable reads as empty in remote mcp server in claude mcp files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-MCP-001", "credential variable reads as empty in remote mcp server", "claude mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MCP-001`
- **Severity**: `HIGH`
- **Category**: `Claude MCP`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/mcp

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
    "gw": { "type": "http", "url": "https://gw.example.com/mcp", "headers": { "Authorization": "Bearer ${ANTHROPIC_AUTH_TOKEN}" } }
  }
}
```

### Valid

```json
{
  "mcpServers": {
    "gw": { "type": "http", "url": "https://gw.example.com/mcp", "headers": { "Authorization": "Bearer ${GW_TOKEN}" } }
  }
}
```
