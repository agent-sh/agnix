---
id: kr-pm-001
title: "KR-PM-001: Invalid Kiro Permissions Document"
sidebar_label: "KR-PM-001"
description: "agnix rule KR-PM-001 checks for invalid kiro permissions document in kiro permissions files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-PM-001", "invalid kiro permissions document", "kiro permissions", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PM-001`
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
- E2E tests: `true`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```yaml
capability: fs_write
match: /**
effect: allow
```

### Valid

```yaml
rules: []
```
