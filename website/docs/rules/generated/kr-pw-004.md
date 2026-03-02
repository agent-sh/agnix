---
id: kr-pw-004
title: "KR-PW-004: Invalid Adjacent Power mcp.json Structure"
sidebar_label: "KR-PW-004"
description: "agnix rule KR-PW-004 checks for invalid adjacent power mcp.json structure in kiro powers files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-PW-004", "invalid adjacent power mcp.json structure", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-004`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Powers`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-02`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/powers/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "mcpServers": []
}
```

### Valid

```json
{
  "mcpServers": {
    "local": {"command": "node", "args": ["server.js"]}
  }
}
```
