---
id: cc-hk-022
title: "CC-HK-022: Invalid Shell Value - Claude Hooks"
sidebar_label: "CC-HK-022"
description: "agnix rule CC-HK-022 checks for invalid shell value in claude hooks files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-HK-022", "invalid shell value", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-022`
- **Severity**: `MEDIUM`
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
{ "type": "command", "command": "echo hi", "shell": "zsh" }
```

### Valid

```json
{ "type": "command", "command": "echo hi", "shell": "bash" }
```
