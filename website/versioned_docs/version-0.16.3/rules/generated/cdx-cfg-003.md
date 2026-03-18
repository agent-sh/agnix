---
id: cdx-cfg-003
title: "CDX-CFG-003: Invalid model_reasoning_effort Value"
sidebar_label: "CDX-CFG-003"
description: "agnix rule CDX-CFG-003 checks for invalid model_reasoning_effort value in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-CFG-003", "invalid model_reasoning_effort value", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-003`
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
model_reasoning_effort = "turbo"
```

### Valid

```toml
model_reasoning_effort = "high"
```
