---
id: ws-hk-004
title: "WS-HK-004: Relative Script Path in Cascade Hook - windsurf"
sidebar_label: "WS-HK-004"
description: "agnix rule WS-HK-004 checks for relative script path in cascade hook in windsurf files. Severity: LOW. See examples and fix guidance."
keywords: ["WS-HK-004", "relative script path in cascade hook", "windsurf", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `WS-HK-004`
- **Severity**: `LOW`
- **Category**: `windsurf`
- **Normative Level**: `BEST_PRACTICE`
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
{"hooks": {"post_write_code": [{"command": "bash hooks/format_code.sh"}]}}
```

### Valid

```json
{"hooks": {"post_write_code": [{"command": "bash /Users/me/hooks/format_code.sh"}]}}
```
