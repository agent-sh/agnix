---
id: oc-cfg-015
title: "OC-CFG-015: Shell-Style Variable Not Expanded - OpenCode"
sidebar_label: "OC-CFG-015"
description: "agnix rule OC-CFG-015 checks for shell-style variable not expanded in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-CFG-015", "shell-style variable not expanded", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-015`
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

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"mcp": {"x": {"type": "remote", "url": "https://x.example/mcp", "headers": {"X-Key": "${MY_API_KEY}"}}}}
```

### Valid

```json
{"mcp": {"x": {"type": "remote", "url": "https://x.example/mcp", "headers": {"X-Key": "{env:MY_API_KEY}"}}}}
```
