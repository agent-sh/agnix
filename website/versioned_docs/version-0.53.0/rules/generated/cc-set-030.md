---
id: cc-set-030
title: "CC-SET-030: Local Command in managedMcpServers"
sidebar_label: "CC-SET-030"
description: "agnix rule CC-SET-030 checks for local command in managedmcpservers in claude settings files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-SET-030", "local command in managedmcpservers", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-030`
- **Severity**: `HIGH`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-09-06`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.259`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.259

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "managedMcpServers": {
    "local": { "command": "npx", "args": ["server"] }
  }
}
```

### Valid

```json
{
  "managedMcpServers": {
    "docs": { "type": "http", "url": "https://mcp.example.com" }
  }
}
```
