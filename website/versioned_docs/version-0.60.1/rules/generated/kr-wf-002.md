---
id: kr-wf-002
title: "KR-WF-002: Invalid Workflow Repeat Cap - Kiro Workflows"
sidebar_label: "KR-WF-002"
description: "agnix rule KR-WF-002 checks for invalid workflow repeat cap in kiro workflows files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-WF-002", "invalid workflow repeat cap", "kiro workflows", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-WF-002`
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
{"type": "repeat", "id": "loop", "steps": [{"type": "step", "id": "work", "agent": "wf-coder", "prompt": "Next item"}]}
```

### Valid

```json
{"type": "repeat", "id": "loop", "maxIterations": 10, "onMaxIterations": "pause", "steps": [{"type": "step", "id": "work", "agent": "wf-coder", "prompt": "Next item"}]}
```
