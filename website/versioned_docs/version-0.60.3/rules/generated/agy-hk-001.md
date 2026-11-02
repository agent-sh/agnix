---
id: agy-hk-001
title: "AGY-HK-001: Invalid Antigravity Hook Structure - antigravity"
sidebar_label: "AGY-HK-001"
description: "agnix rule AGY-HK-001 checks for invalid antigravity hook structure in antigravity files. Severity: HIGH. See examples and fix guidance."
keywords: ["AGY-HK-001", "invalid antigravity hook structure", "antigravity", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `AGY-HK-001`
- **Severity**: `HIGH`
- **Category**: `antigravity`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-29`

## Applicability

- **Tool**: `antigravity`
- **Version Range**: `>=1.4.9`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://antigravity.google/docs/hooks

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
