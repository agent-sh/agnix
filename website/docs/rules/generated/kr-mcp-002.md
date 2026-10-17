---
id: kr-mcp-002
title: "KR-MCP-002: Hardcoded Secret in Kiro MCP Server Config"
sidebar_label: "KR-MCP-002"
description: "agnix rule KR-MCP-002 checks for hardcoded secret in kiro mcp server config in kiro mcp files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-MCP-002", "hardcoded secret in kiro mcp server config", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-002`
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
- https://kiro.dev/docs/mcp/security/
- https://kiro.dev/docs/powers/create/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"mcpServers": {"s": {"command": "npx", "args": ["-y", "server"], "env": {"API_KEY": "x7Rf2kLp9QwZ3mTnB5vD"}}}}
```

### Valid

```json
{"mcpServers": {"s": {"command": "npx", "args": ["-y", "server"], "env": {"API_KEY": "${API_KEY}"}}}}
```
