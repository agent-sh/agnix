---
id: cdx-cfg-006
title: "CDX-CFG-006: Unknown Codex Config Field - Codex CLI"
sidebar_label: "CDX-CFG-006"
description: "agnix rule CDX-CFG-006 checks for unknown codex config field in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-CFG-006", "unknown codex config field", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-006`
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
unknown_flag = true
```

### Valid

```toml
[features]
memories = true
```
