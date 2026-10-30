---
id: cdx-hk-003
title: "CDX-HK-003: Hooks in Both hooks.json and Inline [hooks]"
sidebar_label: "CDX-HK-003"
description: "agnix rule CDX-HK-003 checks for hooks in both hooks.json and inline [hooks] in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-HK-003", "hooks in both hooks.json and inline [hooks]", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-HK-003`
- **Severity**: `LOW`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://learn.chatgpt.com/docs/hooks
- https://learn.chatgpt.com/docs/config-file/config-advanced

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
.codex/hooks.json and [[hooks.Stop]] in .codex/config.toml
```

### Valid

```toml
.codex/hooks.json only
```
