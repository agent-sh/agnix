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
- **Verified On**: `2026-04-26`

## Applicability

- **Tool**: `gemini-cli`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/google-gemini/gemini-cli/pull/24770

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
---
kind: local
name: spanner-agent
mcp_servers:
  spanner:
    auth:
      type: basic-auth
system_prompt: Example
---
```

### Valid

```text
---
kind: local
name: spanner-agent
mcp_servers:
  spanner:
    url: https://spanner.googleapis.com/mcp
    type: http
    auth:
      type: google-credentials
      scopes:
        - https://www.googleapis.com/auth/cloud-platform
system_prompt: Example
---
```
