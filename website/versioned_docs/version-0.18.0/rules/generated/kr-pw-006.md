---
id: kr-pw-006
title: "KR-PW-006: Duplicate Keywords - Kiro Powers"
sidebar_label: "KR-PW-006"
description: "agnix rule KR-PW-006 checks for duplicate keywords in kiro powers files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-PW-006", "duplicate keywords", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-006`
- **Severity**: `LOW`
- **Category**: `Kiro Powers`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-06`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/powers

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
---
name: test
description: desc
keywords: [test, test]
---
# Body
```

### Valid

```text
---
name: test
description: desc
keywords: [test, deploy]
---
# Body
```
