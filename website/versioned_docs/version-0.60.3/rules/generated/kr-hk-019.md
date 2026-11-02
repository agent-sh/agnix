---
id: kr-hk-019
title: "KR-HK-019: Kiro V3 Hook Matcher Is Not a Valid Regex"
sidebar_label: "KR-HK-019"
description: "agnix rule KR-HK-019 checks for kiro v3 hook matcher is not a valid regex in kiro hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-HK-019", "kiro v3 hook matcher is not a valid regex", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-019`
- **Severity**: `HIGH`
- **Category**: `Kiro Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.8.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/hooks/
- https://kiro.dev/docs/hooks/types/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"version":"v1","hooks":[{"name":"format","trigger":"PostFileSave","matcher":"**/*.ts","action":{"type":"command","command":"npx eslint --fix"}}]}
```

### Valid

```json
{"version":"v1","hooks":[{"name":"format","trigger":"PostFileSave","matcher":"\\.ts$","action":{"type":"command","command":"npx eslint --fix"}}]}
```
