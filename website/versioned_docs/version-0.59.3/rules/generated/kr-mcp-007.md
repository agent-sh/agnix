---
id: kr-mcp-007
title: "KR-MCP-007: Invalid Kiro MCP Server Field Type - Kiro MCP"
sidebar_label: "KR-MCP-007"
description: "agnix rule KR-MCP-007 checks for invalid kiro mcp server field type in kiro mcp files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-MCP-007", "invalid kiro mcp server field type", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-007`
- **Severity**: `HIGH`
- **Category**: `Kiro MCP`
- **Normative Level**: `MUST`
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
{"mcpServers": {"s": {"command": "node", "args": "server.js", "timeout": 0}}}
```

### Valid

```json
{"mcpServers": {"s": {"command": "node", "args": ["server.js"], "timeout": 120000}}}
```
