---
id: cdx-cfg-020
title: "CDX-CFG-020: Invalid MCP OAuth Config - Codex CLI"
sidebar_label: "CDX-CFG-020"
description: "agnix rule CDX-CFG-020 checks for invalid mcp oauth config in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-CFG-020", "invalid mcp oauth config", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-020`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-06`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://developers.openai.com/codex/config-reference
- https://developers.openai.com/codex/config-schema.json

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
mcp_oauth_credentials_store = 123
```

### Valid

```toml
mcp_oauth_credentials_store = "file"
```
