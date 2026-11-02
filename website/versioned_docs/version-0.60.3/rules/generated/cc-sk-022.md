---
id: cc-sk-022
title: "CC-SK-022: Reserved Claude Skill Name - Claude Skills"
sidebar_label: "CC-SK-022"
description: "agnix rule CC-SK-022 checks for reserved claude skill name in claude skills files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-SK-022", "reserved claude skill name", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-022`
- **Severity**: `HIGH`
- **Category**: `Claude Skills`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-09-27`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.282`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/skills
- https://github.com/anthropics/claude-code/releases/tag/v2.1.282
- https://github.com/anthropics/claude-code/releases/tag/v2.1.283

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: anthropic-skills:pdf
description: Use when working with PDF files
---
Process the PDF.
```

### Valid

```markdown
---
name: pdf-helper
description: Use when working with PDF files
---
Process the PDF.
```
