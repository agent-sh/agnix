---
id: oc-tool-001
title: "OC-TOOL-001: Custom Tool Shadows Built-in Tool - OpenCode"
sidebar_label: "OC-TOOL-001"
description: "agnix rule OC-TOOL-001 checks for custom tool shadows built-in tool in opencode files. Severity: LOW. See examples and fix guidance."
keywords: ["OC-TOOL-001", "custom tool shadows built-in tool", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-TOOL-001`
- **Severity**: `LOW`
- **Category**: `OpenCode`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/custom-tools/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
.opencode/tools/bash.ts with export default
```

### Valid

```json
.opencode/tools/database.ts
```
