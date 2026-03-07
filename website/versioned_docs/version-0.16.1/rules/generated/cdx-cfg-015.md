---
id: cdx-cfg-015
title: "CDX-CFG-015: Invalid model_provider Value - Codex CLI"
sidebar_label: "CDX-CFG-015"
description: "agnix rule CDX-CFG-015 checks for invalid model_provider value in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-CFG-015", "invalid model_provider value", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-015`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-06`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://developers.openai.com/codex/config-reference
- https://developers.openai.com/codex/config-schema.json

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
model_provider = 123
```

### Valid

```toml
model_provider = "openai"
```
