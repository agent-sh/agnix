---
id: cdx-cfg-025
title: "CDX-CFG-025: Invalid Service Tier Value - Codex CLI"
sidebar_label: "CDX-CFG-025"
description: "agnix rule CDX-CFG-025 checks for invalid service tier value in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-CFG-025", "invalid service tier value", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-025`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
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

```toml
service_tier = "ultra"
```

### Valid

```toml
service_tier = "default"
```
