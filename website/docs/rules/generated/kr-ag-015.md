---
id: kr-ag-015
title: "KR-AG-015: Unconverted CLI 2.x Trust Configuration"
sidebar_label: "KR-AG-015"
description: "agnix rule KR-AG-015 checks for unconverted cli 2.x trust configuration in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-015", "unconverted cli 2.x trust configuration", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-015`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Agents`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-16`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.30.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/custom-agents/configuration-reference/
- https://kiro.dev/docs/cli/v3/migration-guide/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"tools": ["fs_read"], "allowedTools": ["fs_read"], "toolsSettings": {"fs_read": {"allowedPaths": ["src/**"]}}}
```

### Valid

```json
{"tools": ["read", "shell"], "permissions": {"rules": [{"capability": "shell", "match": ["npm *"], "effect": "allow"}]}}
```
