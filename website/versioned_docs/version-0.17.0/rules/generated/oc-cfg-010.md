---
id: oc-cfg-010
title: "OC-CFG-010: Invalid Skills URL - OpenCode"
sidebar_label: "OC-CFG-010"
description: "agnix rule OC-CFG-010 checks for invalid skills url in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-CFG-010", "invalid skills url", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-010`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
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
  "skills": { "urls": ["not-a-url"] }
}
```

### Valid

```json
{
  "skills": { "urls": ["https://example.com"] }
}
```
