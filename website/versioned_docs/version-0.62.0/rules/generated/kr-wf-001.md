---
id: kr-wf-001
title: "KR-WF-001: Workflow Recipe Exceeds Kiro Limits"
sidebar_label: "KR-WF-001"
description: "agnix rule KR-WF-001 checks for workflow recipe exceeds kiro limits in kiro workflows files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-WF-001", "workflow recipe exceeds kiro limits", "kiro workflows", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-WF-001`
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
{"name": "dup", "steps": [{"type": "step", "id": "a", "agent": "wf-coder", "prompt": "x"}, {"type": "step", "id": "a", "agent": "wf-coder", "prompt": "y"}]}
```

### Valid

```json
{"name": "plan-and-implement", "inputs": {"task": "prompt"}, "steps": [{"type": "step", "id": "plan", "agent": "wf-planner", "prompt": "Plan {{task}}"}, {"type": "step", "id": "implement", "agent": "wf-coder", "prompt": "Implement: {{previous.output}}"}]}
```
