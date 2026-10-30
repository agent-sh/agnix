---
id: cc-mp-003
title: "CC-MP-003: Invalid Plugin Source, Duplicate Entry or Unresolved Rename"
sidebar_label: "CC-MP-003"
description: "agnix rule CC-MP-003 checks for invalid plugin source, duplicate entry or unresolved rename in claude marketplaces files. Severity: HIGH. See examples and fi..."
keywords: ["CC-MP-003", "invalid plugin source, duplicate entry or unresolved rename", "claude marketplaces", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MP-003`
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

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "name": "acme-tools",
  "owner": { "name": "Acme" },
  "plugins": [{ "name": "formatter", "source": "plugins/formatter" }]
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
