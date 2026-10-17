---
id: kr-pw-017
title: "KR-PW-017: POWER.md Ignored Next to plugin.json"
sidebar_label: "KR-PW-017"
description: "agnix rule KR-PW-017 checks for power.md ignored next to plugin.json in kiro powers files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-PW-017", "power.md ignored next to plugin.json", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-017`
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

- https://kiro.dev/docs/powers/
- https://kiro.dev/docs/powers/installation/

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
  POWER.md
  steering/setup.md
```

### Valid

```text
power/
  plugin.json
  skills/setup/SKILL.md
```
