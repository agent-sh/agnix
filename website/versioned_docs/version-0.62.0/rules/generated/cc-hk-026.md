---
id: cc-hk-026
title: "CC-HK-026: MCP Tool Hook Missing Server - Claude Hooks"
sidebar_label: "CC-HK-026"
description: "agnix rule CC-HK-026 checks for mcp tool hook missing server in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-026", "mcp tool hook missing server", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-026`
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
{ "type": "mcp_tool", "tool": "security_scan" }
```

### Valid

```json
{ "type": "mcp_tool", "server": "my_server", "tool": "security_scan" }
```
