---
id: kr-mcp-003
title: "KR-MCP-003: Missing Required Args - Kiro MCP"
sidebar_label: "KR-MCP-003"
description: "agnix rule KR-MCP-003 checks for missing required args in kiro mcp files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-MCP-003", "missing required args", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-003`
- **Severity**: `MEDIUM`
- **Category**: `Kiro MCP`
- **Normative Level**: `SHOULD`
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
{"mcpServers": {"fs": {"command": "node"}}}
```

### Valid

```json
{"mcpServers": {"fs": {"command": "node", "args": ["server.js"]}}}
```
