---
id: cc-set-018
title: "CC-SET-018: Non-boolean emojiCompletionEnabled Setting"
sidebar_label: "CC-SET-018"
description: "agnix rule CC-SET-018 checks for non-boolean emojicompletionenabled setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-018", "non-boolean emojicompletionenabled setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-018`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-26`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.217`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.217
- https://code.claude.com/docs/en/settings

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "emojiCompletionEnabled": "true"
}
```

### Valid

```json
{
  "emojiCompletionEnabled": true
}
```
