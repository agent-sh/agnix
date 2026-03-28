---
id: oc-cfg-007
title: "OC-CFG-007: MCP Server Missing Command or URL - OpenCode"
sidebar_label: "OC-CFG-007"
description: "agnix rule OC-CFG-007 checks for mcp server missing command or url in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-CFG-007", "mcp server missing command or url", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-007`
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
  "mcp": { "server": { "type": "local" } }
}
```

### Valid

```json
{
  "mcp": { "server": { "type": "local", "command": ["node"] } }
}
```
