---
id: cdx-cfg-028
title: "CDX-CFG-028: Unsupported Inline MCP bearer_token Field"
sidebar_label: "CDX-CFG-028"
description: "agnix rule CDX-CFG-028 checks for unsupported inline mcp bearer_token field in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-CFG-028", "unsupported inline mcp bearer_token field", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-028`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-04-25`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/pull/19294
- https://github.com/openai/codex/issues/19275

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[mcp_servers.myserver]
url = "https://api.example.com"
bearer_token = "sk-live-..."
```

### Valid

```toml
[mcp_servers.myserver]
url = "https://api.example.com"
bearer_token_env_var = "MY_API_TOKEN"
```
