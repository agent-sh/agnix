---
id: kiro-010
title: "KIRO-010: Missing Inclusion Mode - Kiro Steering"
sidebar_label: "KIRO-010"
description: "agnix rule KIRO-010 checks for missing inclusion mode in kiro steering files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KIRO-010", "missing inclusion mode", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-010`
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
name: test
---
# Steering
```

### Valid

```markdown
---
inclusion: always
---
# Steering
```
