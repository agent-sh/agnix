---
id: pi-cfg-002
title: "PI-CFG-002: Invalid Pi Settings Value - pi"
sidebar_label: "PI-CFG-002"
description: "agnix rule PI-CFG-002 checks for invalid pi settings value in pi files. Severity: HIGH. See examples and fix guidance."
keywords: ["PI-CFG-002", "invalid pi settings value", "pi", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `PI-CFG-002`
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

- https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/docs/settings.md
- https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/schemas/settings.schema.json
- https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/src/core/settings-manager.ts

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
