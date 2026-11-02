---
id: cc-mcp-005
title: "CC-MCP-005: Reserved MCP Server Name - Claude MCP"
sidebar_label: "CC-MCP-005"
description: "agnix rule CC-MCP-005 checks for reserved mcp server name in claude mcp files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-MCP-005", "reserved mcp server name", "claude mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MCP-005`
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
    "workspace": { "type": "stdio", "command": "node" }
  }
}
```

### Valid

```json
{
  "mcpServers": {
    "fs": { "type": "stdio", "command": "node", "args": ["${CLAUDE_PROJECT_DIR:-.}/server.js"], "alwaysLoad": true }
  }
}
```
