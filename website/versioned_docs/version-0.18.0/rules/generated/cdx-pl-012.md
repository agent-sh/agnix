---
id: cdx-pl-012
title: "CDX-PL-012: Invalid Asset Path - Codex CLI"
sidebar_label: "CDX-PL-012"
description: "agnix rule CDX-PL-012 checks for invalid asset path in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-PL-012", "invalid asset path", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-012`
- **Severity**: `MEDIUM`
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
{"assets": ["icons/logo.png", "../outside/file.txt"]}
```

### Valid

```json
{"assets": ["./icons/logo.png", "./styles/theme.css"]}
```
