---
id: kr-pw-018
title: "KR-PW-018: Legacy steering Directory in a plugin.json Power"
sidebar_label: "KR-PW-018"
description: "agnix rule KR-PW-018 checks for legacy steering directory in a plugin.json power in kiro powers files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-PW-018", "legacy steering directory in a plugin.json power", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-018`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Powers`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.16.2`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/powers/create/

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
  steering/setup.md
```

### Valid

```text
power/
  plugin.json
  skills/setup/SKILL.md
```
