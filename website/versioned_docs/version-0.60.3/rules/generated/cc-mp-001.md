---
id: cc-mp-001
title: "CC-MP-001: Marketplace Parse Error or Missing Required Field"
sidebar_label: "CC-MP-001"
description: "agnix rule CC-MP-001 checks for marketplace parse error or missing required field in claude marketplaces files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-MP-001", "marketplace parse error or missing required field", "claude marketplaces", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MP-001`
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
  "plugins": [{ "name": "formatter" }]
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
