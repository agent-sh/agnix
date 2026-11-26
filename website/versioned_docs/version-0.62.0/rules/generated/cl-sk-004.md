---
id: cl-sk-004
title: "CL-SK-004: Cline Skill Without Instructions - Cline Skills"
sidebar_label: "CL-SK-004"
description: "agnix rule CL-SK-004 checks for cline skill without instructions in cline skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CL-SK-004", "cline skill without instructions", "cline skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CL-SK-004`
- **Severity**: `MEDIUM`
- **Category**: `Cline Skills`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `cline`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://docs.cline.bot/customization/skills
- https://github.com/cline/cline/blob/main/sdk/packages/core/src/extensions/config/user-instruction-config-loader.ts

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: changelog
description: Formats changelog entries. Use when editing CHANGELOG.md.
---
```

### Valid

```markdown
---
name: changelog
description: Formats changelog entries. Use when editing CHANGELOG.md.
---
# Changelog

Group entries under Added, Changed and Fixed.
```
