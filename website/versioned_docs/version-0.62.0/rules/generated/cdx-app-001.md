---
id: cdx-app-001
title: "CDX-APP-001: Invalid Tool Approval Mode - Codex CLI"
sidebar_label: "CDX-APP-001"
description: "agnix rule CDX-APP-001 checks for invalid tool approval mode in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-APP-001", "invalid tool approval mode", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-APP-001`
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

- https://learn.chatgpt.com/docs/extend/mcp
- https://learn.chatgpt.com/docs/config-file/config-reference
- https://learn.chatgpt.com/docs/config-schema.json

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[apps.github]
default_tools_approval_mode = "manual"
```

### Valid

```toml
[mcp_servers.docs]
url = "https://example.com/mcp"
default_tools_approval_mode = "writes"
```
