---
id: kr-sk-002
title: "KR-SK-002: Vague Kiro Skill Description - Kiro Skills"
sidebar_label: "KR-SK-002"
description: "agnix rule KR-SK-002 checks for vague kiro skill description in kiro skills files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-SK-002", "vague kiro skill description", "kiro skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-SK-002`
- **Severity**: `LOW`
- **Category**: `Kiro Skills`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/skills/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: pr-review
description: Helps with code review
---
# PR Review
```

### Valid

```markdown
---
name: pr-review
description: Review pull requests for security vulnerabilities and test coverage. Use when reviewing PRs.
---
# PR Review
```
