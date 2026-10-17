---
id: kr-hk-025
title: "KR-HK-025: Standalone Hook File Ignored by Pinned Kiro CLI 2.x"
sidebar_label: "KR-HK-025"
description: "agnix rule KR-HK-025 checks for standalone hook file ignored by pinned kiro cli 2.x in kiro hooks files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-HK-025", "standalone hook file ignored by pinned kiro cli 2.x", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-025`
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
- https://kiro.dev/docs/cli/v3/hooks-migration/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
[tool_versions]
kiro = "2.32.1"
# with .kiro/hooks/lint.json present
```

### Valid

```text
[tool_versions]
kiro = "3.0.0"
```
