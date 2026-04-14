---
id: cl-sk-002
title: "CL-SK-002: Missing Skill Name - Cline Skills"
sidebar_label: "CL-SK-002"
description: "agnix rule CL-SK-002 checks for missing skill name in cline skills files. Severity: HIGH. See examples and fix guidance."
keywords: ["CL-SK-002", "missing skill name", "cline skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CL-SK-002`
- **Severity**: `HIGH`
- **Category**: `Cline Skills`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (safe)`
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
description: A skill
---
# My Skill
```

### Valid

```markdown
---
name: my-skill
description: A skill
---
# My Skill
```
