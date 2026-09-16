---
id: cc-hk-004
title: "CC-HK-004: Matcher on Unsupported Event - Claude Hooks"
sidebar_label: "CC-HK-004"
description: "agnix rule CC-HK-004 checks for matcher on unsupported event in claude hooks files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-HK-004", "matcher on unsupported event", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-004`
- **Severity**: `LOW`
- **Category**: `Claude Hooks`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `Yes (safe)`
- **Verified On**: `2026-07-02`

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
    "PostToolBatch": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "echo batch done", "timeout": 30 }
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
    "PostToolBatch": [
      {
        "hooks": [
          { "type": "command", "command": "echo batch done", "timeout": 30 }
        ]
      }
    ]
  }
}
```
