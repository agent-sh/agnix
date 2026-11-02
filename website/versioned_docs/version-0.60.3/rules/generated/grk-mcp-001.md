---
id: grk-mcp-001
title: "GRK-MCP-001: Invalid Grok MCP Server - grok"
sidebar_label: "GRK-MCP-001"
description: "agnix rule GRK-MCP-001 checks for invalid grok mcp server in grok files. Severity: HIGH. See examples and fix guidance."
keywords: ["GRK-MCP-001", "invalid grok mcp server", "grok", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `GRK-MCP-001`
- **Severity**: `HIGH`
- **Category**: `grok`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-29`

## Applicability

- **Tool**: `grok-code`
- **Version Range**: `>=1.0.70`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/xai-org/grok-build/blob/2bdd1d6a6369de0e8c68132ea4539e9abd9e14a8/crates/codegen/xai-grok-config/src/mcp_server_config.rs
- https://docs.x.ai/build/features/mcp-servers

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
