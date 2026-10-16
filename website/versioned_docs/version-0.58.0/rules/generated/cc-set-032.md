---
id: cc-set-032
title: "CC-SET-032: Ineffective Project allowedProviders Setting"
sidebar_label: "CC-SET-032"
description: "agnix rule CC-SET-032 checks for ineffective project allowedproviders setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-032", "ineffective project allowedproviders setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-032`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-09-30`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.285`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.285

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "allowedProviders": ["anthropic"]
}
```

### Valid

```json
{
  "allowedProviders": ["anthropic", "bedrock"]
}
```
