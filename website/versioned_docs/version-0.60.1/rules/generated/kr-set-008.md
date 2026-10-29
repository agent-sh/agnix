---
id: kr-set-008
title: "KR-SET-008: Kiro CLI Setting Has Wrong Value Type"
sidebar_label: "KR-SET-008"
description: "agnix rule KR-SET-008 checks for kiro cli setting has wrong value type in kiro settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-SET-008", "kiro cli setting has wrong value type", "kiro settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-SET-008`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Settings`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-10-29`

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
{"chat.enableKnowledge": "yes"}
```

### Valid

```json
{"chat.enableKnowledge": true}
```
