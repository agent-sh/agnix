---
id: cc-pl-014
title: "CC-PL-014: Plugin Agent Unsupported Field - Claude Plugins"
sidebar_label: "CC-PL-014"
description: "agnix rule CC-PL-014 checks for plugin agent unsupported field in claude plugins files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-PL-014", "plugin agent unsupported field", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-014`
- **Severity**: `MEDIUM`
- **Category**: `Claude Plugins`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe)`
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
  "agents": [{ "name": "helper", "priority": 1 }]
}
```

### Valid

```json
{
  "agents": [{ "name": "helper", "description": "Assists with code" }]
}
```
