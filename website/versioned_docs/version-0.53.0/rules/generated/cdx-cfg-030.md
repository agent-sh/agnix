---
id: cdx-cfg-030
title: "CDX-CFG-030: Invalid web_search Mode - Codex CLI"
sidebar_label: "CDX-CFG-030"
description: "agnix rule CDX-CFG-030 checks for invalid web_search mode in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-CFG-030", "invalid web_search mode", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-030`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-06-24`

## Applicability

- **Tool**: `codex`
- **Version Range**: `>=0.142.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/releases/tag/rust-v0.142.0
- https://github.com/openai/codex/blob/rust-v0.142.0/codex-rs/protocol/src/config_types.rs

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
web_search = "server_approved"
```

### Valid

```toml
web_search = "indexed"
```
