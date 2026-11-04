---
id: kr-pm-008
title: "KR-PM-008: Empty Kiro Permission match List Matches Everything"
sidebar_label: "KR-PM-008"
description: "agnix rule KR-PM-008 checks for empty kiro permission match list matches everything in kiro permissions files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-PM-008", "empty kiro permission match list matches everything", "kiro permissions", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PM-008`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Permissions`
- **Normative Level**: `SHOULD`
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
rules: [{capability: shell, match: [], effect: allow}]
```

### Valid

```yaml
rules: [{capability: shell, match: ["git status"], effect: allow}]
```
