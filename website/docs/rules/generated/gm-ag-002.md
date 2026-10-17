---
id: gm-ag-002
title: "GM-AG-002: Invalid Gemini Agent Frontmatter - Gemini Agents"
sidebar_label: "GM-AG-002"
description: "agnix rule GM-AG-002 checks for invalid gemini agent frontmatter in gemini agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["GM-AG-002", "invalid gemini agent frontmatter", "gemini agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `GM-AG-002`
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
name: Security Auditor
tools:
  - read_file
---

You are a ruthless Security Auditor.
```

### Valid

```text
---
name: security-auditor
description: Specialized in finding security vulnerabilities in code.
tools:
  - read_file
  - grep_search
---

You are a ruthless Security Auditor.
```
