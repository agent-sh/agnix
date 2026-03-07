---
id: oc-cfg-005
title: "OC-CFG-005: Hardcoded API Key - OpenCode"
sidebar_label: "OC-CFG-005"
description: "agnix rule OC-CFG-005 checks for hardcoded api key in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-CFG-005", "hardcoded api key", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-005`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
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
  "provider": { "options": { "apiKey": "sk-1234567890abcdef" } }
}
```

### Valid

```json
{
  "provider": { "options": { "apiKey": "{env:OPENAI_API_KEY}" } }
}
```
