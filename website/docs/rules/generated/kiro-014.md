---
id: kiro-014
title: "KIRO-014: Steering Body Without Markdown Heading"
sidebar_label: "KIRO-014"
description: "agnix rule KIRO-014 checks for steering body without markdown heading in kiro steering files. Severity: LOW. See examples and fix guidance."
keywords: ["KIRO-014", "steering body without markdown heading", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-014`
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
No heading, just text.
```

### Valid

```markdown
---
inclusion: always
---
# Heading

Content here.
```
