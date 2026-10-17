---
id: kr-ag-028
title: "KR-AG-028: Agent File Not Loaded by a Targeted Kiro Engine"
sidebar_label: "KR-AG-028"
description: "agnix rule KR-AG-028 checks for agent file not loaded by a targeted kiro engine in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-028", "agent file not loaded by a targeted kiro engine", "kiro agents", "validation", "agnix", "linter"]
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
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/cli/2x-reference/
- https://kiro.dev/docs/custom-agents/
- https://kiro.dev/docs/cli/v3/agent-config/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
[tool_versions]
kiro = "2.32.1"
# with .kiro/agents/reviewer.md:
---
description: Reviews code
---
Review the diff.
```

### Valid

```json
{"name": "review-agent", "description": "Reviews code"}
```
