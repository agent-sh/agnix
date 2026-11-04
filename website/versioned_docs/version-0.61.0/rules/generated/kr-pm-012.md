---
id: kr-pm-012
title: "KR-PM-012: Shadowed Kiro sandbox_network Deny"
sidebar_label: "KR-PM-012"
description: "agnix rule KR-PM-012 checks for shadowed kiro sandbox_network deny in kiro permissions files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-PM-012", "shadowed kiro sandbox_network deny", "kiro permissions", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PM-012`
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
rules:
  - {capability: sandbox_network, match: ["*.example.com"], effect: allow}
  - {capability: sandbox_network, match: ["evil.example.com"], effect: deny}
```

### Valid

```yaml
rules:
  - {capability: sandbox_network, match: ["evil.example.com"], effect: deny}
  - {capability: sandbox_network, match: ["*.example.com"], effect: allow}
```
