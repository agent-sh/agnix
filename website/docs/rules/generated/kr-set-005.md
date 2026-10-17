---
id: kr-set-005
title: "KR-SET-005: Unknown Kiro CLI Setting - Kiro Settings"
sidebar_label: "KR-SET-005"
description: "agnix rule KR-SET-005 checks for unknown kiro cli setting in kiro settings files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-SET-005", "unknown kiro cli setting", "kiro settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-SET-005`
- **Severity**: `LOW`
- **Category**: `Kiro Settings`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/reference/settings/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"chat.autoCompact": true}
```

### Valid

```json
{"chat.disableAutoCompaction": false}
```
