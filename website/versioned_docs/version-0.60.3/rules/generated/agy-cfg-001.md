---
id: agy-cfg-001
title: "AGY-CFG-001: Invalid Antigravity Configuration JSON"
sidebar_label: "AGY-CFG-001"
description: "agnix rule AGY-CFG-001 checks for invalid antigravity configuration json in antigravity files. Severity: HIGH. See examples and fix guidance."
keywords: ["AGY-CFG-001", "invalid antigravity configuration json", "antigravity", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `AGY-CFG-001`
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
