---
id: cc-pl-015
title: "CC-PL-015: Default Component Folder Shadowed by Manifest"
sidebar_label: "CC-PL-015"
description: "agnix rule CC-PL-015 checks for default component folder shadowed by manifest in claude plugins files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-PL-015", "default component folder shadowed by manifest", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-015`
- **Severity**: `MEDIUM`
- **Category**: `Claude Plugins`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-31`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.140`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/plugins-reference
- https://github.com/anthropics/claude-code/releases/tag/v2.1.140

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "commands": "./custom-commands"
}
```

### Valid

```json
{
  "commands": ["./custom-commands", "./commands"]
}
```
