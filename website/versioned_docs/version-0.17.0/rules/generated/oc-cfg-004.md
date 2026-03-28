---
id: oc-cfg-004
title: "OC-CFG-004: Invalid Default Agent - OpenCode"
sidebar_label: "OC-CFG-004"
description: "agnix rule OC-CFG-004 checks for invalid default agent in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-CFG-004", "invalid default agent", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-004`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-02`

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
  "default_agent": "unknown-agent"
}
```

### Valid

```json
{
  "default_agent": "build"
}
```
