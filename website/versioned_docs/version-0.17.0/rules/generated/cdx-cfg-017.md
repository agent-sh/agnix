---
id: cdx-cfg-017
title: "CDX-CFG-017: Invalid history Configuration - Codex CLI"
sidebar_label: "CDX-CFG-017"
description: "agnix rule CDX-CFG-017 checks for invalid history configuration in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-CFG-017", "invalid history configuration", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-017`
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
[history]
persistence = "yes"
```

### Valid

```toml
[history]
persistence = true
max_entries = 1000
```
