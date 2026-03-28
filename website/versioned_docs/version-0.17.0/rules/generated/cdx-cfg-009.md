---
id: cdx-cfg-009
title: "CDX-CFG-009: Invalid MCP Server Structure in Codex Config"
sidebar_label: "CDX-CFG-009"
description: "agnix rule CDX-CFG-009 checks for invalid mcp server structure in codex config in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-CFG-009", "invalid mcp server structure in codex config", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-009`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-03`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://developers.openai.com/codex/config-reference
- https://developers.openai.com/codex/config-schema.json
- https://developers.openai.com/codex/enterprise/managed-configuration

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[mcp_servers.local]
enabled = true
```

### Valid

```toml
[mcp_servers.local]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem"]
```
