---
id: cdx-cfg-026
title: "CDX-CFG-026: Unknown Permission Profile Network Key"
sidebar_label: "CDX-CFG-026"
description: "agnix rule CDX-CFG-026 checks for unknown permission profile network key in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-CFG-026", "unknown permission profile network key", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-026`
- **Severity**: `LOW`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe)`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://learn.chatgpt.com/docs/permissions
- https://learn.chatgpt.com/docs/config-schema.json

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[permissions.dev.network]
allowed_domains = ["api.openai.com"]
```

### Valid

```toml
[permissions.dev.network]
enabled = true

[permissions.dev.network.domains]
"api.openai.com" = "allow"
```
