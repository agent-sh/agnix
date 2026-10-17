---
id: kr-wf-003
title: "KR-WF-003: Workflow Recipe Suffix Conflict - Kiro Workflows"
sidebar_label: "KR-WF-003"
description: "agnix rule KR-WF-003 checks for workflow recipe suffix conflict in kiro workflows files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-WF-003", "workflow recipe suffix conflict", "kiro workflows", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-WF-003`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Workflows`
- **Normative Level**: `SHOULD`
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
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
.kiro/workflows/plan.workflow.json
.kiro/workflows/plan.workflow.yaml
```

### Valid

```text
.kiro/workflows/plan.workflow.json
```
