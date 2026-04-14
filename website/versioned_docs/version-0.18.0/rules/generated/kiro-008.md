---
id: kiro-008
title: "KIRO-008: Unknown Kiro Steering Frontmatter Field"
sidebar_label: "KIRO-008"
description: "agnix rule KIRO-008 checks for unknown kiro steering frontmatter field in kiro steering files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KIRO-008", "unknown kiro steering frontmatter field", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-008`
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
inclusions: always
---
Typo in frontmatter key.
```

### Valid

```markdown
---
inclusion: auto
name: coding-style
description: Team coding standards
---
Follow conventions.
```
