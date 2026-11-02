---
id: pi-mcp-005
title: "PI-MCP-005: Invalid Pi MCP OAuth Configuration - pi"
sidebar_label: "PI-MCP-005"
description: "agnix rule PI-MCP-005 checks for invalid pi mcp oauth configuration in pi files. Severity: HIGH. See examples and fix guidance."
keywords: ["PI-MCP-005", "invalid pi mcp oauth configuration", "pi", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `PI-MCP-005`
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
