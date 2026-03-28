---
id: cc-sk-018
title: "CC-SK-018: Invalid Effort Value - Claude Skills"
sidebar_label: "CC-SK-018"
description: "agnix rule CC-SK-018 checks for invalid effort value in claude skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SK-018", "invalid effort value", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-018`
- **Severity**: `MEDIUM`
- **Category**: `Claude Skills`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-03-28`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/skills

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
effort: maximum
---
Skill instructions.
```

### Valid

```markdown
---
name: my-skill
effort: high
---
Skill instructions.
```
