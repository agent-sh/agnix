---
id: cdx-sub-001
title: "CDX-SUB-001: Invalid Codex Custom Agent File - Codex CLI"
sidebar_label: "CDX-SUB-001"
description: "agnix rule CDX-SUB-001 checks for invalid codex custom agent file in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-SUB-001", "invalid codex custom agent file", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-SUB-001`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://learn.chatgpt.com/docs/agent-configuration/subagents
- https://github.com/openai/codex/blob/main/codex-rs/agent-roles/src/agent_role_config.rs

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
name = "reviewer"
model = "gpt-5"
```

### Valid

```toml
name = "reviewer"
description = "PR reviewer focused on correctness."
developer_instructions = "Review code like an owner."
```
