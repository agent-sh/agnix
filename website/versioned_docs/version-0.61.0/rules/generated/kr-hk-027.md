---
id: kr-hk-027
title: "KR-HK-027: Kiro Tool Hook Without a Specific Matcher"
sidebar_label: "KR-HK-027"
description: "agnix rule KR-HK-027 checks for kiro tool hook without a specific matcher in kiro hooks files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-HK-027", "kiro tool hook without a specific matcher", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-027`
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
{"version":"v1","hooks":[{"name":"guard","trigger":"PreToolUse","matcher":"*","action":{"type":"command","command":"./scripts/guard.sh"}}]}
```

### Valid

```json
{"version":"v1","hooks":[{"name":"guard-writes","trigger":"PreToolUse","matcher":"write","action":{"type":"command","command":"./scripts/guard.sh"}}]}
```
