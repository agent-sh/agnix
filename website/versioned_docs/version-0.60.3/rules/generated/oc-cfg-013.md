---
id: oc-cfg-013
title: "OC-CFG-013: Invalid Server Config - OpenCode"
sidebar_label: "OC-CFG-013"
description: "agnix rule OC-CFG-013 checks for invalid server config in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-CFG-013", "invalid server config", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-013`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `<2.0.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/config/
- https://opencode.ai/config.json
- https://opencode.ai/v2/docs/migrate-v1/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"server": {"port": "8080"}}
```

### Valid

```json
{"server": {"port": 4096}}
```
