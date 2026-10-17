---
id: kr-pm-010
title: "KR-PM-010: Unrestricted Kiro Allow Rule for a Write-Capable Capability"
sidebar_label: "KR-PM-010"
description: "agnix rule KR-PM-010 checks for unrestricted kiro allow rule for a write-capable capability in kiro permissions files. Severity: MEDIUM. See examples and fix..."
keywords: ["KR-PM-010", "unrestricted kiro allow rule for a write-capable capability", "kiro permissions", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PM-010`
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
- https://kiro.dev/docs/custom-agents/configuration-reference/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```yaml
rules: [{capability: shell, effect: allow}]
```

### Valid

```yaml
rules: [{capability: shell, match: ["git status", "npm test *"], effect: allow}]
```
