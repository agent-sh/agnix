---
id: kr-mcp-004
title: "KR-MCP-004: Invalid Kiro MCP Server URL - Kiro MCP"
sidebar_label: "KR-MCP-004"
description: "agnix rule KR-MCP-004 checks for invalid kiro mcp server url in kiro mcp files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-MCP-004", "invalid kiro mcp server url", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-004`
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
- https://kiro.dev/docs/mcp/security/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"mcpServers": {"s": {"url": "wss://example.com/mcp"}}}
```

### Valid

```json
{"mcpServers": {"s": {"url": "https://example.com/mcp"}}}
```
