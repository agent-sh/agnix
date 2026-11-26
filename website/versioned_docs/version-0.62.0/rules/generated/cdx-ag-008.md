---
id: cdx-ag-008
title: "CDX-AG-008: AGENTS.md Shadowed by AGENTS.override.md"
sidebar_label: "CDX-AG-008"
description: "agnix rule CDX-AG-008 checks for agents.md shadowed by agents.override.md in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-AG-008", "agents.md shadowed by agents.override.md", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-AG-008`
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

- https://learn.chatgpt.com/docs/agent-configuration/agents-md

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
services/payments/AGENTS.md next to services/payments/AGENTS.override.md
```

### Valid

```toml
services/payments/AGENTS.override.md (no AGENTS.md next to it)
```
