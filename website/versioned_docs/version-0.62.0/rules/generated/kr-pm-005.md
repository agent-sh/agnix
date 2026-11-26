---
id: kr-pm-005
title: "KR-PM-005: Unknown Field in Kiro Permission Rule"
sidebar_label: "KR-PM-005"
description: "agnix rule KR-PM-005 checks for unknown field in kiro permission rule in kiro permissions files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-PM-005", "unknown field in kiro permission rule", "kiro permissions", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PM-005`
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
- https://kiro.dev/docs/enterprise/governance/permissions/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```yaml
rules: [{capability: shell, match: ["git *"], effect: allow, comment: "git only"}]
```

### Valid

```yaml
rules: [{capability: shell, match: ["git *"], effect: allow}]
```
