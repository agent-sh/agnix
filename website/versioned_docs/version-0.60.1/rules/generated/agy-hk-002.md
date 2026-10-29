---
id: agy-hk-002
title: "AGY-HK-002: Unknown Antigravity Hook Event - antigravity"
sidebar_label: "AGY-HK-002"
description: "agnix rule AGY-HK-002 checks for unknown antigravity hook event in antigravity files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["AGY-HK-002", "unknown antigravity hook event", "antigravity", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `AGY-HK-002`
- **Severity**: `MEDIUM`
- **Category**: `antigravity`
- **Normative Level**: `SHOULD`
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
