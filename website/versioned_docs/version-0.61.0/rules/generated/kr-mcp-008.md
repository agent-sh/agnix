---
id: kr-mcp-008
title: "KR-MCP-008: Kiro MCP Remote URL Uses Plain HTTP - Kiro MCP"
sidebar_label: "KR-MCP-008"
description: "agnix rule KR-MCP-008 checks for kiro mcp remote url uses plain http in kiro mcp files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-MCP-008", "kiro mcp remote url uses plain http", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-008`
- **Severity**: `HIGH`
- **Category**: `Kiro MCP`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `Kiro V3 engine (opt-in from CLI 2.8.0, default from CLI 3.0)`

## Evidence Sources

- https://kiro.dev/docs/mcp/configuration/
- https://kiro.dev/docs/custom-agents/configuration-reference/
- https://kiro.dev/docs/mcp/security/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"mcpServers": {"s": {"url": "http://mcp.example.com/mcp"}}}
```

### Valid

```json
{"mcpServers": {"s": {"url": "https://mcp.example.com/mcp"}}}
```
