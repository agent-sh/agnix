---
id: pi-pkg-002
title: "PI-PKG-002: Bundled Pi Host Dependency - pi"
sidebar_label: "PI-PKG-002"
description: "agnix rule PI-PKG-002 checks for bundled pi host dependency in pi files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["PI-PKG-002", "bundled pi host dependency", "pi", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `PI-PKG-002`
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
