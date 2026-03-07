---
id: cdx-cfg-002
title: "CDX-CFG-002: Invalid sandbox_mode Value - Codex CLI"
sidebar_label: "CDX-CFG-002"
description: "agnix rule CDX-CFG-002 checks for invalid sandbox_mode value in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-CFG-002", "invalid sandbox_mode value", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-002`
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
sandbox_mode = "open"
```

### Valid

```toml
sandbox_mode = "workspace-write"
```
