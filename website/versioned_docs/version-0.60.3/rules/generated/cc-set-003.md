---
id: cc-set-003
title: "CC-SET-003: Invalid worktree.baseRef Value - Claude Settings"
sidebar_label: "CC-SET-003"
description: "agnix rule CC-SET-003 checks for invalid worktree.baseref value in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-003", "invalid worktree.baseref value", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-003`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-05-08`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.133`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.133

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "worktree": {"baseRef": "main"}
}
```

### Valid

```json
{
  "worktree": {"baseRef": "fresh"}
}
```
