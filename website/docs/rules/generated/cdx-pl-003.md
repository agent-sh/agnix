---
id: cdx-pl-003
title: "CDX-PL-003: Missing or Empty Plugin Name - Codex CLI"
sidebar_label: "CDX-PL-003"
description: "agnix rule CDX-PL-003 checks for missing or empty plugin name in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-PL-003", "missing or empty plugin name", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-003`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
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
{"name": ""}
```

### Valid

```json
{"name": "my-plugin"}
```
