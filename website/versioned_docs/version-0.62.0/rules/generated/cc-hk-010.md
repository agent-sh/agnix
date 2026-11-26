---
id: cc-hk-010
title: "CC-HK-010: SessionEnd Hook Timeout Above Budget"
sidebar_label: "CC-HK-010"
description: "agnix rule CC-HK-010 checks for sessionend hook timeout above budget in claude hooks files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-HK-010", "sessionend hook timeout above budget", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-010`
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
    "Stop": [
      {
        "hooks": [
          { "type": "agent", "prompt": "Review: $ARGUMENTS", "timeout": 120 }
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
    "Stop": [
      {
        "hooks": [
          { "type": "agent", "prompt": "Review: $ARGUMENTS", "timeout": 60 }
        ]
      }
    ]
  }
}
```
