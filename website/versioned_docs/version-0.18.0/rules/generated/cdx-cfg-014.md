---
id: cdx-cfg-014
title: "CDX-CFG-014: Invalid model Value - Codex CLI"
sidebar_label: "CDX-CFG-014"
description: "agnix rule CDX-CFG-014 checks for invalid model value in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-CFG-014", "invalid model value", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-014`
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
model = 123
```

### Valid

```toml
model = "o4-mini"
```
