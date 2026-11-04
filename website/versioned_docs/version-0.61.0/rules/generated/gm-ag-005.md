---
id: gm-ag-005
title: "GM-AG-005: Literal Secret in Gemini Remote Agent Auth"
sidebar_label: "GM-AG-005"
description: "agnix rule GM-AG-005 checks for literal secret in gemini remote agent auth in gemini agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["GM-AG-005", "literal secret in gemini remote agent auth", "gemini agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `GM-AG-005`
- **Severity**: `MEDIUM`
- **Category**: `Gemini Agents`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `gemini-cli`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://geminicli.com/docs/core/remote-agents
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
kind: remote
name: my-agent
agent_card_url: https://example.com/agent-card
auth:
  type: apiKey
  key: sk-live-4f9a8b7c6d5e4f3a
---
```

### Valid

```text
---
kind: remote
name: my-agent
agent_card_url: https://example.com/agent-card
auth:
  type: apiKey
  key: $MY_API_KEY
---
```
