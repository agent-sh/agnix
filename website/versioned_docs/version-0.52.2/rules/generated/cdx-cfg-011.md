---
id: cdx-cfg-011
title: "CDX-CFG-011: Invalid Feature Flag Name or Shape - Codex CLI"
sidebar_label: "CDX-CFG-011"
description: "agnix rule CDX-CFG-011 checks for invalid feature flag name or shape in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-CFG-011", "invalid feature flag name or shape", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-011`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-30`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://developers.openai.com/codex/config-reference
- https://github.com/openai/codex/blob/rust-v0.146.0/codex-rs/core/config.schema.json

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[features.non_prefixed_mcp_tool_names]
server_names = [42]
```

### Valid

```toml
[features.non_prefixed_mcp_tool_names]
enabled = true
server_names = ["docs"]
```
