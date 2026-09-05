---
id: cc-pl-016
title: "CC-PL-016: Plugin Name Not Kebab-Case - Claude Plugins"
sidebar_label: "CC-PL-016"
description: "agnix rule CC-PL-016 checks for plugin name not kebab-case in claude plugins files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-PL-016", "plugin name not kebab-case", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-016`
- **Severity**: `HIGH`
- **Category**: `Claude Plugins`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-08-27`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/plugins-reference
- https://github.com/anthropics/claude-code/releases/tag/v2.1.247

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "name": "My Plugin"
}
```

### Valid

```json
{
  "name": "my-plugin"
}
```
