---
id: cc-set-019
title: "CC-SET-019: Non-boolean sandbox.network.strictAllowlist Setting"
sidebar_label: "CC-SET-019"
description: "agnix rule CC-SET-019 checks for non-boolean sandbox.network.strictallowlist setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-019", "non-boolean sandbox.network.strictallowlist setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-019`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-26`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.219`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.219
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
  "sandbox": {"network": {"strictAllowlist": "true"}}
}
```

### Valid

```json
{
  "sandbox": {"network": {"strictAllowlist": true}}
}
```
