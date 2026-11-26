---
id: cdx-cfg-032
title: "CDX-CFG-032: Key Ignored in Project Config - Codex CLI"
sidebar_label: "CDX-CFG-032"
description: "agnix rule CDX-CFG-032 checks for key ignored in project config in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-CFG-032", "key ignored in project config", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-032`
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

- https://learn.chatgpt.com/docs/config-file/config-advanced

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
# <repo>/.codex/config.toml
model_provider = "azure"
```

### Valid

```toml
# ~/.codex/config.toml
model_provider = "azure"
```
