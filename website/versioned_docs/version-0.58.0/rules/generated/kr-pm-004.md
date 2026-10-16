---
id: kr-pm-004
title: "KR-PM-004: Invalid Kiro Permission Patterns"
sidebar_label: "KR-PM-004"
description: "agnix rule KR-PM-004 checks for invalid kiro permission patterns in kiro permissions files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-PM-004", "invalid kiro permission patterns", "kiro permissions", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PM-004`
- **Severity**: `HIGH`
- **Category**: `Kiro Permissions`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-16`

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
- E2E tests: `true`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```yaml
rules: [{capability: shell, effect: allow, match: "git *"}]
```

### Valid

```yaml
rules:
  - capability: shell
    match: ["git *"]
    effect: allow
```
