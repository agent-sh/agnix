---
id: cc-sk-027
title: "CC-SK-027: Legacy Custom Command File - Claude Skills"
sidebar_label: "CC-SK-027"
description: "agnix rule CC-SK-027 checks for legacy custom command file in claude skills files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-SK-027", "legacy custom command file", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-027`
- **Severity**: `LOW`
- **Category**: `Claude Skills`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-28`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/skills

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
# .claude/commands/deploy.md
---
name: ship
description: Deploy the app
---
Deploy.
```

### Valid

```markdown
# .claude/skills/deploy/SKILL.md
---
description: Deploy the app
disable-model-invocation: true
---
Deploy.
```
