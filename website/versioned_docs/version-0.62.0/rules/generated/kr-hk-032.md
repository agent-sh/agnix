---
id: kr-hk-032
title: "KR-HK-032: Kiro PermissionRequest Hook Requires Kiro CLI 3.4.0"
sidebar_label: "KR-HK-032"
description: "agnix rule KR-HK-032 checks for kiro permissionrequest hook requires kiro cli 3.4.0 in kiro hooks files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-HK-032", "kiro permissionrequest hook requires kiro cli 3.4.0", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-032`
- **Severity**: `LOW`
- **Category**: `Kiro Hooks`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-11-18`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.8.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/changelog/cli/
- https://kiro.dev/docs/hooks/types/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
A PermissionRequest hook in a repository whose CI or teammates still run Kiro CLI 3.3.x or older
```

### Valid

```json
{ "trigger": "PermissionRequest", "matcher": "^read_file$", "action": { "type": "command", "command": "answer.sh" } } (on Kiro CLI >= 3.4.0 in a V3 session)
```
