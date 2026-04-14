---
id: cdx-app-003
title: "CDX-APP-003: Invalid profile Configuration - Codex CLI"
sidebar_label: "CDX-APP-003"
description: "agnix rule CDX-APP-003 checks for invalid profile configuration in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-APP-003", "invalid profile configuration", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-APP-003`
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
profile = 123
```

### Valid

```toml
profile = "default"
```
