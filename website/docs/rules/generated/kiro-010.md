---
id: kiro-010
title: "KIRO-010: On-Demand Steering Fields Without Inclusion Mode"
sidebar_label: "KIRO-010"
description: "agnix rule KIRO-010 checks for on-demand steering fields without inclusion mode in kiro steering files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KIRO-010", "on-demand steering fields without inclusion mode", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-010`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Steering`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

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
name: api-design
description: REST API conventions
---
# Steering
```

### Valid

```markdown
---
inclusion: auto
name: api-design
description: REST API conventions
---
# Steering
```
