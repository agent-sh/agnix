---
id: kr-ag-023
title: "KR-AG-023: Invalid Inline MCP Server in Kiro Agent"
sidebar_label: "KR-AG-023"
description: "agnix rule KR-AG-023 checks for invalid inline mcp server in kiro agent in kiro agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-AG-023", "invalid inline mcp server in kiro agent", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-023`
- **Severity**: `HIGH`
- **Category**: `Kiro Agents`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/custom-agents/configuration-reference/
- https://kiro.dev/docs/mcp/configuration/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent", "mcpServers": {"git": {"command": "git-mcp", "args": "--repo ."}}}
```

### Valid

```json
{"name": "review-agent", "mcpServers": {"git": {"command": "git-mcp", "args": ["--repo", "."]}}}
```
