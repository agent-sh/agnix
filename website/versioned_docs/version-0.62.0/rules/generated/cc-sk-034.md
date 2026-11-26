---
id: cc-sk-034
title: "CC-SK-034: Long Reference File Without Table of Contents"
sidebar_label: "CC-SK-034"
description: "agnix rule CC-SK-034 checks for long reference file without table of contents in claude skills files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-SK-034", "long reference file without table of contents", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-034`
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

- https://github.com/anthropics/skills/blob/main/skills/skill-creator/SKILL.md
- https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
# reference/finance.md
(400 lines with no contents list)
```

### Valid

```markdown
# reference/finance.md
## Contents
- Revenue
- Costs
...
```
