---
id: cc-mcp-002
title: "CC-MCP-002: CLAUDE_PROJECT_DIR Without Default in Project .mcp.json"
sidebar_label: "CC-MCP-002"
description: "agnix rule CC-MCP-002 checks for claude_project_dir without default in project .mcp.json in claude mcp files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-MCP-002", "claude_project_dir without default in project .mcp.json", "claude mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MCP-002`
- **Severity**: `MEDIUM`
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
    "fs": { "type": "stdio", "command": "${CLAUDE_PROJECT_DIR}/bin/server" }
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
