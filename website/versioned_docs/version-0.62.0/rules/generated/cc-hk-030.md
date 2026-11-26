---
id: cc-hk-030
title: "CC-HK-030: Unquoted Path Placeholder in Shell-Form Hook"
sidebar_label: "CC-HK-030"
description: "agnix rule CC-HK-030 checks for unquoted path placeholder in shell-form hook in claude hooks files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-HK-030", "unquoted path placeholder in shell-form hook", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-030`
- **Severity**: `MEDIUM`
- **Category**: `Claude Hooks`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/hooks
- https://code.claude.com/docs/en/plugins/manifest-reference
- https://code.claude.com/docs/en/hooks-guide

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
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/scripts/format.sh"
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
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "\"${CLAUDE_PLUGIN_ROOT}\"/scripts/format.sh"
          }
        ]
      }
    ]
  }
}
```
