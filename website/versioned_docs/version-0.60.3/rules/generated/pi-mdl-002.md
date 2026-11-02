---
id: pi-mdl-002
title: "PI-MDL-002: Unusable Pi Provider Configuration - pi"
sidebar_label: "PI-MDL-002"
description: "agnix rule PI-MDL-002 checks for unusable pi provider configuration in pi files. Severity: HIGH. See examples and fix guidance."
keywords: ["PI-MDL-002", "unusable pi provider configuration", "pi", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `PI-MDL-002`
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

- https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/docs/models.md
- https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/schemas/models.schema.json
- https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/src/core/model-config.ts

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
