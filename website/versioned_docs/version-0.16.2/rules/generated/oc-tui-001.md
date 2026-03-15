---
id: oc-tui-001
title: "OC-TUI-001: Unknown TUI Key - OpenCode"
sidebar_label: "OC-TUI-001"
description: "agnix rule OC-TUI-001 checks for unknown tui key in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-TUI-001", "unknown tui key", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-TUI-001`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
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
  "tui": { "unknown_opt": true }
}
```

### Valid

```json
{
  "tui": { "theme": "dark" }
}
```
