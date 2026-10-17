---
id: kr-ag-028
title: "KR-AG-028: Agent File Not Loaded by Kiro CLI 2.x"
sidebar_label: "KR-AG-028"
description: "agnix rule KR-AG-028 checks for agent file not loaded by kiro cli 2.x in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-028", "agent file not loaded by kiro cli 2.x", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-028`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Agents`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `<3.0.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/cli/2x-reference/
- https://kiro.dev/docs/custom-agents/
- https://kiro.dev/docs/cli/v3/agent-config/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
---
description: Reviews code
---
Review the diff.
```

### Valid

```json
{"name": "review-agent", "description": "Reviews code"}
```
