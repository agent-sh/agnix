---
id: kr-pm-009
title: "KR-PM-009: Kiro Does Not Read This Permissions File"
sidebar_label: "KR-PM-009"
description: "agnix rule KR-PM-009 checks for kiro does not read this permissions file in kiro permissions files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-PM-009", "kiro does not read this permissions file", "kiro permissions", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PM-009`
- **Severity**: `LOW`
- **Category**: `Kiro Permissions`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `CLI V3 / IDE 1.x capability permissions`

## Evidence Sources

- https://kiro.dev/docs/permissions/
- https://kiro.dev/docs/cli/v3/permissions/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```yaml
# <repo>/.kiro/settings/permissions.yaml
rules: []
```

### Valid

```yaml
# ~/.kiro/settings/permissions.yaml
rules: []
```
