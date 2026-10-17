---
id: kr-hk-005
title: "KR-HK-005: Invalid Kiro Agent Hook Trigger - Kiro Hooks"
sidebar_label: "KR-HK-005"
description: "agnix rule KR-HK-005 checks for invalid kiro agent hook trigger in kiro hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-HK-005", "invalid kiro agent hook trigger", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-005`
- **Severity**: `HIGH`
- **Category**: `Kiro Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/cli/2x-reference/
- https://kiro.dev/docs/hooks/types/
- https://kiro.dev/docs/custom-agents/configuration-reference/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent", "hooks": {"PreToolUse": [{"command": "echo pre"}]}}
```

### Valid

```json
{"name": "review-agent", "hooks": {"preToolUse": [{"command": "echo pre"}]}}
```
