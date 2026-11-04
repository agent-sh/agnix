---
id: kr-set-012
title: "KR-SET-012: Classic-Only Kiro CLI Setting - Kiro Settings"
sidebar_label: "KR-SET-012"
description: "agnix rule KR-SET-012 checks for classic-only kiro cli setting in kiro settings files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-SET-012", "classic-only kiro cli setting", "kiro settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-SET-012`
- **Severity**: `LOW`
- **Category**: `Kiro Settings`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-11-04`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=3.0.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/reference/settings/
- https://kiro.dev/changelog/cli/3-0/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"chat.diffTool": "delta"}
```

### Valid

```json
{"chat.ui": "tui"}
```
