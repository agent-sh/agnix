---
id: agy-rl-001
title: "AGY-RL-001: Invalid Antigravity Rule Trigger - antigravity"
sidebar_label: "AGY-RL-001"
description: "agnix rule AGY-RL-001 checks for invalid antigravity rule trigger in antigravity files. Severity: HIGH. See examples and fix guidance."
keywords: ["AGY-RL-001", "invalid antigravity rule trigger", "antigravity", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `AGY-RL-001`
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
