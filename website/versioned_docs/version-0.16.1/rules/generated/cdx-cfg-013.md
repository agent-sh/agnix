---
id: cdx-cfg-013
title: "CDX-CFG-013: Invalid sandbox_workspace_write Mode"
sidebar_label: "CDX-CFG-013"
description: "agnix rule CDX-CFG-013 checks for invalid sandbox_workspace_write mode in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-CFG-013", "invalid sandbox_workspace_write mode", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-013`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-06`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://developers.openai.com/codex/config-reference
- https://developers.openai.com/codex/config-schema.json

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[sandbox_workspace_write]
mode = "yolo"
```

### Valid

```toml
[sandbox_workspace_write]
mode = "allowlist"
```
