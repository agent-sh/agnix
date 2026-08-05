---
id: cc-set-015
title: "CC-SET-015: Dead pluginConfigs in Project-Level Settings"
sidebar_label: "CC-SET-015"
description: "agnix rule CC-SET-015 checks for dead pluginconfigs in project-level settings in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-015", "dead pluginconfigs in project-level settings", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-015`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-15`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.207`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.207

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "pluginConfigs": {"my-plugin": {"apiKey": "abc"}}
}
```

### Valid

```json
{
  "model": "claude-sonnet-4"
}
```
