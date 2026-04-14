---
id: cdx-cfg-016
title: "CDX-CFG-016: Invalid model_reasoning_summary Value"
sidebar_label: "CDX-CFG-016"
description: "agnix rule CDX-CFG-016 checks for invalid model_reasoning_summary value in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-CFG-016", "invalid model_reasoning_summary value", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-016`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
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
model_reasoning_summary = 123
```

### Valid

```toml
model_reasoning_summary = "auto"
```
