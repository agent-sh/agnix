---
id: kr-set-001
title: "KR-SET-001: Invalid toolSearch.enabled Value - Kiro Settings"
sidebar_label: "KR-SET-001"
description: "agnix rule KR-SET-001 checks for invalid toolsearch.enabled value in kiro settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-SET-001", "invalid toolsearch.enabled value", "kiro settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-SET-001`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Settings`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/reference/settings/
- https://kiro.dev/docs/mcp/tool-search/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "toolSearch.enabled": "true"
}
```

### Valid

```json
{
  "toolSearch.enabled": true
}
```
