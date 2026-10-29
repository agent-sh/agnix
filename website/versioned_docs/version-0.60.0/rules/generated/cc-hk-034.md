---
id: cc-hk-034
title: "CC-HK-034: Plugin Hook Names Own MCP Server by Bare Key"
sidebar_label: "CC-HK-034"
description: "agnix rule CC-HK-034 checks for plugin hook names own mcp server by bare key in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-034", "plugin hook names own mcp server by bare key", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-034`
- **Severity**: `HIGH`
- **Category**: `Claude Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/hooks
- https://code.claude.com/docs/en/plugins/components

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
        "matcher": "mcp__db__.*",
        "hooks": [
          {
            "type": "command",
            "command": "echo db op"
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
        "matcher": "mcp__plugin_my-plugin_db__.*",
        "hooks": [
          {
            "type": "command",
            "command": "echo db op"
          }
        ]
      }
    ]
  }
}
```
