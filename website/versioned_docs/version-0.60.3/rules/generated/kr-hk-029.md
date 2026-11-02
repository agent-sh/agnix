---
id: kr-hk-029
title: "KR-HK-029: Kiro Tool Matcher Is an Unanchored Regex"
sidebar_label: "KR-HK-029"
description: "agnix rule KR-HK-029 checks for kiro tool matcher is an unanchored regex in kiro hooks files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-HK-029", "kiro tool matcher is an unanchored regex", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-029`
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

- https://kiro.dev/docs/hooks/types/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"version":"v1","hooks":[{"name":"guard-writes","trigger":"PreToolUse","matcher":"fs_write|str_replace","action":{"type":"command","command":"./scripts/guard.sh"}}]}
```

### Valid

```json
{"version":"v1","hooks":[{"name":"guard-writes","trigger":"PreToolUse","matcher":"^(fs_write|str_replace)$","action":{"type":"command","command":"./scripts/guard.sh"}}]}
```
