---
id: agy-kb-001
title: "AGY-KB-001: Invalid Antigravity Keybinding - antigravity"
sidebar_label: "AGY-KB-001"
description: "agnix rule AGY-KB-001 checks for invalid antigravity keybinding in antigravity files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["AGY-KB-001", "invalid antigravity keybinding", "antigravity", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `AGY-KB-001`
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
