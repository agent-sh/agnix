---
id: cdx-ag-010
title: "CDX-AG-010: Codex Instruction Chain Exceeds project_doc_max_bytes"
sidebar_label: "CDX-AG-010"
description: "agnix rule CDX-AG-010 checks for codex instruction chain exceeds project_doc_max_bytes in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-AG-010", "codex instruction chain exceeds project_doc_max_bytes", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-AG-010`
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
a 20 KiB root AGENTS.md and a 15 KiB nested AGENTS.md on one path
```

### Valid

```toml
root and nested AGENTS.md together under 32 KiB
```
