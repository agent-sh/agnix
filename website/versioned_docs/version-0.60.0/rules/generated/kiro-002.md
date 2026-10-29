---
id: kiro-002
title: "KIRO-002: Inclusion Mode Without the Field It Loads By"
sidebar_label: "KIRO-002"
description: "agnix rule KIRO-002 checks for inclusion mode without the field it loads by in kiro steering files. Severity: HIGH. See examples and fix guidance."
keywords: ["KIRO-002", "inclusion mode without the field it loads by", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-002`
- **Severity**: `HIGH`
- **Category**: `Kiro Steering`
- **Normative Level**: `MUST`
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
inclusion: auto
name: api-design
---
# API Design
```

### Valid

```markdown
---
inclusion: auto
description: REST API conventions. Use when editing API endpoints.
---
# API Design
```
