---
id: pi-cfg-003
title: "PI-CFG-003: Ignored Pi Project Configuration - pi"
sidebar_label: "PI-CFG-003"
description: "agnix rule PI-CFG-003 checks for ignored pi project configuration in pi files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["PI-CFG-003", "ignored pi project configuration", "pi", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `PI-CFG-003`
- **Severity**: `MEDIUM`
- **Category**: `pi`
- **Normative Level**: `SHOULD`
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
