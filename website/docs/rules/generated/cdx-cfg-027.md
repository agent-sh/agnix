---
id: cdx-cfg-027
title: "CDX-CFG-027: Invalid Windows Sandbox Value - Codex Config"
sidebar_label: "CDX-CFG-027"
description: "agnix rule CDX-CFG-027 checks for invalid windows sandbox value in codex config files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-CFG-027", "invalid windows sandbox value", "codex config", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-027`
- **Severity**: `LOW`
- **Category**: `Codex Config`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-28`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://developers.openai.com/codex/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
[sandbox]
windows = "docker"
```

### Valid

```json
[sandbox]
windows = "wsl"
```
