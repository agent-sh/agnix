---
id: cc-mcp-003
title: "CC-MCP-003: MCP Tool Description or Instructions Truncated"
sidebar_label: "CC-MCP-003"
description: "agnix rule CC-MCP-003 checks for mcp tool description or instructions truncated in claude mcp files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-MCP-003", "mcp tool description or instructions truncated", "claude mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MCP-003`
- **Severity**: `LOW`
- **Category**: `Claude MCP`
- **Normative Level**: `SHOULD`
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
  "tools": [{ "name": "search", "description": "<more than 4,096 characters>", "inputSchema": { "type": "object" } }]
}
```

### Valid

```json
{
  "tools": [{ "name": "search", "description": "Searches the docs index.", "inputSchema": { "type": "object" } }]
}
```
