---
id: oc-lsp-002
title: "OC-LSP-002: Invalid LSP Extensions - OpenCode"
sidebar_label: "OC-LSP-002"
description: "agnix rule OC-LSP-002 checks for invalid lsp extensions in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-LSP-002", "invalid lsp extensions", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-LSP-002`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-06`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/config

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "lsp": { "ts": { "extensions": [] } }
}
```

### Valid

```json
{
  "lsp": { "ts": { "extensions": [".ts", ".tsx"] } }
}
```
