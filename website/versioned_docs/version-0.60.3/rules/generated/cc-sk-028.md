---
id: cc-sk-028
title: "CC-SK-028: Injected Command Not Run - Claude Skills"
sidebar_label: "CC-SK-028"
description: "agnix rule CC-SK-028 checks for injected command not run in claude skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SK-028", "injected command not run", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-028`
- **Severity**: `MEDIUM`
- **Category**: `Claude Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-28`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/skills
- https://code.claude.com/docs/en/errors#no-shell-command-in-a-skill-or-command-was-run

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: status
description: Shows branch status. Use when asked for status.
---
BRANCH=!`git branch --show-current`
```

### Valid

```markdown
---
name: status
description: Shows branch status. Use when asked for status.
---
Branch: !`git branch --show-current`
```
