---
id: kr-set-010
title: "KR-SET-010: Duplicate Kiro CLI Setting Key - Kiro Settings"
sidebar_label: "KR-SET-010"
description: "agnix rule KR-SET-010 checks for duplicate kiro cli setting key in kiro settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-SET-010", "duplicate kiro cli setting key", "kiro settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-SET-010`
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
{"chat.defaultAgent": "a", "chat.defaultAgent": "b"}
```

### Valid

```json
{"chat.defaultAgent": "b"}
```
