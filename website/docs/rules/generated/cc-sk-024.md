---
id: cc-sk-024
title: "CC-SK-024: Skill Without Description - Claude Skills"
sidebar_label: "CC-SK-024"
description: "agnix rule CC-SK-024 checks for skill without description in claude skills files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-SK-024", "skill without description", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-024`
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

- https://code.claude.com/docs/en/skills

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: pdf-tools
---
Extract the text.
```

### Valid

```markdown
---
name: pdf-tools
description: Extracts text from PDF files. Use when working with PDFs.
---
Extract the text.
```
