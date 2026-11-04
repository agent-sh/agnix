---
id: oc-pm-007
title: "OC-PM-007: Shell Pattern Without Argument Wildcard"
sidebar_label: "OC-PM-007"
description: "agnix rule OC-PM-007 checks for shell pattern without argument wildcard in opencode files. Severity: LOW. See examples and fix guidance."
keywords: ["OC-PM-007", "shell pattern without argument wildcard", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-PM-007`
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

- https://opencode.ai/docs/permissions/
- https://opencode.ai/v2/docs/permissions/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
"git status": "allow"
```

### Valid

```json
"git status *": "allow"
```
