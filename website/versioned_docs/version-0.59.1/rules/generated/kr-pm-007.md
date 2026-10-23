---
id: kr-pm-007
title: "KR-PM-007: Kiro Permission Pattern Has No Effect"
sidebar_label: "KR-PM-007"
description: "agnix rule KR-PM-007 checks for kiro permission pattern has no effect in kiro permissions files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-PM-007", "kiro permission pattern has no effect", "kiro permissions", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PM-007`
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
- https://kiro.dev/docs/enterprise/governance/permissions/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```yaml
rules: [{capability: mcp, match: ["@github"], effect: allow}]
```

### Valid

```yaml
rules: [{capability: mcp, match: ["github/*"], effect: allow}]
```
