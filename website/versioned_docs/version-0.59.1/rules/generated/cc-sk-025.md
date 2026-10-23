---
id: cc-sk-025
title: "CC-SK-025: Skill Listing Over Budget - Claude Skills"
sidebar_label: "CC-SK-025"
description: "agnix rule CC-SK-025 checks for skill listing over budget in claude skills files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-SK-025", "skill listing over budget", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-025`
- **Severity**: `LOW`
- **Category**: `Claude Skills`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/skills
- https://code.claude.com/docs/en/settings-reference

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
# .claude/skills/*: dozens of skills with 1,500-character descriptions
```

### Valid

```markdown
# .claude/skills/*: short descriptions with the key use case first
```
