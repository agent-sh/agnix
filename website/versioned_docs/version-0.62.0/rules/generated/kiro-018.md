---
id: kiro-018
title: "KIRO-018: Vague Auto Steering Description - Kiro Steering"
sidebar_label: "KIRO-018"
description: "agnix rule KIRO-018 checks for vague auto steering description in kiro steering files. Severity: LOW. See examples and fix guidance."
keywords: ["KIRO-018", "vague auto steering description", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-018`
- **Severity**: `LOW`
- **Category**: `Kiro Steering`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/steering/
- https://kiro.dev/docs/skills/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
inclusion: auto
description: Helps with APIs
---
# API Design
```

### Valid

```markdown
---
inclusion: auto
description: REST API design patterns and conventions. Use when creating or modifying API endpoints.
---
# API Design
```
