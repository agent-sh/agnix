---
id: kiro-015
title: "KIRO-015: Steering Frontmatter Kiro Cannot Read"
sidebar_label: "KIRO-015"
description: "agnix rule KIRO-015 checks for steering frontmatter kiro cannot read in kiro steering files. Severity: HIGH. See examples and fix guidance."
keywords: ["KIRO-015", "steering frontmatter kiro cannot read", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-015`
- **Severity**: `HIGH`
- **Category**: `Kiro Steering`
- **Normative Level**: `MUST`
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
inclusion: manual
name:
inclusion: always
---
# Release Checklist
```

### Valid

```markdown
---
inclusion: manual
name: release-checklist
---
# Release Checklist
```
