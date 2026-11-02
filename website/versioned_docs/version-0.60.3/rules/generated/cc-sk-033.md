---
id: cc-sk-033
title: "CC-SK-033: Vague Skill Name - Claude Skills"
sidebar_label: "CC-SK-033"
description: "agnix rule CC-SK-033 checks for vague skill name in claude skills files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-SK-033", "vague skill name", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-033`
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
name: helper
description: Extracts text from PDFs. Use when working with PDFs.
---
Extract.
```

### Valid

```markdown
---
name: processing-pdfs
description: Extracts text from PDFs. Use when working with PDFs.
---
Extract.
```
