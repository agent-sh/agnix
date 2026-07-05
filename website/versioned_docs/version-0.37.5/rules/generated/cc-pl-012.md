---
id: cc-pl-012
title: "CC-PL-012: Invalid UserConfig Key - Claude Plugins"
sidebar_label: "CC-PL-012"
description: "agnix rule CC-PL-012 checks for invalid userconfig key in claude plugins files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-PL-012", "invalid userconfig key", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-012`
- **Severity**: `MEDIUM`
- **Category**: `Claude Plugins`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-28`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/plugins-reference

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "userConfig": { "api key!": { "type": "string" } }
}
```

### Valid

```json
{
  "userConfig": { "apiKey": { "type": "string" } }
}
```
