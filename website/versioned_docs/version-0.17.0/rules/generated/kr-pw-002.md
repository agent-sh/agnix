---
id: kr-pw-002
title: "KR-PW-002: Empty POWER.md Keywords Array - Kiro Powers"
sidebar_label: "KR-PW-002"
description: "agnix rule KR-PW-002 checks for empty power.md keywords array in kiro powers files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-PW-002", "empty power.md keywords array", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-002`
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

- https://kiro.dev/docs/powers/

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
keywords: []
---
# Review Power
```

### Valid

```text
---
name: review-power
description: Reviews code changes
keywords:
  - review
---
# Review Power
```
