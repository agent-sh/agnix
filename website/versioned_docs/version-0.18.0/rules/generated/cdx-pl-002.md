---
id: cdx-pl-002
title: "CDX-PL-002: Invalid JSON in Plugin Manifest - Codex CLI"
sidebar_label: "CDX-PL-002"
description: "agnix rule CDX-PL-002 checks for invalid json in plugin manifest in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-PL-002", "invalid json in plugin manifest", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-002`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
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
{name: my-plugin, components: }
```

### Valid

```json
{"name": "my-plugin", "components": [{"path": "./src/index.js"}]}
```
