---
id: amp-sk-003
title: "AMP-SK-003: Amp Skill Too Deep to Be Discovered - Amp Skills"
sidebar_label: "AMP-SK-003"
description: "agnix rule AMP-SK-003 checks for amp skill too deep to be discovered in amp skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["AMP-SK-003", "amp skill too deep to be discovered", "amp skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `AMP-SK-003`
- **Severity**: `MEDIUM`
- **Category**: `Amp Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `amp`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://ampcode.com/docs/customize/skills

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
.agents/skills/a/b/c/d/e/f/SKILL.md
```

### Valid

```markdown
~/.agents/skills/something/deep/SKILL.md
```
