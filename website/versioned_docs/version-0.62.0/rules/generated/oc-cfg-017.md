---
id: oc-cfg-017
title: "OC-CFG-017: Mixed OpenCode 1 and 2 Fields in One Entry"
sidebar_label: "OC-CFG-017"
description: "agnix rule OC-CFG-017 checks for mixed opencode 1 and 2 fields in one entry in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-CFG-017", "mixed opencode 1 and 2 fields in one entry", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-017`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `>=2.0.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/v2/docs/migrate-v1/
- https://opencode.ai/v2/docs/agents/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"agents": {"reviewer": {"prompt": "Review only.", "permissions": []}}}
```

### Valid

```json
{"agents": {"reviewer": {"system": "Review only.", "permissions": []}}}
```
