---
id: cdx-ag-003
title: "CDX-AG-003: Generic AGENTS.md Guidance for Codex - Codex CLI"
sidebar_label: "CDX-AG-003"
description: "agnix rule CDX-AG-003 checks for generic agents.md guidance for codex in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-AG-003", "generic agents.md guidance for codex", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-AG-003`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-03`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://developers.openai.com/codex/guides/agents-md

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
