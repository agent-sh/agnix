---
id: cc-mp-002
title: "CC-MP-002: Invalid or Reserved Marketplace or Plugin Name"
sidebar_label: "CC-MP-002"
description: "agnix rule CC-MP-002 checks for invalid or reserved marketplace or plugin name in claude marketplaces files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-MP-002", "invalid or reserved marketplace or plugin name", "claude marketplaces", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MP-002`
- **Severity**: `HIGH`
- **Category**: `Claude Marketplaces`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/plugins/marketplace-reference
- https://code.claude.com/docs/en/plugins/manifest-reference

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "name": "claude-plugins-official",
  "owner": { "name": "Acme" },
  "plugins": [{ "name": "my formatter", "source": "./plugins/formatter" }]
}
```

### Valid

```json
{
  "name": "acme-tools",
  "description": "Acme plugins",
  "owner": { "name": "Acme" },
  "plugins": [{ "name": "formatter", "source": "./plugins/formatter" }]
}
```
