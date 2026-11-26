---
id: oc-tui-002
title: "OC-TUI-002: Invalid scroll_speed - OpenCode"
sidebar_label: "OC-TUI-002"
description: "agnix rule OC-TUI-002 checks for invalid scroll_speed in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-TUI-002", "invalid scroll_speed", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-TUI-002`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/config/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
"scroll_speed": 0
```

### Valid

```json
"scroll_speed": 3
```
