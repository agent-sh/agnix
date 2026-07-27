---
id: cc-pl-011
title: "CC-PL-011: LSP Server Missing Required Fields"
sidebar_label: "CC-PL-011"
description: "agnix rule CC-PL-011 checks for lsp server missing required fields in claude plugins files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-PL-011", "lsp server missing required fields", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-011`
- **Severity**: `HIGH`
- **Category**: `Claude Plugins`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-28`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/plugins-reference

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "lsp": { "command": "node" }
}
```

### Valid

```json
{
  "lsp": { "command": "node", "args": ["server.js"], "languages": ["typescript"] }
}
```
