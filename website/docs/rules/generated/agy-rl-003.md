---
id: agy-rl-003
title: "AGY-RL-003: Truncated Antigravity Rule - antigravity"
sidebar_label: "AGY-RL-003"
description: "agnix rule AGY-RL-003 checks for truncated antigravity rule in antigravity files. Severity: HIGH. See examples and fix guidance."
keywords: ["AGY-RL-003", "truncated antigravity rule", "antigravity", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `AGY-RL-003`
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

- https://antigravity.google/docs/rules

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
