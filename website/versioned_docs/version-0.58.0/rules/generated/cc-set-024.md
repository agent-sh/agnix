---
id: cc-set-024
title: "CC-SET-024: Ineffective Project sandbox.ripgrep Override"
sidebar_label: "CC-SET-024"
description: "agnix rule CC-SET-024 checks for ineffective project sandbox.ripgrep override in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-024", "ineffective project sandbox.ripgrep override", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-024`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-08-15`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.232`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.232
- https://code.claude.com/docs/en/sandboxing

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "sandbox": {"ripgrep": "/opt/rg/bin/rg"}
}
```

### Valid

```json
{
  "sandbox": {"enabled": true}
}
```
