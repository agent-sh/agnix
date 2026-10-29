---
id: agy-cfg-003
title: "AGY-CFG-003: Misplaced Antigravity Customization"
sidebar_label: "AGY-CFG-003"
description: "agnix rule AGY-CFG-003 checks for misplaced antigravity customization in antigravity files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["AGY-CFG-003", "misplaced antigravity customization", "antigravity", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `AGY-CFG-003`
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

- https://antigravity.google/docs/settings?tab=cli
- https://antigravity.google/schemas/v1/settings.json
- https://antigravity.google/docs/remote-control

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
