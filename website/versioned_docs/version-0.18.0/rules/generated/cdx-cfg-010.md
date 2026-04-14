---
id: cdx-cfg-010
title: "CDX-CFG-010: Hardcoded Secret in Codex Config - Codex CLI"
sidebar_label: "CDX-CFG-010"
description: "agnix rule CDX-CFG-010 checks for hardcoded secret in codex config in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-CFG-010", "hardcoded secret in codex config", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-010`
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
api_key = "sk-live-super-secret-value"
```

### Valid

```toml
api_key = "${OPENAI_API_KEY}"
```
