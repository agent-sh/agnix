---
id: kr-mcp-004
title: "KR-MCP-004: Invalid MCP URL - Kiro MCP"
sidebar_label: "KR-MCP-004"
description: "agnix rule KR-MCP-004 checks for invalid mcp url in kiro mcp files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-MCP-004", "invalid mcp url", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-004`
- **Severity**: `HIGH`
- **Category**: `Kiro MCP`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-06`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/mcp

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"mcpServers": {"remote": {"url": "not-a-url"}}}
```

### Valid

```json
{"mcpServers": {"remote": {"url": "https://example.com/mcp"}}}
```
