---
id: kiro-011
title: "KIRO-011: Steering Doc Excessively Long - Kiro Steering"
sidebar_label: "KIRO-011"
description: "agnix rule KIRO-011 checks for steering doc excessively long in kiro steering files. Severity: LOW. See examples and fix guidance."
keywords: ["KIRO-011", "steering doc excessively long", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-011`
- **Severity**: `LOW`
- **Category**: `Kiro Steering`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-06`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/steering

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
inclusion: always
---
# Very long doc...
```

### Valid

```markdown
---
inclusion: always
---
# Concise guidance
```
