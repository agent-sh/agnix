---
id: cdx-ag-002
title: "CDX-AG-002: Secrets in AGENTS.md for Codex - Codex CLI"
sidebar_label: "CDX-AG-002"
description: "agnix rule CDX-AG-002 checks for secrets in agents.md for codex in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-AG-002", "secrets in agents.md for codex", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-AG-002`
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
OPENAI_API_KEY=sk-live-super-secret-value
```

### Valid

```toml
Use `${OPENAI_API_KEY}` from the environment.
```
