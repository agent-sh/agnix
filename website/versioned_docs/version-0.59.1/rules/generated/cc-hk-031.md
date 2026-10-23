---
id: cc-hk-031
title: "CC-HK-031: Exec-Form Command With Embedded Arguments"
sidebar_label: "CC-HK-031"
description: "agnix rule CC-HK-031 checks for exec-form command with embedded arguments in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-031", "exec-form command with embedded arguments", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-031`
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
    "PostToolUse": [
      {
        "matcher": "Write",
        "hooks": [
          {
            "type": "command",
            "command": "node script.js",
            "args": [
              "--fix"
            ]
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
    "PostToolUse": [
      {
        "matcher": "Write",
        "hooks": [
          {
            "type": "command",
            "command": "node",
            "args": [
              "script.js",
              "--fix"
            ]
          }
        ]
      }
    ]
  }
}
```
