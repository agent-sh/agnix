---
id: gm-ag-001
title: "GM-AG-001: Invalid auth block in Gemini agent MCP server"
sidebar_label: "GM-AG-001"
description: "agnix rule GM-AG-001 checks for invalid auth block in gemini agent mcp server in gemini agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["GM-AG-001", "invalid auth block in gemini agent mcp server", "gemini agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `GM-AG-001`
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
- https://github.com/google-gemini/gemini-cli/pull/24770
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
description: Queries Spanner through its MCP server
mcp_servers:
  spanner:
    auth:
      type: basic-auth
---

You are a Spanner agent.
```

### Valid

```text
---
name: spanner-agent
description: Queries Spanner through its MCP server
mcp_servers:
  spanner:
    url: https://spanner.googleapis.com/mcp
    type: http
    auth:
      type: google-credentials
      scopes:
        - https://www.googleapis.com/auth/cloud-platform
---

You are a Spanner agent.
```
