---
id: cdx-rl-002
title: "CDX-RL-002: Broad Allow Rule Outside the Sandbox - Codex CLI"
sidebar_label: "CDX-RL-002"
description: "agnix rule CDX-RL-002 checks for broad allow rule outside the sandbox in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-RL-002", "broad allow rule outside the sandbox", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-RL-002`
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
prefix_rule(pattern = ["bash"])
```

### Valid

```toml
prefix_rule(pattern = ["gh", "pr", "view"], decision = "prompt")
```
