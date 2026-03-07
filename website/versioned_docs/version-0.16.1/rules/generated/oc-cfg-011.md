---
id: oc-cfg-011
title: "OC-CFG-011: Invalid MCP Timeout - OpenCode"
sidebar_label: "OC-CFG-011"
description: "agnix rule OC-CFG-011 checks for invalid mcp timeout in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-CFG-011", "invalid mcp timeout", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-011`
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
  "mcp": { "srv": { "timeout": -5 } }
}
```

### Valid

```json
{
  "mcp": { "srv": { "timeout": 5000 } }
}
```
