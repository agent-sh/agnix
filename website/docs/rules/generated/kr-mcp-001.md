---
id: kr-mcp-001
title: "KR-MCP-001: Kiro MCP Config Fails to Load - Kiro MCP"
sidebar_label: "KR-MCP-001"
description: "agnix rule KR-MCP-001 checks for kiro mcp config fails to load in kiro mcp files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-MCP-001", "kiro mcp config fails to load", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-001`
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

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"mcpServers": {"broken": {"args": ["--debug"]}}}
```

### Valid

```json
{"mcpServers": {"local": {"command": "node", "args": ["server.js"]}}}
```
