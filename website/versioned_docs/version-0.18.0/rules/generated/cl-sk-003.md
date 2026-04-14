---
id: cl-sk-003
title: "CL-SK-003: Missing Skill Description - Cline Skills"
sidebar_label: "CL-SK-003"
description: "agnix rule CL-SK-003 checks for missing skill description in cline skills files. Severity: HIGH. See examples and fix guidance."
keywords: ["CL-SK-003", "missing skill description", "cline skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CL-SK-003`
- **Severity**: `HIGH`
- **Category**: `Cline Skills`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-28`

## Applicability

- **Tool**: `cline`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://docs.cline.bot/features/cline-rules/overview

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: my-skill
---
# My Skill
```

### Valid

```markdown
---
name: my-skill
description: A useful development skill
---
# My Skill
```
