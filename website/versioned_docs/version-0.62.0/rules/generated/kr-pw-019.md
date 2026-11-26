---
id: kr-pw-019
title: "KR-PW-019: Legacy POWER.md Power Format - Kiro Powers"
sidebar_label: "KR-PW-019"
description: "agnix rule KR-PW-019 checks for legacy power.md power format in kiro powers files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-PW-019", "legacy power.md power format", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-019`
- **Severity**: `LOW`
- **Category**: `Kiro Powers`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/powers/
- https://kiro.dev/docs/powers/create/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
power/
  POWER.md
  steering/setup.md
  mcp.json
```

### Valid

```text
power/
  plugin.json
  skills/setup/SKILL.md
  mcp.json
```
