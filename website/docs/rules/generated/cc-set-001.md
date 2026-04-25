---
id: cc-set-001
title: "CC-SET-001: Invalid prUrlTemplate Setting - Claude Settings"
sidebar_label: "CC-SET-001"
description: "agnix rule CC-SET-001 checks for invalid prurltemplate setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-001", "invalid prurltemplate setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-001`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-04-26`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/settings

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "prUrlTemplate": "https://reviews.example.com/"
}
```

### Valid

```json
{
  "prUrlTemplate": "https://reviews.example.com/{owner}/{repo}/pull/{number}"
}
```
