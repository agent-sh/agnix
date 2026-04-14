---
id: kiro-012
title: "KIRO-012: Duplicate Steering Name - Kiro Steering"
sidebar_label: "KIRO-012"
description: "agnix rule KIRO-012 checks for duplicate steering name in kiro steering files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KIRO-012", "duplicate steering name", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-012`
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
# Two steering files with name: duplicate-name
```

### Valid

```markdown
---
inclusion: auto
name: unique-name
description: desc
---
# Content
```
