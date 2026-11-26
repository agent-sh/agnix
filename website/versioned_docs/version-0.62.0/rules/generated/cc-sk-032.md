---
id: cc-sk-032
title: "CC-SK-032: Skill Description Not in Third Person"
sidebar_label: "CC-SK-032"
description: "agnix rule CC-SK-032 checks for skill description not in third person in claude skills files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-SK-032", "skill description not in third person", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-032`
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
name: excel-reports
description: I can help you process Excel files
---
Build the report.
```

### Valid

```markdown
---
name: excel-reports
description: Processes Excel files and generates reports. Use when working with spreadsheets.
---
Build the report.
```
