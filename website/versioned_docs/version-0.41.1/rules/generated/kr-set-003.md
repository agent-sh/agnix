---
id: kr-set-003
title: "KR-SET-003: Invalid toolSearch.minTokens Value"
sidebar_label: "KR-SET-003"
description: "agnix rule KR-SET-003 checks for invalid toolsearch.mintokens value in kiro settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-SET-003", "invalid toolsearch.mintokens value", "kiro settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-SET-003`
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
  "toolSearch.minTokens": -1
}
```

### Valid

```json
{
  "toolSearch.minTokens": 50000
}
```
