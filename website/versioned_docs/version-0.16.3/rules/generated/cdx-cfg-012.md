---
id: cdx-cfg-012
title: "CDX-CFG-012: Invalid cli_auth_credentials_store Value"
sidebar_label: "CDX-CFG-012"
description: "agnix rule CDX-CFG-012 checks for invalid cli_auth_credentials_store value in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-CFG-012", "invalid cli_auth_credentials_store value", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-012`
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
cli_auth_credentials_store = "vault"
```

### Valid

```toml
cli_auth_credentials_store = "keyring"
```
