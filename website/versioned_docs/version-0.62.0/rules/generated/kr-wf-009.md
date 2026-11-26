---
id: kr-wf-009
title: "KR-WF-009: Workflow Loop Stop File Never Written"
sidebar_label: "KR-WF-009"
description: "agnix rule KR-WF-009 checks for workflow loop stop file never written in kiro workflows files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-WF-009", "workflow loop stop file never written", "kiro workflows", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-WF-009`
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
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"type": "repeat", "id": "loop", "maxIterations": 5, "onMaxIterations": "pause", "stopCondition": {"fileCheck": {"path": "status.json", "jsonPath": "done", "value": true}}, "steps": [{"type": "step", "id": "work", "agent": "wf-coder", "prompt": "Do the next item"}]}
```

### Valid

```json
{"type": "repeat", "id": "loop", "maxIterations": 5, "onMaxIterations": "pause", "stopCondition": {"fileCheck": {"path": "status.json", "jsonPath": "done", "value": true}}, "steps": [{"type": "step", "id": "work", "agent": "wf-coder", "prompt": "Do the next item and update status.json"}]}
```
