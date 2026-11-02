---
id: kr-set-011
title: "KR-SET-011: Global-Only Kiro CLI Setting in Workspace File"
sidebar_label: "KR-SET-011"
description: "agnix rule KR-SET-011 checks for global-only kiro cli setting in workspace file in kiro settings files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-SET-011", "global-only kiro cli setting in workspace file", "kiro settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-SET-011`
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
{"chat.uiMode": "tui"}
```

### Valid

```json
{"chat.defaultAgent": "project-agent"}
```
