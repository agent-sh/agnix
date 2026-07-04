---
id: kr-set-002
title: "KR-SET-002: Invalid toolSearch.minPct Value - Kiro Settings"
sidebar_label: "KR-SET-002"
description: "agnix rule KR-SET-002 checks for invalid toolsearch.minpct value in kiro settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-SET-002", "invalid toolsearch.minpct value", "kiro settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-SET-002`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Settings`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe)`
- **Verified On**: `2026-04-26`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/cli/mcp/tool-search/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "toolSearch.minPct": -5
}
```

### Valid

```json
{
  "toolSearch.minPct": 5
}
```
