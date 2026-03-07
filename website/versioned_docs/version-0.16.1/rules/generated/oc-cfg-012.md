---
id: oc-cfg-012
title: "OC-CFG-012: Invalid MCP OAuth Config - OpenCode"
sidebar_label: "OC-CFG-012"
description: "agnix rule OC-CFG-012 checks for invalid mcp oauth config in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-CFG-012", "invalid mcp oauth config", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-012`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-06`

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
  "mcp": { "srv": { "oauth": { "client_id": "abc" } } }
}
```

### Valid

```json
{
  "mcp": { "srv": { "oauth": { "client_id": "abc", "authorization_url": "https://auth.example.com" } } }
}
```
