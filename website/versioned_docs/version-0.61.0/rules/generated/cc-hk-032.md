---
id: cc-hk-032
title: "CC-HK-032: PowerShell Hook Placeholder Not Expanded"
sidebar_label: "CC-HK-032"
description: "agnix rule CC-HK-032 checks for powershell hook placeholder not expanded in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-032", "powershell hook placeholder not expanded", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-032`
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
            "shell": "powershell",
            "command": "& \"$CLAUDE_PROJECT_DIR\\.claude\\hooks\\check.ps1\""
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
            "shell": "powershell",
            "command": "& \"$env:CLAUDE_PROJECT_DIR\\.claude\\hooks\\check.ps1\""
          }
        ]
      }
    ]
  }
}
```
