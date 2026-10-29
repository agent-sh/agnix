---
id: pi-th-002
title: "PI-TH-002: Invalid Pi Theme Resolution - pi"
sidebar_label: "PI-TH-002"
description: "agnix rule PI-TH-002 checks for invalid pi theme resolution in pi files. Severity: HIGH. See examples and fix guidance."
keywords: ["PI-TH-002", "invalid pi theme resolution", "pi", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `PI-TH-002`
- **Severity**: `HIGH`
- **Category**: `pi`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-29`

## Applicability

- **Tool**: `pi`
- **Version Range**: `>=1.4.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/docs/themes.md
- https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/schemas/theme.schema.json
- https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/src/modes/interactive/theme/theme.ts
- https://github.com/earendil-works/pi/blob/v1.4.0/packages/tui/src/colors.ts

## Test Coverage Metadata

- Unit tests: `false`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
Configuration omitted required fields for this rule.
```

### Valid

```text
Configuration includes required fields and follows the rule.
```
