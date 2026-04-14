---
id: cdx-pl-004
title: "CDX-PL-004: Invalid Plugin Name Characters - Codex CLI"
sidebar_label: "CDX-PL-004"
description: "agnix rule CDX-PL-004 checks for invalid plugin name characters in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-PL-004", "invalid plugin name characters", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-004`
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
{"name": "my plugin!@#"}
```

### Valid

```json
{"name": "my-cool-plugin"}
```
