---
id: kiro-017
title: "KIRO-017: Kiro Steering Frontmatter in AGENTS.md"
sidebar_label: "KIRO-017"
description: "agnix rule KIRO-017 checks for kiro steering frontmatter in agents.md in kiro steering files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KIRO-017", "kiro steering frontmatter in agents.md", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-017`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Steering`
- **Normative Level**: `SHOULD`
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
inclusion: fileMatch
fileMatchPattern: "src/**"
---
# Project Rules
```

### Valid

```markdown
# Project Rules

Use pnpm for every package script.
```
