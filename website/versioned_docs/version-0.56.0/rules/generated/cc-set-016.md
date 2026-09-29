---
id: cc-set-016
title: "CC-SET-016: Deprecated Tool-Scoped Permission Rule Form"
sidebar_label: "CC-SET-016"
description: "agnix rule CC-SET-016 checks for deprecated tool-scoped permission rule form in claude settings files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-SET-016", "deprecated tool-scoped permission rule form", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-016`
- **Severity**: `LOW`
- **Category**: `Claude Settings`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-16`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.210`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.210

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "permissions": {"allow": ["Write(src/**)", "Glob(docs/**)"]}
}
```

### Valid

```json
{
  "permissions": {"allow": ["Edit(src/**)", "Read(docs/**)"]}
}
```
