---
id: cdx-pl-015
title: "CDX-PL-015: Invalid Skills Path Type - Codex CLI"
sidebar_label: "CDX-PL-015"
description: "agnix rule CDX-PL-015 checks for invalid skills path type in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-PL-015", "invalid skills path type", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-015`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-06-04`

## Applicability

- **Tool**: `codex`
- **Version Range**: `>=0.137.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/blob/rust-v0.137.0/codex-rs/core-plugins/src/manifest.rs

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "my-plugin", "skills": ["./skills"]}
```

### Valid

```json
{"name": "my-plugin", "skills": "./skills"}
```
