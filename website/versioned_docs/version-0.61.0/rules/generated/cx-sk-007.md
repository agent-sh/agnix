---
id: cx-sk-007
title: "CX-SK-007: Auxiliary Documentation in Codex Skill Folder"
sidebar_label: "CX-SK-007"
description: "agnix rule CX-SK-007 checks for auxiliary documentation in codex skill folder in codex skills files. Severity: LOW. See examples and fix guidance."
keywords: ["CX-SK-007", "auxiliary documentation in codex skill folder", "codex skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CX-SK-007`
- **Severity**: `LOW`
- **Category**: `Codex Skills`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/blob/main/codex-rs/skills/src/assets/samples/skill-creator/SKILL.md

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
my-skill/SKILL.md, my-skill/README.md
```

### Valid

```markdown
my-skill/SKILL.md, my-skill/references/api.md
```
