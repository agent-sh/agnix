---
id: oc-dep-006
title: "OC-DEP-006: Deprecated maxSteps Field - OpenCode"
sidebar_label: "OC-DEP-006"
description: "agnix rule OC-DEP-006 checks for deprecated maxsteps field in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-DEP-006", "deprecated maxsteps field", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-DEP-006`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/agents/
- https://opencode.ai/v2/docs/migrate-v1/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
maxSteps: 20
```

### Valid

```json
steps: 20
```
