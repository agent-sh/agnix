---
id: cc-pl-021
title: "CC-PL-021: Plugin Settings Key Dropped - Claude Plugins"
sidebar_label: "CC-PL-021"
description: "agnix rule CC-PL-021 checks for plugin settings key dropped in claude plugins files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-PL-021", "plugin settings key dropped", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-021`
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
- https://code.claude.com/docs/en/plugins/manifest-reference

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "agent": "reviewer",
  "permissions": { "allow": ["Bash"] }
}
```

### Valid

```json
{
  "agent": "reviewer"
}
```
