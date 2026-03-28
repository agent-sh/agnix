---
id: oc-tui-003
title: "OC-TUI-003: Invalid diff_style - OpenCode"
sidebar_label: "OC-TUI-003"
description: "agnix rule OC-TUI-003 checks for invalid diff_style in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-TUI-003", "invalid diff_style", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-TUI-003`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-03-06`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/config

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "tui": { "diff_style": "unified" }
}
```

### Valid

```json
{
  "tui": { "diff_style": "auto" }
}
```
