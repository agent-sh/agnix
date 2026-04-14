---
id: cdx-cfg-011
title: "CDX-CFG-011: Invalid Feature Flag Name - Codex CLI"
sidebar_label: "CDX-CFG-011"
description: "agnix rule CDX-CFG-011 checks for invalid feature flag name in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-CFG-011", "invalid feature flag name", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-011`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
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
[features]
future_flag = true
```

### Valid

```toml
[features]
memories = true
```
