---
id: cdx-ag-003
title: "CDX-AG-003: Generic AGENTS.md Guidance for Codex - Codex CLI"
sidebar_label: "CDX-AG-003"
description: "agnix rule CDX-AG-003 checks for generic agents.md guidance for codex in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-AG-003", "generic agents.md guidance for codex", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-AG-003`
- **Severity**: `LOW`
- **Category**: `Codex CLI`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://learn.chatgpt.com/guides/best-practices

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
Be helpful and accurate.
```

### Valid

```toml
Run `cargo test -p agnix-core` and `cargo test -p agnix-cli` before merge.
```
