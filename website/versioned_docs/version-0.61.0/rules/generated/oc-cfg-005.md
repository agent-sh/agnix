---
id: oc-cfg-005
title: "OC-CFG-005: Hardcoded Secret in Config - OpenCode"
sidebar_label: "OC-CFG-005"
description: "agnix rule OC-CFG-005 checks for hardcoded secret in config in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-CFG-005", "hardcoded secret in config", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-005`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/v2/docs/mcp-servers/
- https://opencode.ai/docs/config/
- https://opencode.ai/v2/docs/config/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"provider": {"acme": {"options": {"apiKey": "sk-live-1234567890abcdef"}}}}
```

### Valid

```json
{"mcp": {"x": {"type": "remote", "url": "https://x.example/mcp", "headers": {"Authorization": "Bearer {env:MY_API_KEY}"}}}}
```
