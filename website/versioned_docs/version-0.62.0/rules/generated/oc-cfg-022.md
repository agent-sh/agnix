---
id: oc-cfg-022
title: "OC-CFG-022: Global-Only Setting in Project Config - OpenCode"
sidebar_label: "OC-CFG-022"
description: "agnix rule OC-CFG-022 checks for global-only setting in project config in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-CFG-022", "global-only setting in project config", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-022`
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

- https://opencode.ai/v2/docs/config/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```xml
<project>/opencode.json: {"update": "auto"}
```

### Valid

```xml
~/.config/opencode/opencode.json: {"update": "notify"}
```
