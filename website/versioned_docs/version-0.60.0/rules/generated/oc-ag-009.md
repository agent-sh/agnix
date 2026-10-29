---
id: oc-ag-009
title: "OC-AG-009: Invalid Agent disable or disabled Type - OpenCode"
sidebar_label: "OC-AG-009"
description: "agnix rule OC-AG-009 checks for invalid agent disable or disabled type in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-AG-009", "invalid agent disable or disabled type", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-AG-009`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/config.json
- https://opencode.ai/v2/docs/agents/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
disable: "yes"
```

### Valid

```json
disabled: true
```
