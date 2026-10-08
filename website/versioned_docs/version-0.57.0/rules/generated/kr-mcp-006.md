---
id: kr-mcp-006
title: "KR-MCP-006: Invalid OAuth Configuration - Kiro MCP"
sidebar_label: "KR-MCP-006"
description: "agnix rule KR-MCP-006 checks for invalid oauth configuration in kiro mcp files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-MCP-006", "invalid oauth configuration", "kiro mcp", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-MCP-006`
- **Severity**: `MEDIUM`
- **Category**: `Kiro MCP`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-11`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.3.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/cli/mcp/configuration/#oauth-configuration
- https://kiro.dev/changelog/cli/2-12/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"mcpServers": {"remote": {"url": "https://example.com/mcp", "oauth": {"clientSecret": "secret-without-client-id", "redirectUri": "https://example.com/callback"}}}}
```

### Valid

```json
{"mcpServers": {"remote": {"url": "https://example.com/mcp", "oauth": {"clientId": "registered-client", "clientSecret": "registered-secret", "redirectUri": "http://localhost:7778/oauth/callback", "oauthScopes": ["read"]}}}}
```
