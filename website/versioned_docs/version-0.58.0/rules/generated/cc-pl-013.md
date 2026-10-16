---
id: cc-pl-013
title: "CC-PL-013: Channel Missing Server Reference - Claude Plugins"
sidebar_label: "CC-PL-013"
description: "agnix rule CC-PL-013 checks for channel missing server reference in claude plugins files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-PL-013", "channel missing server reference", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-013`
- **Severity**: `MEDIUM`
- **Category**: `Claude Plugins`
- **Normative Level**: `MUST`
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
  "channels": [{ "name": "stable" }]
}
```

### Valid

```json
{
  "channels": [{ "name": "stable", "server": "https://example.com" }]
}
```
