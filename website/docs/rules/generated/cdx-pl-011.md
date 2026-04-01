---
id: cdx-pl-011
title: "CDX-PL-011: Invalid Interface URL - Codex CLI"
sidebar_label: "CDX-PL-011"
description: "agnix rule CDX-PL-011 checks for invalid interface url in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-PL-011", "invalid interface url", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-011`
- **Severity**: `MEDIUM`
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
{"interface": "not a url"}
```

### Valid

```json
{"interface": "https://example.com/plugin-ui"}
```
