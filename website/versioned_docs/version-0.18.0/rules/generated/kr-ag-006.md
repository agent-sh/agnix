---
id: kr-ag-006
title: "KR-AG-006: Kiro Agent References Unknown Subagent"
sidebar_label: "KR-AG-006"
description: "agnix rule KR-AG-006 checks for kiro agent references unknown subagent in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-006", "kiro agent references unknown subagent", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-006`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Agents`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-02`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/kirodotdev/kiro/issues/5743
- https://github.com/kirodotdev/kiro/issues/4262

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "name": "orchestrator",
  "prompt": "Delegate review to @missing-agent"
}
```

### Valid

```json
{
  "name": "orchestrator",
  "prompt": "Delegate review to @reviewer-agent"
}
```
