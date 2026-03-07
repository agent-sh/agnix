---
id: cdx-ag-007
title: "CDX-AG-007: AGENTS.md Contradicts config.toml - Codex CLI"
sidebar_label: "CDX-AG-007"
description: "agnix rule CDX-AG-007 checks for agents.md contradicts config.toml in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-AG-007", "agents.md contradicts config.toml", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-AG-007`
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
Use suggest mode.

approvalMode = "full-auto"
```

### Valid

```toml
Use full-auto mode for CI.

approvalMode = "full-auto"
```
