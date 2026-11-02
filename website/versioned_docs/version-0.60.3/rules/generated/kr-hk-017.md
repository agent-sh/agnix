---
id: kr-hk-017
title: "KR-HK-017: Invalid Kiro V3 Hook Field Value - Kiro Hooks"
sidebar_label: "KR-HK-017"
description: "agnix rule KR-HK-017 checks for invalid kiro v3 hook field value in kiro hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-HK-017", "invalid kiro v3 hook field value", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-017`
- **Severity**: `HIGH`
- **Category**: `Kiro Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.8.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/hooks/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"version":"v1","hooks":[{"name":"build","trigger":"Stop","action":{"type":"command","command":"make"},"timeout":"120","enabled":"true"}]}
```

### Valid

```json
{"version":"v1","hooks":[{"name":"build","trigger":"Stop","action":{"type":"command","command":"make"},"timeout":120,"enabled":true}]}
```
