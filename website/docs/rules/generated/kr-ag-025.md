---
id: kr-ag-025
title: "KR-AG-025: Agent Hook Matcher Never Fires in Kiro CLI 2.x"
sidebar_label: "KR-AG-025"
description: "agnix rule KR-AG-025 checks for agent hook matcher never fires in kiro cli 2.x in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-025", "agent hook matcher never fires in kiro cli 2.x", "kiro agents", "validation", "agnix", "linter"]
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

- https://kiro.dev/docs/cli/2x-reference/
- https://kiro.dev/docs/cli/v3/hooks-migration/
- https://kiro.dev/docs/reference/built-in-tools/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent", "hooks": {"preToolUse": [{"matcher": "read_file", "command": "./audit.sh"}]}}
```

### Valid

```json
{"name": "review-agent", "hooks": {"preToolUse": [{"matcher": "fs_read", "command": "./audit.sh"}]}}
```
