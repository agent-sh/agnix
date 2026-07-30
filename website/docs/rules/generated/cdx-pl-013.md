---
id: cdx-pl-013
title: "CDX-PL-013: Invalid Hooks Value - Codex CLI"
sidebar_label: "CDX-PL-013"
description: "agnix rule CDX-PL-013 checks for invalid hooks value in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-PL-013", "invalid hooks value", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-013`
- **Severity**: `LOW`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-30`

## Applicability

- **Tool**: `codex`
- **Version Range**: `>=0.117.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/blob/rust-v0.146.0/codex-rs/core-plugins/src/manifest.rs
- https://github.com/openai/codex/blob/rust-v0.146.0/codex-rs/core-plugins/src/agent_plugin_manifest.rs

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name":"my-plugin","hooks":42}
```

### Valid

```json
{"name":"my-plugin","hooks":["./hooks.json"]}
```
