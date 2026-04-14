---
id: cdx-cfg-026
title: "CDX-CFG-026: Invalid Network Permission Field - Codex CLI"
sidebar_label: "CDX-CFG-026"
description: "agnix rule CDX-CFG-026 checks for invalid network permission field in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-CFG-026", "invalid network permission field", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-026`
- **Severity**: `LOW`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe)`
- **Verified On**: `2026-03-28`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://developers.openai.com/codex/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[network]
allow = "all"
```

### Valid

```toml
[network]
allow = ["*.example.com"]
```
