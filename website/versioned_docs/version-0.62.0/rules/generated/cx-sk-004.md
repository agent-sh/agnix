---
id: cx-sk-004
title: "CX-SK-004: Skill Catalog Exceeds Codex Fallback Budget"
sidebar_label: "CX-SK-004"
description: "agnix rule CX-SK-004 checks for skill catalog exceeds codex fallback budget in codex skills files. Severity: LOW. See examples and fix guidance."
keywords: ["CX-SK-004", "skill catalog exceeds codex fallback budget", "codex skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CX-SK-004`
- **Severity**: `LOW`
- **Category**: `Codex Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://learn.chatgpt.com/docs/build-skills
- https://learn.chatgpt.com/docs/config-file/config-reference

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
40 skills with 500-character descriptions in .agents/skills
```

### Valid

```markdown
12 skills with one-sentence descriptions
```
