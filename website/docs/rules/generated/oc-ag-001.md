---
id: oc-ag-001
title: "OC-AG-001: Invalid Agent Mode Value - OpenCode"
sidebar_label: "OC-AG-001"
description: "agnix rule OC-AG-001 checks for invalid agent mode value in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-AG-001", "invalid agent mode value", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-AG-001`
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

- https://opencode.ai/docs/agents/
- https://opencode.ai/v2/docs/agents/
- https://opencode.ai/config.json

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
mode: helper
```

### Valid

```json
mode: subagent
```
