---
id: kr-ag-012
title: "KR-AG-012: toolAliases References Unknown Tool - Kiro Agents"
sidebar_label: "KR-AG-012"
description: "agnix rule KR-AG-012 checks for toolaliases references unknown tool in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-012", "toolaliases references unknown tool", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-012`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Agents`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-06`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/agents
- https://kiro.dev/docs/configuration

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"tools": ["readFiles"], "toolAliases": {"wf": "writeFiles"}}
```

### Valid

```json
{"tools": ["readFiles"], "toolAliases": {"rf": "readFiles"}}
```
