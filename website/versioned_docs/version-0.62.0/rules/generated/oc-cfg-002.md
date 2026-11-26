---
id: oc-cfg-002
title: "OC-CFG-002: Invalid autoupdate or update Value - OpenCode"
sidebar_label: "OC-CFG-002"
description: "agnix rule OC-CFG-002 checks for invalid autoupdate or update value in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-CFG-002", "invalid autoupdate or update value", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-002`
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
- https://opencode.ai/v2/docs/config/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"update": "always"}
```

### Valid

```json
{"autoupdate": "notify"}
```
