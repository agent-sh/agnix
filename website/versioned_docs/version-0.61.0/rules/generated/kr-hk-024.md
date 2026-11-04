---
id: kr-hk-024
title: "KR-HK-024: Kiro V3 Hook Never Runs - Kiro Hooks"
sidebar_label: "KR-HK-024"
description: "agnix rule KR-HK-024 checks for kiro v3 hook never runs in kiro hooks files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-HK-024", "kiro v3 hook never runs", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-024`
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

- https://kiro.dev/docs/hooks/types/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"version":"v1","hooks":[{"name":"farewell","trigger":"SessionEnd","action":{"type":"agent","prompt":"Summarize the session"}}]}
```

### Valid

```json
{"version":"v1","hooks":[{"name":"cleanup","trigger":"SessionEnd","action":{"type":"command","command":"rm -f .lock"}}]}
```
