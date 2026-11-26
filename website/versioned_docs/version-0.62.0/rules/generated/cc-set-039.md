---
id: cc-set-039
title: "CC-SET-039: Secret Files Not Denied - Claude Settings"
sidebar_label: "CC-SET-039"
description: "agnix rule CC-SET-039 checks for secret files not denied in claude settings files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-SET-039", "secret files not denied", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-039`
- **Severity**: `LOW`
- **Category**: `Claude Settings`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/settings-reference
- https://code.claude.com/docs/en/permissions

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
Project with .env and .claude/settings.json that has no Read deny rule
```

### Valid

```json
{"permissions": {"deny": ["Read(./.env)", "Read(./.env.*)", "Read(./secrets/**)"]}}
```
