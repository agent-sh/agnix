---
id: kr-wf-008
title: "KR-WF-008: Invalid Workflow Watch Configuration"
sidebar_label: "KR-WF-008"
description: "agnix rule KR-WF-008 checks for invalid workflow watch configuration in kiro workflows files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-WF-008", "invalid workflow watch configuration", "kiro workflows", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-WF-008`
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
{"type": "watch", "id": "review", "handler": "command", "config": {"command": "node", "args": ["watch-review.mjs"], "pollIntervalSec": 5}}
```

### Valid

```json
{"type": "watch", "id": "review", "handler": "command", "config": {"command": "node watch-review.mjs", "pollIntervalSec": 10}}
```
