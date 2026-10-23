---
id: kr-wf-006
title: "KR-WF-006: Invalid Workflow Stop Condition - Kiro Workflows"
sidebar_label: "KR-WF-006"
description: "agnix rule KR-WF-006 checks for invalid workflow stop condition in kiro workflows files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-WF-006", "invalid workflow stop condition", "kiro workflows", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-WF-006`
- **Severity**: `HIGH`
- **Category**: `Kiro Workflows`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.26.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/workflows/authoring/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `true`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"stopCondition": {"fileCheck": {"path": "status.json", "jsonPath": "done", "equals": true}}}
```

### Valid

```json
{"stopWhen": "wait-for-pr.terminal"}
```
