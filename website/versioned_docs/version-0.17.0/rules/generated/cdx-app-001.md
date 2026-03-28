---
id: cdx-app-001
title: "CDX-APP-001: Invalid default_tools_approval_mode Value"
sidebar_label: "CDX-APP-001"
description: "agnix rule CDX-APP-001 checks for invalid default_tools_approval_mode value in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-APP-001", "invalid default_tools_approval_mode value", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-APP-001`
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
[apps.github]
default_tools_approval_mode = "manual"
```

### Valid

```toml
[apps.github]
default_tools_approval_mode = "prompt"
```
