---
id: kiro-005
title: "KIRO-005: Empty Steering Body After Frontmatter"
sidebar_label: "KIRO-005"
description: "agnix rule KIRO-005 checks for empty steering body after frontmatter in kiro steering files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KIRO-005", "empty steering body after frontmatter", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-005`
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
---
```

### Valid

```markdown
---
inclusion: always
---
# Team Standards

Explain reasoning and include concrete examples.
```
