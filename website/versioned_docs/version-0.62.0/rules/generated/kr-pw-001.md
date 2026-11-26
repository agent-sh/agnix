---
id: kr-pw-001
title: "KR-PW-001: POWER.md Frontmatter Kiro Cannot Use"
sidebar_label: "KR-PW-001"
description: "agnix rule KR-PW-001 checks for power.md frontmatter kiro cannot use in kiro powers files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-PW-001", "power.md frontmatter kiro cannot use", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-001`
- **Severity**: `HIGH`
- **Category**: `Kiro Powers`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/powers/
- https://kiro.dev/docs/powers/installation/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
# Missing frontmatter
This power has no description or activation keywords.
```

### Valid

```text
---
name: review-power
description: Reviews code changes
keywords:
  - review
  - quality
---
# Review Power
```
