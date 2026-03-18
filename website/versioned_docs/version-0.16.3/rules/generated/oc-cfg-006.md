---
id: oc-cfg-006
title: "OC-CFG-006: Invalid MCP Server Structure - OpenCode"
sidebar_label: "OC-CFG-006"
description: "agnix rule OC-CFG-006 checks for invalid mcp server structure in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-CFG-006", "invalid mcp server structure", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-006`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-02`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/config

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "mcp": { "server": { "type": "invalid" } }
}
```

### Valid

```json
{
  "mcp": { "server": { "type": "local", "command": ["node"] } }
}
```
