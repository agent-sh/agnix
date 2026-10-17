---
id: cc-mp-004
title: "CC-MP-004: Marketplace Entry Fails to Load"
sidebar_label: "CC-MP-004"
description: "agnix rule CC-MP-004 checks for marketplace entry fails to load in claude marketplaces files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-MP-004", "marketplace entry fails to load", "claude marketplaces", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MP-004`
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
  "name": "acme-tools",
  "owner": { "name": "Acme" },
  "plugins": [{ "name": "formatter", "source": "./plugins/formatter", "hooks": "./hooks.json" }]
}
```

### Valid

```json
{
  "name": "acme-tools",
  "owner": { "name": "Acme" },
  "plugins": [{ "name": "formatter", "source": "./plugins/formatter", "hooks": { "PostToolUse": [] } }]
}
```
