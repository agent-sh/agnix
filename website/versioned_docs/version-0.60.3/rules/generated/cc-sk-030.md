---
id: cc-sk-030
title: "CC-SK-030: Skill Name Uses a Reserved Word - Claude Skills"
sidebar_label: "CC-SK-030"
description: "agnix rule CC-SK-030 checks for skill name uses a reserved word in claude skills files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-SK-030", "skill name uses a reserved word", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-030`
- **Severity**: `LOW`
- **Category**: `Claude Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://platform.claude.com/docs/en/build-with-claude/skills-guide
- https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: claude-pdf-tools
description: Fills PDF forms. Use when filling forms.
---
Fill the form.
```

### Valid

```markdown
---
name: pdf-helper-tools
description: Fills PDF forms. Use when filling forms.
---
Fill the form.
```
