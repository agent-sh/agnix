---
id: kr-mcp-005
title: "KR-MCP-005: Duplicate Kiro MCP Server Name in One File"
sidebar_label: "KR-MCP-005"
description: "agnix rule KR-MCP-005 checks for duplicate kiro mcp server name in one file in kiro mcp files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-MCP-005", "duplicate kiro mcp server name in one file", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-005`
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
{"mcpServers": {"git": {"command": "uvx"}, "git": {"command": "npx"}}}
```

### Valid

```json
{"mcpServers": {"git": {"command": "uvx", "args": ["mcp-server-git"]}}}
```
