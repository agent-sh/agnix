---
id: cc-mcp-004
title: "CC-MCP-004: Non-boolean alwaysLoad in Claude Code MCP Config"
sidebar_label: "CC-MCP-004"
description: "agnix rule CC-MCP-004 checks for non-boolean alwaysload in claude code mcp config in claude mcp files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-MCP-004", "non-boolean alwaysload in claude code mcp config", "claude mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MCP-004`
- **Severity**: `MEDIUM`
- **Category**: `Claude MCP`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
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
    "fs": { "type": "stdio", "command": "node", "alwaysLoad": "true" }
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
