---
id: gm-ag-003
title: "GM-AG-003: Unknown Gemini Agent Frontmatter Key"
sidebar_label: "GM-AG-003"
description: "agnix rule GM-AG-003 checks for unknown gemini agent frontmatter key in gemini agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["GM-AG-003", "unknown gemini agent frontmatter key", "gemini agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `GM-AG-003`
- **Severity**: `HIGH`
- **Category**: `Gemini Agents`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `gemini-cli`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://raw.githubusercontent.com/google-gemini/gemini-cli/main/packages/core/src/agents/agentLoader.ts
- https://geminicli.com/docs/core/subagents

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
---
name: spanner-agent
description: Queries Spanner
tools: [read_file]
system_prompt: You are a Spanner agent.
---
```

### Valid

```text
---
name: spanner-agent
description: Queries Spanner
tools: [read_file]
---

You are a Spanner agent.
```
