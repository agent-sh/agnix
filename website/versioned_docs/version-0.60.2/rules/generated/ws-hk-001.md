---
id: ws-hk-001
title: "WS-HK-001: Invalid Cascade Hooks File - windsurf"
sidebar_label: "WS-HK-001"
description: "agnix rule WS-HK-001 checks for invalid cascade hooks file in windsurf files. Severity: HIGH. See examples and fix guidance."
keywords: ["WS-HK-001", "invalid cascade hooks file", "windsurf", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `WS-HK-001`
- **Severity**: `HIGH`
- **Category**: `windsurf`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `windsurf`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://docs.devin.ai/desktop/cascade/hooks

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
    "post_write_code": [
      { "show_output": true }
    ]
  }
}
```

### Valid

```json
{
  "hooks": {
    "post_write_code": [
      { "command": "python3 /path/to/another/script.py", "show_output": true }
    ]
  }
}
```
