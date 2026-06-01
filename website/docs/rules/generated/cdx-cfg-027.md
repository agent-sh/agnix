---
id: cdx-cfg-027
title: "CDX-CFG-027: Invalid Windows Sandbox Value - Codex CLI"
sidebar_label: "CDX-CFG-027"
description: "agnix rule CDX-CFG-027 checks for invalid windows sandbox value in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-CFG-027", "invalid windows sandbox value", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-027`
- **Severity**: `LOW`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-06-02`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/blob/rust-v0.136.0/codex-rs/core/config.schema.json

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[windows]
sandbox = "docker"
```

### Valid

```toml
[windows]
sandbox = "elevated"
```
