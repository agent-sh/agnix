---
id: cc-hk-025
title: "CC-HK-025: Invalid Matcher Value - Claude Hooks"
sidebar_label: "CC-HK-025"
description: "agnix rule CC-HK-025 checks for invalid matcher value in claude hooks files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-HK-025", "invalid matcher value", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-025`
- **Severity**: `LOW`
- **Category**: `Claude Hooks`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (unsafe)`
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
{ "matcher": 123, "hooks": [{ "type": "command", "command": "echo ok" }] }
```

### Valid

```json
{ "matcher": "Bash", "hooks": [{ "type": "command", "command": "echo ok" }] }
```
