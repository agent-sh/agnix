---
id: cdx-cfg-001
title: "CDX-CFG-001: Invalid approval_policy Value - Codex CLI"
sidebar_label: "CDX-CFG-001"
description: "agnix rule CDX-CFG-001 checks for invalid approval_policy value in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-CFG-001", "invalid approval_policy value", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-001`
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
approval_policy = "always"
```

### Valid

```toml
approval_policy = "on-request"
```
