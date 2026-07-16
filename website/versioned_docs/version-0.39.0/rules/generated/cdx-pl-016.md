---
id: cdx-pl-016
title: "CDX-PL-016: Invalid Dark-mode Logo Path - Codex CLI"
sidebar_label: "CDX-PL-016"
description: "agnix rule CDX-PL-016 checks for invalid dark-mode logo path in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-PL-016", "invalid dark-mode logo path", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-016`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-06-27`

## Applicability

- **Tool**: `codex`
- **Version Range**: `>=0.142.2`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/pull/29488
- https://github.com/openai/codex/blob/rust-v0.142.2/codex-rs/core-plugins/src/manifest.rs

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "my-plugin", "interface": {"logoDark": "assets/logo-dark.png"}}
```

### Valid

```json
{"name": "my-plugin", "interface": {"logoDark": "./assets/logo-dark.png"}}
```
