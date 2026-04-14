---
id: oc-cfg-008
title: "OC-CFG-008: Invalid Log Level - OpenCode"
sidebar_label: "OC-CFG-008"
description: "agnix rule OC-CFG-008 checks for invalid log level in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-CFG-008", "invalid log level", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-008`
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
  "logLevel": "verbose"
}
```

### Valid

```json
{
  "logLevel": "info"
}
```
