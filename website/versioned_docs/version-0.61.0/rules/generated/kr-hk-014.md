---
id: kr-hk-014
title: "KR-HK-014: Kiro Hook Migration and Canonical Trigger Hint"
sidebar_label: "KR-HK-014"
description: "agnix rule KR-HK-014 checks for kiro hook migration and canonical trigger hint in kiro hooks files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-HK-014", "kiro hook migration and canonical trigger hint", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-014`
- **Severity**: `LOW`
- **Category**: `Kiro Hooks`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `Yes (safe)`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.8.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/hooks/
- https://kiro.dev/docs/hooks/types/
- https://kiro.dev/docs/ide/whats-new-v1/hooks/
- https://kiro.dev/docs/custom-agents/configuration-reference/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `true`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"version":"v1","hooks":[{"name":"init","trigger":"agentSpawn","action":{"type":"command","command":"git status"}}]}
```

### Valid

```json
{"version":"v1","hooks":[{"name":"init","trigger":"SessionStart","action":{"type":"command","command":"git status"}}]}
```
