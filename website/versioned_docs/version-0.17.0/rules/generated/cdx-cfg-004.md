---
id: cdx-cfg-004
title: "CDX-CFG-004: Invalid model_verbosity Value - Codex CLI"
sidebar_label: "CDX-CFG-004"
description: "agnix rule CDX-CFG-004 checks for invalid model_verbosity value in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-CFG-004", "invalid model_verbosity value", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-004`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-03`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://developers.openai.com/codex/config-reference
- https://developers.openai.com/codex/config-schema.json
- https://developers.openai.com/codex/enterprise/managed-configuration

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
model_verbosity = "verbose"
```

### Valid

```toml
model_verbosity = "medium"
```
