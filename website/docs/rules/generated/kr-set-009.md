---
id: kr-set-009
title: "KR-SET-009: Unrecognized Kiro CLI Setting Value"
sidebar_label: "KR-SET-009"
description: "agnix rule KR-SET-009 checks for unrecognized kiro cli setting value in kiro settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-SET-009", "unrecognized kiro cli setting value", "kiro settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-SET-009`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Settings`
- **Normative Level**: `SHOULD`
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
{"chat.uiMode": "compact"}
```

### Valid

```json
{"chat.uiMode": "tui"}
```
