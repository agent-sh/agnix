---
id: oc-pm-004
title: "OC-PM-004: Broad external_directory Allow - OpenCode"
sidebar_label: "OC-PM-004"
description: "agnix rule OC-PM-004 checks for broad external_directory allow in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-PM-004", "broad external_directory allow", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-PM-004`
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
{"permission": {"external_directory": {"~/**": "allow"}}}
```

### Valid

```json
{"permission": {"external_directory": {"~/projects/personal/**": "allow"}}}
```
