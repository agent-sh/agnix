---
id: pi-mcp-003
title: "PI-MCP-003: Invalid Pi Provider Authentication Scope - pi"
sidebar_label: "PI-MCP-003"
description: "agnix rule PI-MCP-003 checks for invalid pi provider authentication scope in pi files. Severity: HIGH. See examples and fix guidance."
keywords: ["PI-MCP-003", "invalid pi provider authentication scope", "pi", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `PI-MCP-003`
- **Severity**: `HIGH`
- **Category**: `pi`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-29`

## Applicability

- **Tool**: `pi`
- **Version Range**: `>=1.4.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/docs/mcp.md
- https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/src/core/mcp-servers.ts

## Test Coverage Metadata

- Unit tests: `false`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
Configuration omitted required fields for this rule.
```

### Valid

```text
Configuration includes required fields and follows the rule.
```
