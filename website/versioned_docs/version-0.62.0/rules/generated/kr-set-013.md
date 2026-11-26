---
id: kr-set-013
title: "KR-SET-013: Invalid compaction.autoCompactionThreshold Value"
sidebar_label: "KR-SET-013"
description: "agnix rule KR-SET-013 checks for invalid compaction.autocompactionthreshold value in kiro settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-SET-013", "invalid compaction.autocompactionthreshold value", "kiro settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-SET-013`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-11-15`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=3.3.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/reference/settings/
- https://kiro.dev/changelog/cli/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"compaction.autoCompactionThreshold": 150}
```

### Valid

```json
{"compaction.autoCompactionThreshold": 70}
```
