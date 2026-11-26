---
id: oc-cfg-021
title: "OC-CFG-021: OpenCode 1 Field in an mcp.servers Entry"
sidebar_label: "OC-CFG-021"
description: "agnix rule OC-CFG-021 checks for opencode 1 field in an mcp.servers entry in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-CFG-021", "opencode 1 field in an mcp.servers entry", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-021`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `>=2.0.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/v2/docs/mcp-servers/
- https://opencode.ai/v2/docs/migrate-v1/
- https://opencode.ai/config.json

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"mcp": {"servers": {"x": {"type": "local", "command": ["x"], "enabled": false}}}}
```

### Valid

```json
{"mcp": {"servers": {"x": {"type": "local", "command": ["x"], "disabled": true}}}}
```
