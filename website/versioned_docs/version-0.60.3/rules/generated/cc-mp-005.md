---
id: cc-mp-005
title: "CC-MP-005: Marketplace Field Ignored or Missing Metadata"
sidebar_label: "CC-MP-005"
description: "agnix rule CC-MP-005 checks for marketplace field ignored or missing metadata in claude marketplaces files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-MP-005", "marketplace field ignored or missing metadata", "claude marketplaces", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MP-005`
- **Severity**: `LOW`
- **Category**: `Claude Marketplaces`
- **Normative Level**: `SHOULD`
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
  "plugin": [],
  "plugins": [{ "name": "formatter", "source": "./plugins/formatter" }]
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
