---
id: cc-set-038
title: "CC-SET-038: .claudeignore Has No Effect - Claude Settings"
sidebar_label: "CC-SET-038"
description: "agnix rule CC-SET-038 checks for .claudeignore has no effect in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-038", ".claudeignore has no effect", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-038`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/permissions

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
.claudeignore:
.env
secrets/
```

### Valid

```json
{"permissions": {"deny": ["Read(./.env)", "Read(./secrets/**)"]}}
```
