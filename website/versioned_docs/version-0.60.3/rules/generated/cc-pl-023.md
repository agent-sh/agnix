---
id: cc-pl-023
title: "CC-PL-023: Invalid Key or Value in Strict Manifest Object"
sidebar_label: "CC-PL-023"
description: "agnix rule CC-PL-023 checks for invalid key or value in strict manifest object in claude plugins files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-PL-023", "invalid key or value in strict manifest object", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-023`
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

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "userConfig": {
    "api_token": { "type": "string", "title": "API token", "description": "Token", "secret": true }
  }
}
```

### Valid

```json
{
  "userConfig": {
    "api_token": { "type": "string", "title": "API token", "description": "Token", "sensitive": true }
  }
}
```
