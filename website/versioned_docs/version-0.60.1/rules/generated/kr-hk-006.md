---
id: kr-hk-006
title: "KR-HK-006: Invalid Kiro Agent Hook Entry - Kiro Hooks"
sidebar_label: "KR-HK-006"
description: "agnix rule KR-HK-006 checks for invalid kiro agent hook entry in kiro hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-HK-006", "invalid kiro agent hook entry", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-006`
- **Severity**: `HIGH`
- **Category**: `Kiro Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/cli/2x-reference/
- https://kiro.dev/docs/hooks/
- https://kiro.dev/docs/custom-agents/configuration-reference/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent", "hooks": [{"name": "lint", "trigger": "PostToolUse", "action": {"type": "command"}}]}
```

### Valid

```json
{"name": "review-agent", "hooks": [{"name": "lint", "trigger": "PostToolUse", "matcher": "write", "action": {"type": "command", "command": "npm run lint"}}]}
```
