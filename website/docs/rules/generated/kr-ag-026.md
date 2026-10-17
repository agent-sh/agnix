---
id: kr-ag-026
title: "KR-AG-026: Kiro Agent Hook Timeout Field or Unit Mismatch"
sidebar_label: "KR-AG-026"
description: "agnix rule KR-AG-026 checks for kiro agent hook timeout field or unit mismatch in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-026", "kiro agent hook timeout field or unit mismatch", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-026`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Agents`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/hooks/
- https://kiro.dev/docs/hooks/actions/
- https://kiro.dev/docs/cli/2x-reference/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent", "hooks": {"preToolUse": [{"command": "./audit.sh", "timeout": 5}]}}
```

### Valid

```json
{"name": "review-agent", "hooks": {"preToolUse": [{"command": "./audit.sh", "timeout_ms": 5000}]}}
```
