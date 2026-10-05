---
id: kiro-009
title: "KIRO-009: Broken Inline Steering Reference - Kiro Steering"
sidebar_label: "KIRO-009"
description: "agnix rule KIRO-009 checks for broken inline steering reference in kiro steering files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KIRO-009", "broken inline steering reference", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-009`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Steering`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-04`

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
Read #[[file:docs/missing-style-guide.md]] before generating code.
```

### Valid

```markdown
Read #[[file:docs/style-guide.md:12-28]] and #[[folder:config]] before generating code.
```
