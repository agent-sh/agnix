---
id: oc-dep-009
title: "OC-DEP-009: Field Ignored by OpenCode 2 - OpenCode"
sidebar_label: "OC-DEP-009"
description: "agnix rule OC-DEP-009 checks for field ignored by opencode 2 in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-DEP-009", "field ignored by opencode 2", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-DEP-009`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `>=2.0.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/v2/docs/migrate-v1/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"permissions": [], "logLevel": "INFO"}
```

### Valid

```json
OPENCODE_LOG_LEVEL=INFO opencode
```
