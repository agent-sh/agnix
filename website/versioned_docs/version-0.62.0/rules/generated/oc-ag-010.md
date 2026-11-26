---
id: oc-ag-010
title: "OC-AG-010: OpenCode Agent Without Description - OpenCode"
sidebar_label: "OC-AG-010"
description: "agnix rule OC-AG-010 checks for opencode agent without description in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-AG-010", "opencode agent without description", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-AG-010`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/v2/docs/agents/
- https://opencode.ai/docs/agents/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
---
mode: subagent
---
```

### Valid

```json
---
description: Reviews code
mode: subagent
---
```
