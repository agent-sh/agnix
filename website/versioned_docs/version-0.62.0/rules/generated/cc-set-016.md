---
id: cc-set-016
title: "CC-SET-016: File Path Rule Never Consulted - Claude Settings"
sidebar_label: "CC-SET-016"
description: "agnix rule CC-SET-016 checks for file path rule never consulted in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-016", "file path rule never consulted", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-016`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.210`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/permissions
- https://code.claude.com/docs/en/errors

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "permissions": {"deny": ["Write(docs/**)", "MultiEdit(docs/**)", "Glob(./.env)"]}
}
```

### Valid

```json
{
  "permissions": {"deny": ["Edit(docs/**)", "Read(./.env)"]}
}
```
