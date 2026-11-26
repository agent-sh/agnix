---
id: cdx-rl-001
title: "CDX-RL-001: Forbidden Rule Without Justification - Codex CLI"
sidebar_label: "CDX-RL-001"
description: "agnix rule CDX-RL-001 checks for forbidden rule without justification in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-RL-001", "forbidden rule without justification", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-RL-001`
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

- https://learn.chatgpt.com/docs/agent-configuration/rules
- https://learn.chatgpt.com/docs/sandboxing

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
prefix_rule(pattern = ["grep"], decision = "forbidden")
```

### Valid

```toml
prefix_rule(pattern = ["grep"], decision = "forbidden", justification = "Use rg instead of grep.")
```
