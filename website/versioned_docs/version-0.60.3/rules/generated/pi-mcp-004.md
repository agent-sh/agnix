---
id: pi-mcp-004
title: "PI-MCP-004: Colliding Pi MCP Namespaces - pi"
sidebar_label: "PI-MCP-004"
description: "agnix rule PI-MCP-004 checks for colliding pi mcp namespaces in pi files. Severity: HIGH. See examples and fix guidance."
keywords: ["PI-MCP-004", "colliding pi mcp namespaces", "pi", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `PI-MCP-004`
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
