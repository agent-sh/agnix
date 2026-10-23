---
id: oc-cfg-007
title: "OC-CFG-007: Invalid MCP Server Command, URL, cwd, or Environment"
sidebar_label: "OC-CFG-007"
description: "agnix rule OC-CFG-007 checks for invalid mcp server command, url, cwd, or environment in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-CFG-007", "invalid mcp server command, url, cwd, or environment", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-007`
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
{"mcp": {"x": {"type": "remote"}}}
```

### Valid

```json
{"mcp": {"x": {"type": "remote", "url": "https://mcp.example.com/mcp"}}}
```
