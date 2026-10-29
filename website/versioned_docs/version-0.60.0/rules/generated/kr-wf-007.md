---
id: kr-wf-007
title: "KR-WF-007: Invalid Workflow Template Reference"
sidebar_label: "KR-WF-007"
description: "agnix rule KR-WF-007 checks for invalid workflow template reference in kiro workflows files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-WF-007", "invalid workflow template reference", "kiro workflows", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-WF-007`
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
{"name": "x", "steps": [{"type": "step", "id": "a", "agent": "wf-coder", "prompt": "{{b.output}}"}, {"type": "step", "id": "b", "agent": "wf-coder", "prompt": "x"}]}
```

### Valid

```json
{"name": "plan-and-implement", "inputs": {"task": "prompt"}, "steps": [{"type": "step", "id": "plan", "agent": "wf-planner", "prompt": "Plan {{task}}"}, {"type": "step", "id": "implement", "agent": "wf-coder", "prompt": "Implement: {{previous.output}}"}]}
```
