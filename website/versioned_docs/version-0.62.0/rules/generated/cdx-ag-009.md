---
id: cdx-ag-009
title: "CDX-AG-009: AGENTS.md Exceeds Codex project_doc_max_bytes"
sidebar_label: "CDX-AG-009"
description: "agnix rule CDX-AG-009 checks for agents.md exceeds codex project_doc_max_bytes in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-AG-009", "agents.md exceeds codex project_doc_max_bytes", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-AG-009`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://learn.chatgpt.com/docs/agent-configuration/agents-md
- https://learn.chatgpt.com/docs/config-file/config-advanced

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
one AGENTS.md of 40 KiB
```

### Valid

```toml
AGENTS.md under 32 KiB, details in nested AGENTS.md files
```
