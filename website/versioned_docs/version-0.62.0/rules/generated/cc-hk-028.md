---
id: cc-hk-028
title: "CC-HK-028: Rejected user_config Interpolation in Shell-Form Command"
sidebar_label: "CC-HK-028"
description: "agnix rule CC-HK-028 checks for rejected user_config interpolation in shell-form command in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-028", "rejected user_config interpolation in shell-form command", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-028`
- **Severity**: `HIGH`
- **Category**: `Claude Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-31`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.207`
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
{ "type": "command", "command": "my-script ${user_config.api_key}" }
```

### Valid

```json
{ "type": "command", "command": "my-script $CLAUDE_PLUGIN_OPTION_API_KEY" }
```
