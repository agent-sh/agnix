---
id: cdx-pl-013
title: "CDX-PL-013: Unsupported Hooks Field - Codex CLI"
sidebar_label: "CDX-PL-013"
description: "agnix rule CDX-PL-013 checks for unsupported hooks field in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-PL-013", "unsupported hooks field", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-013`
- **Severity**: `LOW`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-04-01`

## Applicability

- **Tool**: `codex`
- **Version Range**: `>=0.117.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/blob/main/codex-rs/core/src/plugins/manifest.rs

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "my-plugin", "hooks": {"on_load": "./init.js"}}
```

### Valid

```json
{"name": "my-plugin", "components": [{"path": "./src/index.js"}]}
```
