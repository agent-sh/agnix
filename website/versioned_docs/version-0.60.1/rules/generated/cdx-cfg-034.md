---
id: cdx-cfg-034
title: "CDX-CFG-034: Profile Domain Rules Without Network Proxy"
sidebar_label: "CDX-CFG-034"
description: "agnix rule CDX-CFG-034 checks for profile domain rules without network proxy in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-CFG-034", "profile domain rules without network proxy", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-034`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://learn.chatgpt.com/docs/permissions

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[permissions.dev.network]
enabled = true

[permissions.dev.network.domains]
"api.openai.com" = "allow"
```

### Valid

```toml
[features]
network_proxy = true

[permissions.dev.network]
enabled = true

[permissions.dev.network.domains]
"api.openai.com" = "allow"
```
