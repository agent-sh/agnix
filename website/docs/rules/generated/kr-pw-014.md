---
id: kr-pw-014
title: "KR-PW-014: Plugin Skill Kiro Skips - Kiro Powers"
sidebar_label: "KR-PW-014"
description: "agnix rule KR-PW-014 checks for plugin skill kiro skips in kiro powers files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-PW-014", "plugin skill kiro skips", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-014`
- **Severity**: `HIGH`
- **Category**: `Kiro Powers`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.16.2`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/powers/create/
- https://kiro.dev/docs/skills/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
power/
  plugin.json
  skills/
    Setup_DB/SKILL.md
```

### Valid

```text
power/
  plugin.json
  skills/
    setup-db/SKILL.md
```
