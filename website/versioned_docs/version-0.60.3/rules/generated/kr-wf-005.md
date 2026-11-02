---
id: kr-wf-005
title: "KR-WF-005: Interactive Workflow Step Inside Parallel"
sidebar_label: "KR-WF-005"
description: "agnix rule KR-WF-005 checks for interactive workflow step inside parallel in kiro workflows files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-WF-005", "interactive workflow step inside parallel", "kiro workflows", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-WF-005`
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
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"type": "parallel", "id": "p", "joinPolicy": "all", "branches": [{"type": "step", "id": "chat", "agent": "wf-coder", "prompt": "Ask me", "completion": {"completionSignal": "success"}}]}
```

### Valid

```json
{"type": "step", "id": "chat", "agent": "wf-coder", "prompt": "Ask me", "completion": {"completionSignal": "success"}}
```
