---
id: kiro-011
title: "KIRO-011: Always-Included Steering Doc Excessively Long"
sidebar_label: "KIRO-011"
description: "agnix rule KIRO-011 checks for always-included steering doc excessively long in kiro steering files. Severity: LOW. See examples and fix guidance."
keywords: ["KIRO-011", "always-included steering doc excessively long", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-011`
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
- https://kiro.dev/docs/cli/chat/context/

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
# Everything
(more than 50,000 characters)
```

### Valid

```markdown
---
inclusion: manual
---
# Long Reference
(loaded only on request)
```
