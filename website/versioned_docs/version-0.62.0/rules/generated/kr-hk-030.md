---
id: kr-hk-030
title: "KR-HK-030: Kiro Hook File Name Not Kebab-Case - Kiro Hooks"
sidebar_label: "KR-HK-030"
description: "agnix rule KR-HK-030 checks for kiro hook file name not kebab-case in kiro hooks files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-HK-030", "kiro hook file name not kebab-case", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-030`
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

- https://kiro.dev/docs/hooks/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
.kiro/hooks/Lint_On_Save.json
```

### Valid

```text
.kiro/hooks/lint-on-save.json
```
