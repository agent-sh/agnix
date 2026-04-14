---
id: cdx-pl-001
title: "CDX-PL-001: Codex Plugin Manifest Location - Codex CLI"
sidebar_label: "CDX-PL-001"
description: "agnix rule CDX-PL-001 checks for codex plugin manifest location in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-PL-001", "codex plugin manifest location", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-001`
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

```toml
plugin.json (at project root)
```

### Valid

```toml
.codex-plugin/plugin.json
```
