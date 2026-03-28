---
id: cc-hk-024
title: "CC-HK-024: Headers Missing AllowedEnvVars - Claude Hooks"
sidebar_label: "CC-HK-024"
description: "agnix rule CC-HK-024 checks for headers missing allowedenvvars in claude hooks files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-HK-024", "headers missing allowedenvvars", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-024`
- **Severity**: `MEDIUM`
- **Category**: `Claude Hooks`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe)`
- **Verified On**: `2026-03-28`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/hooks

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{ "type": "http", "url": "https://ex.com", "headers": { "Authorization": "$TOKEN" } }
```

### Valid

```json
{ "type": "http", "url": "https://ex.com", "headers": { "Authorization": "$TOKEN" }, "allowedEnvVars": ["TOKEN"] }
```
