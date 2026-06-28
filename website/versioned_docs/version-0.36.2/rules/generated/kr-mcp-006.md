---
id: kr-mcp-006
title: "KR-MCP-006: Invalid OAuth Client ID Configuration - Kiro MCP"
sidebar_label: "KR-MCP-006"
description: "agnix rule KR-MCP-006 checks for invalid oauth client id configuration in kiro mcp files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-MCP-006", "invalid oauth client id configuration", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-006`
- **Severity**: `MEDIUM`
- **Category**: `Kiro MCP`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-05-14`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.3.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/changelog/cli/2-3/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"mcpServers": {"local": {"command": "node", "args": ["server.js"], "oauth": {"clientId": "registered-client"}}}}
```

### Valid

```json
{"mcpServers": {"remote": {"url": "https://example.com/mcp", "oauth": {"clientId": "registered-client"}}}}
```
