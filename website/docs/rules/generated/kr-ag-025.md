---
id: kr-ag-025
title: "KR-AG-025: Kiro Agent Hook Matcher Never Fires - Kiro Agents"
sidebar_label: "KR-AG-025"
description: "agnix rule KR-AG-025 checks for kiro agent hook matcher never fires in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-025", "kiro agent hook matcher never fires", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-025`
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

- https://kiro.dev/docs/hooks/types/
- https://kiro.dev/docs/cli/v3/hooks-migration/
- https://kiro.dev/docs/cli/v3/upgrade-agent/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent", "hooks": [{"name": "audit", "trigger": "PreToolUse", "matcher": "fs_read", "action": {"type": "command", "command": "./audit.sh"}}]}
```

### Valid

```json
{"name": "review-agent", "hooks": [{"name": "audit", "trigger": "PreToolUse", "matcher": "read", "action": {"type": "command", "command": "./audit.sh"}}]}
```
