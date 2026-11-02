---
id: kr-hk-028
title: "KR-HK-028: Kiro File Hook Matching Every File - Kiro Hooks"
sidebar_label: "KR-HK-028"
description: "agnix rule KR-HK-028 checks for kiro file hook matching every file in kiro hooks files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-HK-028", "kiro file hook matching every file", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-028`
- **Severity**: `LOW`
- **Category**: `Kiro Hooks`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.8.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/hooks/best-practices/
- https://kiro.dev/docs/hooks/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"version":"v1","hooks":[{"name":"lint","trigger":"PostFileSave","matcher":".*","action":{"type":"command","command":"npx eslint --fix"}}]}
```

### Valid

```json
{"version":"v1","hooks":[{"name":"lint-ts","trigger":"PostFileSave","matcher":"\\.ts$","action":{"type":"command","command":"npx eslint --fix"}}]}
```
