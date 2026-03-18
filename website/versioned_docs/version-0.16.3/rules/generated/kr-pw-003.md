---
id: kr-pw-003
title: "KR-PW-003: Empty POWER.md Body - Kiro Powers"
sidebar_label: "KR-PW-003"
description: "agnix rule KR-PW-003 checks for empty power.md body in kiro powers files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-PW-003", "empty power.md body", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-003`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Powers`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-02`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/powers/create

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
---
name: review-power
description: Reviews code changes
keywords:
  - review
---
```

### Valid

```text
---
name: review-power
description: Reviews code changes
keywords:
  - review
---
# Onboarding
Use this power for review flows.
```
