---
id: kr-ag-007
title: "KR-AG-007: Kiro Agent Tool Scope Broader Than Referenced Subagent"
sidebar_label: "KR-AG-007"
description: "agnix rule KR-AG-007 checks for kiro agent tool scope broader than referenced subagent in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-007", "kiro agent tool scope broader than referenced subagent", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-007`
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

- https://github.com/kirodotdev/kiro/issues/5071
- https://github.com/kirodotdev/kiro/issues/5449

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
  "allowedTools": ["readFiles", "runShellCommand"],
  "prompt": "Delegate review to @reviewer-agent"
}
```

### Valid

```json
{
  "name": "orchestrator",
  "allowedTools": ["readFiles"],
  "prompt": "Delegate review to @reviewer-agent"
}
```
