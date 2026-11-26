---
id: oc-cfg-023
title: "OC-CFG-023: MCP Server pool Flag Not a Boolean - OpenCode"
sidebar_label: "OC-CFG-023"
description: "agnix rule OC-CFG-023 checks for mcp server pool flag not a boolean in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-CFG-023", "mcp server pool flag not a boolean", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-023`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-11-08`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `>=2.2.6`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anomalyco/opencode/releases/tag/v2.2.6
- https://github.com/anomalyco/opencode/blob/v2.2.6/packages/schema/src/mcp.ts
- https://opencode.ai/v2/docs/mcp-servers/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{ "mcp": { "servers": { "shared": { "type": "local", "command": ["x"], "pool": "true" } } } }
```

### Valid

```json
{ "mcp": { "servers": { "shared": { "type": "local", "command": ["x"], "pool": true } } } }
```
