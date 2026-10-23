---
id: kr-hk-020
title: "KR-HK-020: Kiro V3 Hook Matcher Has No Effect - Kiro Hooks"
sidebar_label: "KR-HK-020"
description: "agnix rule KR-HK-020 checks for kiro v3 hook matcher has no effect in kiro hooks files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-HK-020", "kiro v3 hook matcher has no effect", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-020`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Hooks`
- **Normative Level**: `SHOULD`
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
{"version":"v1","hooks":[{"name":"context","trigger":"UserPromptSubmit","matcher":"deploy","action":{"type":"agent","prompt":"Read docs/deploy.md first"}}]}
```

### Valid

```json
{"version":"v1","hooks":[{"name":"context","trigger":"UserPromptSubmit","action":{"type":"agent","prompt":"Read docs/deploy.md first"}}]}
```
