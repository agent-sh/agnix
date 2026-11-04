---
id: kr-set-006
title: "KR-SET-006: Config Section Misplaced in Kiro CLI Settings"
sidebar_label: "KR-SET-006"
description: "agnix rule KR-SET-006 checks for config section misplaced in kiro cli settings in kiro settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-SET-006", "config section misplaced in kiro cli settings", "kiro settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-SET-006`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Settings`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/reference/settings/
- https://kiro.dev/docs/mcp/configuration/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"chat.defaultAgent": "my-agent", "mcpServers": {"git": {"command": "uvx"}}}
```

### Valid

```json
{"chat.defaultAgent": "my-agent"}
```
