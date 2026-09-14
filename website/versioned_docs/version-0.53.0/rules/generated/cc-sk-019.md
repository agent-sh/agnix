---
id: cc-sk-019
title: "CC-SK-019: Invalid Paths Format - Claude Skills"
sidebar_label: "CC-SK-019"
description: "agnix rule CC-SK-019 checks for invalid paths format in claude skills files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-SK-019", "invalid paths format", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-019`
- **Severity**: `LOW`
- **Category**: `Claude Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
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
paths: src/**/*.ts
---
Skill instructions.
```

### Valid

```markdown
---
name: my-skill
paths:
  - "src/**/*.ts"
---
Skill instructions.
```
