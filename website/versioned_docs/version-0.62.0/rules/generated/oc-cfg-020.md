---
id: oc-cfg-020
title: "OC-CFG-020: Many Enabled MCP Servers - OpenCode"
sidebar_label: "OC-CFG-020"
description: "agnix rule OC-CFG-020 checks for many enabled mcp servers in opencode files. Severity: LOW. See examples and fix guidance."
keywords: ["OC-CFG-020", "many enabled mcp servers", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-020`
- **Severity**: `LOW`
- **Category**: `OpenCode`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/mcp-servers/
- https://opencode.ai/v2/docs/mcp-servers/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
nine enabled servers under mcp
```

### Valid

```json
{"mcp": {"docs": {"type": "remote", "url": "https://x.example/mcp"}}}
```
