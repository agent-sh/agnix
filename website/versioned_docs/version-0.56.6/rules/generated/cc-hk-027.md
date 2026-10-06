---
id: cc-hk-027
title: "CC-HK-027: MCP Tool Hook Missing Tool - Claude Hooks"
sidebar_label: "CC-HK-027"
description: "agnix rule CC-HK-027 checks for mcp tool hook missing tool in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-027", "mcp tool hook missing tool", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-027`
- **Severity**: `HIGH`
- **Category**: `Claude Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-04-26`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/hooks

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{ "type": "mcp_tool", "server": "my_server" }
```

### Valid

```json
{ "type": "mcp_tool", "server": "my_server", "tool": "security_scan" }
```
