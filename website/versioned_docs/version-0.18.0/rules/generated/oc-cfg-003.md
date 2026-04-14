---
id: oc-cfg-003
title: "OC-CFG-003: Unknown Top-level Config Field - OpenCode"
sidebar_label: "OC-CFG-003"
description: "agnix rule OC-CFG-003 checks for unknown top-level config field in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-CFG-003", "unknown top-level config field", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-003`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-03`

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
  "mdoel": "anthropic/claude-3-opus"
}
```

### Valid

```json
{
  "model": "anthropic/claude-3-opus",
  "share": "manual"
}
```
