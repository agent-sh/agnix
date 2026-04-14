---
id: cdx-app-002
title: "CDX-APP-002: Invalid skills Configuration - Codex CLI"
sidebar_label: "CDX-APP-002"
description: "agnix rule CDX-APP-002 checks for invalid skills configuration in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-APP-002", "invalid skills configuration", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-APP-002`
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
[skills]
max_concurrent = "many"
```

### Valid

```toml
[skills]
max_concurrent = 3
```
