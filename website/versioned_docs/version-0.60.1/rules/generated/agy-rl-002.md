---
id: agy-rl-002
title: "AGY-RL-002: Invalid Antigravity Rule Metadata - antigravity"
sidebar_label: "AGY-RL-002"
description: "agnix rule AGY-RL-002 checks for invalid antigravity rule metadata in antigravity files. Severity: HIGH. See examples and fix guidance."
keywords: ["AGY-RL-002", "invalid antigravity rule metadata", "antigravity", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `AGY-RL-002`
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
