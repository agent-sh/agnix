---
id: cc-pl-014
title: "CC-PL-014: Plugin Agent Field Ignored - Claude Plugins"
sidebar_label: "CC-PL-014"
description: "agnix rule CC-PL-014 checks for plugin agent field ignored in claude plugins files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-PL-014", "plugin agent field ignored", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-014`
- **Severity**: `MEDIUM`
- **Category**: `Claude Plugins`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/plugins/components
- https://code.claude.com/docs/en/sub-agents

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
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
