---
id: cc-sk-020
title: "CC-SK-020: Invalid Shell Value - Claude Skills"
sidebar_label: "CC-SK-020"
description: "agnix rule CC-SK-020 checks for invalid shell value in claude skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SK-020", "invalid shell value", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-020`
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
shell: zsh
---
Skill instructions.
```

### Valid

```markdown
---
name: my-skill
shell: bash
---
Skill instructions.
```
