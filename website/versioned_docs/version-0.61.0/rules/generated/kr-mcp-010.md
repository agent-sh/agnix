---
id: kr-mcp-010
title: "KR-MCP-010: Unknown Kiro MCP Server Field - Kiro MCP"
sidebar_label: "KR-MCP-010"
description: "agnix rule KR-MCP-010 checks for unknown kiro mcp server field in kiro mcp files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-MCP-010", "unknown kiro mcp server field", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-010`
- **Severity**: `LOW`
- **Category**: `Kiro MCP`
- **Normative Level**: `BEST_PRACTICE`
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
{"mcpServers": {"s": {"command": "uvx", "args": ["server"], "evn": {"LOG": "info"}}}}
```

### Valid

```json
{"mcpServers": {"s": {"command": "uvx", "args": ["server"], "env": {"LOG": "info"}}}}
```
