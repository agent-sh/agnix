---
id: pi-pkg-001
title: "PI-PKG-001: Invalid Pi Package Resources - pi"
sidebar_label: "PI-PKG-001"
description: "agnix rule PI-PKG-001 checks for invalid pi package resources in pi files. Severity: HIGH. See examples and fix guidance."
keywords: ["PI-PKG-001", "invalid pi package resources", "pi", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `PI-PKG-001`
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

- https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/docs/packages.md
- https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/src/core/package-manager.ts

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
