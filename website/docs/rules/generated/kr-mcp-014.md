---
id: kr-mcp-014
title: "KR-MCP-014: Invalid Agent Plugins MCP Server Entry"
sidebar_label: "KR-MCP-014"
description: "agnix rule KR-MCP-014 checks for invalid agent plugins mcp server entry in kiro mcp files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-MCP-014", "invalid agent plugins mcp server entry", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-014`
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

- https://kiro.dev/docs/powers/create/
- https://agent-plugins.org/schemas/1.0.0/mcp.schema.json
- https://github.com/kirodotdev/powers/blob/e974b7be3c5d83b0b1cd6e3fa71a9a04196803e0/power-builder/skills/migrate-plugin/SKILL.md

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"$schema": "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json", "mcpServers": {"docs": {"command": "uvx", "disabled": false}}}
```

### Valid

```json
{"$schema": "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json", "mcpServers": {"docs": {"type": "stdio", "command": "uvx", "args": ["docs-mcp"]}}}
```
