---
id: cdx-ag-005
title: "CDX-AG-005: AGENTS.md References Missing File - Codex CLI"
sidebar_label: "CDX-AG-005"
description: "agnix rule CDX-AG-005 checks for agents.md references missing file in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-AG-005", "agents.md references missing file", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-AG-005`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
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
See `docs/nonexistent.md` for details.
```

### Valid

```toml
See `docs/setup.md` for details.
```
