---
id: kr-mcp-003
title: "KR-MCP-003: Kiro MCP Command Includes Arguments - Kiro MCP"
sidebar_label: "KR-MCP-003"
description: "agnix rule KR-MCP-003 checks for kiro mcp command includes arguments in kiro mcp files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-MCP-003", "kiro mcp command includes arguments", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-003`
- **Severity**: `MEDIUM`
- **Category**: `Kiro MCP`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/mcp/configuration/
- https://kiro.dev/docs/custom-agents/configuration-reference/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"mcpServers": {"s": {"command": "npx -y @modelcontextprotocol/server-github"}}}
```

### Valid

```json
{"mcpServers": {"s": {"command": "npx", "args": ["-y", "@modelcontextprotocol/server-github"]}}}
```
