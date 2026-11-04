---
id: xp-sk-004
title: "XP-SK-004: Backslash Path in Skill - Cross-Platform"
sidebar_label: "XP-SK-004"
description: "agnix rule XP-SK-004 checks for backslash path in skill in cross-platform files. Severity: LOW. See examples and fix guidance."
keywords: ["XP-SK-004", "backslash path in skill", "cross-platform", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `XP-SK-004`
- **Severity**: `LOW`
- **Category**: `Cross-Platform`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `all`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices
- https://code.visualstudio.com/docs/agent-customization/agent-skills

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
Run scripts\setup.py
```

### Valid

```markdown
Run scripts/setup.py
```
