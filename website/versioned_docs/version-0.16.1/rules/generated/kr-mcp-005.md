---
id: kr-mcp-005
title: "KR-MCP-005: Duplicate MCP Server Names - Kiro MCP"
sidebar_label: "KR-MCP-005"
description: "agnix rule KR-MCP-005 checks for duplicate mcp server names in kiro mcp files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-MCP-005", "duplicate mcp server names", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-005`
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
# Two servers with same name in different config files
```

### Valid

```json
{"mcpServers": {"a": {"command": "a"}, "b": {"command": "b"}}}
```
