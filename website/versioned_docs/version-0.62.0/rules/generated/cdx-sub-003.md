---
id: cdx-sub-003
title: "CDX-SUB-003: Custom Agent Replaces a Built-in Agent"
sidebar_label: "CDX-SUB-003"
description: "agnix rule CDX-SUB-003 checks for custom agent replaces a built-in agent in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-SUB-003", "custom agent replaces a built-in agent", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-SUB-003`
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

- https://learn.chatgpt.com/docs/agent-configuration/subagents

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
name = "explorer"
```

### Valid

```toml
name = "pr_explorer"
```
