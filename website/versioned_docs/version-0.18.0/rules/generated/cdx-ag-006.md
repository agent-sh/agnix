---
id: cdx-ag-006
title: "CDX-AG-006: AGENTS.md Missing Project Context - Codex CLI"
sidebar_label: "CDX-AG-006"
description: "agnix rule CDX-AG-006 checks for agents.md missing project context in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-AG-006", "agents.md missing project context", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-AG-006`
- **Severity**: `LOW`
- **Category**: `Codex CLI`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-06`

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
# AGENTS.md

Follow best practices.
```

### Valid

```toml
# AGENTS.md

This is a Rust project using cargo.

## Commands

`cargo test`
```
