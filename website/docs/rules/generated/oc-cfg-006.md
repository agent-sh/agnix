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
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/mcp-servers/
- https://opencode.ai/v2/docs/mcp-servers/
- https://opencode.ai/config.json

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"mcp": {"pw": {"command": ["npx", "@playwright/mcp"]}}}
```

### Valid

```json
{"mcp": {"servers": {"pw": {"type": "local", "command": ["npx", "@playwright/mcp"]}}}}
```
