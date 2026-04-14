---
id: cdx-pl-005
title: "CDX-PL-005: Component Path Missing ./ Prefix - Codex CLI"
sidebar_label: "CDX-PL-005"
description: "agnix rule CDX-PL-005 checks for component path missing ./ prefix in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-PL-005", "component path missing ./ prefix", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-005`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (safe)`
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
{"components": [{"path": "src/index.js"}]}
```

### Valid

```json
{"components": [{"path": "./src/index.js"}]}
```
