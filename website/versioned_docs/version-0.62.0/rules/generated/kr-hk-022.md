---
id: kr-hk-022
title: "KR-HK-022: Misplaced or Misspelled Kiro V3 Hook Field"
sidebar_label: "KR-HK-022"
description: "agnix rule KR-HK-022 checks for misplaced or misspelled kiro v3 hook field in kiro hooks files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-HK-022", "misplaced or misspelled kiro v3 hook field", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-022`
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
- https://kiro.dev/docs/cli/v3/hooks-migration/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"version":"v1","hooks":[{"name":"clippy","trigger":"Stop","action":{"type":"command","command":"cargo clippy","timeout":300}}]}
```

### Valid

```json
{"version":"v1","hooks":[{"name":"clippy","trigger":"Stop","action":{"type":"command","command":"cargo clippy"},"timeout":300}]}
```
