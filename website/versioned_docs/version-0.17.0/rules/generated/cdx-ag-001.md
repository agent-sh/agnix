---
id: cdx-ag-001
title: "CDX-AG-001: Empty AGENTS.md for Codex - Codex CLI"
sidebar_label: "CDX-AG-001"
description: "agnix rule CDX-AG-001 checks for empty agents.md for codex in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-AG-001", "empty agents.md for codex", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-AG-001`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
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

```

### Valid

```toml
# AGENTS.md

Use `cargo test` before opening PRs.
```
