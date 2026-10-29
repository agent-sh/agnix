---
id: kr-pm-006
title: "KR-PM-006: Invalid Kiro Permission Policy Presets"
sidebar_label: "KR-PM-006"
description: "agnix rule KR-PM-006 checks for invalid kiro permission policy presets in kiro permissions files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-PM-006", "invalid kiro permission policy presets", "kiro permissions", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PM-006`
- **Severity**: `HIGH`
- **Category**: `Kiro Permissions`
- **Normative Level**: `MUST`
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
policies: read-only-shell
rules: []
```

### Valid

```yaml
policies: [read-only-shell]
rules: []
```
