---
id: cc-hk-029
title: "CC-HK-029: Invalid or Ineffective onFailure - Claude Hooks"
sidebar_label: "CC-HK-029"
description: "agnix rule CC-HK-029 checks for invalid or ineffective onfailure in claude hooks files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-HK-029", "invalid or ineffective onfailure", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-029`
- **Severity**: `MEDIUM`
- **Category**: `Claude Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.295`
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
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "check.sh",
            "onFailure": "deny"
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
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "node",
            "args": [
              "check.js"
            ],
            "onFailure": "block"
          }
        ]
      }
    ]
  }
}
```
