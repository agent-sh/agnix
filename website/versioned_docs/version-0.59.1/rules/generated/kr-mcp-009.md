---
id: kr-mcp-009
title: "KR-MCP-009: Unexpanded Variable Reference in Kiro MCP Config"
sidebar_label: "KR-MCP-009"
description: "agnix rule KR-MCP-009 checks for unexpanded variable reference in kiro mcp config in kiro mcp files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-MCP-009", "unexpanded variable reference in kiro mcp config", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-009`
- **Severity**: `MEDIUM`
- **Category**: `Kiro MCP`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/mcp/configuration/
- https://kiro.dev/docs/custom-agents/configuration-reference/
- https://kiro.dev/changelog/cli/2-4/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"mcpServers": {"s": {"command": "uvx", "args": ["mcp-atlassian"], "env": {"JIRA_API_TOKEN": "$JIRA_API_TOKEN"}}}}
```

### Valid

```json
{"mcpServers": {"s": {"command": "uvx", "args": ["mcp-atlassian"], "env": {"JIRA_API_TOKEN": "${JIRA_API_TOKEN}"}}}}
```
