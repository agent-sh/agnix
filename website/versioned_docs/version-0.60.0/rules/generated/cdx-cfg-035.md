---
id: cdx-cfg-035
title: "CDX-CFG-035: Invalid Permission Profile Inheritance"
sidebar_label: "CDX-CFG-035"
description: "agnix rule CDX-CFG-035 checks for invalid permission profile inheritance in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-CFG-035", "invalid permission profile inheritance", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-035`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://learn.chatgpt.com/docs/permissions

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[permissions.project-edit]
extends = ":danger-full-access"
```

### Valid

```toml
[permissions.project-edit]
extends = ":workspace"
```
