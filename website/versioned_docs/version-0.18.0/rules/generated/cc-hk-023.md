---
id: cc-hk-023
title: "CC-HK-023: Once Field Not Boolean - Claude Hooks"
sidebar_label: "CC-HK-023"
description: "agnix rule CC-HK-023 checks for once field not boolean in claude hooks files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-HK-023", "once field not boolean", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-023`
- **Severity**: `LOW`
- **Category**: `Claude Hooks`
- **Normative Level**: `MUST`
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
{ "type": "command", "command": "echo hi", "once": "yes" }
```

### Valid

```json
{ "type": "command", "command": "echo hi", "once": true }
```
