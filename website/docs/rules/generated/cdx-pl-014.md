---
id: cdx-pl-014
title: "CDX-PL-014: Missing Description - Codex CLI"
sidebar_label: "CDX-PL-014"
description: "agnix rule CDX-PL-014 checks for missing description in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-PL-014", "missing description", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-014`
- **Severity**: `LOW`
- **Category**: `Codex CLI`
- **Normative Level**: `BEST_PRACTICE`
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
{"name": "my-plugin"}
```

### Valid

```json
{"name": "my-plugin", "description": "Adds code review capabilities"}
```
