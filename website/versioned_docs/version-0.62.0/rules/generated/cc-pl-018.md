---
id: cc-pl-018
title: "CC-PL-018: Reserved Plugin Name - Claude Plugins"
sidebar_label: "CC-PL-018"
description: "agnix rule CC-PL-018 checks for reserved plugin name in claude plugins files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-PL-018", "reserved plugin name", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-018`
- **Severity**: `HIGH`
- **Category**: `Claude Plugins`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/plugins/manifest-reference
- https://code.claude.com/docs/en/plugins/marketplace-reference

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "name": "claude-deploy-tools"
}
```

### Valid

```json
{
  "name": "deploy-tools"
}
```
