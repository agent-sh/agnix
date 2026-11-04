---
id: kr-mcp-015
title: "KR-MCP-015: Kiro MCP autoApprove Wildcard - Kiro MCP"
sidebar_label: "KR-MCP-015"
description: "agnix rule KR-MCP-015 checks for kiro mcp autoapprove wildcard in kiro mcp files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-MCP-015", "kiro mcp autoapprove wildcard", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-015`
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

- https://kiro.dev/docs/mcp/security/
- https://kiro.dev/docs/mcp/configuration/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"mcpServers": {"s": {"command": "uvx", "args": ["aws-docs-mcp"], "autoApprove": ["*"]}}}
```

### Valid

```json
{"mcpServers": {"s": {"command": "uvx", "args": ["aws-docs-mcp"], "autoApprove": ["search_documentation"]}}}
```
