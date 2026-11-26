---
id: kr-set-007
title: "KR-SET-007: Nested Kiro CLI Settings Object - Kiro Settings"
sidebar_label: "KR-SET-007"
description: "agnix rule KR-SET-007 checks for nested kiro cli settings object in kiro settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-SET-007", "nested kiro cli settings object", "kiro settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-SET-007`
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
{"toolSearch": {"enabled": true}}
```

### Valid

```json
{"toolSearch.enabled": true}
```
