---
id: gm-ag-004
title: "GM-AG-004: Gemini Agent Numeric Field Out of Range"
sidebar_label: "GM-AG-004"
description: "agnix rule GM-AG-004 checks for gemini agent numeric field out of range in gemini agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["GM-AG-004", "gemini agent numeric field out of range", "gemini agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `GM-AG-004`
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

- https://geminicli.com/docs/core/subagents
- https://raw.githubusercontent.com/google-gemini/gemini-cli/main/packages/core/src/agents/agentLoader.ts

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
---
name: a
description: d
tools: [read_file]
temperature: 3
max_turns: 0
---
```

### Valid

```text
---
name: a
description: d
tools: [read_file]
temperature: 0.2
max_turns: 10
timeout_mins: 5
---
```
