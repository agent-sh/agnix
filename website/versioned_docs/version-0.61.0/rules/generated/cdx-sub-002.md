---
id: cdx-sub-002
title: "CDX-SUB-002: Session Setting in Codex Custom Agent File"
sidebar_label: "CDX-SUB-002"
description: "agnix rule CDX-SUB-002 checks for session setting in codex custom agent file in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-SUB-002", "session setting in codex custom agent file", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-SUB-002`
- **Severity**: `MEDIUM`
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
description = "d"
developer_instructions = "i"
sandbox_mode = "read-only"
```

### Valid

```toml
name = "reviewer"
description = "d"
developer_instructions = "Do not edit files."
```
