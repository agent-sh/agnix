---
id: cdx-pl-017
title: "CDX-PL-017: Codex Compatibility Manifest Without Portable Root Manifest"
sidebar_label: "CDX-PL-017"
description: "agnix rule CDX-PL-017 checks for codex compatibility manifest without portable root manifest in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-PL-017", "codex compatibility manifest without portable root manifest", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-017`
- **Severity**: `LOW`
- **Category**: `Codex CLI`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://developers.openai.com/plugins/build/plugins

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
.codex-plugin/plugin.json only
```

### Valid

```toml
plugin.json with "$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json"
```
