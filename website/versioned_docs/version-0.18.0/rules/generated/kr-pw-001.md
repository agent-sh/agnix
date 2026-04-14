---
id: kr-pw-001
title: "KR-PW-001: Missing Required POWER.md Frontmatter Fields"
sidebar_label: "KR-PW-001"
description: "agnix rule KR-PW-001 checks for missing required power.md frontmatter fields in kiro powers files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-PW-001", "missing required power.md frontmatter fields", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-001`
- **Severity**: `HIGH`
- **Category**: `Kiro Powers`
- **Normative Level**: `MUST`
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
# Missing frontmatter
This power omits required metadata.
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
