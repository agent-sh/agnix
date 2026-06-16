---
id: oc-cfg-007
title: "OC-CFG-007: Invalid MCP Server Command, URL, or cwd"
sidebar_label: "OC-CFG-007"
description: "agnix rule OC-CFG-007 checks for invalid mcp server command, url, or cwd in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-CFG-007", "invalid mcp server command, url, or cwd", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-007`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-06-16`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/config
- https://github.com/sst/opencode/pull/30676

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "mcp": { "server": { "type": "local", "command": ["node"], "cwd": 42 } }
}
```

### Valid

```json
{
  "mcp": { "server": { "type": "local", "command": ["node"], "cwd": "plugins/sub" } }
}
```
