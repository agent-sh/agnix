---
id: cc-hk-033
title: "CC-HK-033: MCP Matcher Without Wildcard - Claude Hooks"
sidebar_label: "CC-HK-033"
description: "agnix rule CC-HK-033 checks for mcp matcher without wildcard in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-033", "mcp matcher without wildcard", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-033`
- **Severity**: `HIGH`
- **Category**: `Claude Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/hooks

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "mcp__memory",
        "hooks": [
          {
            "type": "command",
            "command": "echo memory op"
          }
        ]
      }
    ]
  }
}
```

### Valid

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "mcp__memory__.*",
        "hooks": [
          {
            "type": "command",
            "command": "echo memory op"
          }
        ]
      }
    ]
  }
}
```
