---
id: cdx-pl-019
title: "CDX-PL-019: Secret in Plugin MCP Configuration - Codex CLI"
sidebar_label: "CDX-PL-019"
description: "agnix rule CDX-PL-019 checks for secret in plugin mcp configuration in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-PL-019", "secret in plugin mcp configuration", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-019`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://developers.openai.com/plugins/build/plugins
- https://developers.openai.com/plugins/deploy/submission

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"mcpServers": {"docs": {"url": "https://example.com/mcp", "headers": {"Authorization": "Bearer sk-live-1234567890abcdef"}}}}
```

### Valid

```json
{"mcpServers": {"docs": {"url": "https://example.com/mcp", "extensions": {"com.openai": {"auth": {"type": "api_key", "headerScheme": "bearer"}}}}}}
```
