---
id: oc-dep-008
title: "OC-DEP-008: Legacy Singular .opencode Subdirectory"
sidebar_label: "OC-DEP-008"
description: "agnix rule OC-DEP-008 checks for legacy singular .opencode subdirectory in opencode files. Severity: LOW. See examples and fix guidance."
keywords: ["OC-DEP-008", "legacy singular .opencode subdirectory", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-DEP-008`
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

- https://opencode.ai/docs/config/
- https://opencode.ai/v2/docs/commands/
- https://opencode.ai/v2/docs/migrate-v1/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
.opencode/agent/review.md
```

### Valid

```json
.opencode/agents/review.md
```
