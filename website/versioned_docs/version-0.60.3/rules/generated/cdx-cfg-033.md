---
id: cdx-cfg-033
title: "CDX-CFG-033: Permission Profile Shadowed by Legacy Sandbox Settings"
sidebar_label: "CDX-CFG-033"
description: "agnix rule CDX-CFG-033 checks for permission profile shadowed by legacy sandbox settings in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-CFG-033", "permission profile shadowed by legacy sandbox settings", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-033`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://learn.chatgpt.com/docs/permissions
- https://learn.chatgpt.com/docs/config-file/config-reference

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
default_permissions = ":workspace"
sandbox_mode = "read-only"
```

### Valid

```toml
default_permissions = ":workspace"
```
