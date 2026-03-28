---
id: oc-lsp-001
title: "OC-LSP-001: LSP Command Without Extensions - OpenCode"
sidebar_label: "OC-LSP-001"
description: "agnix rule OC-LSP-001 checks for lsp command without extensions in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-LSP-001", "lsp command without extensions", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-LSP-001`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
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
  "lsp": { "ts": { "command": "tsc" } }
}
```

### Valid

```json
{
  "lsp": { "ts": { "command": "tsc", "extensions": [".ts"] } }
}
```
