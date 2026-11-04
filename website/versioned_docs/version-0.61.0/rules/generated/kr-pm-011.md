---
id: kr-pm-011
title: "KR-PM-011: Unsupported Pattern Syntax in Kiro Shell, Web or MCP Rule"
sidebar_label: "KR-PM-011"
description: "agnix rule KR-PM-011 checks for unsupported pattern syntax in kiro shell, web or mcp rule in kiro permissions files. Severity: MEDIUM. See examples and fix g..."
keywords: ["KR-PM-011", "unsupported pattern syntax in kiro shell, web or mcp rule", "kiro permissions", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PM-011`
- **Severity**: `MEDIUM`
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
rules: [{capability: shell, match: ["git **"], effect: allow}]
```

### Valid

```yaml
rules: [{capability: shell, match: ["git *"], effect: allow}]
```
