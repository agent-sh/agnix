---
id: kiro-007
title: "KIRO-007: fileMatchPattern Without fileMatch Inclusion"
sidebar_label: "KIRO-007"
description: "agnix rule KIRO-007 checks for filematchpattern without filematch inclusion in kiro steering files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KIRO-007", "filematchpattern without filematch inclusion", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-007`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Steering`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-02`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/steering/

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
fileMatchPattern: "**/*.ts"
---
This pattern is never applied.
```

### Valid

```markdown
---
inclusion: fileMatch
fileMatchPattern: "**/*.ts"
---
TypeScript guidance.
```
