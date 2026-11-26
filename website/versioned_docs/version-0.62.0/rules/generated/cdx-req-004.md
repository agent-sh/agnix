---
id: cdx-req-004
title: "CDX-REQ-004: Conflicting experimental_network Domain Settings"
sidebar_label: "CDX-REQ-004"
description: "agnix rule CDX-REQ-004 checks for conflicting experimental_network domain settings in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-REQ-004", "conflicting experimental_network domain settings", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-REQ-004`
- **Severity**: `LOW`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://learn.chatgpt.com/docs/enterprise/managed-configuration

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[experimental_network]
allowed_domains = ["api.openai.com"]

[experimental_network.domains]
"api.openai.com" = "allow"
```

### Valid

```toml
[experimental_network]
enabled = true

[experimental_network.domains]
"api.openai.com" = "allow"
```
