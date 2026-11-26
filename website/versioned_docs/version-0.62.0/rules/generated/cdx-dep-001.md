---
id: cdx-dep-001
title: "CDX-DEP-001: Deprecated Codex Custom Prompt - Codex CLI"
sidebar_label: "CDX-DEP-001"
description: "agnix rule CDX-DEP-001 checks for deprecated codex custom prompt in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-DEP-001", "deprecated codex custom prompt", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-DEP-001`
- **Severity**: `LOW`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://learn.chatgpt.com/docs/custom-prompts

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
~/.codex/prompts/draftpr.md
```

### Valid

```toml
.agents/skills/draftpr/SKILL.md
```
