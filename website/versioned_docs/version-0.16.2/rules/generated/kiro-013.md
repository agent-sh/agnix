---
id: kiro-013
title: "KIRO-013: Conflicting Inclusion Modes - Kiro Steering"
sidebar_label: "KIRO-013"
description: "agnix rule KIRO-013 checks for conflicting inclusion modes in kiro steering files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KIRO-013", "conflicting inclusion modes", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-013`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Steering`
- **Normative Level**: `SHOULD`
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
inclusion: manual
---
# Content
```

### Valid

```markdown
---
inclusion: always
---
# Content
```
