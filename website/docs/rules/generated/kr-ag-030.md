---
id: kr-ag-030
title: "KR-AG-030: Sub-agent Description Missing or Too Short"
sidebar_label: "KR-AG-030"
description: "agnix rule KR-AG-030 checks for sub-agent description missing or too short in kiro agents files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-AG-030", "sub-agent description missing or too short", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-030`
- **Severity**: `LOW`
- **Category**: `Kiro Agents`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/custom-agents/subagents/
- https://kiro.dev/docs/custom-agents/configuration-reference/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent", "description": "Reviewer"}
```

### Valid

```json
{"name": "review-agent", "description": "Reviews Rust pull requests for safety and style issues"}
```
