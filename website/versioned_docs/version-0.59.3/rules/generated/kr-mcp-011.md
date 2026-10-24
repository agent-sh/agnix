---
id: kr-mcp-011
title: "KR-MCP-011: Kiro MCP Server Has Both command and url"
sidebar_label: "KR-MCP-011"
description: "agnix rule KR-MCP-011 checks for kiro mcp server has both command and url in kiro mcp files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-MCP-011", "kiro mcp server has both command and url", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-011`
- **Severity**: `LOW`
- **Category**: `Kiro MCP`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `Kiro V3 engine (opt-in from CLI 2.8.0, default from CLI 3.0)`

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
{"mcpServers": {"s": {"command": "node", "url": "https://example.com/mcp"}}}
```

### Valid

```json
{"mcpServers": {"s": {"url": "https://example.com/mcp"}}}
```
