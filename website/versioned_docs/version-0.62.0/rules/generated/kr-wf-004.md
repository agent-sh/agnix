---
id: kr-wf-004
title: "KR-WF-004: Invalid Workflow Recipe Document - Kiro Workflows"
sidebar_label: "KR-WF-004"
description: "agnix rule KR-WF-004 checks for invalid workflow recipe document in kiro workflows files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-WF-004", "invalid workflow recipe document", "kiro workflows", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-WF-004`
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
{"name": "x", "steps": [{"type": "step", "id": "plan", "agent": "wf-planner"}]}
```

### Valid

```json
{"name": "plan-and-implement", "inputs": {"task": "prompt"}, "steps": [{"type": "step", "id": "plan", "agent": "wf-planner", "prompt": "Plan {{task}}"}, {"type": "step", "id": "implement", "agent": "wf-coder", "prompt": "Implement: {{previous.output}}"}]}
```
